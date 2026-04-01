//! Import orchestration for Dashboard resources, including input normalization and apply contract handling.

use reqwest::Method;
use serde_json::Value;

use crate::common::{message, Result};
use crate::dashboard::{
    build_http_client_for_org, build_import_payload, extract_dashboard_object,
    import_dashboard_request_with_request, load_export_metadata, load_folder_inventory,
    load_json_file, validate, DiffArgs, FolderInventoryItem, FolderInventoryStatusKind, ImportArgs,
    DEFAULT_UNKNOWN_UID, FOLDER_INVENTORY_FILENAME,
};
use crate::http::{JsonHttpClient, JsonHttpClientConfig};

use super::super::import_compare::diff_dashboards_with_request;
use super::super::import_lookup::{
    apply_folder_path_guard_to_action, build_folder_path_match_result,
    determine_dashboard_import_action_with_request,
    determine_import_folder_uid_override_with_request, ensure_folder_inventory_entry_cached,
    resolve_dashboard_import_folder_path_with_request,
    resolve_existing_dashboard_folder_path_with_request, ImportLookupCache,
};
use super::super::import_render::{
    format_import_progress_line, format_import_verbose_line, render_import_dry_run_json,
    render_import_dry_run_table,
};
use super::super::import_validation::{
    validate_dashboard_import_dependencies_with_request, validate_matching_export_org_with_request,
};
use super::import_dry_run::{
    collect_import_dry_run_report_with_request, folder_inventory_status_output_lines,
};

/// Purpose: implementation note.
pub fn diff_dashboards_with_client(client: &JsonHttpClient, args: &DiffArgs) -> Result<usize> {
    diff_dashboards_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        args,
    )
}

/// Purpose: implementation note.
pub(crate) fn import_dashboards_with_request<F>(
    mut request_json: F,
    args: &ImportArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut lookup_cache = ImportLookupCache::default();
    if args.table && !args.dry_run {
        return Err(message(
            "--table is only supported with --dry-run for import-dashboard.",
        ));
    }
    if args.json && !args.dry_run {
        return Err(message(
            "--json is only supported with --dry-run for import-dashboard.",
        ));
    }
    if args.table && args.json {
        return Err(message(
            "--table and --json are mutually exclusive for import-dashboard.",
        ));
    }
    if args.no_header && !args.table {
        return Err(message(
            "--no-header is only supported with --dry-run --table for import-dashboard.",
        ));
    }
    if !args.output_columns.is_empty() && !args.table {
        return Err(message(
            "--output-columns is only supported with --dry-run --table or table-like --output-format for import-dashboard.",
        ));
    }
    if args.require_matching_folder_path && args.import_folder_uid.is_some() {
        return Err(message(
            "--require-matching-folder-path cannot be combined with --import-folder-uid.",
        ));
    }
    if args.ensure_folders && args.import_folder_uid.is_some() {
        return Err(message(
            "--ensure-folders cannot be combined with --import-folder-uid.",
        ));
    }
    let resolved_import = super::resolve_import_source(args)?;
    let metadata = load_export_metadata(
        &resolved_import.metadata_dir,
        Some(super::import_metadata_variant(args)),
    )?;
    validate_matching_export_org_with_request(
        &mut request_json,
        &mut lookup_cache,
        args,
        &resolved_import.metadata_dir,
        metadata.as_ref(),
        None,
    )?;
    let folder_inventory = if args.ensure_folders || args.dry_run {
        load_folder_inventory(&resolved_import.metadata_dir, metadata.as_ref())?
    } else {
        Vec::new()
    };
    if args.ensure_folders && folder_inventory.is_empty() {
        let folders_file = metadata
            .as_ref()
            .and_then(|item| item.folders_file.as_deref())
            .unwrap_or(FOLDER_INVENTORY_FILENAME);
        return Err(message(format!(
            "Folder inventory file not found for --ensure-folders: {}. Re-export dashboards with raw folder inventory or omit --ensure-folders.",
            resolved_import.metadata_dir.join(folders_file).display()
        )));
    }
    let folder_statuses = if args.dry_run && args.ensure_folders {
        super::super::import_lookup::collect_folder_inventory_statuses_cached(
            &mut request_json,
            &mut lookup_cache,
            &folder_inventory,
        )?
    } else {
        Vec::new()
    };
    let folders_by_uid: std::collections::BTreeMap<String, FolderInventoryItem> = folder_inventory
        .into_iter()
        .map(|item| (item.uid.clone(), item))
        .collect();
    if !args.dry_run {
        validate_dashboard_import_dependencies_with_request(
            &mut request_json,
            &resolved_import.dashboard_dir,
            args.strict_schema,
            args.target_schema_version,
        )?;
    }
    let discovered_dashboard_files =
        super::dashboard_files_for_import(&resolved_import.dashboard_dir)?;
    let dashboard_files = match super::selected_dashboard_files(
        &mut request_json,
        &mut lookup_cache,
        args,
        discovered_dashboard_files.clone(),
    )? {
        Some(selected) => selected,
        None if args.interactive => {
            println!(
                "{} cancelled.",
                if args.dry_run {
                    "Interactive dry-run"
                } else {
                    "Import"
                }
            );
            return Ok(0);
        }
        None => discovered_dashboard_files,
    };
    let total = dashboard_files.len();
    let effective_replace_existing = args.replace_existing || args.update_existing_only;
    let mut dry_run_records: Vec<[String; 8]> = Vec::new();
    let mut imported_count = 0usize;
    let mut skipped_missing_count = 0usize;
    let mut skipped_folder_mismatch_count = 0usize;
    let mode = super::import_render::describe_dashboard_import_mode(
        args.replace_existing,
        args.update_existing_only,
    );
    if !args.json {
        println!("Import mode: {}", mode);
    }
    if args.dry_run && args.ensure_folders {
        folder_inventory_status_output_lines(
            &folder_statuses,
            args.no_header,
            args.json,
            args.table,
        );
        let missing_folder_count = folder_statuses
            .iter()
            .filter(|status| status.kind == FolderInventoryStatusKind::Missing)
            .count();
        let mismatched_folder_count = folder_statuses
            .iter()
            .filter(|status| status.kind == FolderInventoryStatusKind::Mismatch)
            .count();
        let folders_file = metadata
            .as_ref()
            .and_then(|item| item.folders_file.as_deref())
            .unwrap_or(super::FOLDER_INVENTORY_FILENAME);
        if !args.json {
            println!(
                "Dry-run checked {} folder(s) from {}; {} missing, {} mismatched",
                folder_statuses.len(),
                args.import_dir.join(folders_file).display(),
                missing_folder_count,
                mismatched_folder_count
            );
        }
    }
    for (index, dashboard_file) in dashboard_files.iter().enumerate() {
        let document = load_json_file(dashboard_file)?;
        if args.strict_schema {
            validate::validate_dashboard_import_document(
                &document,
                dashboard_file,
                true,
                args.target_schema_version,
            )?;
        }
        let document_object =
            crate::common::value_as_object(&document, "Dashboard payload must be a JSON object.")?;
        let dashboard = extract_dashboard_object(document_object)?;
        let uid = crate::common::string_field(dashboard, "uid", "");
        let source_folder_path = if args.require_matching_folder_path {
            Some(
                super::super::import_lookup::resolve_source_dashboard_folder_path(
                    &document,
                    dashboard_file,
                    &resolved_import.metadata_dir,
                    &folders_by_uid,
                )?,
            )
        } else {
            None
        };
        let folder_uid_override = determine_import_folder_uid_override_with_request(
            &mut request_json,
            &mut lookup_cache,
            &uid,
            args.import_folder_uid.as_deref(),
            effective_replace_existing,
        )?;
        let payload = build_import_payload(
            &document,
            folder_uid_override.as_deref(),
            effective_replace_existing,
            &args.import_message,
        )?;
        let action = if args.dry_run
            || args.update_existing_only
            || args.ensure_folders
            || args.require_matching_folder_path
        {
            Some(determine_dashboard_import_action_with_request(
                &mut request_json,
                &mut lookup_cache,
                &payload,
                args.replace_existing,
                args.update_existing_only,
            )?)
        } else {
            None
        };
        let destination_folder_path = if args.require_matching_folder_path {
            resolve_existing_dashboard_folder_path_with_request(
                &mut request_json,
                &mut lookup_cache,
                &uid,
            )?
        } else {
            None
        };
        let (
            folder_paths_match,
            folder_match_reason,
            normalized_source_folder_path,
            normalized_destination_folder_path,
        ) = if args.require_matching_folder_path {
            build_folder_path_match_result(
                source_folder_path.as_deref(),
                destination_folder_path.as_deref(),
                destination_folder_path.is_some(),
                true,
            )
        } else {
            (true, "", String::new(), None::<String>)
        };
        let action =
            action.map(|value| apply_folder_path_guard_to_action(value, folder_paths_match));
        if args.dry_run {
            let needs_dry_run_folder_path =
                args.table || args.json || args.verbose || args.progress;
            let folder_path = if needs_dry_run_folder_path {
                let prefer_live_folder_path = folder_uid_override.is_some()
                    && args.import_folder_uid.is_none()
                    && !uid.is_empty();
                Some(resolve_dashboard_import_folder_path_with_request(
                    &mut request_json,
                    &mut lookup_cache,
                    &payload,
                    &folders_by_uid,
                    prefer_live_folder_path,
                )?)
            } else {
                None
            };
            let payload_object = crate::common::value_as_object(
                &payload,
                "Dashboard import payload must be a JSON object.",
            )?;
            let dashboard = payload_object
                .get("dashboard")
                .and_then(Value::as_object)
                .ok_or_else(|| message("Dashboard import payload is missing dashboard."))?;
            let uid = crate::common::string_field(dashboard, "uid", DEFAULT_UNKNOWN_UID);
            if args.table || args.json {
                dry_run_records.push(super::super::import_render::build_import_dry_run_record(
                    dashboard_file,
                    &uid,
                    action.unwrap_or(DEFAULT_UNKNOWN_UID),
                    folder_path.as_deref().unwrap_or(""),
                    &normalized_source_folder_path,
                    normalized_destination_folder_path.as_deref(),
                    folder_match_reason,
                ));
            } else if args.verbose {
                println!(
                    "{}",
                    format_import_verbose_line(
                        dashboard_file,
                        true,
                        Some(&uid),
                        Some(action.unwrap_or(DEFAULT_UNKNOWN_UID)),
                        folder_path.as_deref(),
                    )
                );
            } else if args.progress {
                println!(
                    "{}",
                    format_import_progress_line(
                        index + 1,
                        total,
                        &uid,
                        true,
                        Some(action.unwrap_or(DEFAULT_UNKNOWN_UID)),
                        folder_path.as_deref(),
                    )
                );
            }
            continue;
        }
        if args.update_existing_only || args.require_matching_folder_path {
            let payload_object = crate::common::value_as_object(
                &payload,
                "Dashboard import payload must be a JSON object.",
            )?;
            let dashboard = payload_object
                .get("dashboard")
                .and_then(Value::as_object)
                .ok_or_else(|| message("Dashboard import payload is missing dashboard."))?;
            let uid = crate::common::string_field(dashboard, "uid", DEFAULT_UNKNOWN_UID);
            if action == Some("would-skip-missing") {
                skipped_missing_count += 1;
                if args.verbose {
                    println!(
                        "Skipped import uid={} dest=missing action=skip-missing file={}",
                        uid,
                        dashboard_file.display()
                    );
                } else if args.progress {
                    println!(
                        "Skipping dashboard {}/{}: {} dest=missing action=skip-missing",
                        index + 1,
                        total,
                        uid
                    );
                }
                continue;
            }
            if action == Some("would-skip-folder-mismatch") {
                skipped_folder_mismatch_count += 1;
                if args.verbose {
                    println!(
                        "Skipped import uid={} dest=exists action=skip-folder-mismatch sourceFolderPath={} destinationFolderPath={} file={}",
                        uid,
                        normalized_source_folder_path,
                        normalized_destination_folder_path.as_deref().unwrap_or("-"),
                        dashboard_file.display()
                    );
                } else if args.progress {
                    println!(
                        "Skipping dashboard {}/{}: {} dest=exists action=skip-folder-mismatch",
                        index + 1,
                        total,
                        uid
                    );
                }
                continue;
            }
        }
        if args.ensure_folders {
            let payload_object = crate::common::value_as_object(
                &payload,
                "Dashboard import payload must be a JSON object.",
            )?;
            let folder_uid = payload_object
                .get("folderUid")
                .and_then(Value::as_str)
                .unwrap_or("");
            if !folder_uid.is_empty() && action != Some("would-fail-existing") {
                ensure_folder_inventory_entry_cached(
                    &mut request_json,
                    &mut lookup_cache,
                    &folders_by_uid,
                    folder_uid,
                )?;
            }
        }
        let _result = import_dashboard_request_with_request(&mut request_json, &payload)?;
        imported_count += 1;
        if args.verbose {
            println!(
                "{}",
                format_import_verbose_line(dashboard_file, false, None, None, None)
            );
        } else if args.progress {
            println!(
                "{}",
                format_import_progress_line(
                    index + 1,
                    total,
                    &dashboard_file.display().to_string(),
                    false,
                    None,
                    None,
                )
            );
        }
    }
    if args.dry_run {
        if args.update_existing_only {
            skipped_missing_count = dry_run_records
                .iter()
                .filter(|record| record[2] == "skip-missing")
                .count();
        }
        skipped_folder_mismatch_count = dry_run_records
            .iter()
            .filter(|record| record[2] == "skip-folder-mismatch")
            .count();
        if args.json {
            println!(
                "{}",
                render_import_dry_run_json(
                    mode,
                    &folder_statuses,
                    &dry_run_records,
                    &args.import_dir,
                    skipped_missing_count,
                    skipped_folder_mismatch_count,
                )?
            );
        } else if args.table {
            for line in render_import_dry_run_table(
                &dry_run_records,
                !args.no_header,
                if args.output_columns.is_empty() {
                    None
                } else {
                    Some(args.output_columns.as_slice())
                },
            ) {
                println!("{line}");
            }
        }
        if args.json {
        } else if args.update_existing_only
            && skipped_missing_count > 0
            && skipped_folder_mismatch_count > 0
        {
            println!(
                "Dry-run checked {} dashboard(s) from {}; would skip {} missing dashboards and {} folder-mismatched dashboards",
                dashboard_files.len(),
                args.import_dir.display(),
                skipped_missing_count,
                skipped_folder_mismatch_count
            );
        } else if args.update_existing_only && skipped_missing_count > 0 {
            println!(
                "Dry-run checked {} dashboard(s) from {}; would skip {} missing dashboards",
                dashboard_files.len(),
                args.import_dir.display(),
                skipped_missing_count
            );
        } else if skipped_folder_mismatch_count > 0 {
            println!(
                "Dry-run checked {} dashboard(s) from {}; would skip {} folder-mismatched dashboards",
                dashboard_files.len(),
                args.import_dir.display(),
                skipped_folder_mismatch_count
            );
        } else {
            println!(
                "Dry-run checked {} dashboard(s) from {}",
                dashboard_files.len(),
                args.import_dir.display()
            );
        }
        return Ok(dashboard_files.len());
    }
    if args.update_existing_only && skipped_missing_count > 0 && skipped_folder_mismatch_count > 0 {
        println!(
            "Imported {} dashboard files from {}; skipped {} missing dashboards and {} folder-mismatched dashboards",
            imported_count,
            args.import_dir.display(),
            skipped_missing_count,
            skipped_folder_mismatch_count
        );
    } else if args.update_existing_only && skipped_missing_count > 0 {
        println!(
            "Imported {} dashboard files from {}; skipped {} missing dashboards",
            imported_count,
            args.import_dir.display(),
            skipped_missing_count
        );
    } else if skipped_folder_mismatch_count > 0 {
        println!(
            "Imported {} dashboard files from {}; skipped {} folder-mismatched dashboards",
            imported_count,
            args.import_dir.display(),
            skipped_folder_mismatch_count
        );
    }
    Ok(imported_count)
}

/// Purpose: implementation note.
pub fn import_dashboards_with_client(client: &JsonHttpClient, args: &ImportArgs) -> Result<usize> {
    import_dashboards_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        args,
    )
}

/// Purpose: implementation note.
pub(crate) fn import_dashboards_with_org_clients(args: &ImportArgs) -> Result<usize> {
    let context = super::build_import_auth_context(args)?;
    let client = JsonHttpClient::new(JsonHttpClientConfig {
        base_url: context.url.clone(),
        headers: context.headers.clone(),
        timeout_secs: context.timeout,
        verify_ssl: context.verify_ssl,
    })?;
    if !args.use_export_org {
        return import_dashboards_with_request(
            |method, path, params, payload| client.request_json(method, path, params, payload),
            args,
        );
    }
    super::super::import_routed::import_dashboards_by_export_org_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        |target_org_id, scoped_args| {
            let scoped_client = build_http_client_for_org(&args.common, target_org_id)?;
            import_dashboards_with_client(&scoped_client, scoped_args)
        },
        |target_org_id, scoped_args| {
            let scoped_client = build_http_client_for_org(&args.common, target_org_id)?;
            collect_import_dry_run_report_with_request(
                |method, path, params, payload| {
                    scoped_client.request_json(method, path, params, payload)
                },
                scoped_args,
            )
        },
        args,
    )
}
