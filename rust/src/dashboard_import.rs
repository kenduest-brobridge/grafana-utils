//! Import orchestration for dashboards.
//! Loads local export artifacts, computes target orgs, and applies idempotent upsert behavior
//! through the shared dashboard HTTP/auth context.
use reqwest::Method;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use crate::common::{message, object_field, string_field, value_as_object, Result};
use crate::http::{JsonHttpClient, JsonHttpClientConfig};

use super::*;

#[derive(Default)]
struct ImportLookupCache {
    dashboards_by_uid: BTreeMap<String, Option<Value>>,
    folders_by_uid: BTreeMap<String, Option<Map<String, Value>>>,
    current_org_id: Option<String>,
    orgs: Option<Vec<Map<String, Value>>>,
}

fn fetch_dashboard_if_exists_cached<F>(
    request_json: &mut F,
    cache: &mut ImportLookupCache,
    uid: &str,
) -> Result<Option<Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if uid.is_empty() {
        return Ok(None);
    }
    if let Some(cached) = cache.dashboards_by_uid.get(uid) {
        return Ok(cached.clone());
    }
    let fetched = fetch_dashboard_if_exists_with_request(&mut *request_json, uid)?;
    cache
        .dashboards_by_uid
        .insert(uid.to_string(), fetched.clone());
    Ok(fetched)
}

fn fetch_folder_if_exists_cached<F>(
    request_json: &mut F,
    cache: &mut ImportLookupCache,
    uid: &str,
) -> Result<Option<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if uid.is_empty() {
        return Ok(None);
    }
    if let Some(cached) = cache.folders_by_uid.get(uid) {
        return Ok(cached.clone());
    }
    let fetched = fetch_folder_if_exists_with_request(&mut *request_json, uid)?;
    cache
        .folders_by_uid
        .insert(uid.to_string(), fetched.clone());
    Ok(fetched)
}

fn create_folder_entry_with_request<F>(
    request_json: &mut F,
    title: &str,
    uid: &str,
    parent_uid: Option<&str>,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut payload = Map::new();
    payload.insert("uid".to_string(), Value::String(uid.to_string()));
    payload.insert("title".to_string(), Value::String(title.to_string()));
    if let Some(parent_uid) = parent_uid.filter(|value| !value.is_empty()) {
        payload.insert(
            "parentUid".to_string(),
            Value::String(parent_uid.to_string()),
        );
    }
    let _ = request_json(
        Method::POST,
        "/api/folders",
        &[],
        Some(&Value::Object(payload)),
    )?;
    Ok(())
}

fn cached_parent_uid_from_folder(folder: &Map<String, Value>) -> Option<String> {
    folder
        .get("parents")
        .and_then(Value::as_array)
        .and_then(|parents| parents.last())
        .and_then(Value::as_object)
        .map(|parent| string_field(parent, "uid", ""))
        .filter(|uid| !uid.is_empty())
}

fn build_cached_folder_inventory_status(
    folder: &FolderInventoryItem,
    destination_folder: Option<&Map<String, Value>>,
) -> FolderInventoryStatus {
    let expected_parent_uid = folder.parent_uid.clone();
    let mut status = FolderInventoryStatus {
        uid: folder.uid.clone(),
        expected_title: folder.title.clone(),
        expected_parent_uid,
        expected_path: folder.path.clone(),
        actual_title: None,
        actual_parent_uid: None,
        actual_path: None,
        kind: FolderInventoryStatusKind::Missing,
    };
    let Some(destination_folder) = destination_folder else {
        return status;
    };

    status.actual_title = Some(string_field(destination_folder, "title", ""));
    status.actual_parent_uid = cached_parent_uid_from_folder(destination_folder);
    status.actual_path = Some(build_folder_path(destination_folder, &folder.title));
    let title_matches = status.actual_title.as_deref() == Some(folder.title.as_str());
    let parent_matches = status.actual_parent_uid == folder.parent_uid;
    let path_matches = status.actual_path.as_deref() == Some(folder.path.as_str());
    status.kind = if title_matches && parent_matches && path_matches {
        FolderInventoryStatusKind::Matches
    } else {
        FolderInventoryStatusKind::Mismatch
    };
    status
}

fn validate_import_org_auth(context: &DashboardAuthContext, args: &ImportArgs) -> Result<()> {
    if args.org_id.is_some() && context.auth_mode != "basic" {
        return Err(message(
            "Dashboard import with --org-id requires Basic auth (--basic-user / --basic-password).",
        ));
    }
    if args.use_export_org && context.auth_mode != "basic" {
        return Err(message(
            "Dashboard import with --use-export-org requires Basic auth (--basic-user / --basic-password).",
        ));
    }
    Ok(())
}

/// Purpose: implementation note.
pub(crate) fn build_import_auth_context(args: &ImportArgs) -> Result<DashboardAuthContext> {
    let mut context = build_auth_context(&args.common)?;
    validate_import_org_auth(&context, args)?;
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

fn load_export_org_names(
    import_dir: &Path,
    metadata: Option<&ExportMetadata>,
) -> Result<std::collections::BTreeSet<String>> {
    let mut org_names = std::collections::BTreeSet::new();
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
            let org_name = entry.org.trim();
            if !org_name.is_empty() {
                org_names.insert(org_name.to_string());
            }
        }
    }

    for folder in load_folder_inventory(import_dir, metadata)? {
        let org_name = folder.org.trim();
        if !org_name.is_empty() {
            org_names.insert(org_name.to_string());
        }
    }
    for datasource in load_datasource_inventory(import_dir, metadata)? {
        let org_name = datasource.org.trim();
        if !org_name.is_empty() {
            org_names.insert(org_name.to_string());
        }
    }
    Ok(org_names)
}

#[derive(Debug, Clone)]
struct ExportOrgImportScope {
    source_org_id: i64,
    source_org_name: String,
    import_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExportOrgTargetPlan {
    source_org_id: i64,
    source_org_name: String,
    target_org_id: Option<i64>,
    org_action: &'static str,
    import_dir: PathBuf,
}

/// Struct definition for ImportDryRunReport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ImportDryRunReport {
    pub mode: String,
    pub import_dir: PathBuf,
    pub folder_statuses: Vec<FolderInventoryStatus>,
    pub dashboard_records: Vec<[String; 8]>,
    pub skipped_missing_count: usize,
    pub skipped_folder_mismatch_count: usize,
}

fn org_id_string_from_value(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.trim().to_string(),
        Some(Value::Number(number)) => number.to_string(),
        _ => String::new(),
    }
}

fn parse_export_org_scope(import_root: &Path, raw_dir: &Path) -> Result<ExportOrgImportScope> {
// Call graph (hierarchy): this function is used in related modules.
// Upstream callers: dashboard_import.rs:discover_export_org_import_scopes
// Downstream callees: common.rs:message, dashboard_import.rs:load_export_org_ids, dashboard_import.rs:load_export_org_names

    let metadata = load_export_metadata(raw_dir, Some(RAW_EXPORT_SUBDIR))?;
    let export_org_ids = load_export_org_ids(raw_dir, metadata.as_ref())?;
    if export_org_ids.is_empty() {
        return Err(message(format!(
            "Cannot route import by export org for {}: raw export orgId metadata was not found in index.json, folders.json, or datasources.json.",
            raw_dir.display()
        )));
    }
    if export_org_ids.len() > 1 {
        return Err(message(format!(
            "Cannot route import by export org for {}: found multiple export orgIds ({}).",
            raw_dir.display(),
            export_org_ids
                .into_iter()
                .collect::<Vec<String>>()
                .join(", ")
        )));
    }
    let source_org_id_text = export_org_ids.into_iter().next().unwrap_or_default();
    let source_org_id = source_org_id_text.parse::<i64>().map_err(|_| {
        message(format!(
            "Cannot route import by export org for {}: export orgId '{}' is not a valid integer.",
            raw_dir.display(),
            source_org_id_text
        ))
    })?;
    let org_names = load_export_org_names(raw_dir, metadata.as_ref())?;
    if org_names.len() > 1 {
        return Err(message(format!(
            "Cannot route import by export org for {}: found multiple export org names ({}).",
            raw_dir.display(),
            org_names.into_iter().collect::<Vec<String>>().join(", ")
        )));
    }
    let source_org_name = org_names.into_iter().next().unwrap_or_else(|| {
        import_root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("org")
            .to_string()
    });
    Ok(ExportOrgImportScope {
        source_org_id,
        source_org_name,
        import_dir: raw_dir.to_path_buf(),
    })
}

fn discover_export_org_import_scopes(args: &ImportArgs) -> Result<Vec<ExportOrgImportScope>> {
    if !args.use_export_org {
        return Ok(Vec::new());
    }
    let selected_org_ids: std::collections::BTreeSet<i64> =
        args.only_org_id.iter().copied().collect();
    let mut scopes = Vec::new();
    for entry in fs::read_dir(&args.import_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|item| item.to_str()) else {
            continue;
        };
        if !name.starts_with("org_") {
            continue;
        }
        let raw_dir = path.join(RAW_EXPORT_SUBDIR);
        if !raw_dir.is_dir() {
            continue;
        }
        let scope = parse_export_org_scope(&path, &raw_dir)?;
        if !selected_org_ids.is_empty() && !selected_org_ids.contains(&scope.source_org_id) {
            continue;
        }
        scopes.push(scope);
    }
    scopes.sort_by(|left, right| left.source_org_id.cmp(&right.source_org_id));
    if scopes.is_empty() {
        if args.import_dir.join(EXPORT_METADATA_FILENAME).is_file() {
            return Err(message(
                "Dashboard import with --use-export-org expects the combined export root, not one raw/ export directory.",
            ));
        }
        if selected_org_ids.is_empty() {
            return Err(message(format!(
                "Dashboard import with --use-export-org did not find any org-specific raw exports under {}.",
                args.import_dir.display()
            )));
        }
        return Err(message(format!(
            "Dashboard import with --use-export-org did not find the selected exported org IDs ({}) under {}.",
            selected_org_ids
                .into_iter()
                .map(|id| id.to_string())
                .collect::<Vec<String>>()
                .join(", "),
            args.import_dir.display()
        )));
    }
    let found_org_ids: std::collections::BTreeSet<i64> =
        scopes.iter().map(|scope| scope.source_org_id).collect();
    let missing_org_ids: Vec<String> = selected_org_ids
        .difference(&found_org_ids)
        .map(|id| id.to_string())
        .collect();
    if !missing_org_ids.is_empty() {
        return Err(message(format!(
            "Dashboard import with --use-export-org did not find the selected exported org IDs ({}).",
            missing_org_ids.join(", ")
        )));
    }
    Ok(scopes)
}

fn create_org_with_request<F>(mut request_json: F, org_name: &str) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match request_json(
        Method::POST,
        "/api/orgs",
        &[],
        Some(&Value::Object(Map::from_iter(vec![(
            "name".to_string(),
            Value::String(org_name.to_string()),
        )]))),
    )? {
        Some(Value::Object(object)) => Ok(object),
        _ => Err(message(
            "Unexpected organization create response from Grafana during dashboard import.",
        )),
    }
}

fn resolve_import_target_org_id_with_request<F>(
    request_json: &mut F,
    cache: &mut ImportLookupCache,
    args: &ImportArgs,
) -> Result<String>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if let Some(org_id) = args.org_id {
        return Ok(org_id.to_string());
    }
    if let Some(org_id) = cache.current_org_id.as_ref() {
        return Ok(org_id.clone());
    }
    let org = super::dashboard_list::fetch_current_org_with_request(&mut *request_json)?;
    let org_id = super::dashboard_list::org_id_value(&org)?.to_string();
    cache.current_org_id = Some(org_id.clone());
    Ok(org_id)
}

fn list_orgs_cached<F>(
    request_json: &mut F,
    cache: &mut ImportLookupCache,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if let Some(orgs) = cache.orgs.as_ref() {
        return Ok(orgs.clone());
    }
    let orgs = super::dashboard_list::list_orgs_with_request(&mut *request_json)?;
    cache.orgs = Some(orgs.clone());
    Ok(orgs)
}

fn validate_matching_export_org_with_request<F>(
    request_json: &mut F,
    cache: &mut ImportLookupCache,
    args: &ImportArgs,
    import_dir: &Path,
    metadata: Option<&ExportMetadata>,
    target_org_id_override: Option<i64>,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if !args.require_matching_export_org {
        return Ok(());
    }
    let export_org_ids = load_export_org_ids(import_dir, metadata)?;
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
    let target_org_id = match target_org_id_override {
        Some(org_id) => org_id.to_string(),
        None => resolve_import_target_org_id_with_request(request_json, cache, args)?,
    };
    if export_org_id != target_org_id {
        return Err(message(format!(
            "Dashboard import export org mismatch: raw export orgId {export_org_id} does not match target org {target_org_id}. Use matching credentials/org selection or omit --require-matching-export-org."
        )));
    }
    Ok(())
}

fn resolve_target_org_plan_for_export_scope_with_request<F>(
    request_json: &mut F,
    cache: &mut ImportLookupCache,
    args: &ImportArgs,
    scope: &ExportOrgImportScope,
) -> Result<ExportOrgTargetPlan>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let orgs = list_orgs_cached(request_json, cache)?;
    for org in &orgs {
        let org_id_text = org_id_string_from_value(org.get("id"));
        if org_id_text == scope.source_org_id.to_string() {
            return Ok(ExportOrgTargetPlan {
                source_org_id: scope.source_org_id,
                source_org_name: scope.source_org_name.clone(),
                target_org_id: Some(scope.source_org_id),
                org_action: "exists",
                import_dir: scope.import_dir.clone(),
            });
        }
    }
    if args.dry_run && !args.create_missing_orgs {
        return Ok(ExportOrgTargetPlan {
            source_org_id: scope.source_org_id,
            source_org_name: scope.source_org_name.clone(),
            target_org_id: None,
            org_action: "missing",
            import_dir: scope.import_dir.clone(),
        });
    }
    if !args.create_missing_orgs {
        return Err(message(format!(
            "Dashboard import could not find destination Grafana org {} ({}) for --use-export-org. Use --create-missing-orgs to create it first.",
            scope.source_org_id, scope.source_org_name
        )));
    }
    if scope.source_org_name.trim().is_empty() {
        return Err(message(format!(
            "Dashboard import with --create-missing-orgs could not determine an exported org name for source orgId {}.",
            scope.source_org_id
        )));
    }
    if args.dry_run {
        return Ok(ExportOrgTargetPlan {
            source_org_id: scope.source_org_id,
            source_org_name: scope.source_org_name.clone(),
            target_org_id: None,
            org_action: "would-create",
            import_dir: scope.import_dir.clone(),
        });
    }
    let created = create_org_with_request(&mut *request_json, &scope.source_org_name)?;
    let created_org_id =
        org_id_string_from_value(created.get("orgId").or_else(|| created.get("id")));
    if created_org_id.is_empty() {
        return Err(message(format!(
            "Grafana did not return a usable orgId after creating destination org '{}' for exported org {}.",
            scope.source_org_name, scope.source_org_id
        )));
    }
    let parsed_org_id = created_org_id.parse::<i64>().map_err(|_| {
        message(format!(
            "Grafana returned non-numeric orgId '{}' after creating destination org '{}' for exported org {}.",
            created_org_id, scope.source_org_name, scope.source_org_id
        ))
    })?;
    Ok(ExportOrgTargetPlan {
        source_org_id: scope.source_org_id,
        source_org_name: scope.source_org_name.clone(),
        target_org_id: Some(parsed_org_id),
        org_action: "created",
        import_dir: scope.import_dir.clone(),
    })
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
    request_json: &mut F,
    cache: &mut ImportLookupCache,
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
    if fetch_dashboard_if_exists_cached(request_json, cache, &uid)?.is_none() {
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
    request_json: &mut F,
    cache: &mut ImportLookupCache,
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
    let Some(existing_payload) = fetch_dashboard_if_exists_cached(request_json, cache, uid)? else {
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

/// describe dashboard import mode.
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

// Normalize a dashboard folder path from optional CLI input, defaulting to the
// CLI convention "General" when empty or unset.
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
    request_json: &mut F,
    cache: &mut ImportLookupCache,
    uid: &str,
) -> Result<Option<String>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if uid.is_empty() {
        return Ok(None);
    }
    let Some(existing_payload) = fetch_dashboard_if_exists_cached(request_json, cache, uid)? else {
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
    let Some(folder) = fetch_folder_if_exists_cached(request_json, cache, &folder_uid)? else {
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
    request_json: &mut F,
    cache: &mut ImportLookupCache,
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
    if let Some(folder) = fetch_folder_if_exists_cached(request_json, cache, &folder_uid)? {
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

fn collect_folder_inventory_statuses_cached<F>(
    request_json: &mut F,
    cache: &mut ImportLookupCache,
    folder_inventory: &[FolderInventoryItem],
) -> Result<Vec<FolderInventoryStatus>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut statuses = Vec::new();
    for folder in folder_inventory {
        let destination_folder = fetch_folder_if_exists_cached(request_json, cache, &folder.uid)?;
        statuses.push(build_cached_folder_inventory_status(
            folder,
            destination_folder.as_ref(),
        ));
    }
    Ok(statuses)
}

fn ensure_folder_inventory_entry_cached<F>(
    request_json: &mut F,
    cache: &mut ImportLookupCache,
    folders_by_uid: &BTreeMap<String, FolderInventoryItem>,
    folder_uid: &str,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if folder_uid.is_empty() {
        return Ok(());
    }
    let mut create_chain = Vec::new();
    let mut current_uid = folder_uid.to_string();
    loop {
        if fetch_folder_if_exists_cached(request_json, cache, &current_uid)?.is_some() {
            break;
        }
        let folder = folders_by_uid.get(&current_uid).ok_or_else(|| {
            message(format!(
                "Missing exported folder inventory for folderUid {current_uid}."
            ))
        })?;
        create_chain.push(folder.clone());
        let Some(parent_uid) = folder.parent_uid.as_deref() else {
            break;
        };
        current_uid = parent_uid.to_string();
    }
    for folder in create_chain.into_iter().rev() {
        if fetch_folder_if_exists_cached(request_json, cache, &folder.uid)?.is_some() {
            continue;
        }
        create_folder_entry_with_request(
            &mut *request_json,
            &folder.title,
            &folder.uid,
            folder.parent_uid.as_deref(),
        )?;
        let mut created = Map::new();
        created.insert("uid".to_string(), Value::String(folder.uid.clone()));
        created.insert("title".to_string(), Value::String(folder.title.clone()));
        if let Some(parent_uid) = folder.parent_uid.as_ref() {
            let parents = if let Some(parent_folder) =
                fetch_folder_if_exists_cached(request_json, cache, parent_uid)?
            {
                let parent_title = string_field(&parent_folder, "title", parent_uid);
                vec![Value::Object(Map::from_iter(vec![
                    ("uid".to_string(), Value::String(parent_uid.clone())),
                    ("title".to_string(), Value::String(parent_title)),
                ]))]
            } else {
                vec![Value::Object(Map::from_iter(vec![
                    ("uid".to_string(), Value::String(parent_uid.clone())),
                    ("title".to_string(), Value::String(parent_uid.clone())),
                ]))]
            };
            created.insert("parents".to_string(), Value::Array(parents));
        } else {
            created.insert("parents".to_string(), Value::Array(Vec::new()));
        }
        cache
            .folders_by_uid
            .insert(folder.uid.clone(), Some(created));
    }
    Ok(())
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

fn build_folder_inventory_dry_run_record(status: &FolderInventoryStatus) -> [String; 6] {
    let destination = match status.kind {
        FolderInventoryStatusKind::Missing => "missing",
        _ => "exists",
    };
    let reason = match status.kind {
        FolderInventoryStatusKind::Missing => "would-create".to_string(),
        FolderInventoryStatusKind::Matches => String::new(),
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
            reasons.join(",")
        }
    };
    [
        status.uid.clone(),
        destination.to_string(),
        match status.kind {
            FolderInventoryStatusKind::Missing => "missing",
            FolderInventoryStatusKind::Matches => "match",
            FolderInventoryStatusKind::Mismatch => "mismatch",
        }
        .to_string(),
        reason,
        status.expected_path.clone(),
        status.actual_path.clone().unwrap_or_default(),
    ]
}

/// Purpose: implementation note.
pub(crate) fn render_folder_inventory_dry_run_table(
    records: &[[String; 6]],
    include_header: bool,
) -> Vec<String> {
    let headers = [
        "UID",
        "DESTINATION",
        "STATUS",
        "REASON",
        "EXPECTED_PATH",
        "ACTUAL_PATH",
    ];
    let mut widths = headers.map(str::len);
    for row in records {
        for (index, value) in row.iter().enumerate() {
            widths[index] = widths[index].max(value.len());
        }
    }
    let format_row = |values: &[String; 6]| -> String {
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
            headers[5].to_string(),
        ];
        let divider_values = [
            "-".repeat(widths[0]),
            "-".repeat(widths[1]),
            "-".repeat(widths[2]),
            "-".repeat(widths[3]),
            "-".repeat(widths[4]),
            "-".repeat(widths[5]),
        ];
        lines.push(format_row(&header_values));
        lines.push(format_row(&divider_values));
    }
    for row in records {
        lines.push(format_row(row));
    }
    lines
}

/// Purpose: implementation note.
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

fn build_routed_import_org_row(plan: &ExportOrgTargetPlan, dashboard_count: usize) -> [String; 5] {
    [
        plan.source_org_id.to_string(),
        if plan.source_org_name.is_empty() {
            "-".to_string()
        } else {
            plan.source_org_name.clone()
        },
        plan.org_action.to_string(),
        plan.target_org_id
            .map(|value| value.to_string())
            .unwrap_or_else(|| "<new>".to_string()),
        dashboard_count.to_string(),
    ]
}

/// Purpose: implementation note.
pub(crate) fn render_routed_import_org_table(
    rows: &[[String; 5]],
    include_header: bool,
) -> Vec<String> {
    let headers = [
        "SOURCE_ORG_ID",
        "SOURCE_ORG_NAME",
        "ORG_ACTION",
        "TARGET_ORG_ID",
        "DASHBOARD_COUNT",
    ];
    let mut widths = headers.map(str::len);
    for row in rows {
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
    for row in rows {
        lines.push(format_row(row));
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

/// Purpose: implementation note.
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

fn build_import_dry_run_json_value(report: &ImportDryRunReport) -> Value {
    let folders = report
        .folder_statuses
        .iter()
        .map(|status| {
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
            serde_json::json!({
                "uid": status.uid,
                "destination": destination,
                "status": status_label,
                "reason": reason,
                "expectedPath": status.expected_path,
                "actualPath": status.actual_path.clone().unwrap_or_default(),
            })
        })
        .collect::<Vec<Value>>();
    let dashboards = report
        .dashboard_records
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
    serde_json::json!({
        "mode": report.mode,
        "folders": folders,
        "dashboards": dashboards,
        "summary": {
            "importDir": report.import_dir.display().to_string(),
            "folderCount": report.folder_statuses.len(),
            "missingFolders": report.folder_statuses.iter().filter(|status| status.kind == FolderInventoryStatusKind::Missing).count(),
            "mismatchedFolders": report.folder_statuses.iter().filter(|status| status.kind == FolderInventoryStatusKind::Mismatch).count(),
            "dashboardCount": report.dashboard_records.len(),
            "missingDashboards": report.dashboard_records.iter().filter(|row| row[1] == "missing").count(),
            "skippedMissingDashboards": report.skipped_missing_count,
            "skippedFolderMismatchDashboards": report.skipped_folder_mismatch_count,
        }
    })
}

/// Purpose: implementation note.
pub(crate) fn build_routed_import_dry_run_json_document(
    orgs: &[Value],
    imports: &[Value],
) -> Result<String> {
    let payload = serde_json::json!({
        "mode": "routed-import-preview",
        "orgs": orgs,
        "imports": imports,
        "summary": {
            "orgCount": orgs.len(),
            "existingOrgCount": orgs.iter().filter(|entry| entry.get("orgAction") == Some(&Value::String("exists".to_string()))).count(),
            "missingOrgCount": orgs.iter().filter(|entry| entry.get("orgAction") == Some(&Value::String("missing".to_string()))).count(),
            "wouldCreateOrgCount": orgs.iter().filter(|entry| entry.get("orgAction") == Some(&Value::String("would-create".to_string()))).count(),
            "dashboardCount": orgs.iter().map(|entry| entry.get("dashboardCount").and_then(Value::as_u64).unwrap_or(0)).sum::<u64>(),
        }
    });
    Ok(serde_json::to_string_pretty(&payload)?)
}

/// collect import dry run report with request.
pub(crate) fn collect_import_dry_run_report_with_request<F>(
    mut request_json: F,
    args: &ImportArgs,
) -> Result<ImportDryRunReport>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut lookup_cache = ImportLookupCache::default();
    let metadata = load_export_metadata(&args.import_dir, Some(RAW_EXPORT_SUBDIR))?;
    validate_matching_export_org_with_request(
        &mut request_json,
        &mut lookup_cache,
        args,
        &args.import_dir,
        metadata.as_ref(),
        None,
    )?;
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
    let mut dashboard_files = discover_dashboard_files(&args.import_dir)?;
    dashboard_files.retain(|path| {
        path.file_name().and_then(|name| name.to_str()) != Some(FOLDER_INVENTORY_FILENAME)
    });
    let effective_replace_existing = args.replace_existing || args.update_existing_only;
    let mut dashboard_records: Vec<[String; 8]> = Vec::new();
    for dashboard_file in &dashboard_files {
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
            (true, "", String::new(), None)
        };
        let action = apply_folder_path_guard_to_action(action, folder_paths_match);
        let folder_path = resolve_dashboard_import_folder_path_with_request(
            &mut request_json,
            &mut lookup_cache,
            &payload,
            &folders_by_uid,
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

/// format import progress line.
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

/// format import verbose line.
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
    let metadata = load_export_metadata(&args.import_dir, Some(RAW_EXPORT_SUBDIR))?;
    validate_matching_export_org_with_request(
        &mut request_json,
        &mut lookup_cache,
        args,
        &args.import_dir,
        metadata.as_ref(),
        None,
    )?;
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
            (true, "", String::new(), None)
        };
        let action =
            action.map(|value| apply_folder_path_guard_to_action(value, folder_paths_match));
        if args.dry_run {
            let folder_path = resolve_dashboard_import_folder_path_with_request(
                &mut request_json,
                &mut lookup_cache,
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
///
/// Args: see function signature.
/// Returns: see implementation.
pub fn import_dashboards_with_client(client: &JsonHttpClient, args: &ImportArgs) -> Result<usize> {
    import_dashboards_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        args,
    )
}

/// Purpose: implementation note.
pub(crate) fn build_routed_import_dry_run_json_with_request<F, G>(
    mut request_json: F,
    mut collect_preview_for_org: G,
    args: &ImportArgs,
) -> Result<String>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
    G: FnMut(i64, &ImportArgs) -> Result<ImportDryRunReport>,
{
    let scopes = discover_export_org_import_scopes(args)?;
    let mut lookup_cache = ImportLookupCache::default();
    let mut orgs = Vec::new();
    let mut imports = Vec::new();
    for scope in scopes {
        let target_plan = resolve_target_org_plan_for_export_scope_with_request(
            &mut request_json,
            &mut lookup_cache,
            args,
            &scope,
        )?;
        let dashboard_count = discover_dashboard_files(&target_plan.import_dir)?
            .into_iter()
            .filter(|path| {
                path.file_name().and_then(|name| name.to_str()) != Some(FOLDER_INVENTORY_FILENAME)
            })
            .count();
        orgs.push(serde_json::json!({
            "sourceOrgId": target_plan.source_org_id,
            "sourceOrgName": target_plan.source_org_name,
            "orgAction": target_plan.org_action,
            "targetOrgId": target_plan.target_org_id,
            "dashboardCount": dashboard_count,
            "importDir": target_plan.import_dir.display().to_string(),
        }));
        let preview = if let Some(target_org_id) = target_plan.target_org_id {
            let mut scoped_args = args.clone();
            scoped_args.org_id = Some(target_org_id);
            scoped_args.use_export_org = false;
            scoped_args.only_org_id = Vec::new();
            scoped_args.create_missing_orgs = false;
            scoped_args.import_dir = target_plan.import_dir.clone();
            build_import_dry_run_json_value(&collect_preview_for_org(target_org_id, &scoped_args)?)
        } else {
            serde_json::json!({
                "mode": describe_dashboard_import_mode(args.replace_existing, args.update_existing_only),
                "folders": [],
                "dashboards": [],
                "summary": {
                    "importDir": target_plan.import_dir.display().to_string(),
                    "folderCount": 0,
                    "missingFolders": 0,
                    "mismatchedFolders": 0,
                    "dashboardCount": dashboard_count,
                    "missingDashboards": 0,
                    "skippedMissingDashboards": 0,
                    "skippedFolderMismatchDashboards": 0,
                }
            })
        };
        let mut import_entry = serde_json::Map::new();
        import_entry.insert(
            "sourceOrgId".to_string(),
            Value::from(target_plan.source_org_id),
        );
        import_entry.insert(
            "sourceOrgName".to_string(),
            Value::from(target_plan.source_org_name.clone()),
        );
        import_entry.insert("orgAction".to_string(), Value::from(target_plan.org_action));
        import_entry.insert(
            "targetOrgId".to_string(),
            target_plan
                .target_org_id
                .map(Value::from)
                .unwrap_or(Value::Null),
        );
        if let Some(preview_object) = preview.as_object() {
            for (key, value) in preview_object {
                import_entry.insert(key.clone(), value.clone());
            }
        }
        imports.push(Value::Object(import_entry));
    }
    build_routed_import_dry_run_json_document(&orgs, &imports)
}

/// Purpose: implementation note.
pub(crate) fn import_dashboards_by_export_org_with_request<F, G, H>(
    mut request_json: F,
    mut import_for_org: G,
    collect_preview_for_org: H,
    args: &ImportArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
    G: FnMut(i64, &ImportArgs) -> Result<usize>,
    H: FnMut(i64, &ImportArgs) -> Result<ImportDryRunReport>,
{
    let scopes = discover_export_org_import_scopes(args)?;
    let mut lookup_cache = ImportLookupCache::default();
    if args.dry_run && args.json {
        println!(
            "{}",
            build_routed_import_dry_run_json_with_request(
                request_json,
                collect_preview_for_org,
                args,
            )?
        );
        return Ok(0);
    }
    let mut imported_count = 0;
    let mut org_rows = Vec::new();
    let mut resolved_plans = Vec::new();
    for scope in scopes {
        let target_plan = resolve_target_org_plan_for_export_scope_with_request(
            &mut request_json,
            &mut lookup_cache,
            args,
            &scope,
        )?;
        let dashboard_count = discover_dashboard_files(&target_plan.import_dir)?
            .into_iter()
            .filter(|path| {
                path.file_name().and_then(|name| name.to_str()) != Some(FOLDER_INVENTORY_FILENAME)
            })
            .count();
        org_rows.push(build_routed_import_org_row(&target_plan, dashboard_count));
        resolved_plans.push(target_plan);
    }
    if args.dry_run && args.table {
        for line in render_routed_import_org_table(&org_rows, !args.no_header) {
            println!("{line}");
        }
        return Ok(0);
    }
    for target_plan in resolved_plans {
        let target_org_id_label = target_plan
            .target_org_id
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string());
        if !args.table {
            println!(
                "Importing export orgId={} name={} orgAction={} targetOrgId={} from {}",
                target_plan.source_org_id,
                if target_plan.source_org_name.is_empty() {
                    "-"
                } else {
                    &target_plan.source_org_name
                },
                target_plan.org_action,
                target_org_id_label,
                target_plan.import_dir.display()
            );
        }
        let Some(target_org_id) = target_plan.target_org_id else {
            continue;
        };
        let mut scoped_args = args.clone();
        scoped_args.org_id = Some(target_org_id);
        scoped_args.use_export_org = false;
        scoped_args.only_org_id = Vec::new();
        scoped_args.create_missing_orgs = false;
        scoped_args.import_dir = target_plan.import_dir.clone();
        imported_count += import_for_org(target_org_id, &scoped_args)?;
    }
    Ok(imported_count)
}

/// Purpose: implementation note.
pub(crate) fn import_dashboards_with_org_clients(args: &ImportArgs) -> Result<usize> {
    let context = build_import_auth_context(args)?;
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
    import_dashboards_by_export_org_with_request(
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

/// Purpose: implementation note.
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

/// Purpose: implementation note.
///
/// Args: see function signature.
/// Returns: see implementation.
pub fn diff_dashboards_with_client(client: &JsonHttpClient, args: &DiffArgs) -> Result<usize> {
    diff_dashboards_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        args,
    )
}
