use reqwest::Method;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use crate::common::{message, object_field, string_field, value_as_object, Result};
use crate::http::{JsonHttpClient, JsonHttpClientConfig};

use super::*;

fn validate_import_org_auth(context: &DashboardAuthContext, org_id: Option<i64>) -> Result<()> {
    if org_id.is_some() && context.auth_mode != "basic" {
        return Err(message(
            "Dashboard import with --org-id requires Basic auth (--basic-user / --basic-password).",
        ));
    }
    Ok(())
}

pub(crate) fn build_import_auth_context(args: &ImportArgs) -> Result<DashboardAuthContext> {
    let mut context = build_auth_context(&args.common)?;
    validate_import_org_auth(&context, args.org_id)?;
    if let Some(org_id) = args.org_id {
        context
            .headers
            .push(("X-Grafana-Org-Id".to_string(), org_id.to_string()));
    }
    Ok(context)
}

fn load_export_org_ids(
    import_dir: &Path,
    metadata: Option<&ExportMetadata>,
) -> Result<std::collections::BTreeSet<String>> {
    let mut org_ids = std::collections::BTreeSet::new();
    let index_file = metadata
        .map(|item| item.index_file.clone())
        .unwrap_or_else(|| "index.json".to_string());
    let index_path = import_dir.join(&index_file);
    if index_path.is_file() {
        let raw = fs::read_to_string(&index_path)?;
        let entries: Vec<VariantIndexEntry> = serde_json::from_str(&raw).map_err(|error| {
            message(format!(
                "Invalid dashboard export index in {}: {error}",
                index_path.display()
            ))
        })?;
        for entry in entries {
            let org_id = entry.org_id.trim();
            if !org_id.is_empty() {
                org_ids.insert(org_id.to_string());
            }
        }
    }

    for folder in load_folder_inventory(import_dir, metadata)? {
        let org_id = folder.org_id.trim();
        if !org_id.is_empty() {
            org_ids.insert(org_id.to_string());
        }
    }
    for datasource in load_datasource_inventory(import_dir, metadata)? {
        let org_id = datasource.org_id.trim();
        if !org_id.is_empty() {
            org_ids.insert(org_id.to_string());
        }
    }
    Ok(org_ids)
}

fn resolve_import_target_org_id_with_request<F>(
    mut request_json: F,
    args: &ImportArgs,
) -> Result<String>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if let Some(org_id) = args.org_id {
        return Ok(org_id.to_string());
    }
    let org = super::dashboard_list::fetch_current_org_with_request(&mut request_json)?;
    Ok(super::dashboard_list::org_id_value(&org)?.to_string())
}

fn validate_matching_export_org_with_request<F>(
    mut request_json: F,
    args: &ImportArgs,
    metadata: Option<&ExportMetadata>,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if !args.require_matching_export_org {
        return Ok(());
    }
    let export_org_ids = load_export_org_ids(&args.import_dir, metadata)?;
    if export_org_ids.is_empty() {
        return Err(message(
            "Cannot verify exported org for import: raw export orgId metadata was not found in index.json, folders.json, or datasources.json.",
        ));
    }
    if export_org_ids.len() > 1 {
        return Err(message(format!(
            "Cannot verify exported org for import: found multiple export orgIds ({}).",
            export_org_ids
                .into_iter()
                .collect::<Vec<String>>()
                .join(", ")
        )));
    }
    let export_org_id = export_org_ids.into_iter().next().unwrap_or_default();
    let target_org_id = resolve_import_target_org_id_with_request(&mut request_json, args)?;
    if export_org_id != target_org_id {
        return Err(message(format!(
            "Dashboard import export org mismatch: raw export orgId {export_org_id} does not match target org {target_org_id}. Use matching credentials/org selection or omit --require-matching-export-org."
        )));
    }
    Ok(())
}

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
        "would-skip-folder-mismatch" => ("exists", "skip-folder-mismatch"),
        "would-fail-existing" => ("exists", "blocked-existing"),
        _ => (DEFAULT_UNKNOWN_UID, action),
    }
}

fn normalize_folder_path(path: Option<&str>) -> String {
    let value = path.unwrap_or("").trim();
    if value.is_empty() {
        DEFAULT_FOLDER_TITLE.to_string()
    } else {
        value.to_string()
    }
}

fn resolve_source_dashboard_folder_path(
    document: &Value,
    dashboard_file: &Path,
    import_dir: &Path,
    folders_by_uid: &BTreeMap<String, FolderInventoryItem>,
) -> Result<String> {
    let document_object = value_as_object(document, "Dashboard payload must be a JSON object.")?;
    if let Some(folder_uid) = object_field(document_object, "meta")
        .and_then(|meta| meta.get("folderUid"))
        .and_then(Value::as_str)
        .map(str::trim)
    {
        if folder_uid.is_empty() || folder_uid == DEFAULT_FOLDER_UID {
            return Ok(DEFAULT_FOLDER_TITLE.to_string());
        }
        if let Some(folder) = folders_by_uid.get(folder_uid) {
            if !folder.path.is_empty() {
                return Ok(folder.path.clone());
            }
            if !folder.title.is_empty() {
                return Ok(folder.title.clone());
            }
        }
    }
    let relative = dashboard_file.strip_prefix(import_dir).map_err(|error| {
        message(format!(
            "Failed to resolve import-relative dashboard path for {}: {error}",
            dashboard_file.display()
        ))
    })?;
    let parts = relative
        .parent()
        .map(|path| {
            path.iter()
                .map(|part| part.to_string_lossy().into_owned())
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();
    if parts.is_empty() {
        Ok(DEFAULT_FOLDER_TITLE.to_string())
    } else {
        Ok(parts.join(" / "))
    }
}

fn resolve_existing_dashboard_folder_path_with_request<F>(
    mut request_json: F,
    uid: &str,
) -> Result<Option<String>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if uid.is_empty() {
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
        .trim()
        .to_string();
    if folder_uid.is_empty() || folder_uid == DEFAULT_FOLDER_UID {
        return Ok(Some(DEFAULT_FOLDER_TITLE.to_string()));
    }
    let Some(folder) = fetch_folder_if_exists_with_request(&mut request_json, &folder_uid)? else {
        return Ok(None);
    };
    let title = string_field(&folder, "title", &folder_uid);
    let path = build_folder_path(&folder, &title);
    if path.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(path))
    }
}

fn build_folder_path_match_result(
    source_folder_path: Option<&str>,
    destination_folder_path: Option<&str>,
    destination_exists: bool,
    require_matching_folder_path: bool,
) -> (bool, &'static str, String, Option<String>) {
    let normalized_source = normalize_folder_path(source_folder_path);
    let normalized_destination =
        destination_folder_path.map(|path| normalize_folder_path(Some(path)));
    if !require_matching_folder_path || !destination_exists {
        return (true, "", normalized_source, normalized_destination);
    }
    let Some(ref destination_path) = normalized_destination else {
        return (
            false,
            "folder-path-unknown",
            normalized_source,
            normalized_destination,
        );
    };
    if normalized_source == *destination_path {
        (true, "", normalized_source, normalized_destination)
    } else {
        (
            false,
            "folder-path-mismatch",
            normalized_source,
            normalized_destination,
        )
    }
}

fn apply_folder_path_guard_to_action(action: &'static str, matches: bool) -> &'static str {
    if action == "would-update" && !matches {
        "would-skip-folder-mismatch"
    } else {
        action
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
    source_folder_path: &str,
    destination_folder_path: Option<&str>,
    reason: &str,
) -> [String; 8] {
    let (destination, action_label) = describe_import_action(action);
    [
        uid.to_string(),
        destination.to_string(),
        action_label.to_string(),
        folder_path.to_string(),
        source_folder_path.to_string(),
        destination_folder_path.unwrap_or("").to_string(),
        reason.to_string(),
        dashboard_file.display().to_string(),
    ]
}

pub(crate) fn render_import_dry_run_table(
    records: &[[String; 8]],
    include_header: bool,
    selected_columns: Option<&[String]>,
) -> Vec<String> {
    let columns = resolve_dashboard_import_table_columns(records, selected_columns);
    let headers = columns
        .iter()
        .map(|(_, header)| *header)
        .collect::<Vec<&str>>();
    let mut widths = headers
        .iter()
        .map(|header| header.len())
        .collect::<Vec<usize>>();
    for row in records {
        let visible = columns
            .iter()
            .map(|(index, _)| row[*index].as_str())
            .collect::<Vec<&str>>();
        for (index, value) in visible.iter().enumerate() {
            widths[index] = widths[index].max(value.len());
        }
    }
    let format_row = |values: &[String]| -> String {
        values
            .iter()
            .enumerate()
            .map(|(index, value)| format!("{value:<width$}", width = widths[index]))
            .collect::<Vec<String>>()
            .join("  ")
    };
    let mut lines = Vec::new();
    if include_header {
        let header_values = headers
            .iter()
            .map(|item| item.to_string())
            .collect::<Vec<String>>();
        let divider_values = widths
            .iter()
            .map(|width| "-".repeat(*width))
            .collect::<Vec<String>>();
        lines.push(format_row(&header_values));
        lines.push(format_row(&divider_values));
    }
    for row in records {
        let visible = columns
            .iter()
            .map(|(index, _)| row[*index].clone())
            .collect::<Vec<String>>();
        lines.push(format_row(&visible));
    }
    lines
}

fn resolve_dashboard_import_table_columns(
    records: &[[String; 8]],
    selected_columns: Option<&[String]>,
) -> Vec<(usize, &'static str)> {
    if let Some(columns) = selected_columns {
        return columns
            .iter()
            .map(|column| match column.as_str() {
                "uid" => (0usize, "UID"),
                "destination" => (1usize, "DESTINATION"),
                "action" => (2usize, "ACTION"),
                "folder_path" => (3usize, "FOLDER_PATH"),
                "source_folder_path" => (4usize, "SOURCE_FOLDER_PATH"),
                "destination_folder_path" => (5usize, "DESTINATION_FOLDER_PATH"),
                "reason" => (6usize, "REASON"),
                "file" => (7usize, "FILE"),
                _ => unreachable!("validated dashboard import output column"),
            })
            .collect();
    }
    let include_source_folder = records.iter().any(|row| !row[4].is_empty());
    let include_destination_folder = records.iter().any(|row| !row[5].is_empty());
    let include_reason = records.iter().any(|row| !row[6].is_empty());
    let mut columns = vec![
        (0usize, "UID"),
        (1usize, "DESTINATION"),
        (2usize, "ACTION"),
        (3usize, "FOLDER_PATH"),
    ];
    if include_source_folder {
        columns.push((4usize, "SOURCE_FOLDER_PATH"));
    }
    if include_destination_folder {
        columns.push((5usize, "DESTINATION_FOLDER_PATH"));
    }
    if include_reason {
        columns.push((6usize, "REASON"));
    }
    columns.push((7usize, "FILE"));
    columns
}

pub(crate) fn render_import_dry_run_json(
    mode: &str,
    folder_statuses: &[FolderInventoryStatus],
    dashboard_records: &[[String; 8]],
    import_dir: &Path,
    skipped_missing_count: usize,
    skipped_folder_mismatch_count: usize,
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
                "sourceFolderPath": row[4],
                "destinationFolderPath": row[5],
                "reason": row[6],
                "file": row[7],
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
            "skippedFolderMismatchDashboards": skipped_folder_mismatch_count,
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
    let metadata = load_export_metadata(&args.import_dir, Some(RAW_EXPORT_SUBDIR))?;
    validate_matching_export_org_with_request(&mut request_json, args, metadata.as_ref())?;
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
    let folders_by_uid: BTreeMap<String, FolderInventoryItem> = folder_inventory
        .into_iter()
        .map(|item| (item.uid.clone(), item))
        .collect();
    let mut dashboard_files = discover_dashboard_files(&args.import_dir)?;
    dashboard_files.retain(|path| {
        path.file_name().and_then(|name| name.to_str()) != Some(FOLDER_INVENTORY_FILENAME)
    });
    let total = dashboard_files.len();
    let effective_replace_existing = args.replace_existing || args.update_existing_only;
    let mut dry_run_records: Vec<[String; 8]> = Vec::new();
    let mut imported_count = 0usize;
    let mut skipped_missing_count = 0usize;
    let mut skipped_folder_mismatch_count = 0usize;
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
        let source_folder_path = if args.require_matching_folder_path {
            Some(resolve_source_dashboard_folder_path(
                &document,
                dashboard_file,
                &args.import_dir,
                &folders_by_uid,
            )?)
        } else {
            None
        };
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
        let action = if args.dry_run
            || args.update_existing_only
            || args.ensure_folders
            || args.require_matching_folder_path
        {
            Some(determine_dashboard_import_action_with_request(
                &mut request_json,
                &payload,
                args.replace_existing,
                args.update_existing_only,
            )?)
        } else {
            None
        };
        let destination_folder_path = if args.require_matching_folder_path {
            resolve_existing_dashboard_folder_path_with_request(&mut request_json, &uid)?
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
            (true, "", String::new(), None)
        };
        let action =
            action.map(|value| apply_folder_path_guard_to_action(value, folder_paths_match));
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
                    &normalized_source_folder_path,
                    normalized_destination_folder_path.as_deref(),
                    &folder_match_reason,
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
        if args.update_existing_only || args.require_matching_folder_path {
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

pub fn import_dashboards_with_client(client: &JsonHttpClient, args: &ImportArgs) -> Result<usize> {
    import_dashboards_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        args,
    )
}

pub(crate) fn import_dashboards_with_org_clients(args: &ImportArgs) -> Result<usize> {
    let context = build_import_auth_context(args)?;
    let client = JsonHttpClient::new(JsonHttpClientConfig {
        base_url: context.url,
        headers: context.headers,
        timeout_secs: context.timeout,
        verify_ssl: context.verify_ssl,
    })?;
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
