//! Dashboard review-first plan builder and renderer.

use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::common::{
    message, print_supported_columns, render_json_value, string_field, tool_version,
    value_as_object, Result,
};
use crate::grafana_api::DashboardResourceClient;

use super::build_folder_path;
use super::cli_defs::{DashboardImportInputFormat, InspectExportInputType};
use super::import_target::build_dashboard_target_review;
use super::{
    build_datasource_catalog, build_http_client, build_http_client_for_org,
    collect_datasource_refs, discover_dashboard_files, extract_dashboard_object,
    load_dashboard_source, load_export_metadata, load_folder_inventory, load_json_file,
    lookup_datasource, FolderInventoryItem, LoadedDashboardSource, DEFAULT_FOLDER_TITLE,
    DEFAULT_FOLDER_UID, DEFAULT_PAGE_SIZE,
};

const PLAN_KIND: &str = "grafana-util-dashboard-plan";
const PLAN_SCHEMA_VERSION: i64 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DashboardPlanChange {
    pub(crate) field: String,
    pub(crate) before: Value,
    pub(crate) after: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DashboardPlanAction {
    pub(crate) action_id: String,
    pub(crate) domain: String,
    pub(crate) resource_kind: String,
    pub(crate) dashboard_uid: String,
    pub(crate) title: String,
    pub(crate) folder_uid: String,
    pub(crate) folder_path: String,
    pub(crate) source_org_id: Option<String>,
    pub(crate) source_org_name: String,
    pub(crate) target_org_id: Option<String>,
    pub(crate) target_org_name: String,
    pub(crate) match_basis: String,
    pub(crate) action: String,
    pub(crate) status: String,
    pub(crate) changed_fields: Vec<String>,
    pub(crate) changes: Vec<DashboardPlanChange>,
    pub(crate) source_file: Option<String>,
    pub(crate) target_uid: Option<String>,
    pub(crate) target_version: Option<i64>,
    pub(crate) target_evidence: Vec<String>,
    pub(crate) dependency_hints: Vec<String>,
    pub(crate) blocked_reason: Option<String>,
    pub(crate) review_hints: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DashboardPlanOrgSummary {
    pub(crate) source_org_id: Option<String>,
    pub(crate) source_org_name: String,
    pub(crate) target_org_id: Option<String>,
    pub(crate) target_org_name: String,
    pub(crate) org_action: String,
    pub(crate) input_dir: String,
    pub(crate) checked: usize,
    pub(crate) same: usize,
    pub(crate) create: usize,
    pub(crate) update: usize,
    pub(crate) extra: usize,
    pub(crate) delete: usize,
    pub(crate) blocked: usize,
    pub(crate) warning: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DashboardPlanSummary {
    pub(crate) checked: usize,
    pub(crate) same: usize,
    pub(crate) create: usize,
    pub(crate) update: usize,
    pub(crate) extra: usize,
    pub(crate) delete: usize,
    pub(crate) blocked: usize,
    pub(crate) warning: usize,
    pub(crate) org_count: usize,
    pub(crate) would_create_org_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DashboardPlanReport {
    pub(crate) kind: String,
    #[serde(rename = "schemaVersion")]
    pub(crate) schema_version: i64,
    pub(crate) tool_version: String,
    pub(crate) mode: String,
    pub(crate) scope: String,
    pub(crate) input_type: String,
    pub(crate) prune: bool,
    pub(crate) summary: DashboardPlanSummary,
    pub(crate) orgs: Vec<DashboardPlanOrgSummary>,
    pub(crate) actions: Vec<DashboardPlanAction>,
}

#[derive(Debug, Clone)]
struct LocalDashboard {
    file_path: String,
    dashboard: Value,
    dashboard_uid: String,
    title: String,
    folder_uid: String,
    folder_path: String,
}

#[derive(Debug, Clone)]
struct LiveDashboard {
    uid: String,
    title: String,
    folder_uid: String,
    folder_path: String,
    version: Option<i64>,
    evidence: Vec<String>,
    payload: Value,
}

#[derive(Debug, Clone)]
struct OrgPlanInput {
    source_org_id: Option<String>,
    source_org_name: String,
    target_org_id: Option<String>,
    target_org_name: String,
    org_action: String,
    input_dir: PathBuf,
    local_dashboards: Vec<LocalDashboard>,
    live_dashboards: Vec<LiveDashboard>,
    live_datasources: Vec<Map<String, Value>>,
    folder_inventory: Vec<FolderInventoryItem>,
}

#[derive(Debug, Clone)]
pub(crate) struct DashboardPlanInput {
    scope: String,
    input_type: String,
    prune: bool,
    org: OrgPlanInput,
}

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

fn resolve_live_folder_path(
    client: &DashboardResourceClient<'_>,
    folder_uid: &str,
) -> Result<String> {
    if folder_uid.trim().is_empty() || folder_uid == DEFAULT_FOLDER_UID {
        return Ok(DEFAULT_FOLDER_TITLE.to_string());
    }
    match client.fetch_folder_if_exists(folder_uid)? {
        Some(folder) => Ok(build_folder_path(&folder, DEFAULT_FOLDER_TITLE)),
        None => Ok(folder_uid.to_string()),
    }
}

fn build_live_dashboard(
    client: &DashboardResourceClient<'_>,
    payload: &Value,
) -> Result<LiveDashboard> {
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
    let folder_path = resolve_live_folder_path(client, &folder_uid)?;
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

fn build_action_id(org_id: &Option<String>, uid: &str, seed: usize) -> String {
    let org = org_id.as_deref().unwrap_or("unknown");
    let resource = if uid.is_empty() { "dashboard" } else { uid };
    format!("org:{org}/dashboard:{resource}:{seed}")
}

fn build_org_actions(org: &OrgPlanInput, prune: bool) -> Vec<DashboardPlanAction> {
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
        let (dependency_hints, mut review_hints) =
            build_dependency_hints(&local.dashboard, &org.live_datasources);
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

        let target_review_blocked = live
            .as_ref()
            .map(|dashboard| {
                dashboard
                    .evidence
                    .iter()
                    .any(|value| value.starts_with("provisioned=true"))
            })
            .unwrap_or(false);

        let mut action = if live.is_none() {
            "would-create".to_string()
        } else if changed_fields.is_empty()
            && local.folder_uid == live.map(|item| item.folder_uid.clone()).unwrap_or_default()
        {
            "same".to_string()
        } else {
            "would-update".to_string()
        };
        let mut status = if action == "same" {
            "same".to_string()
        } else {
            "ready".to_string()
        };
        let mut blocked_reason = None;
        if target_review_blocked && matches!(action.as_str(), "would-update" | "would-delete") {
            action = "blocked-target".to_string();
            status = "blocked".to_string();
            blocked_reason = Some("target-provisioned-or-managed".to_string());
        } else if action != "same"
            && (!dependency_hints.is_empty()
                || review_hints
                    .iter()
                    .any(|hint| hint.starts_with("missing-folder-uid=")))
        {
            status = "warning".to_string();
        }

        let target_uid = live.as_ref().map(|item| item.uid.clone());
        let target_version = live.as_ref().and_then(|item| item.version);
        let target_evidence = live
            .as_ref()
            .map(|item| item.evidence.clone())
            .unwrap_or_default();
        actions.push(DashboardPlanAction {
            action_id: build_action_id(&org.target_org_id, &local.dashboard_uid, index),
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
            action_id: build_action_id(&org.target_org_id, &live.uid, index),
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
                "would-delete".to_string()
            } else {
                "extra-remote".to_string()
            },
            status: if blocked {
                "blocked".to_string()
            } else if prune {
                "ready".to_string()
            } else {
                "warning".to_string()
            },
            changed_fields: Vec::new(),
            changes: Vec::new(),
            source_file: None,
            target_uid: Some(live.uid.clone()),
            target_version: live.version,
            target_evidence: live.evidence.clone(),
            dependency_hints: Vec::new(),
            blocked_reason: blocked.then_some("target-provisioned-or-managed".to_string()),
            review_hints: vec!["remote-only dashboard candidate".to_string()],
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
            "same" => summary.same += 1,
            "would-create" => summary.create += 1,
            "would-update" => summary.update += 1,
            "extra-remote" => summary.extra += 1,
            "would-delete" => summary.delete += 1,
            _ => {}
        }
        match action.status.as_str() {
            "blocked" => summary.blocked += 1,
            "warning" => summary.warning += 1,
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
        if org.org_action == "would-create" {
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
        if !show_same && action.action == "same" {
            continue;
        }
        lines.push(format!(
            "{} org={} uid={} title={} folder={} action={} status={} changed={}",
            if action.status == "blocked" {
                "BLOCK"
            } else if action.action == "would-delete" {
                "DELETE"
            } else if action.action == "would-create" {
                "CREATE"
            } else if action.action == "would-update" {
                "UPDATE"
            } else if action.action == "extra-remote" {
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
        .filter(|action| show_same || action.action != "same")
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

fn collect_single_scope(
    resolved: &LoadedDashboardSource,
    target_org_id: Option<String>,
    target_org_name: String,
    source_org_id: Option<String>,
    source_org_name: String,
    org_action: String,
    client: &DashboardResourceClient<'_>,
) -> Result<OrgPlanInput> {
    let metadata = load_export_metadata(&resolved.input_dir, Some(resolved.expected_variant))?;
    let folder_inventory = load_folder_inventory(&resolved.input_dir, metadata.as_ref())?;
    let mut local_dashboards = Vec::new();
    for file in discover_dashboard_files(&resolved.input_dir)? {
        let document = load_json_file(&file)?;
        local_dashboards.push(build_local_dashboard(&document, &file, &folder_inventory)?);
    }
    let live_datasources = client.list_datasources()?;

    let mut live_dashboards = Vec::new();
    let summaries = client.list_dashboard_summaries(DEFAULT_PAGE_SIZE)?;
    let mut seen_uids = BTreeSet::new();
    for summary in summaries {
        let uid = string_field(&summary, "uid", "");
        if uid.is_empty() || !seen_uids.insert(uid.clone()) {
            continue;
        }
        let payload = client.fetch_dashboard(&uid)?;
        live_dashboards.push(build_live_dashboard(client, &payload)?);
    }

    Ok(OrgPlanInput {
        source_org_id,
        source_org_name,
        target_org_id,
        target_org_name,
        org_action,
        input_dir: resolved.input_dir.clone(),
        local_dashboards,
        live_dashboards,
        live_datasources,
        folder_inventory,
    })
}

fn collect_plan_input(args: &super::PlanArgs) -> Result<DashboardPlanInput> {
    if args.use_export_org || !args.only_org_id.is_empty() || args.create_missing_orgs {
        return Err(message(
            "Dashboard plan export-org routing is not enabled in this minimal slice yet.",
        ));
    }

    let expected_variant = match args.input_type {
        InspectExportInputType::Raw => super::RAW_EXPORT_SUBDIR,
        InspectExportInputType::Source => super::PROMPT_EXPORT_SUBDIR,
    };
    let resolved = load_dashboard_source(
        &args.input_dir,
        DashboardImportInputFormat::Raw,
        Some(args.input_type),
        false,
    )?;
    let client = if let Some(org_id) = args.org_id {
        build_http_client_for_org(&args.common, org_id)?
    } else {
        build_http_client(&args.common)?
    };
    let dashboard_client = DashboardResourceClient::new(&client);
    let current_org = dashboard_client.fetch_current_org()?;
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
    let org = collect_single_scope(
        &resolved,
        target_org_id.clone(),
        target_org_name.clone(),
        source_org_id,
        source_org_name,
        if args.org_id.is_some() {
            "explicit-org".to_string()
        } else {
            "current-org".to_string()
        },
        &dashboard_client,
    )?;
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
        org,
    })
}

pub(crate) fn build_dashboard_plan(input: DashboardPlanInput) -> DashboardPlanReport {
    let actions = build_org_actions(&input.org, input.prune);
    let org_summary = build_org_summary(&input.org, &actions);
    let orgs = vec![org_summary];
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
