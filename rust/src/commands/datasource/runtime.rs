//! Datasource command runtime and lifecycle orchestration.
//!
//! Purpose:
//! - Normalize datasource commands before execution.
//! - Handle early exits and command-shape validation.
//! - Materialize auth once before dispatching to command handlers.

use serde_json::Value;

use crate::common::{message, print_supported_columns, render_json_value, string_field, Result};
use crate::dashboard::{
    build_api_client, build_auth_context, build_http_client, build_http_client_for_org_from_api,
    materialize_dashboard_common_auth, SimpleOutputFormat,
};
use crate::datasource_catalog::{
    render_supported_datasource_catalog_csv, render_supported_datasource_catalog_json,
    render_supported_datasource_catalog_table, render_supported_datasource_catalog_text,
    render_supported_datasource_catalog_yaml,
};
use crate::grafana_api::DatasourceResourceClient;
use crate::tabular_output::render_yaml;

use super::{
    build_add_payload, build_all_orgs_export_index, build_all_orgs_export_metadata,
    build_all_orgs_output_dir, build_datasource_export_metadata, build_datasource_plan,
    build_datasource_provisioning_document, build_export_index, build_export_records,
    build_list_records, build_modify_payload, build_modify_updates, datasource_list_column_ids,
    datasource_plan_column_ids, diff_datasources_with_live, discover_export_org_import_scopes,
    import_datasources_by_export_org, import_datasources_with_client, load_import_records,
    print_datasource_plan_report, render_data_source_csv, render_data_source_json,
    render_data_source_table, render_live_mutation_json, render_live_mutation_table,
    resolve_datasource_export_root_dir, resolve_delete_match, resolve_export_org_target_plan,
    resolve_live_mutation_match, resolve_target_client, validate_import_org_auth,
    validate_live_mutation_dry_run_args, write_json_file, write_yaml_file, DatasourceGroupCommand,
    DatasourceImportArgs, DatasourcePlanInput, DatasourcePlanOrgInput, DATASOURCE_EXPORT_FILENAME,
    DATASOURCE_PROVISIONING_FILENAME, DATASOURCE_PROVISIONING_SUBDIR, EXPORT_METADATA_FILENAME,
};

const DATASOURCE_IMPORT_LIST_COLUMNS: &[&str] = &[
    "uid",
    "name",
    "type",
    "match_basis",
    "destination",
    "action",
    "org_id",
    "file",
    "target_uid",
    "target_version",
    "target_read_only",
    "blocked_reason",
];

// Datasource runtime boundary:
// normalize shared flags, validate, materialize auth, then dispatch by command kind.
pub fn run_datasource_cli(command: DatasourceGroupCommand) -> Result<()> {
    // Runtime boundary for datasource commands:
    // normalize legacy flags once, apply command-only exits, then materialize auth and execute.
    let command = super::normalize_datasource_group_command(command);
    if handle_datasource_command_early_exits(&command)? {
        return Ok(());
    }
    validate_datasource_command_inputs(&command)?;
    let command = materialize_datasource_command_auth(command)?;
    execute_datasource_command(command)
}

fn handle_datasource_command_early_exits(command: &DatasourceGroupCommand) -> Result<bool> {
    match command {
        DatasourceGroupCommand::List(args) if args.list_columns => {
            print_supported_columns(datasource_list_column_ids());
            Ok(true)
        }
        DatasourceGroupCommand::Import(args) if args.list_columns => {
            print_supported_columns(DATASOURCE_IMPORT_LIST_COLUMNS);
            Ok(true)
        }
        DatasourceGroupCommand::Plan(args) if args.list_columns => {
            print_supported_columns(datasource_plan_column_ids());
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn validate_datasource_command_inputs(command: &DatasourceGroupCommand) -> Result<()> {
    match command {
        DatasourceGroupCommand::Import(args) => {
            if !args.output_columns.is_empty() && !args.table {
                return Err(message(
                    "--output-columns is only supported with --dry-run --table or table-like --output-format for datasource import.",
                ));
            }
        }
        DatasourceGroupCommand::Plan(args) => {
            if !args.output_columns.is_empty()
                && args.output_format != super::DatasourcePlanOutputFormat::Table
            {
                return Err(message(
                    "--output-columns is only supported with --output-format table for datasource plan.",
                ));
            }
            if args.no_header && args.output_format != super::DatasourcePlanOutputFormat::Table {
                return Err(message(
                    "--no-header is only supported with --output-format table for datasource plan.",
                ));
            }
            if args.use_export_org
                && args.input_format != super::DatasourceImportInputFormat::Inventory
            {
                return Err(message(
                    "Datasource plan with --use-export-org requires --input-format inventory.",
                ));
            }
        }
        _ => {}
    }
    Ok(())
}

// Execute one normalized datasource command against local artifacts or live Grafana.
fn execute_datasource_command(command: DatasourceGroupCommand) -> Result<()> {
    match command {
        DatasourceGroupCommand::Types(args) => {
            match args.output_format {
                SimpleOutputFormat::Text => {
                    for line in render_supported_datasource_catalog_text() {
                        println!("{line}");
                    }
                }
                SimpleOutputFormat::Table => {
                    for line in render_supported_datasource_catalog_table() {
                        println!("{line}");
                    }
                }
                SimpleOutputFormat::Csv => {
                    for line in render_supported_datasource_catalog_csv() {
                        println!("{line}");
                    }
                }
                SimpleOutputFormat::Json => {
                    print!(
                        "{}",
                        render_json_value(&render_supported_datasource_catalog_json())?
                    );
                }
                SimpleOutputFormat::Yaml => {
                    print!("{}", render_supported_datasource_catalog_yaml()?);
                }
            }
            Ok(())
        }
        DatasourceGroupCommand::List(args) => {
            if args.input_dir.is_some() {
                return super::run_local_datasource_list(&args);
            }
            if args.interactive {
                return Err(message(
                    "Datasource list --interactive requires --input-dir. Use datasource browse for live interactive review.",
                ));
            }
            let datasources = if args.all_orgs {
                let context = build_auth_context(&args.common)?;
                if context.auth_mode != "basic" {
                    return Err(message(
                        "Datasource list with --all-orgs requires Basic auth (--basic-user / --basic-password).",
                    ));
                }
                let admin_api = build_api_client(&args.common)?;
                let admin_client = admin_api.http_client();
                let admin_datasource = DatasourceResourceClient::new(admin_client);
                let mut rows = Vec::new();
                for org in admin_datasource.list_orgs()? {
                    let org_id = org
                        .get("id")
                        .and_then(Value::as_i64)
                        .ok_or_else(|| message("Grafana org list entry is missing numeric id."))?;
                    let org_client = build_http_client_for_org_from_api(&admin_api, org_id)?;
                    rows.extend(build_list_records(&org_client)?);
                }
                rows.sort_by(|left, right| {
                    let left_org_id = string_field(left, "orgId", "");
                    let right_org_id = string_field(right, "orgId", "");
                    left_org_id
                        .cmp(&right_org_id)
                        .then_with(|| {
                            string_field(left, "name", "").cmp(&string_field(right, "name", ""))
                        })
                        .then_with(|| {
                            string_field(left, "uid", "").cmp(&string_field(right, "uid", ""))
                        })
                });
                rows
            } else if args.org_id.is_some() {
                let client = resolve_target_client(&args.common, args.org_id)?;
                build_list_records(&client)?
            } else {
                let client = build_http_client(&args.common)?;
                DatasourceResourceClient::new(&client).list_datasources()?
            };
            if args.json {
                print!(
                    "{}",
                    render_json_value(&render_data_source_json(
                        &datasources,
                        (!args.output_columns.is_empty()).then_some(args.output_columns.as_slice()),
                    ))?
                );
            } else if args.yaml {
                print!(
                    "{}",
                    render_yaml(&render_data_source_json(
                        &datasources,
                        (!args.output_columns.is_empty()).then_some(args.output_columns.as_slice()),
                    ))?
                );
            } else if args.csv {
                for line in render_data_source_csv(
                    &datasources,
                    (!args.output_columns.is_empty()).then_some(args.output_columns.as_slice()),
                ) {
                    println!("{line}");
                }
            } else if args.text {
                for line in super::render_datasource_text(&datasources, &args.output_columns) {
                    println!("{line}");
                }
                println!();
                println!("Listed {} data source(s).", datasources.len());
            } else {
                for line in render_data_source_table(
                    &datasources,
                    !args.no_header,
                    (!args.output_columns.is_empty()).then_some(args.output_columns.as_slice()),
                ) {
                    println!("{line}");
                }
                println!();
                println!("Listed {} data source(s).", datasources.len());
            }
            Ok(())
        }
        DatasourceGroupCommand::Browse(args) => {
            let _ = super::datasource_browse::browse_datasources(&args)?;
            Ok(())
        }
        DatasourceGroupCommand::Add(args) => {
            validate_live_mutation_dry_run_args(
                args.table,
                args.json,
                args.dry_run,
                args.no_header,
                "add",
            )?;
            let payload = build_add_payload(&args)?;
            let client = build_http_client(&args.common)?;
            let datasource_client = DatasourceResourceClient::new(&client);
            let live = datasource_client.list_datasources()?;
            let matching =
                resolve_live_mutation_match(args.uid.as_deref(), Some(&args.name), &live);
            let row = vec![
                "add".to_string(),
                args.uid.clone().unwrap_or_default(),
                args.name.clone(),
                args.datasource_type.clone(),
                matching.destination.to_string(),
                matching.action.to_string(),
                matching
                    .target_id
                    .map(|id| id.to_string())
                    .unwrap_or_default(),
            ];
            if args.dry_run {
                if args.json {
                    print!("{}", render_json_value(&render_live_mutation_json(&[row]))?);
                } else if args.table {
                    for line in render_live_mutation_table(&[row], !args.no_header) {
                        println!("{line}");
                    }
                    println!("Dry-run checked 1 datasource add request");
                } else {
                    println!(
                        "Dry-run datasource add uid={} name={} match={} action={}",
                        args.uid.clone().unwrap_or_default(),
                        args.name,
                        matching.destination,
                        matching.action
                    );
                    println!("Dry-run checked 1 datasource add request");
                }
                return Ok(());
            }
            if matching.action != "would-create" {
                return Err(message(format!(
                    "Datasource add blocked for name={} uid={}: destination={} action={}.",
                    args.name,
                    args.uid.clone().unwrap_or_default(),
                    matching.destination,
                    matching.action
                )));
            }
            datasource_client.create_datasource(
                payload
                    .as_object()
                    .ok_or_else(|| message("Datasource add payload must be an object."))?,
            )?;
            println!(
                "Created datasource uid={} name={}",
                args.uid.unwrap_or_default(),
                args.name
            );
            Ok(())
        }
        DatasourceGroupCommand::Modify(args) => {
            validate_live_mutation_dry_run_args(
                args.table,
                args.json,
                args.dry_run,
                args.no_header,
                "modify",
            )?;
            let updates = build_modify_updates(&args)?;
            let client = build_http_client(&args.common)?;
            let datasource_client = DatasourceResourceClient::new(&client);
            let existing = super::fetch_datasource_by_uid_if_exists(&client, &args.uid)?;
            let (action, destination, payload, name, datasource_type, target_id) =
                if let Some(existing) = existing {
                    let payload = build_modify_payload(&existing, &updates)?;
                    (
                        "would-update",
                        "exists-uid",
                        Some(payload),
                        string_field(&existing, "name", ""),
                        string_field(&existing, "type", ""),
                        existing.get("id").and_then(Value::as_i64),
                    )
                } else {
                    (
                        "would-fail-missing",
                        "missing",
                        None,
                        String::new(),
                        String::new(),
                        None,
                    )
                };
            let row = vec![
                "modify".to_string(),
                args.uid.clone(),
                name.clone(),
                datasource_type.clone(),
                destination.to_string(),
                action.to_string(),
                target_id.map(|id| id.to_string()).unwrap_or_default(),
            ];
            if args.dry_run {
                if args.json {
                    print!("{}", render_json_value(&render_live_mutation_json(&[row]))?);
                } else if args.table {
                    for line in render_live_mutation_table(&[row], !args.no_header) {
                        println!("{line}");
                    }
                    println!("Dry-run checked 1 datasource modify request");
                } else {
                    println!(
                        "Dry-run datasource modify uid={} name={} match={} action={}",
                        args.uid, name, destination, action
                    );
                    println!("Dry-run checked 1 datasource modify request");
                }
                return Ok(());
            }
            if action != "would-update" {
                return Err(message(format!(
                    "Datasource modify blocked for uid={}: destination={} action={}.",
                    args.uid, destination, action
                )));
            }
            let payload =
                payload.ok_or_else(|| message("Datasource modify did not build a payload."))?;
            let target_id = target_id
                .ok_or_else(|| message("Datasource modify requires a live datasource id."))?;
            datasource_client.update_datasource(
                &target_id.to_string(),
                payload
                    .as_object()
                    .ok_or_else(|| message("Datasource modify payload must be an object."))?,
            )?;
            println!(
                "Modified datasource uid={} name={} id={}",
                args.uid, name, target_id
            );
            Ok(())
        }
        DatasourceGroupCommand::Delete(args) => {
            validate_live_mutation_dry_run_args(
                args.table,
                args.json,
                args.dry_run,
                args.no_header,
                "delete",
            )?;
            let client = build_http_client(&args.common)?;
            let datasource_client = DatasourceResourceClient::new(&client);
            let live = datasource_client.list_datasources()?;
            let matching = resolve_delete_match(args.uid.as_deref(), args.name.as_deref(), &live);
            let delete_type = super::resolve_delete_preview_type(matching.target_id, &live);
            let row = vec![
                "delete".to_string(),
                args.uid
                    .clone()
                    .or_else(|| {
                        if matching.target_uid.is_empty() {
                            None
                        } else {
                            Some(matching.target_uid.clone())
                        }
                    })
                    .unwrap_or_default(),
                args.name
                    .clone()
                    .unwrap_or_else(|| matching.target_name.clone()),
                delete_type.clone(),
                matching.destination.to_string(),
                matching.action.to_string(),
                matching
                    .target_id
                    .map(|id| id.to_string())
                    .unwrap_or_default(),
            ];
            if args.dry_run {
                if args.json {
                    print!("{}", render_json_value(&render_live_mutation_json(&[row]))?);
                } else if args.table {
                    for line in render_live_mutation_table(&[row], !args.no_header) {
                        println!("{line}");
                    }
                    println!("Dry-run checked 1 datasource delete request");
                } else {
                    println!(
                        "Dry-run datasource delete uid={} name={} type={} match={} action={}",
                        args.uid.clone().unwrap_or_default(),
                        args.name.clone().unwrap_or_default(),
                        delete_type,
                        matching.destination,
                        matching.action
                    );
                    println!("Dry-run checked 1 datasource delete request");
                }
                return Ok(());
            }
            if !args.yes {
                return Err(message(
                    "Datasource delete requires --yes unless --dry-run is set.",
                ));
            }
            if matching.action != "would-delete" {
                return Err(message(format!(
                    "Datasource delete blocked for uid={} name={} type={}: destination={} action={}.",
                    args.uid.clone().unwrap_or_default(),
                    args.name.clone().unwrap_or_default(),
                    super::resolve_delete_preview_type(matching.target_id, &live),
                    matching.destination,
                    matching.action
                )));
            }
            let target_id = matching
                .target_id
                .ok_or_else(|| message("Datasource delete requires a live datasource id."))?;
            datasource_client.delete_datasource(&target_id.to_string())?;
            println!(
                "Deleted datasource uid={} name={} type={} id={}",
                if matching.target_uid.is_empty() {
                    args.uid.unwrap_or_default()
                } else {
                    matching.target_uid
                },
                if matching.target_name.is_empty() {
                    args.name.unwrap_or_default()
                } else {
                    matching.target_name
                },
                super::resolve_delete_preview_type(Some(target_id), &live),
                target_id
            );
            Ok(())
        }
        DatasourceGroupCommand::Export(args) => {
            if args.all_orgs {
                let context = build_auth_context(&args.common)?;
                if context.auth_mode != "basic" {
                    return Err(message(
                        "Datasource export with --all-orgs requires Basic auth (--basic-user / --basic-password).",
                    ));
                }
                let admin_api = build_api_client(&args.common)?;
                let admin_client = admin_api.http_client();
                let admin_datasource = DatasourceResourceClient::new(admin_client);
                let mut total = 0usize;
                let mut org_count = 0usize;
                let mut root_items = Vec::new();
                let mut root_records = Vec::new();
                for org in admin_datasource.list_orgs()? {
                    let org_id = org
                        .get("id")
                        .and_then(Value::as_i64)
                        .ok_or_else(|| message("Grafana org list entry is missing numeric id."))?;
                    let org_id_string = org_id.to_string();
                    let org_name = string_field(&org, "name", "");
                    let org_client = build_http_client_for_org_from_api(&admin_api, org_id)?;
                    let records = build_export_records(&org_client)?;
                    let scoped_output_dir = build_all_orgs_output_dir(&args.output_dir, &org);
                    let datasources_path = scoped_output_dir.join(DATASOURCE_EXPORT_FILENAME);
                    let index_path = scoped_output_dir.join("index.json");
                    let metadata_path = scoped_output_dir.join(EXPORT_METADATA_FILENAME);
                    let provisioning_path = scoped_output_dir
                        .join(DATASOURCE_PROVISIONING_SUBDIR)
                        .join(DATASOURCE_PROVISIONING_FILENAME);
                    if !args.dry_run {
                        write_json_file(
                            &datasources_path,
                            &Value::Array(records.clone().into_iter().map(Value::Object).collect()),
                            args.overwrite,
                        )?;
                        write_json_file(
                            &index_path,
                            &build_export_index(&records),
                            args.overwrite,
                        )?;
                        write_json_file(
                            &metadata_path,
                            &build_datasource_export_metadata(
                                &args.common.url,
                                args.common.profile.as_deref(),
                                Some("org"),
                                Some(&org_id_string),
                                Some(&org_name),
                                &scoped_output_dir,
                                records.len(),
                            ),
                            args.overwrite,
                        )?;
                        if !args.without_datasource_provisioning {
                            write_yaml_file(
                                &provisioning_path,
                                &build_datasource_provisioning_document(&records),
                                args.overwrite,
                            )?;
                        }
                    }
                    let summary_verb = if args.dry_run {
                        "Would export"
                    } else {
                        "Exported"
                    };
                    println!(
                        "{summary_verb} {} datasource(s). Datasources: {} Index: {} Manifest: {}{}",
                        records.len(),
                        datasources_path.display(),
                        index_path.display(),
                        metadata_path.display(),
                        if args.without_datasource_provisioning {
                            String::new()
                        } else {
                            format!(" Provisioning: {}", provisioning_path.display())
                        }
                    );
                    for item in build_export_index(&records)
                        .get("items")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                    {
                        if let Some(object) = item.as_object() {
                            let mut entry = object.clone();
                            entry.insert(
                                "exportDir".to_string(),
                                Value::String(scoped_output_dir.display().to_string()),
                            );
                            root_items.push(entry);
                        }
                    }
                    root_records.extend(records.iter().cloned());
                    total += records.len();
                    org_count += 1;
                }
                if !args.dry_run {
                    write_json_file(
                        &args.output_dir.join("index.json"),
                        &build_all_orgs_export_index(&root_items),
                        args.overwrite,
                    )?;
                    write_json_file(
                        &args.output_dir.join(EXPORT_METADATA_FILENAME),
                        &build_all_orgs_export_metadata(
                            &args.common.url,
                            args.common.profile.as_deref(),
                            &args.output_dir,
                            org_count,
                            total,
                        ),
                        args.overwrite,
                    )?;
                    if !args.without_datasource_provisioning {
                        write_yaml_file(
                            &args
                                .output_dir
                                .join(DATASOURCE_PROVISIONING_SUBDIR)
                                .join(DATASOURCE_PROVISIONING_FILENAME),
                            &build_datasource_provisioning_document(&root_records),
                            args.overwrite,
                        )?;
                    }
                }
                println!(
                    "{} datasource(s) across {} exported org(s) under {}",
                    total,
                    org_count,
                    args.output_dir.display()
                );
                return Ok(());
            }
            let client = resolve_target_client(&args.common, args.org_id)?;
            let records = build_export_records(&client)?;
            let datasources_path = args.output_dir.join(DATASOURCE_EXPORT_FILENAME);
            let index_path = args.output_dir.join("index.json");
            let metadata_path = args.output_dir.join(EXPORT_METADATA_FILENAME);
            let provisioning_path = args
                .output_dir
                .join(DATASOURCE_PROVISIONING_SUBDIR)
                .join(DATASOURCE_PROVISIONING_FILENAME);
            if !args.dry_run {
                write_json_file(
                    &datasources_path,
                    &Value::Array(records.clone().into_iter().map(Value::Object).collect()),
                    args.overwrite,
                )?;
                write_json_file(&index_path, &build_export_index(&records), args.overwrite)?;
                write_json_file(
                    &metadata_path,
                    &build_datasource_export_metadata(
                        &args.common.url,
                        args.common.profile.as_deref(),
                        Some("org"),
                        None,
                        None,
                        &args.output_dir,
                        records.len(),
                    ),
                    args.overwrite,
                )?;
                if !args.without_datasource_provisioning {
                    write_yaml_file(
                        &provisioning_path,
                        &build_datasource_provisioning_document(&records),
                        args.overwrite,
                    )?;
                }
            }
            let summary_verb = if args.dry_run {
                "Would export"
            } else {
                "Exported"
            };
            println!(
                "{summary_verb} {} datasource(s). Datasources: {} Index: {} Manifest: {}{}",
                records.len(),
                datasources_path.display(),
                index_path.display(),
                metadata_path.display(),
                if args.without_datasource_provisioning {
                    String::new()
                } else {
                    format!(" Provisioning: {}", provisioning_path.display())
                }
            );
            Ok(())
        }
        DatasourceGroupCommand::Import(args) => {
            validate_import_org_auth(&args.common, &args)?;
            if args.table && !args.dry_run {
                return Err(message(
                    "--table is only supported with --dry-run for datasource import.",
                ));
            }
            if args.json && !args.dry_run {
                return Err(message(
                    "--json is only supported with --dry-run for datasource import.",
                ));
            }
            if args.table && args.json {
                return Err(message(
                    "--table and --json are mutually exclusive for datasource import.",
                ));
            }
            if args.no_header && !args.table {
                return Err(message(
                    "--no-header is only supported with --dry-run --table for datasource import.",
                ));
            }
            if !args.output_columns.is_empty() && !args.table {
                return Err(message(
                    "--output-columns is only supported with --dry-run --table or table-like --output-format for datasource import.",
                ));
            }
            if args.use_export_org {
                if !args.output_columns.is_empty() {
                    return Err(message(
                        "--output-columns is not supported with --use-export-org for datasource import.",
                    ));
                }
                import_datasources_by_export_org(&args)?;
                return Ok(());
            }
            let client = resolve_target_client(&args.common, args.org_id)?;
            import_datasources_with_client(&client, &args)?;
            Ok(())
        }
        DatasourceGroupCommand::Diff(args) => {
            let client = build_http_client(&args.common)?;
            let datasource_client = DatasourceResourceClient::new(&client);
            let live = datasource_client.list_datasources()?;
            let (compared_count, differences) = diff_datasources_with_live(
                &args.diff_dir,
                args.input_format,
                &live,
                args.output_format,
            )?;
            if differences > 0 {
                return Err(message(format!(
                    "Found {} datasource difference(s) across {} exported datasource(s).",
                    differences, compared_count
                )));
            }
            println!(
                "No datasource differences across {} exported datasource(s).",
                compared_count
            );
            Ok(())
        }
        DatasourceGroupCommand::Plan(args) => {
            let plan_input = if args.use_export_org {
                build_routed_datasource_plan_input(&args)?
            } else {
                let client = resolve_target_client(&args.common, args.org_id)?;
                let input_dir =
                    if args.input_format == super::DatasourceImportInputFormat::Inventory {
                        resolve_datasource_export_root_dir(&args.input_dir)?
                    } else {
                        args.input_dir.clone()
                    };
                let (_metadata, records) = load_import_records(&input_dir, args.input_format)?;
                let live = DatasourceResourceClient::new(&client).list_datasources()?;
                DatasourcePlanInput {
                    scope: if args.org_id.is_some() {
                        "explicit-org".to_string()
                    } else {
                        "current-org".to_string()
                    },
                    input_format: format!("{:?}", args.input_format).to_ascii_lowercase(),
                    prune: args.prune,
                    orgs: vec![DatasourcePlanOrgInput {
                        source_org_id: String::new(),
                        source_org_name: String::new(),
                        target_org_id: args.org_id.map(|value| value.to_string()),
                        target_org_name: String::new(),
                        org_action: "current".to_string(),
                        input_dir,
                        records,
                        live,
                    }],
                }
            };
            let report = build_datasource_plan(plan_input);
            print_datasource_plan_report(
                &report,
                args.output_format,
                args.show_same,
                !args.no_header,
                &args.output_columns,
            )
        }
    }
}

fn build_routed_datasource_plan_input(
    args: &super::DatasourcePlanArgs,
) -> Result<DatasourcePlanInput> {
    let context = build_auth_context(&args.common)?;
    if context.auth_mode != "basic" {
        return Err(message(
            "Datasource plan with --use-export-org requires Basic auth (--basic-user / --basic-password).",
        ));
    }
    let admin_api = build_api_client(&args.common)?;
    let admin_client = admin_api.http_client();
    let import_args = datasource_plan_to_import_args(args);
    let scopes = discover_export_org_import_scopes(&import_args)?;
    let mut orgs = Vec::new();
    for scope in scopes {
        let target_plan = resolve_export_org_target_plan(admin_client, &import_args, &scope)?;
        let (_metadata, records) = load_import_records(&target_plan.input_dir, args.input_format)?;
        let live = if let Some(target_org_id) = target_plan.target_org_id {
            let org_client = build_http_client_for_org_from_api(&admin_api, target_org_id)?;
            DatasourceResourceClient::new(&org_client).list_datasources()?
        } else {
            Vec::new()
        };
        orgs.push(DatasourcePlanOrgInput {
            source_org_id: target_plan.source_org_id.to_string(),
            source_org_name: target_plan.source_org_name,
            target_org_id: target_plan.target_org_id.map(|value| value.to_string()),
            target_org_name: String::new(),
            org_action: target_plan.org_action.to_string(),
            input_dir: target_plan.input_dir,
            records,
            live,
        });
    }
    Ok(DatasourcePlanInput {
        scope: "export-orgs".to_string(),
        input_format: format!("{:?}", args.input_format).to_ascii_lowercase(),
        prune: args.prune,
        orgs,
    })
}

fn datasource_plan_to_import_args(args: &super::DatasourcePlanArgs) -> DatasourceImportArgs {
    DatasourceImportArgs {
        common: args.common.clone(),
        input_dir: args.input_dir.clone(),
        input_format: args.input_format,
        org_id: args.org_id,
        use_export_org: args.use_export_org,
        only_org_id: args.only_org_id.clone(),
        create_missing_orgs: args.create_missing_orgs,
        require_matching_export_org: false,
        replace_existing: false,
        update_existing_only: false,
        secret_values: None,
        secret_values_file: None,
        dry_run: true,
        table: false,
        json: false,
        output_format: None,
        no_header: false,
        output_columns: Vec::new(),
        list_columns: false,
        progress: false,
        verbose: false,
    }
}

fn materialize_datasource_command_auth(
    mut command: DatasourceGroupCommand,
) -> Result<DatasourceGroupCommand> {
    match &mut command {
        DatasourceGroupCommand::List(inner) => {
            inner.common = materialize_dashboard_common_auth(inner.common.clone())?;
        }
        DatasourceGroupCommand::Add(inner) => {
            inner.common = materialize_dashboard_common_auth(inner.common.clone())?;
        }
        DatasourceGroupCommand::Modify(inner) => {
            inner.common = materialize_dashboard_common_auth(inner.common.clone())?;
        }
        DatasourceGroupCommand::Delete(inner) => {
            inner.common = materialize_dashboard_common_auth(inner.common.clone())?;
        }
        DatasourceGroupCommand::Export(inner) => {
            inner.common = materialize_dashboard_common_auth(inner.common.clone())?;
        }
        DatasourceGroupCommand::Import(inner) => {
            inner.common = materialize_dashboard_common_auth(inner.common.clone())?;
        }
        DatasourceGroupCommand::Diff(inner) => {
            inner.common = materialize_dashboard_common_auth(inner.common.clone())?;
        }
        DatasourceGroupCommand::Plan(inner) => {
            inner.common = materialize_dashboard_common_auth(inner.common.clone())?;
        }
        DatasourceGroupCommand::Browse(inner) => {
            inner.common = materialize_dashboard_common_auth(inner.common.clone())?;
        }
        DatasourceGroupCommand::Types(_) => {}
    }
    Ok(command)
}
