//! Dashboard review-first plan builder and renderer.

use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::build_folder_path;
use super::cli_defs::{DashboardImportInputFormat, InspectExportInputType};
use super::files::resolve_dashboard_export_root;
use super::import_lookup::ImportLookupCache;
use super::import_target::build_dashboard_target_review;
use super::import_validation::resolve_target_org_plan_for_export_scope_with_request;
use super::plan_types::{
    DashboardPlanAction, DashboardPlanChange, DashboardPlanInput, DashboardPlanOrgSummary,
    DashboardPlanReport, DashboardPlanSummary, LiveDashboard, LocalDashboard, OrgPlanInput,
    PlanLiveState,
};
use super::source_loader::resolve_dashboard_workspace_variant_dir;
use super::{
    build_auth_context, build_datasource_catalog, build_http_client, build_http_client_for_org,
    collect_datasource_refs, discover_dashboard_files, extract_dashboard_object,
    load_dashboard_source, load_datasource_inventory, load_export_metadata, load_folder_inventory,
    load_json_file, lookup_datasource, FolderInventoryItem, DEFAULT_FOLDER_TITLE,
    DEFAULT_FOLDER_UID, DEFAULT_PAGE_SIZE, PROMPT_EXPORT_SUBDIR, RAW_EXPORT_SUBDIR,
};
use crate::common::{
    message, print_supported_columns, render_json_value, string_field, tool_version,
    value_as_object, Result,
};
use crate::review_contract::{
    REVIEW_ACTION_BLOCKED_TARGET, REVIEW_ACTION_EXTRA_REMOTE, REVIEW_ACTION_SAME,
    REVIEW_ACTION_WOULD_CREATE, REVIEW_ACTION_WOULD_DELETE, REVIEW_ACTION_WOULD_UPDATE,
    REVIEW_HINT_REMOTE_ONLY, REVIEW_REASON_TARGET_ORG_MISSING,
    REVIEW_REASON_TARGET_PROVISIONED_OR_MANAGED, REVIEW_STATUS_BLOCKED, REVIEW_STATUS_READY,
    REVIEW_STATUS_SAME, REVIEW_STATUS_WARNING,
};

const PLAN_KIND: &str = "grafana-util-dashboard-plan";
const PLAN_SCHEMA_VERSION: i64 = 1;

pub(crate) fn dashboard_plan_column_ids() -> &'static [&'static str] {
    &[
        "action_id",
        "action",
        "status",
        "dashboard_uid",
        "dashboard_title",
        "folder_uid",
        "folder_path",
        "source_org_id",
        "source_org_name",
        "target_org_id",
        "target_org_name",
        "match_basis",
        "changed_fields",
        "blocked_reason",
        "source_file",
    ]
}

fn plan_output_columns(selected: &[String]) -> Vec<&'static str> {
    if selected.is_empty() || selected.iter().any(|value| value == "all") {
        return dashboard_plan_column_ids().to_vec();
    }
    selected
        .iter()
        .filter_map(|value| match value.as_str() {
            "action_id" => Some("action_id"),
            "action" => Some("action"),
            "status" => Some("status"),
            "dashboard_uid" => Some("dashboard_uid"),
            "dashboard_title" => Some("dashboard_title"),
            "folder_uid" => Some("folder_uid"),
            "folder_path" => Some("folder_path"),
            "source_org_id" => Some("source_org_id"),
            "source_org_name" => Some("source_org_name"),
            "target_org_id" => Some("target_org_id"),
            "target_org_name" => Some("target_org_name"),
            "match_basis" => Some("match_basis"),
            "changed_fields" => Some("changed_fields"),
            "blocked_reason" => Some("blocked_reason"),
            "source_file" => Some("source_file"),
            _ => None,
        })
        .collect()
}

fn summarize_value(value: &Value) -> Value {
    match value {
        Value::Object(object) if object.len() > 8 => {
            Value::String(format!("object({} keys)", object.len()))
        }
        Value::Array(items) if items.len() > 8 => {
            Value::String(format!("array({} items)", items.len()))
        }
        other => other.clone(),
    }
}

fn strip_dashboard_noise(value: &mut Value) {
    match value {
        Value::Object(object) => {
            for key in ["id", "version", "iteration", "schemaVersion"] {
                object.remove(key);
            }
            for child in object.values_mut() {
                strip_dashboard_noise(child);
            }
        }
        Value::Array(items) => {
            for item in items {
                strip_dashboard_noise(item);
            }
        }
        _ => {}
    }
}

fn normalize_dashboard_document(document: &Value) -> Result<Value> {
    let object = value_as_object(document, "Dashboard payload must be a JSON object.")?;
    let mut normalized = Value::Object(extract_dashboard_object(object)?.clone());
    strip_dashboard_noise(&mut normalized);
    Ok(normalized)
}

fn compare_dashboard_documents(
    left: &Value,
    right: &Value,
) -> (Vec<String>, Vec<DashboardPlanChange>) {
    let mut changed_fields = Vec::new();
    let mut changes = Vec::new();
    compare_json_values("dashboard", left, right, &mut changed_fields, &mut changes);
    (changed_fields, changes)
}

fn compare_json_values(
    prefix: &str,
    left: &Value,
    right: &Value,
    changed_fields: &mut Vec<String>,
    changes: &mut Vec<DashboardPlanChange>,
) {
    if left == right {
        return;
    }
    match (left, right) {
        (Value::Object(left_object), Value::Object(right_object)) => {
            let mut keys = BTreeSet::new();
            for key in left_object.keys().chain(right_object.keys()) {
                keys.insert(key.clone());
            }
            for key in keys {
                let before = left_object.get(&key).unwrap_or(&Value::Null);
                let after = right_object.get(&key).unwrap_or(&Value::Null);
                let field = format!("{prefix}.{key}");
                compare_json_values(&field, before, after, changed_fields, changes);
            }
        }
        (Value::Array(_), Value::Array(_)) => {
            changed_fields.push(prefix.to_string());
            changes.push(DashboardPlanChange {
                field: prefix.to_string(),
                before: summarize_value(left),
                after: summarize_value(right),
            });
        }
        _ => {
            changed_fields.push(prefix.to_string());
            changes.push(DashboardPlanChange {
                field: prefix.to_string(),
                before: summarize_value(left),
                after: summarize_value(right),
            });
        }
    }
}

fn folder_path_for_uid(folder_uid: &str, folder_inventory: &[FolderInventoryItem]) -> String {
    if folder_uid.trim().is_empty() || folder_uid == DEFAULT_FOLDER_UID {
        return DEFAULT_FOLDER_TITLE.to_string();
    }
    folder_inventory
        .iter()
        .find(|folder| folder.uid == folder_uid)
        .map(|folder| folder.path.clone())
        .unwrap_or_else(|| folder_uid.to_string())
}

fn build_local_dashboard(
    document: &Value,
    file_path: &Path,
    folder_inventory: &[FolderInventoryItem],
) -> Result<LocalDashboard> {
    let object = value_as_object(document, "Dashboard plan input must be a JSON object.")?;
    let dashboard = normalize_dashboard_document(document)?;
    let dashboard_object = dashboard
        .as_object()
        .ok_or_else(|| message("Dashboard plan input must be a JSON object."))?;
    let dashboard_uid = string_field(dashboard_object, "uid", "");
    let title = string_field(
        dashboard_object,
        "title",
        file_path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("dashboard"),
    );
    let folder_uid = object
        .get("meta")
        .and_then(Value::as_object)
        .and_then(|meta| meta.get("folderUid"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let folder_path = folder_path_for_uid(&folder_uid, folder_inventory);
    Ok(LocalDashboard {
        file_path: file_path.display().to_string(),
        dashboard,
        dashboard_uid,
        title,
        folder_uid,
        folder_path,
    })
}

fn resolve_live_folder_path_with_request<F>(
    request_json: &mut F,
    folder_uid: &str,
) -> Result<String>
where
    F: FnMut(reqwest::Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if folder_uid.trim().is_empty() || folder_uid == DEFAULT_FOLDER_UID {
        return Ok(DEFAULT_FOLDER_TITLE.to_string());
    }
    match super::fetch_folder_if_exists_with_request(&mut *request_json, folder_uid)? {
        Some(folder) => Ok(build_folder_path(&folder, DEFAULT_FOLDER_TITLE)),
        None => Ok(folder_uid.to_string()),
    }
}

fn build_live_dashboard_with_request<F>(
    request_json: &mut F,
    payload: &Value,
) -> Result<LiveDashboard>
where
    F: FnMut(reqwest::Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let dashboard = normalize_dashboard_document(payload)?;
    let dashboard_object = dashboard
        .as_object()
        .ok_or_else(|| message("Unexpected dashboard payload from Grafana."))?;
    let payload_object = value_as_object(payload, "Unexpected dashboard payload from Grafana.")?;
    let uid = string_field(dashboard_object, "uid", "");
    let title = string_field(dashboard_object, "title", "");
    let folder_uid = payload_object
        .get("meta")
        .and_then(Value::as_object)
        .and_then(|meta| meta.get("folderUid"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let folder_path = resolve_live_folder_path_with_request(request_json, &folder_uid)?;
    let review = build_dashboard_target_review(payload)?;
    Ok(LiveDashboard {
        uid,
        title,
        folder_uid,
        folder_path,
        version: dashboard_object.get("version").and_then(Value::as_i64),
        evidence: review.evidence,
        payload: dashboard,
    })
}

fn count_library_panels(node: &Value) -> usize {
    match node {
        Value::Object(object) => {
            usize::from(object.contains_key("libraryPanel"))
                + object.values().map(count_library_panels).sum::<usize>()
        }
        Value::Array(items) => items.iter().map(count_library_panels).sum(),
        _ => 0,
    }
}

fn build_dependency_hints(
    dashboard: &Value,
    live_datasources: &[Map<String, Value>],
) -> (Vec<String>, Vec<String>) {
    let catalog = build_datasource_catalog(live_datasources);
    let mut refs = Vec::new();
    collect_datasource_refs(dashboard, &mut refs);
    let mut missing = BTreeSet::new();
    for reference in refs {
        let resolved = lookup_datasource(
            &catalog,
            reference.get("uid").and_then(Value::as_str),
            reference.get("name").and_then(Value::as_str),
        );
        if resolved.is_none() {
            let label = reference
                .get("name")
                .and_then(Value::as_str)
                .or_else(|| reference.get("uid").and_then(Value::as_str))
                .unwrap_or("unknown");
            missing.insert(label.to_string());
        }
    }
    let mut dependency_hints = Vec::new();
    let mut review_hints = Vec::new();
    if !missing.is_empty() {
        let labels = missing.into_iter().collect::<Vec<String>>().join(", ");
        dependency_hints.push(format!("missing-datasources={labels}"));
        review_hints.push("dashboard references unresolved datasources".to_string());
    }
    let library_panel_count = count_library_panels(dashboard);
    if library_panel_count > 0 {
        review_hints.push(format!("library-panel-references={library_panel_count}"));
    }
    (dependency_hints, review_hints)
}

fn plan_export_org_variant_dir(input_type: InspectExportInputType) -> &'static str {
    match input_type {
        InspectExportInputType::Raw => RAW_EXPORT_SUBDIR,
        InspectExportInputType::Source => PROMPT_EXPORT_SUBDIR,
    }
}

fn has_plan_export_org_scopes(input_dir: &Path, variant_dir_name: &str) -> Result<bool> {
    if !input_dir.is_dir() {
        return Ok(false);
    }
    for entry in fs::read_dir(input_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if name.starts_with("org_") && path.join(variant_dir_name).is_dir() {
            return Ok(true);
        }
    }
    Ok(false)
}

fn normalize_plan_export_org_scopes_root(
    input_dir: &Path,
    variant_dir_name: &str,
) -> Result<PathBuf> {
    if let Some(resolved_root) = resolve_dashboard_export_root(input_dir)? {
        if resolved_root.manifest.scope_kind.is_aggregate()
            || has_plan_export_org_scopes(&resolved_root.metadata_dir, variant_dir_name)?
        {
            return Ok(resolved_root.metadata_dir);
        }
    }
    if has_plan_export_org_scopes(input_dir, variant_dir_name)? {
        return Ok(input_dir.to_path_buf());
    }
    if let Some(workspace_variant_dir) =
        resolve_dashboard_workspace_variant_dir(input_dir, variant_dir_name)
    {
        if has_plan_export_org_scopes(&workspace_variant_dir, variant_dir_name)? {
            return Ok(workspace_variant_dir);
        }
    }
    Ok(input_dir.to_path_buf())
}

fn load_plan_export_org_index_entries(
    input_dir: &Path,
    metadata: Option<&super::ExportMetadata>,
) -> Result<Vec<super::VariantIndexEntry>> {
    let index_file = metadata
        .map(|item| item.index_file.clone())
        .unwrap_or_else(|| "index.json".to_string());
    let index_path = input_dir.join(index_file);
    if !index_path.is_file() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(&index_path)?;
    serde_json::from_str(&raw).map_err(|error| {
        message(format!(
            "Invalid dashboard export index in {}: {error}",
            index_path.display()
        ))
    })
}

fn org_id_text_from_value(value: Option<&Value>) -> Option<String> {
    match value {
        Some(Value::String(text)) => Some(text.trim().to_string()),
        Some(Value::Number(number)) => Some(number.to_string()),
        _ => None,
    }
}

fn collect_plan_export_org_ids(
    input_dir: &Path,
    metadata: Option<&super::ExportMetadata>,
) -> Result<BTreeSet<String>> {
    let mut org_ids = BTreeSet::new();
    if let Some(metadata) = metadata {
        if let Some(org_id) = metadata
            .org_id
            .as_ref()
            .map(|value| value.trim().to_string())
        {
            if !org_id.is_empty() {
                org_ids.insert(org_id);
            }
        }
        if let Some(orgs) = metadata.orgs.as_ref() {
            for org in orgs {
                let org_id = org.org_id.trim();
                if !org_id.is_empty() {
                    org_ids.insert(org_id.to_string());
                }
            }
        }
    }
    for entry in load_plan_export_org_index_entries(input_dir, metadata)? {
        if !entry.org_id.trim().is_empty() {
            org_ids.insert(entry.org_id.trim().to_string());
        }
    }
    for folder in load_folder_inventory(input_dir, metadata)? {
        if !folder.org_id.trim().is_empty() {
            org_ids.insert(folder.org_id.trim().to_string());
        }
    }
    for datasource in load_datasource_inventory(input_dir, metadata)? {
        if !datasource.org_id.trim().is_empty() {
            org_ids.insert(datasource.org_id.trim().to_string());
        }
    }
    Ok(org_ids)
}

fn collect_plan_export_org_names(
    input_dir: &Path,
    metadata: Option<&super::ExportMetadata>,
) -> Result<BTreeSet<String>> {
    let mut org_names = BTreeSet::new();
    if let Some(metadata) = metadata {
        if let Some(org) = metadata.org.as_ref().map(|value| value.trim().to_string()) {
            if !org.is_empty() {
                org_names.insert(org);
            }
        }
        if let Some(orgs) = metadata.orgs.as_ref() {
            for org in orgs {
                let org_name = org.org.trim();
                if !org_name.is_empty() {
                    org_names.insert(org_name.to_string());
                }
            }
        }
    }
    for entry in load_plan_export_org_index_entries(input_dir, metadata)? {
        if !entry.org.trim().is_empty() {
            org_names.insert(entry.org.trim().to_string());
        }
    }
    for folder in load_folder_inventory(input_dir, metadata)? {
        if !folder.org.trim().is_empty() {
            org_names.insert(folder.org.trim().to_string());
        }
    }
    for datasource in load_datasource_inventory(input_dir, metadata)? {
        if !datasource.org.trim().is_empty() {
            org_names.insert(datasource.org.trim().to_string());
        }
    }
    Ok(org_names)
}

fn parse_plan_export_org_scope_for_variant(
    import_root: &Path,
    variant_dir: &Path,
    expected_variant: &'static str,
) -> Result<super::import_validation::ExportOrgImportScope> {
    let metadata = load_export_metadata(variant_dir, Some(expected_variant))?;
    let export_org_ids = collect_plan_export_org_ids(variant_dir, metadata.as_ref())?;
    if export_org_ids.is_empty() {
        return Err(message(format!(
            "Dashboard plan with --use-export-org could not find {expected_variant} export orgId metadata in {}.",
            variant_dir.display()
        )));
    }
    if export_org_ids.len() > 1 {
        return Err(message(format!(
            "Dashboard plan with --use-export-org found multiple export orgIds in {}: {}",
            variant_dir.display(),
            export_org_ids
                .into_iter()
                .collect::<Vec<String>>()
                .join(", ")
        )));
    }
    let source_org_id_text = export_org_ids.into_iter().next().unwrap_or_default();
    let source_org_id = source_org_id_text.parse::<i64>().map_err(|_| {
        message(format!(
            "Dashboard plan with --use-export-org found a non-numeric export orgId '{}' in {}.",
            source_org_id_text,
            variant_dir.display()
        ))
    })?;
    let export_org_names = collect_plan_export_org_names(variant_dir, metadata.as_ref())?;
    if export_org_names.len() > 1 {
        return Err(message(format!(
            "Dashboard plan with --use-export-org found multiple export org names in {}: {}",
            variant_dir.display(),
            export_org_names
                .into_iter()
                .collect::<Vec<String>>()
                .join(", ")
        )));
    }
    let source_org_name = export_org_names.into_iter().next().unwrap_or_else(|| {
        variant_dir
            .file_name()
            .and_then(|value| value.to_str())
            .or_else(|| import_root.file_name().and_then(|value| value.to_str()))
            .unwrap_or("org")
            .to_string()
    });
    Ok(super::import_validation::ExportOrgImportScope {
        source_org_id,
        source_org_name,
        input_dir: variant_dir.to_path_buf(),
    })
}

fn discover_plan_export_org_scopes(
    args: &super::PlanArgs,
) -> Result<Vec<super::import_validation::ExportOrgImportScope>> {
    if !args.use_export_org {
        return Ok(Vec::new());
    }
    let variant_dir_name = plan_export_org_variant_dir(args.input_type);
    let scan_root = normalize_plan_export_org_scopes_root(&args.input_dir, variant_dir_name)?;
    let selected_org_ids: BTreeSet<i64> = args.only_org_id.iter().copied().collect();
    let mut scopes = Vec::new();
    if scan_root.is_dir() {
        for entry in fs::read_dir(&scan_root)? {
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
            let variant_dir = path.join(variant_dir_name);
            if !variant_dir.is_dir() {
                continue;
            }
            let scope = parse_plan_export_org_scope_for_variant(
                &scan_root,
                &variant_dir,
                variant_dir_name,
            )?;
            if !selected_org_ids.is_empty() && !selected_org_ids.contains(&scope.source_org_id) {
                continue;
            }
            scopes.push(scope);
        }
    }
    scopes.sort_by(|left, right| left.source_org_id.cmp(&right.source_org_id));
    if scopes.is_empty() {
        if selected_org_ids.is_empty() {
            return Err(message(format!(
                "Dashboard plan with --use-export-org did not find any org-specific {variant_dir_name} exports under {}.",
                scan_root.display()
            )));
        }
        return Err(message(format!(
            "Dashboard plan with --use-export-org did not find the selected exported org IDs ({}) under {}.",
            selected_org_ids
                .into_iter()
                .map(|id| id.to_string())
                .collect::<Vec<String>>()
                .join(", "),
            scan_root.display()
        )));
    }
    let found_org_ids: BTreeSet<i64> = scopes.iter().map(|scope| scope.source_org_id).collect();
    let missing_org_ids: Vec<String> = selected_org_ids
        .difference(&found_org_ids)
        .map(|id| id.to_string())
        .collect();
    if !missing_org_ids.is_empty() {
        return Err(message(format!(
            "Dashboard plan with --use-export-org did not find the selected exported org IDs ({}).",
            missing_org_ids.join(", ")
        )));
    }
    Ok(scopes)
}

fn export_org_target_org_name(
    target_org_id: Option<i64>,
    lookup_cache: &ImportLookupCache,
    fallback_name: &str,
) -> String {
    let Some(target_org_id) = target_org_id else {
        return "<new>".to_string();
    };
    if let Some(orgs) = lookup_cache.orgs.as_ref() {
        let target_id_text = target_org_id.to_string();
        for org in orgs {
            if org_id_text_from_value(org.get("id")).as_deref() == Some(target_id_text.as_str()) {
                if let Some(name) = org.get("name").and_then(Value::as_str) {
                    return name.to_string();
                }
            }
        }
    }
    fallback_name.to_string()
}

fn build_export_routing_import_args(args: &super::PlanArgs) -> super::ImportArgs {
    super::ImportArgs {
        common: args.common.clone(),
        org_id: None,
        use_export_org: true,
        only_org_id: args.only_org_id.clone(),
        create_missing_orgs: args.create_missing_orgs,
        input_dir: args.input_dir.clone(),
        input_format: DashboardImportInputFormat::Raw,
        import_folder_uid: None,
        ensure_folders: false,
        replace_existing: false,
        update_existing_only: false,
        require_matching_folder_path: false,
        require_matching_export_org: false,
        strict_schema: false,
        target_schema_version: None,
        import_message: String::new(),
        interactive: false,
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

fn build_action_id(org_id: Option<&str>, uid: &str, seed: usize) -> String {
    let org = org_id.unwrap_or("unknown");
    let resource = if uid.is_empty() { "dashboard" } else { uid };
    format!("org:{org}/dashboard:{resource}:{seed}")
}

fn build_org_actions(org: &OrgPlanInput, prune: bool) -> Vec<DashboardPlanAction> {
    let no_live_target = org.target_org_id.is_none();
    let missing_target = no_live_target && org.org_action == "missing";
    let would_create_target = no_live_target && org.org_action == REVIEW_ACTION_WOULD_CREATE;
    let mut live_by_uid = BTreeMap::new();
    let mut live_by_title: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let mut live_matched = vec![false; org.live_dashboards.len()];
    for (index, live) in org.live_dashboards.iter().enumerate() {
        if !live.uid.trim().is_empty() {
            live_by_uid.insert(live.uid.clone(), index);
        }
        if !live.title.trim().is_empty() {
            live_by_title
                .entry(live.title.clone())
                .or_default()
                .push(index);
        }
    }

    let mut actions = Vec::new();
    for (index, local) in org.local_dashboards.iter().enumerate() {
        let mut match_basis = "none".to_string();
        let mut live_index = None;
        if !local.dashboard_uid.trim().is_empty() {
            if let Some(position) = live_by_uid.get(&local.dashboard_uid) {
                match_basis = "uid".to_string();
                live_index = Some(*position);
            }
        }
        if live_index.is_none() {
            let title = local.title.trim();
            if !title.is_empty() {
                if let Some(positions) = live_by_title.get(title) {
                    if positions.len() == 1 {
                        match_basis = "title".to_string();
                        live_index = Some(positions[0]);
                    }
                }
            }
        }
        if let Some(position) = live_index {
            live_matched[position] = true;
        }
        let live = live_index.map(|position| &org.live_dashboards[position]);

        let (changed_fields, changes) = if let Some(live) = live {
            compare_dashboard_documents(&local.dashboard, &live.payload)
        } else {
            (Vec::new(), Vec::new())
        };
        let (dependency_hints, mut review_hints) = if no_live_target {
            (Vec::new(), Vec::new())
        } else {
            build_dependency_hints(&local.dashboard, &org.live_datasources)
        };
        if !local.folder_uid.is_empty() && local.folder_uid != DEFAULT_FOLDER_UID {
            review_hints.push(format!("folder-uid={}", local.folder_uid));
        }
        if local.folder_uid.is_empty() || local.folder_uid == DEFAULT_FOLDER_UID {
            review_hints.push("folder=General".to_string());
        }
        if local.folder_uid != DEFAULT_FOLDER_UID
            && !org
                .folder_inventory
                .iter()
                .any(|folder| folder.uid == local.folder_uid)
        {
            review_hints.push(format!("missing-folder-uid={}", local.folder_uid));
        }

        if missing_target {
            review_hints.push(REVIEW_REASON_TARGET_ORG_MISSING.to_string());
        }
        if would_create_target {
            review_hints.push("target-org-would-create".to_string());
        }

        let target_review_blocked = live
            .as_ref()
            .map(|dashboard| {
                dashboard
                    .evidence
                    .iter()
                    .any(|value| value.starts_with("provisioned=true"))
            })
            .unwrap_or(false);

        let mut action = if missing_target || would_create_target || live.is_none() {
            REVIEW_ACTION_WOULD_CREATE.to_string()
        } else if changed_fields.is_empty()
            && local.folder_uid == live.map(|item| item.folder_uid.clone()).unwrap_or_default()
        {
            REVIEW_ACTION_SAME.to_string()
        } else {
            REVIEW_ACTION_WOULD_UPDATE.to_string()
        };
        let mut status = if action == REVIEW_ACTION_SAME {
            REVIEW_STATUS_SAME.to_string()
        } else if missing_target {
            REVIEW_STATUS_BLOCKED.to_string()
        } else if would_create_target {
            REVIEW_STATUS_WARNING.to_string()
        } else {
            REVIEW_STATUS_READY.to_string()
        };
        let mut blocked_reason = None;
        if missing_target {
            blocked_reason = Some(REVIEW_REASON_TARGET_ORG_MISSING.to_string());
        } else if target_review_blocked
            && matches!(
                action.as_str(),
                REVIEW_ACTION_WOULD_UPDATE | REVIEW_ACTION_WOULD_DELETE
            )
        {
            action = REVIEW_ACTION_BLOCKED_TARGET.to_string();
            status = REVIEW_STATUS_BLOCKED.to_string();
            blocked_reason = Some(REVIEW_REASON_TARGET_PROVISIONED_OR_MANAGED.to_string());
        } else if action != REVIEW_ACTION_SAME
            && (!dependency_hints.is_empty()
                || review_hints
                    .iter()
                    .any(|hint| hint.starts_with("missing-folder-uid=")))
        {
            status = REVIEW_STATUS_WARNING.to_string();
        }

        let target_uid = live.as_ref().map(|item| item.uid.clone());
        let target_version = live.as_ref().and_then(|item| item.version);
        let target_evidence = live
            .as_ref()
            .map(|item| item.evidence.clone())
            .unwrap_or_default();
        actions.push(DashboardPlanAction {
            action_id: build_action_id(
                org.target_org_id
                    .as_deref()
                    .or(org.source_org_id.as_deref()),
                &local.dashboard_uid,
                index,
            ),
            domain: "dashboard".to_string(),
            resource_kind: "dashboard".to_string(),
            dashboard_uid: local.dashboard_uid.clone(),
            title: local.title.clone(),
            folder_uid: local.folder_uid.clone(),
            folder_path: local.folder_path.clone(),
            source_org_id: org.source_org_id.clone(),
            source_org_name: org.source_org_name.clone(),
            target_org_id: org.target_org_id.clone(),
            target_org_name: org.target_org_name.clone(),
            match_basis,
            action,
            status,
            changed_fields,
            changes,
            source_file: Some(local.file_path.clone()),
            target_uid,
            target_version,
            target_evidence,
            dependency_hints,
            blocked_reason,
            review_hints,
        });
    }

    for (index, live) in org.live_dashboards.iter().enumerate() {
        if live_matched.get(index).copied().unwrap_or(false) {
            continue;
        }
        let blocked = live
            .evidence
            .iter()
            .any(|value| value.starts_with("provisioned=true"));
        actions.push(DashboardPlanAction {
            action_id: build_action_id(
                org.target_org_id
                    .as_deref()
                    .or(org.source_org_id.as_deref()),
                &live.uid,
                index,
            ),
            domain: "dashboard".to_string(),
            resource_kind: "dashboard".to_string(),
            dashboard_uid: live.uid.clone(),
            title: live.title.clone(),
            folder_uid: live.folder_uid.clone(),
            folder_path: live.folder_path.clone(),
            source_org_id: org.source_org_id.clone(),
            source_org_name: org.source_org_name.clone(),
            target_org_id: org.target_org_id.clone(),
            target_org_name: org.target_org_name.clone(),
            match_basis: "live-only".to_string(),
            action: if prune {
                REVIEW_ACTION_WOULD_DELETE.to_string()
            } else {
                REVIEW_ACTION_EXTRA_REMOTE.to_string()
            },
            status: if blocked {
                REVIEW_STATUS_BLOCKED.to_string()
            } else if prune {
                REVIEW_STATUS_READY.to_string()
            } else {
                REVIEW_STATUS_WARNING.to_string()
            },
            changed_fields: Vec::new(),
            changes: Vec::new(),
            source_file: None,
            target_uid: Some(live.uid.clone()),
            target_version: live.version,
            target_evidence: live.evidence.clone(),
            dependency_hints: Vec::new(),
            blocked_reason: blocked
                .then_some(REVIEW_REASON_TARGET_PROVISIONED_OR_MANAGED.to_string()),
            review_hints: vec![format!("{REVIEW_HINT_REMOTE_ONLY} dashboard candidate")],
        });
    }

    actions
}

fn build_org_summary(
    org: &OrgPlanInput,
    actions: &[DashboardPlanAction],
) -> DashboardPlanOrgSummary {
    let mut summary = DashboardPlanOrgSummary {
        source_org_id: org.source_org_id.clone(),
        source_org_name: org.source_org_name.clone(),
        target_org_id: org.target_org_id.clone(),
        target_org_name: org.target_org_name.clone(),
        org_action: org.org_action.clone(),
        input_dir: org.input_dir.display().to_string(),
        checked: actions.len(),
        same: 0,
        create: 0,
        update: 0,
        extra: 0,
        delete: 0,
        blocked: 0,
        warning: 0,
    };
    for action in actions {
        match action.action.as_str() {
            REVIEW_ACTION_SAME => summary.same += 1,
            REVIEW_ACTION_WOULD_CREATE => summary.create += 1,
            REVIEW_ACTION_WOULD_UPDATE => summary.update += 1,
            REVIEW_ACTION_EXTRA_REMOTE => summary.extra += 1,
            REVIEW_ACTION_WOULD_DELETE => summary.delete += 1,
            _ => {}
        }
        match action.status.as_str() {
            REVIEW_STATUS_BLOCKED => summary.blocked += 1,
            REVIEW_STATUS_WARNING => summary.warning += 1,
            _ => {}
        }
    }
    summary
}

fn build_summary(
    orgs: &[DashboardPlanOrgSummary],
    actions: &[DashboardPlanAction],
) -> DashboardPlanSummary {
    let mut summary = DashboardPlanSummary {
        checked: actions.len(),
        same: 0,
        create: 0,
        update: 0,
        extra: 0,
        delete: 0,
        blocked: 0,
        warning: 0,
        org_count: orgs.len(),
        would_create_org_count: 0,
    };
    for org in orgs {
        summary.same += org.same;
        summary.create += org.create;
        summary.update += org.update;
        summary.extra += org.extra;
        summary.delete += org.delete;
        summary.blocked += org.blocked;
        summary.warning += org.warning;
        if org.org_action == REVIEW_ACTION_WOULD_CREATE {
            summary.would_create_org_count += 1;
        }
    }
    summary
}

fn plan_summary_line(report: &DashboardPlanReport) -> String {
    format!(
        "Dashboard plan: checked={} same={} create={} update={} extra={} delete={} blocked={} warning={} orgs={} prune={}",
        report.summary.checked,
        report.summary.same,
        report.summary.create,
        report.summary.update,
        report.summary.extra,
        report.summary.delete,
        report.summary.blocked,
        report.summary.warning,
        report.summary.org_count,
        report.prune
    )
}

fn render_plan_text(report: &DashboardPlanReport, show_same: bool) -> Vec<String> {
    let mut lines = Vec::new();
    for org in &report.orgs {
        lines.push(format!(
            "Org {} / {} -> {} / {}: checked={} same={} create={} update={} extra={} delete={} blocked={} warning={} action={}",
            org.source_org_id.as_deref().unwrap_or("-"),
            org.source_org_name,
            org.target_org_id.as_deref().unwrap_or("<current>"),
            org.target_org_name,
            org.checked,
            org.same,
            org.create,
            org.update,
            org.extra,
            org.delete,
            org.blocked,
            org.warning,
            org.org_action
        ));
    }
    for action in &report.actions {
        if !show_same && action.action == REVIEW_ACTION_SAME {
            continue;
        }
        lines.push(format!(
            "{} org={} uid={} title={} folder={} action={} status={} changed={}",
            if action.status == REVIEW_STATUS_BLOCKED {
                "BLOCK"
            } else if action.action == REVIEW_ACTION_WOULD_DELETE {
                "DELETE"
            } else if action.action == REVIEW_ACTION_WOULD_CREATE {
                "CREATE"
            } else if action.action == REVIEW_ACTION_WOULD_UPDATE {
                "UPDATE"
            } else if action.action == REVIEW_ACTION_EXTRA_REMOTE {
                "EXTRA"
            } else {
                "SAME"
            },
            action.target_org_name,
            action.dashboard_uid,
            action.title,
            action.folder_path,
            action.action,
            action.status,
            if action.changed_fields.is_empty() {
                "none".to_string()
            } else {
                action.changed_fields.join(",")
            }
        ));
    }
    lines
}

fn render_plan_table(
    report: &DashboardPlanReport,
    show_same: bool,
    include_header: bool,
    selected_columns: &[String],
) -> Vec<String> {
    let columns = plan_output_columns(selected_columns);
    let rows = report
        .actions
        .iter()
        .filter(|action| show_same || action.action != REVIEW_ACTION_SAME)
        .map(|action| {
            columns
                .iter()
                .map(|column| match *column {
                    "action_id" => action.action_id.clone(),
                    "action" => action.action.clone(),
                    "status" => action.status.clone(),
                    "dashboard_uid" => action.dashboard_uid.clone(),
                    "dashboard_title" => action.title.clone(),
                    "folder_uid" => action.folder_uid.clone(),
                    "folder_path" => action.folder_path.clone(),
                    "source_org_id" => action.source_org_id.clone().unwrap_or_default(),
                    "source_org_name" => action.source_org_name.clone(),
                    "target_org_id" => action.target_org_id.clone().unwrap_or_default(),
                    "target_org_name" => action.target_org_name.clone(),
                    "match_basis" => action.match_basis.clone(),
                    "changed_fields" => {
                        if action.changed_fields.is_empty() {
                            String::new()
                        } else {
                            action.changed_fields.join(",")
                        }
                    }
                    "blocked_reason" => action.blocked_reason.clone().unwrap_or_default(),
                    "source_file" => action.source_file.clone().unwrap_or_default(),
                    _ => String::new(),
                })
                .collect::<Vec<String>>()
        })
        .collect::<Vec<Vec<String>>>();
    let headers = columns
        .iter()
        .map(|value| value.to_ascii_uppercase())
        .collect::<Vec<String>>();
    let widths = {
        let mut widths = headers
            .iter()
            .map(|header| header.len())
            .collect::<Vec<usize>>();
        for row in &rows {
            for (index, value) in row.iter().enumerate() {
                widths[index] = widths[index].max(value.len());
            }
        }
        widths
    };
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
        lines.push(format_row(&headers));
        lines.push(
            widths
                .iter()
                .map(|width| "-".repeat(*width))
                .collect::<Vec<String>>()
                .join("  "),
        );
    }
    for row in rows {
        lines.push(format_row(&row));
    }
    lines
}

fn load_local_org_plan_input(
    input_dir: &Path,
    expected_variant: &'static str,
    source_org_id: Option<String>,
    source_org_name: String,
    target_org_id: Option<String>,
    target_org_name: String,
    org_action: String,
) -> Result<OrgPlanInput> {
    let metadata = load_export_metadata(input_dir, Some(expected_variant))?;
    let folder_inventory = load_folder_inventory(input_dir, metadata.as_ref())?;
    let mut local_dashboards = Vec::new();
    for file in discover_dashboard_files(input_dir)? {
        let document = load_json_file(&file)?;
        local_dashboards.push(build_local_dashboard(&document, &file, &folder_inventory)?);
    }
    Ok(OrgPlanInput {
        source_org_id,
        source_org_name,
        target_org_id,
        target_org_name,
        org_action,
        input_dir: input_dir.to_path_buf(),
        local_dashboards,
        live_dashboards: Vec::new(),
        live_datasources: Vec::new(),
        folder_inventory,
    })
}

fn collect_live_org_state_with_request<F>(request_json: &mut F) -> Result<PlanLiveState>
where
    F: FnMut(reqwest::Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let live_datasources = super::list_datasources_with_request(&mut *request_json)?;
    let mut live_dashboards = Vec::new();
    let summaries =
        super::list_dashboard_summaries_with_request(&mut *request_json, DEFAULT_PAGE_SIZE)?;
    let mut seen_uids = BTreeSet::new();
    for summary in summaries {
        let uid = string_field(&summary, "uid", "");
        if uid.is_empty() || !seen_uids.insert(uid.clone()) {
            continue;
        }
        let payload = super::fetch_dashboard_with_request(&mut *request_json, &uid)?;
        live_dashboards.push(build_live_dashboard_with_request(request_json, &payload)?);
    }
    Ok((live_datasources, live_dashboards))
}

fn collect_single_scope_with_request<F>(
    args: &super::PlanArgs,
    request_json: &mut F,
) -> Result<DashboardPlanInput>
where
    F: FnMut(reqwest::Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let expected_variant = plan_export_org_variant_dir(args.input_type);
    let resolved = load_dashboard_source(
        &args.input_dir,
        DashboardImportInputFormat::Raw,
        Some(args.input_type),
        false,
    )?;
    let current_org = super::list::fetch_current_org_with_request(&mut *request_json)?;
    let target_org_id = current_org.get("id").map(|value| match value {
        Value::String(text) => text.clone(),
        _ => value.to_string(),
    });
    let target_org_name = string_field(&current_org, "name", "org");
    let metadata = load_export_metadata(&resolved.input_dir, Some(expected_variant))?;
    let source_org_id = metadata
        .as_ref()
        .and_then(|item| item.org_id.as_ref())
        .cloned();
    let source_org_name = metadata
        .as_ref()
        .and_then(|item| item.org.as_ref())
        .cloned()
        .unwrap_or_else(|| target_org_name.clone());
    let mut org = load_local_org_plan_input(
        &resolved.input_dir,
        expected_variant,
        source_org_id,
        source_org_name,
        target_org_id.clone(),
        target_org_name.clone(),
        if args.org_id.is_some() {
            "explicit-org".to_string()
        } else {
            "current-org".to_string()
        },
    )?;
    let (live_datasources, live_dashboards) = collect_live_org_state_with_request(request_json)?;
    org.live_datasources = live_datasources;
    org.live_dashboards = live_dashboards;
    Ok(DashboardPlanInput {
        scope: if args.org_id.is_some() {
            "explicit-org".to_string()
        } else {
            "current-org".to_string()
        },
        input_type: match args.input_type {
            InspectExportInputType::Raw => "raw".to_string(),
            InspectExportInputType::Source => "source".to_string(),
        },
        prune: args.prune,
        orgs: vec![org],
    })
}

#[cfg(test)]
fn collect_export_org_scope_with_request<F>(
    args: &super::PlanArgs,
    request_json: &mut F,
) -> Result<DashboardPlanInput>
where
    F: FnMut(reqwest::Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    collect_export_org_scope_with_live_collector(args, request_json, |_, request_json| {
        collect_live_org_state_with_request(request_json)
    })
}

fn collect_export_org_scope_with_live_collector<F, G>(
    args: &super::PlanArgs,
    request_json: &mut F,
    mut collect_live_for_org: G,
) -> Result<DashboardPlanInput>
where
    F: FnMut(reqwest::Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
    G: FnMut(i64, &mut F) -> Result<PlanLiveState>,
{
    let export_args = build_export_routing_import_args(args);
    let mut lookup_cache = ImportLookupCache::default();
    let scopes = discover_plan_export_org_scopes(args)?;
    let mut orgs = Vec::new();
    for scope in scopes {
        let target_plan = resolve_target_org_plan_for_export_scope_with_request(
            request_json,
            &mut lookup_cache,
            &export_args,
            &scope,
        )?;
        let target_org_name = export_org_target_org_name(
            target_plan.target_org_id,
            &lookup_cache,
            &target_plan.source_org_name,
        );
        let mut org = load_local_org_plan_input(
            &target_plan.input_dir,
            plan_export_org_variant_dir(args.input_type),
            Some(target_plan.source_org_id.to_string()),
            target_plan.source_org_name.clone(),
            target_plan.target_org_id.map(|value| value.to_string()),
            target_org_name,
            target_plan.org_action.to_string(),
        )?;
        if let Some(target_org_id) = target_plan.target_org_id {
            let (live_datasources, live_dashboards) =
                collect_live_for_org(target_org_id, request_json)?;
            org.live_datasources = live_datasources;
            org.live_dashboards = live_dashboards;
        }
        orgs.push(org);
    }
    Ok(DashboardPlanInput {
        scope: "export-org".to_string(),
        input_type: match args.input_type {
            InspectExportInputType::Raw => "raw".to_string(),
            InspectExportInputType::Source => "source".to_string(),
        },
        prune: args.prune,
        orgs,
    })
}

#[cfg(test)]
fn collect_plan_input_with_request<F>(
    args: &super::PlanArgs,
    request_json: &mut F,
) -> Result<DashboardPlanInput>
where
    F: FnMut(reqwest::Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if args.use_export_org {
        let context = build_auth_context(&args.common)?;
        if context.auth_mode != "basic" {
            return Err(message(
                "Dashboard plan with --use-export-org requires Basic auth (--basic-user / --basic-password).",
            ));
        }
        return collect_export_org_scope_with_request(args, request_json);
    }
    collect_single_scope_with_request(args, request_json)
}

fn collect_plan_input(args: &super::PlanArgs) -> Result<DashboardPlanInput> {
    let client = if let Some(org_id) = args.org_id {
        build_http_client_for_org(&args.common, org_id)?
    } else {
        build_http_client(&args.common)?
    };
    let mut request_json =
        |method: reqwest::Method,
         path: &str,
         params: &[(String, String)],
         payload: Option<&Value>| { client.request_json(method, path, params, payload) };
    if args.use_export_org {
        let context = build_auth_context(&args.common)?;
        if context.auth_mode != "basic" {
            return Err(message(
                "Dashboard plan with --use-export-org requires Basic auth (--basic-user / --basic-password).",
            ));
        }
        return collect_export_org_scope_with_live_collector(
            args,
            &mut request_json,
            |target_org_id, _request_json| {
                let scoped_client = build_http_client_for_org(&args.common, target_org_id)?;
                let mut scoped_request_json =
                    |method: reqwest::Method,
                     path: &str,
                     params: &[(String, String)],
                     payload: Option<&Value>| {
                        scoped_client.request_json(method, path, params, payload)
                    };
                collect_live_org_state_with_request(&mut scoped_request_json)
            },
        );
    }
    collect_single_scope_with_request(args, &mut request_json)
}

pub(crate) fn build_dashboard_plan(input: DashboardPlanInput) -> DashboardPlanReport {
    let mut orgs = Vec::new();
    let mut actions = Vec::new();
    for org in &input.orgs {
        let org_actions = build_org_actions(org, input.prune);
        orgs.push(build_org_summary(org, &org_actions));
        actions.extend(org_actions);
    }
    let summary = build_summary(&orgs, &actions);
    DashboardPlanReport {
        kind: PLAN_KIND.to_string(),
        schema_version: PLAN_SCHEMA_VERSION,
        tool_version: tool_version().to_string(),
        mode: "review".to_string(),
        scope: input.scope,
        input_type: input.input_type,
        prune: input.prune,
        summary,
        orgs,
        actions,
    }
}

pub(crate) fn build_dashboard_plan_json(report: &DashboardPlanReport) -> Result<Value> {
    serde_json::to_value(report).map_err(|error| message(error.to_string()))
}

pub(crate) fn print_dashboard_plan_report(
    report: &DashboardPlanReport,
    output_format: super::DashboardPlanOutputFormat,
    show_same: bool,
    no_header: bool,
    selected_columns: &[String],
) -> Result<()> {
    match output_format {
        super::DashboardPlanOutputFormat::Json => {
            print!(
                "{}",
                render_json_value(&build_dashboard_plan_json(report)?)?
            );
        }
        super::DashboardPlanOutputFormat::Table => {
            for line in render_plan_table(report, show_same, !no_header, selected_columns) {
                println!("{line}");
            }
            println!("{}", plan_summary_line(report));
        }
        super::DashboardPlanOutputFormat::Text => {
            println!("{}", plan_summary_line(report));
            for line in render_plan_text(report, show_same) {
                println!("{line}");
            }
        }
    }
    Ok(())
}

pub(crate) fn run_dashboard_plan(args: &super::PlanArgs) -> Result<usize> {
    if !args.output_columns.is_empty()
        && args.output_format != super::DashboardPlanOutputFormat::Table
    {
        return Err(message(
            "--output-columns is only supported with --output-format table for dashboard plan.",
        ));
    }
    if args.no_header && args.output_format != super::DashboardPlanOutputFormat::Table {
        return Err(message(
            "--no-header is only supported with --output-format table for dashboard plan.",
        ));
    }
    if args.list_columns {
        print_supported_columns(dashboard_plan_column_ids());
        return Ok(0);
    }
    let input = collect_plan_input(args)?;
    let report = build_dashboard_plan(input);
    print_dashboard_plan_report(
        &report,
        args.output_format,
        args.show_same,
        args.no_header,
        &args.output_columns,
    )?;
    Ok(report.summary.checked)
}

#[cfg(test)]
#[path = "dashboard_plan_rust_tests.rs"]
mod dashboard_plan_rust_tests;
