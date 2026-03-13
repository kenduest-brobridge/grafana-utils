use reqwest::Method;
use serde_json::{Map, Value};
use std::fmt::Write as _;
use std::path::Path;

use crate::common::{message, object_field, string_field, value_as_object, Result};
use crate::http::JsonHttpClient;

use super::*;

fn build_compare_document(dashboard: &Map<String, Value>, folder_uid: Option<&str>) -> Value {
    let mut compare = Map::new();
    compare.insert("dashboard".to_string(), Value::Object(dashboard.clone()));
    if let Some(folder_uid) = folder_uid.filter(|value| !value.is_empty()) {
        compare.insert(
            "folderUid".to_string(),
            Value::String(folder_uid.to_string()),
        );
    }
    Value::Object(compare)
}

fn build_local_compare_document(
    document: &Value,
    folder_uid_override: Option<&str>,
) -> Result<Value> {
    let payload = build_import_payload(document, folder_uid_override, false, "")?;
    let payload_object =
        value_as_object(&payload, "Dashboard import payload must be a JSON object.")?;
    let dashboard = payload_object
        .get("dashboard")
        .and_then(Value::as_object)
        .ok_or_else(|| message("Dashboard import payload is missing dashboard."))?;
    let folder_uid = payload_object.get("folderUid").and_then(Value::as_str);
    Ok(build_compare_document(dashboard, folder_uid))
}

fn build_remote_compare_document(
    payload: &Value,
    folder_uid_override: Option<&str>,
) -> Result<Value> {
    let dashboard = build_preserved_web_import_document(payload)?;
    let dashboard_object =
        value_as_object(&dashboard, "Unexpected dashboard payload from Grafana.")?;
    let payload_object = value_as_object(payload, "Unexpected dashboard payload from Grafana.")?;
    let folder_uid = folder_uid_override.or_else(|| {
        object_field(payload_object, "meta")
            .and_then(|meta| meta.get("folderUid"))
            .and_then(Value::as_str)
    });
    Ok(build_compare_document(dashboard_object, folder_uid))
}

fn serialize_compare_document(document: &Value) -> Result<String> {
    Ok(serde_json::to_string(document)?)
}

fn build_compare_diff_text(
    remote_compare: &Value,
    local_compare: &Value,
    uid: &str,
    dashboard_file: &Path,
    _context_lines: usize,
) -> Result<String> {
    let remote_pretty = serde_json::to_string_pretty(remote_compare)?;
    let local_pretty = serde_json::to_string_pretty(local_compare)?;
    let mut text = String::new();
    let _ = writeln!(&mut text, "--- grafana:{uid}");
    let _ = writeln!(&mut text, "+++ {}", dashboard_file.display());
    for line in remote_pretty.lines() {
        let _ = writeln!(&mut text, "-{line}");
    }
    for line in local_pretty.lines() {
        let _ = writeln!(&mut text, "+{line}");
    }
    Ok(text)
}

fn determine_dashboard_import_action_with_request<F>(
    mut request_json: F,
    payload: &Value,
    replace_existing: bool,
    update_existing_only: bool,
) -> Result<&'static str>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let payload_object =
        value_as_object(payload, "Dashboard import payload must be a JSON object.")?;
    let dashboard = payload_object
        .get("dashboard")
        .and_then(Value::as_object)
        .ok_or_else(|| message("Dashboard import payload is missing dashboard."))?;
    let uid = string_field(dashboard, "uid", "");
    if uid.is_empty() {
        return Ok("would-create");
    }
    if fetch_dashboard_if_exists_with_request(&mut request_json, &uid)?.is_none() {
        if update_existing_only {
            return Ok("would-skip-missing");
        }
        return Ok("would-create");
    }
    if replace_existing || update_existing_only {
        Ok("would-update")
    } else {
        Ok("would-fail-existing")
    }
}

fn determine_import_folder_uid_override_with_request<F>(
    mut request_json: F,
    uid: &str,
    folder_uid_override: Option<&str>,
    preserve_existing_folder: bool,
) -> Result<Option<String>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if let Some(value) = folder_uid_override {
        return Ok(Some(value.to_string()));
    }
    if !preserve_existing_folder || uid.is_empty() {
        return Ok(None);
    }
    let Some(existing_payload) = fetch_dashboard_if_exists_with_request(&mut request_json, uid)?
    else {
        return Ok(None);
    };
    let object = value_as_object(
        &existing_payload,
        &format!("Unexpected dashboard payload for UID {uid}."),
    )?;
    let folder_uid = object_field(object, "meta")
        .and_then(|meta| meta.get("folderUid"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    Ok(Some(folder_uid))
}

pub(crate) fn describe_dashboard_import_mode(
    replace_existing: bool,
    update_existing_only: bool,
) -> &'static str {
    if update_existing_only {
        "update-or-skip-missing"
    } else if replace_existing {
        "create-or-update"
    } else {
        "create-only"
    }
}

fn describe_import_action(action: &str) -> (&'static str, &str) {
    match action {
        "would-create" => ("missing", "create"),
        "would-update" => ("exists", "update"),
        "would-skip-missing" => ("missing", "skip-missing"),
        "would-fail-existing" => ("exists", "blocked-existing"),
        _ => (DEFAULT_UNKNOWN_UID, action),
    }
}

fn resolve_dashboard_import_folder_path_with_request<F>(
    mut request_json: F,
    payload: &Value,
    folders_by_uid: &std::collections::BTreeMap<String, FolderInventoryItem>,
) -> Result<String>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let payload_object =
        value_as_object(payload, "Dashboard import payload must be a JSON object.")?;
    let folder_uid = payload_object
        .get("folderUid")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if folder_uid.is_empty() || folder_uid == DEFAULT_FOLDER_UID {
        return Ok(DEFAULT_FOLDER_TITLE.to_string());
    }
    if let Some(folder) = fetch_folder_if_exists_with_request(&mut request_json, &folder_uid)? {
        let fallback_title = string_field(&folder, "title", &folder_uid);
        return Ok(build_folder_path(&folder, &fallback_title));
    }
    if let Some(folder) = folders_by_uid.get(&folder_uid) {
        if !folder.path.is_empty() {
            return Ok(folder.path.clone());
        }
        if !folder.title.is_empty() {
            return Ok(folder.title.clone());
        }
    }
    Ok(folder_uid)
}

fn build_import_dry_run_record(
    dashboard_file: &Path,
    uid: &str,
    action: &str,
    folder_path: &str,
) -> [String; 5] {
    let (destination, action_label) = describe_import_action(action);
    [
        uid.to_string(),
        destination.to_string(),
        action_label.to_string(),
        folder_path.to_string(),
        dashboard_file.display().to_string(),
    ]
}

pub(crate) fn render_import_dry_run_table(
    records: &[[String; 5]],
    include_header: bool,
) -> Vec<String> {
    let headers = ["UID", "DESTINATION", "ACTION", "FOLDER_PATH", "FILE"];
    let mut widths = headers.map(str::len);
    for row in records {
        for (index, value) in row.iter().enumerate() {
            widths[index] = widths[index].max(value.len());
        }
    }
    let format_row = |values: &[String; 5]| -> String {
        values
            .iter()
            .enumerate()
            .map(|(index, value)| format!("{value:<width$}", width = widths[index]))
            .collect::<Vec<String>>()
            .join("  ")
    };
    let mut lines = Vec::new();
    if include_header {
        let header_values = [
            headers[0].to_string(),
            headers[1].to_string(),
            headers[2].to_string(),
            headers[3].to_string(),
            headers[4].to_string(),
        ];
        let divider_values = [
            "-".repeat(widths[0]),
            "-".repeat(widths[1]),
            "-".repeat(widths[2]),
            "-".repeat(widths[3]),
            "-".repeat(widths[4]),
        ];
        lines.push(format_row(&header_values));
        lines.push(format_row(&divider_values));
    }
    for row in records {
        lines.push(format_row(row));
    }
    lines
}

pub(crate) fn render_import_dry_run_json(
    mode: &str,
    folder_statuses: &[FolderInventoryStatus],
    dashboard_records: &[[String; 5]],
    import_dir: &Path,
    skipped_missing_count: usize,
) -> Result<String> {
    let mut folders = Vec::new();
    for status in folder_statuses {
        let (destination, status_label, reason) = match status.kind {
            FolderInventoryStatusKind::Missing => {
                ("missing", "missing", "would-create".to_string())
            }
            FolderInventoryStatusKind::Matches => ("exists", "match", String::new()),
            FolderInventoryStatusKind::Mismatch => {
                let mut reasons = Vec::new();
                if status.actual_title.as_deref() != Some(status.expected_title.as_str()) {
                    reasons.push("title");
                }
                if status.actual_parent_uid != status.expected_parent_uid {
                    reasons.push("parentUid");
                }
                if status.actual_path.as_deref() != Some(status.expected_path.as_str()) {
                    reasons.push("path");
                }
                ("exists", "mismatch", reasons.join(","))
            }
        };
        folders.push(serde_json::json!({
            "uid": status.uid,
            "destination": destination,
            "status": status_label,
            "reason": reason,
            "expectedPath": status.expected_path,
            "actualPath": status.actual_path.clone().unwrap_or_default(),
        }));
    }
    let dashboards = dashboard_records
        .iter()
        .map(|row| {
            serde_json::json!({
                "uid": row[0],
                "destination": row[1],
                "action": row[2],
                "folderPath": row[3],
                "file": row[4],
            })
        })
        .collect::<Vec<Value>>();
    let payload = serde_json::json!({
        "mode": mode,
        "folders": folders,
        "dashboards": dashboards,
        "summary": {
            "importDir": import_dir.display().to_string(),
            "folderCount": folder_statuses.len(),
            "missingFolders": folder_statuses.iter().filter(|status| status.kind == FolderInventoryStatusKind::Missing).count(),
            "mismatchedFolders": folder_statuses.iter().filter(|status| status.kind == FolderInventoryStatusKind::Mismatch).count(),
            "dashboardCount": dashboard_records.len(),
            "missingDashboards": dashboard_records.iter().filter(|row| row[1] == "missing").count(),
            "skippedMissingDashboards": skipped_missing_count,
        }
    });
    Ok(serde_json::to_string_pretty(&payload)?)
}

pub(crate) fn format_import_progress_line(
    current: usize,
    total: usize,
    dashboard_target: &str,
    dry_run: bool,
    action: Option<&str>,
    folder_path: Option<&str>,
) -> String {
    if dry_run {
        let (destination, action_label) =
            describe_import_action(action.unwrap_or(DEFAULT_UNKNOWN_UID));
        let mut line = format!(
            "Dry-run dashboard {current}/{total}: {dashboard_target} dest={destination} action={action_label}"
        );
        if let Some(path) = folder_path.filter(|value| !value.is_empty()) {
            let _ = write!(&mut line, " folderPath={path}");
        }
        line
    } else {
        format!("Importing dashboard {current}/{total}: {dashboard_target}")
    }
}

pub(crate) fn format_import_verbose_line(
    dashboard_file: &Path,
    dry_run: bool,
    uid: Option<&str>,
    action: Option<&str>,
    folder_path: Option<&str>,
) -> String {
    if dry_run {
        let (destination, action_label) =
            describe_import_action(action.unwrap_or(DEFAULT_UNKNOWN_UID));
        let mut line = format!(
            "Dry-run import uid={} dest={} action={} file={}",
            uid.unwrap_or(DEFAULT_UNKNOWN_UID),
            destination,
            action_label,
            dashboard_file.display()
        );
        if let Some(path) = folder_path.filter(|value| !value.is_empty()) {
            line = format!(
                "Dry-run import uid={} dest={} action={} folderPath={} file={}",
                uid.unwrap_or(DEFAULT_UNKNOWN_UID),
                destination,
                action_label,
                path,
                dashboard_file.display()
            );
        }
        line
    } else {
        format!("Imported {}", dashboard_file.display())
    }
}

pub(crate) fn import_dashboards_with_request<F>(
    mut request_json: F,
    args: &ImportArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
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
    if args.ensure_folders && args.import_folder_uid.is_some() {
        return Err(message(
            "--ensure-folders cannot be combined with --import-folder-uid.",
        ));
    }
    let metadata = load_export_metadata(&args.import_dir, Some(RAW_EXPORT_SUBDIR))?;
    let folder_inventory = if args.ensure_folders {
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
    let folder_statuses = if args.dry_run && args.ensure_folders {
        collect_folder_inventory_statuses_with_request(&mut request_json, &folder_inventory)?
    } else {
        Vec::new()
    };
    let folders_by_uid: std::collections::BTreeMap<String, FolderInventoryItem> = folder_inventory
        .into_iter()
        .map(|item| (item.uid.clone(), item))
        .collect();
    let mut dashboard_files = discover_dashboard_files(&args.import_dir)?;
    dashboard_files.retain(|path| {
        path.file_name().and_then(|name| name.to_str()) != Some(FOLDER_INVENTORY_FILENAME)
    });
    let total = dashboard_files.len();
    let effective_replace_existing = args.replace_existing || args.update_existing_only;
    let mut dry_run_records: Vec<[String; 5]> = Vec::new();
    let mut imported_count = 0usize;
    let mut skipped_missing_count = 0usize;
    let mode = describe_dashboard_import_mode(args.replace_existing, args.update_existing_only);
    if !args.json {
        println!("Import mode: {}", mode);
    }
    if args.dry_run && args.ensure_folders {
        let folder_dry_run_records: Vec<[String; 6]> = folder_statuses
            .iter()
            .map(build_folder_inventory_dry_run_record)
            .collect();
        if args.json {
        } else if args.table {
            for line in
                render_folder_inventory_dry_run_table(&folder_dry_run_records, !args.no_header)
            {
                println!("{line}");
            }
        } else {
            for status in &folder_statuses {
                println!("{}", format_folder_inventory_status_line(status));
            }
        }
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
            .unwrap_or(FOLDER_INVENTORY_FILENAME);
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
        if dashboard_file.file_name().and_then(|name| name.to_str())
            == Some(FOLDER_INVENTORY_FILENAME)
        {
            continue;
        }
        let document = load_json_file(dashboard_file)?;
        let document_object =
            value_as_object(&document, "Dashboard payload must be a JSON object.")?;
        let dashboard = extract_dashboard_object(document_object)?;
        let uid = string_field(dashboard, "uid", "");
        let folder_uid_override = determine_import_folder_uid_override_with_request(
            &mut request_json,
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
        let action = if args.dry_run || args.update_existing_only || args.ensure_folders {
            Some(determine_dashboard_import_action_with_request(
                &mut request_json,
                &payload,
                args.replace_existing,
                args.update_existing_only,
            )?)
        } else {
            None
        };
        if args.dry_run {
            let folder_path = resolve_dashboard_import_folder_path_with_request(
                &mut request_json,
                &payload,
                &folders_by_uid,
            )?;
            let payload_object =
                value_as_object(&payload, "Dashboard import payload must be a JSON object.")?;
            let dashboard = payload_object
                .get("dashboard")
                .and_then(Value::as_object)
                .ok_or_else(|| message("Dashboard import payload is missing dashboard."))?;
            let uid = string_field(dashboard, "uid", DEFAULT_UNKNOWN_UID);
            if args.table || args.json {
                dry_run_records.push(build_import_dry_run_record(
                    dashboard_file,
                    &uid,
                    action.unwrap_or(DEFAULT_UNKNOWN_UID),
                    &folder_path,
                ));
            } else if args.verbose {
                println!(
                    "{}",
                    format_import_verbose_line(
                        dashboard_file,
                        true,
                        Some(&uid),
                        Some(action.unwrap_or(DEFAULT_UNKNOWN_UID)),
                        Some(&folder_path),
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
                        Some(&folder_path),
                    )
                );
            }
            continue;
        }
        if args.update_existing_only {
            let payload_object =
                value_as_object(&payload, "Dashboard import payload must be a JSON object.")?;
            let dashboard = payload_object
                .get("dashboard")
                .and_then(Value::as_object)
                .ok_or_else(|| message("Dashboard import payload is missing dashboard."))?;
            let uid = string_field(dashboard, "uid", DEFAULT_UNKNOWN_UID);
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
        }
        if args.ensure_folders {
            let payload_object =
                value_as_object(&payload, "Dashboard import payload must be a JSON object.")?;
            let folder_uid = payload_object
                .get("folderUid")
                .and_then(Value::as_str)
                .unwrap_or("");
            if !folder_uid.is_empty() && action != Some("would-fail-existing") {
                ensure_folder_inventory_entry_with_request(
                    &mut request_json,
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
        if args.json {
            println!(
                "{}",
                render_import_dry_run_json(
                    mode,
                    &folder_statuses,
                    &dry_run_records,
                    &args.import_dir,
                    skipped_missing_count,
                )?
            );
        } else if args.table {
            for line in render_import_dry_run_table(&dry_run_records, !args.no_header) {
                println!("{line}");
            }
        }
        if args.json {
        } else if args.update_existing_only && skipped_missing_count > 0 {
            println!(
                "Dry-run checked {} dashboard(s) from {}; would skip {} missing dashboards",
                dashboard_files.len(),
                args.import_dir.display(),
                skipped_missing_count
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
    if args.update_existing_only && skipped_missing_count > 0 {
        println!(
            "Imported {} dashboard files from {}; skipped {} missing dashboards",
            imported_count,
            args.import_dir.display(),
            skipped_missing_count
        );
    }
    Ok(imported_count)
}

pub fn import_dashboards_with_client(client: &JsonHttpClient, args: &ImportArgs) -> Result<usize> {
    import_dashboards_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        args,
    )
}

pub(crate) fn diff_dashboards_with_request<F>(mut request_json: F, args: &DiffArgs) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let _ = load_export_metadata(&args.import_dir, Some(RAW_EXPORT_SUBDIR))?;
    let dashboard_files = discover_dashboard_files(&args.import_dir)?;
    let mut differences = 0;
    for dashboard_file in &dashboard_files {
        let document = load_json_file(dashboard_file)?;
        let payload = build_import_payload(&document, None, false, "")?;
        let payload_object =
            value_as_object(&payload, "Dashboard import payload must be a JSON object.")?;
        let dashboard = payload_object
            .get("dashboard")
            .and_then(Value::as_object)
            .ok_or_else(|| message("Dashboard import payload is missing dashboard."))?;
        let uid = string_field(dashboard, "uid", "");
        let local_compare =
            build_local_compare_document(&document, args.import_folder_uid.as_deref())?;
        let Some(remote_payload) = fetch_dashboard_if_exists_with_request(&mut request_json, &uid)?
        else {
            println!(
                "Diff missing in Grafana for uid={} from {}",
                uid,
                dashboard_file.display()
            );
            differences += 1;
            continue;
        };
        let remote_compare =
            build_remote_compare_document(&remote_payload, args.import_folder_uid.as_deref())?;
        if serialize_compare_document(&local_compare)?
            != serialize_compare_document(&remote_compare)?
        {
            let diff_text = build_compare_diff_text(
                &remote_compare,
                &local_compare,
                &uid,
                dashboard_file,
                args.context_lines,
            )?;
            println!("{diff_text}");
            differences += 1;
        } else {
            println!("Diff matched uid={} for {}", uid, dashboard_file.display());
        }
    }
    println!(
        "Diff checked {} dashboard(s); {} difference(s) found.",
        dashboard_files.len(),
        differences
    );
    Ok(differences)
}

pub fn diff_dashboards_with_client(client: &JsonHttpClient, args: &DiffArgs) -> Result<usize> {
    diff_dashboards_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        args,
    )
}
