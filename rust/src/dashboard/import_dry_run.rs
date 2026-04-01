//! Import orchestration for Dashboard resources, including input normalization and apply contract handling.

use reqwest::Method;
use serde_json::Value;
use std::collections::BTreeMap;

use crate::common::{message, string_field, value_as_object, Result};
use crate::dashboard::{
    build_import_payload, extract_dashboard_object, format_folder_inventory_status_line,
    load_export_metadata, load_folder_inventory, load_json_file, validate, FolderInventoryItem,
    FolderInventoryStatus, ImportArgs, FOLDER_INVENTORY_FILENAME, RAW_EXPORT_SUBDIR,
};

use super::super::import_lookup::{
    apply_folder_path_guard_to_action, build_folder_path_match_result,
    collect_folder_inventory_statuses_cached, determine_dashboard_import_action_with_request,
    determine_import_folder_uid_override_with_request,
    resolve_dashboard_import_folder_path_with_request,
    resolve_existing_dashboard_folder_path_with_request, ImportLookupCache,
};
use super::super::import_render::{
    build_folder_inventory_dry_run_record, build_import_dry_run_record,
    describe_dashboard_import_mode, render_folder_inventory_dry_run_table, ImportDryRunReport,
};

pub(crate) fn collect_import_dry_run_report_with_request<F>(
    mut request_json: F,
    args: &ImportArgs,
) -> Result<ImportDryRunReport>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut lookup_cache = ImportLookupCache::default();
    let metadata = load_export_metadata(&args.import_dir, Some(RAW_EXPORT_SUBDIR))?;
    super::super::import_validation::validate_matching_export_org_with_request(
        &mut request_json,
        &mut lookup_cache,
        args,
        &args.import_dir,
        metadata.as_ref(),
        None,
    )?;
    let folder_inventory = if args.ensure_folders || args.dry_run {
        load_folder_inventory(&args.import_dir, metadata.as_ref())?
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
            args.import_dir.join(folders_file).display()
        )));
    }
    let folder_statuses = if args.ensure_folders {
        collect_folder_inventory_statuses_cached(
            &mut request_json,
            &mut lookup_cache,
            &folder_inventory,
        )?
    } else {
        Vec::new()
    };
    let folders_by_uid: BTreeMap<String, FolderInventoryItem> = folder_inventory
        .into_iter()
        .map(|item| (item.uid.clone(), item))
        .collect();
    let discovered_dashboard_files = super::dashboard_files_for_import(&args.import_dir)?;
    let dashboard_files = super::selected_dashboard_files(
        &mut request_json,
        &mut lookup_cache,
        args,
        discovered_dashboard_files.clone(),
    )?
    .unwrap_or(discovered_dashboard_files);
    let effective_replace_existing = args.replace_existing || args.update_existing_only;
    let mut dashboard_records: Vec<[String; 8]> = Vec::new();
    for dashboard_file in &dashboard_files {
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
            value_as_object(&document, "Dashboard payload must be a JSON object.")?;
        let dashboard = extract_dashboard_object(document_object)?;
        let uid = string_field(dashboard, "uid", "");
        let source_folder_path = if args.require_matching_folder_path {
            Some(
                super::super::import_lookup::resolve_source_dashboard_folder_path(
                    &document,
                    dashboard_file,
                    &args.import_dir,
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
        let action = determine_dashboard_import_action_with_request(
            &mut request_json,
            &mut lookup_cache,
            &payload,
            args.replace_existing,
            args.update_existing_only,
        )?;
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
        let action = apply_folder_path_guard_to_action(action, folder_paths_match);
        let prefer_live_folder_path =
            folder_uid_override.is_some() && args.import_folder_uid.is_none() && !uid.is_empty();
        let folder_path = resolve_dashboard_import_folder_path_with_request(
            &mut request_json,
            &mut lookup_cache,
            &payload,
            &folders_by_uid,
            prefer_live_folder_path,
        )?;
        dashboard_records.push(build_import_dry_run_record(
            dashboard_file,
            &uid,
            action,
            &folder_path,
            &normalized_source_folder_path,
            normalized_destination_folder_path.as_deref(),
            folder_match_reason,
        ));
    }
    Ok(ImportDryRunReport {
        mode: describe_dashboard_import_mode(args.replace_existing, args.update_existing_only)
            .to_string(),
        import_dir: args.import_dir.clone(),
        folder_statuses,
        skipped_missing_count: if args.update_existing_only {
            dashboard_records
                .iter()
                .filter(|record| record[2] == "skip-missing")
                .count()
        } else {
            0
        },
        skipped_folder_mismatch_count: dashboard_records
            .iter()
            .filter(|record| record[2] == "skip-folder-mismatch")
            .count(),
        dashboard_records,
    })
}

pub(crate) fn folder_inventory_status_output_lines(
    folder_statuses: &[FolderInventoryStatus],
    no_header: bool,
    json_output: bool,
    table_output: bool,
) {
    if json_output {
        return;
    }
    if table_output {
        let records: Vec<[String; 6]> = folder_statuses
            .iter()
            .map(build_folder_inventory_dry_run_record)
            .collect();
        for line in render_folder_inventory_dry_run_table(&records, !no_header) {
            println!("{line}");
        }
    } else {
        for status in folder_statuses {
            println!("{}", format_folder_inventory_status_line(status));
        }
    }
}
