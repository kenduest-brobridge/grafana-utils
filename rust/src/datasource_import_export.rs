use reqwest::Method;
use serde_json::{Map, Value};

use crate::common::{message, Result};
use crate::dashboard::{
    build_http_client, build_http_client_for_org, list_datasources, DEFAULT_ORG_ID,
};
use crate::datasource::{render_import_table, resolve_match, DatasourceImportArgs};
use crate::http::JsonHttpClient;

#[path = "datasource_export_support.rs"]
mod datasource_export_support;
#[path = "datasource_import_export_routed.rs"]
mod datasource_import_export_routed;
#[path = "datasource_import_export_support.rs"]
mod datasource_import_export_support;

pub(crate) use datasource_export_support::{
    build_all_orgs_export_index, build_all_orgs_export_metadata, build_all_orgs_output_dir,
    build_datasource_export_metadata, build_export_index, build_export_records, build_list_records,
    describe_datasource_import_mode, export_datasource_scope, render_data_source_csv,
    render_data_source_json, render_data_source_table, resolve_target_client,
    validate_import_org_auth,
};
pub(crate) use datasource_import_export_routed::{
    build_routed_datasource_import_dry_run_json, format_routed_datasource_scope_summary_fields,
    format_routed_datasource_target_org_label, render_routed_datasource_import_org_table,
    resolve_export_org_target_plan,
};
pub(crate) use datasource_import_export_support::{
    create_org, discover_export_org_import_scopes, fetch_current_org, list_orgs,
    load_diff_record_values, load_import_records, org_id_string_from_value,
    validate_matching_export_org, DatasourceExportMetadata, DatasourceExportOrgScope,
    DatasourceExportOrgTargetPlan, DatasourceImportDryRunReport, DatasourceImportRecord,
    DATASOURCE_EXPORT_FILENAME, EXPORT_METADATA_FILENAME,
};
pub(crate) fn collect_datasource_import_dry_run_report(
    client: &JsonHttpClient,
    args: &DatasourceImportArgs,
) -> Result<DatasourceImportDryRunReport> {
    let replace_existing = args.replace_existing || args.update_existing_only;
    let (metadata, records) = load_import_records(&args.import_dir)?;
    validate_matching_export_org(client, args, &args.import_dir, &metadata)?;
    let live = list_datasources(client)?;
    let target_org = fetch_current_org(client)?;
    let target_org_id = target_org
        .get("id")
        .map(|value| value.to_string())
        .unwrap_or_else(|| DEFAULT_ORG_ID.to_string());
    let mode = describe_datasource_import_mode(args.replace_existing, args.update_existing_only);
    let mut rows = Vec::new();
    let mut created = 0usize;
    let mut updated = 0usize;
    let mut skipped = 0usize;
    let mut blocked = 0usize;
    for (index, record) in records.iter().enumerate() {
        let matching = resolve_match(record, &live, replace_existing, args.update_existing_only);
        let file_ref = format!("{}#{}", metadata.datasources_file, index);
        rows.push(vec![
            record.uid.clone(),
            record.name.clone(),
            record.datasource_type.clone(),
            matching.destination.to_string(),
            matching.action.to_string(),
            target_org_id.clone(),
            file_ref,
        ]);
        match matching.action {
            "would-create" => created += 1,
            "would-update" => updated += 1,
            "would-skip-missing" => skipped += 1,
            _ => blocked += 1,
        }
    }
    Ok(DatasourceImportDryRunReport {
        mode: mode.to_string(),
        import_dir: args.import_dir.clone(),
        source_org_id: records
            .iter()
            .find(|item| !item.org_id.is_empty())
            .map(|item| item.org_id.clone())
            .unwrap_or_default(),
        target_org_id,
        rows,
        datasource_count: records.len(),
        would_create: created,
        would_update: updated,
        would_skip: skipped,
        would_block: blocked,
    })
}

pub(crate) fn build_datasource_import_dry_run_json_value(
    report: &DatasourceImportDryRunReport,
) -> Value {
    Value::Object(Map::from_iter(vec![
        ("mode".to_string(), Value::String(report.mode.clone())),
        (
            "sourceOrgId".to_string(),
            Value::String(report.source_org_id.clone()),
        ),
        (
            "targetOrgId".to_string(),
            Value::String(report.target_org_id.clone()),
        ),
        (
            "datasources".to_string(),
            Value::Array(
                report
                    .rows
                    .iter()
                    .map(|row| {
                        Value::Object(Map::from_iter(vec![
                            ("uid".to_string(), Value::String(row[0].clone())),
                            ("name".to_string(), Value::String(row[1].clone())),
                            ("type".to_string(), Value::String(row[2].clone())),
                            ("destination".to_string(), Value::String(row[3].clone())),
                            ("action".to_string(), Value::String(row[4].clone())),
                            ("orgId".to_string(), Value::String(row[5].clone())),
                            ("file".to_string(), Value::String(row[6].clone())),
                        ]))
                    })
                    .collect(),
            ),
        ),
        (
            "summary".to_string(),
            Value::Object(Map::from_iter(vec![
                (
                    "datasourceCount".to_string(),
                    Value::Number((report.datasource_count as i64).into()),
                ),
                (
                    "wouldCreate".to_string(),
                    Value::Number((report.would_create as i64).into()),
                ),
                (
                    "wouldUpdate".to_string(),
                    Value::Number((report.would_update as i64).into()),
                ),
                (
                    "wouldSkip".to_string(),
                    Value::Number((report.would_skip as i64).into()),
                ),
                (
                    "wouldBlock".to_string(),
                    Value::Number((report.would_block as i64).into()),
                ),
            ])),
        ),
    ]))
}

pub(crate) fn print_datasource_import_dry_run_report(
    report: &DatasourceImportDryRunReport,
    args: &DatasourceImportArgs,
) -> Result<()> {
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&build_datasource_import_dry_run_json_value(report))?
        );
    } else if args.table {
        for line in render_import_table(
            &report.rows,
            !args.no_header,
            if args.output_columns.is_empty() {
                None
            } else {
                Some(args.output_columns.as_slice())
            },
        ) {
            println!("{line}");
        }
        println!(
            "Dry-run checked {} datasource(s) from {}",
            report.datasource_count,
            report.import_dir.display()
        );
    } else {
        println!("Import mode: {}", report.mode);
        for row in &report.rows {
            println!(
                "Dry-run datasource uid={} name={} dest={} action={} file={}",
                row[0], row[1], row[3], row[4], row[6]
            );
        }
        println!(
            "Dry-run checked {} datasource(s) from {}",
            report.datasource_count,
            report.import_dir.display()
        );
    }
    Ok(())
}

pub(crate) fn build_import_payload(record: &DatasourceImportRecord) -> Value {
    Value::Object(Map::from_iter(vec![
        ("name".to_string(), Value::String(record.name.clone())),
        (
            "type".to_string(),
            Value::String(record.datasource_type.clone()),
        ),
        ("url".to_string(), Value::String(record.url.clone())),
        ("access".to_string(), Value::String(record.access.clone())),
        ("uid".to_string(), Value::String(record.uid.clone())),
        ("isDefault".to_string(), Value::Bool(record.is_default)),
    ]))
}

pub(crate) fn import_datasources_with_client(
    client: &JsonHttpClient,
    args: &DatasourceImportArgs,
) -> Result<usize> {
    if args.dry_run {
        let report = collect_datasource_import_dry_run_report(client, args)?;
        print_datasource_import_dry_run_report(&report, args)?;
        return Ok(0);
    }
    let replace_existing = args.replace_existing || args.update_existing_only;
    let (metadata, records) = load_import_records(&args.import_dir)?;
    validate_matching_export_org(client, args, &args.import_dir, &metadata)?;
    let live = list_datasources(client)?;
    let mut created = 0usize;
    let mut updated = 0usize;
    let mut skipped = 0usize;
    let blocked = 0usize;
    for record in &records {
        let matching = resolve_match(record, &live, replace_existing, args.update_existing_only);
        match matching.action {
            "would-create" => {
                client.request_json(
                    Method::POST,
                    "/api/datasources",
                    &[],
                    Some(&build_import_payload(record)),
                )?;
                created += 1;
            }
            "would-update" => {
                let target_id = matching.target_id.ok_or_else(|| {
                    message(format!(
                        "Matched datasource {} does not expose a usable numeric id for update.",
                        matching.target_name
                    ))
                })?;
                let payload = build_import_payload(record);
                client.request_json(
                    Method::PUT,
                    &format!("/api/datasources/{target_id}"),
                    &[],
                    Some(&payload),
                )?;
                updated += 1;
            }
            "would-skip-missing" => {
                skipped += 1;
            }
            _ => {
                return Err(message(format!(
                    "Datasource import blocked for {}: destination={} action={}.",
                    if record.uid.is_empty() {
                        &record.name
                    } else {
                        &record.uid
                    },
                    matching.destination,
                    matching.action
                )));
            }
        }
    }
    println!(
        "Imported {} datasource(s) from {}; updated {}, skipped {}, blocked {}",
        created + updated,
        args.import_dir.display(),
        updated,
        skipped,
        blocked
    );
    Ok(created + updated)
}

pub(crate) fn import_datasources_by_export_org(args: &DatasourceImportArgs) -> Result<usize> {
    let admin_client = build_http_client(&args.common)?;
    let scopes = discover_export_org_import_scopes(args)?;
    if args.dry_run && args.json {
        println!("{}", build_routed_datasource_import_dry_run_json(args)?);
        return Ok(0);
    }
    let mut org_rows = Vec::new();
    let mut plans = Vec::new();
    for scope in scopes {
        let plan = resolve_export_org_target_plan(&admin_client, args, &scope)?;
        let datasource_count = load_import_records(&plan.import_dir)?.1.len();
        org_rows.push(vec![
            plan.source_org_id.to_string(),
            if plan.source_org_name.is_empty() {
                "-".to_string()
            } else {
                plan.source_org_name.clone()
            },
            plan.org_action.to_string(),
            format_routed_datasource_target_org_label(plan.target_org_id),
            datasource_count.to_string(),
            plan.import_dir.display().to_string(),
        ]);
        plans.push(plan);
    }
    if args.dry_run && args.table {
        for line in render_routed_datasource_import_org_table(&org_rows, !args.no_header) {
            println!("{line}");
        }
        return Ok(0);
    }
    let mut imported_count = 0usize;
    for plan in plans {
        println!(
            "Importing {}",
            format_routed_datasource_scope_summary_fields(
                plan.source_org_id,
                &plan.source_org_name,
                plan.org_action,
                plan.target_org_id,
                &plan.import_dir,
            )
        );
        let Some(target_org_id) = plan.target_org_id else {
            continue;
        };
        let mut scoped_args = args.clone();
        scoped_args.org_id = Some(target_org_id);
        scoped_args.use_export_org = false;
        scoped_args.only_org_id = Vec::new();
        scoped_args.create_missing_orgs = false;
        scoped_args.import_dir = plan.import_dir.clone();
        let scoped_client = build_http_client_for_org(&args.common, target_org_id)?;
        imported_count +=
            import_datasources_with_client(&scoped_client, &scoped_args).map_err(|error| {
                message(format!(
                    "Datasource routed import failed for {}: {}",
                    format_routed_datasource_scope_summary_fields(
                        plan.source_org_id,
                        &plan.source_org_name,
                        plan.org_action,
                        plan.target_org_id,
                        &plan.import_dir,
                    ),
                    error
                ))
            })?;
    }
    Ok(imported_count)
}
