//! Staged declarative sync resource helpers.
//!
//! Purpose:
//! - Normalize reviewable sync resource specs before any Rust CLI wiring lands.
//! - Keep staged sync summary contracts import-safe and free of Grafana I/O.

use crate::common::{message, Result};
use crate::sync_bundle_alert_contracts::build_alert_bundle_contract_document;
use serde::Serialize;
use serde_json::{Map, Value};

pub const SYNC_SUMMARY_KIND: &str = "grafana-utils-sync-summary";
pub const SYNC_SUMMARY_SCHEMA_VERSION: i64 = 1;
pub const SYNC_SOURCE_BUNDLE_KIND: &str = "grafana-utils-sync-source-bundle";
pub const SYNC_SOURCE_BUNDLE_SCHEMA_VERSION: i64 = 1;
pub const SYNC_PLAN_KIND: &str = "grafana-utils-sync-plan";
pub const SYNC_PLAN_SCHEMA_VERSION: i64 = 1;
pub const SYNC_APPLY_INTENT_KIND: &str = "grafana-utils-sync-apply-intent";
pub const SYNC_APPLY_INTENT_SCHEMA_VERSION: i64 = 1;
pub const RESOURCE_KINDS: &[&str] = &["dashboard", "datasource", "folder", "alert"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SyncResourceSpec {
    pub kind: String,
    pub identity: String,
    pub title: String,
    pub body: Map<String, Value>,
    pub managed_fields: Vec<String>,
    pub source_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SyncSummary {
    pub resource_count: usize,
    pub dashboard_count: usize,
    pub datasource_count: usize,
    pub folder_count: usize,
    pub alert_count: usize,
}

fn normalize_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.trim().to_string(),
        Some(Value::Number(number)) => number.to_string(),
        Some(Value::Bool(flag)) => {
            if *flag {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        _ => String::new(),
    }
}

fn require_object<'a>(value: Option<&'a Value>, label: &str) -> Result<&'a Map<String, Value>> {
    match value {
        None => Err(message(format!("{label} must be a JSON object."))),
        Some(Value::Object(object)) => Ok(object),
        Some(_) => Err(message(format!("{label} must be a JSON object."))),
    }
}

fn normalize_string_list(value: Option<&Value>, label: &str) -> Result<Vec<String>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let items = value
        .as_array()
        .ok_or_else(|| message(format!("{label} must be a list.")))?;
    let mut normalized = Vec::with_capacity(items.len());
    for item in items {
        let text = normalize_text(Some(item));
        if text.is_empty() {
            return Err(message(format!("{label} cannot contain empty values.")));
        }
        normalized.push(text);
    }
    Ok(normalized)
}

fn extract_identity(spec: &Map<String, Value>) -> String {
    for field in ["uid", "name", "title", "path"] {
        let value = normalize_text(spec.get(field));
        if !value.is_empty() {
            return value;
        }
    }
    String::new()
}

fn extract_title(spec: &Map<String, Value>, fallback_identity: &str) -> String {
    for field in ["title", "name", "uid", "path"] {
        let value = normalize_text(spec.get(field));
        if !value.is_empty() {
            return value;
        }
    }
    fallback_identity.to_string()
}

fn extract_body(spec: &Map<String, Value>) -> Result<Map<String, Value>> {
    if let Some(body) = spec.get("body") {
        return Ok(require_object(Some(body), "body")?.clone());
    }
    if let Some(body) = spec.get("spec") {
        return Ok(require_object(Some(body), "spec")?.clone());
    }
    Ok(Map::new())
}

pub fn normalize_resource_spec(raw_spec: &Value) -> Result<SyncResourceSpec> {
    let spec = require_object(Some(raw_spec), "Sync resource spec")?;
    let kind = normalize_text(spec.get("kind")).to_lowercase();
    if !RESOURCE_KINDS.contains(&kind.as_str()) {
        return Err(message(format!(
            "Unsupported sync resource kind {:?}. Expected one of {}.",
            kind,
            RESOURCE_KINDS.join(", ")
        )));
    }
    let identity = extract_identity(spec);
    if identity.is_empty() {
        return Err(message(
            "Sync resource spec requires uid, name, title, or path.",
        ));
    }
    let managed_fields = normalize_string_list(spec.get("managedFields"), "managedFields")?;
    if kind == "alert" && managed_fields.is_empty() {
        return Err(message(
            "Alert sync specs must declare managedFields to keep partial ownership explicit.",
        ));
    }
    Ok(SyncResourceSpec {
        kind,
        identity: identity.clone(),
        title: extract_title(spec, &identity),
        body: extract_body(spec)?,
        managed_fields,
        source_path: normalize_text(spec.get("sourcePath")),
    })
}

pub fn normalize_resource_specs(raw_specs: &[Value]) -> Result<Vec<SyncResourceSpec>> {
    raw_specs
        .iter()
        .map(normalize_resource_spec)
        .collect::<Result<Vec<_>>>()
}

pub fn summarize_resource_specs(specs: &[SyncResourceSpec]) -> SyncSummary {
    SyncSummary {
        resource_count: specs.len(),
        dashboard_count: specs.iter().filter(|item| item.kind == "dashboard").count(),
        datasource_count: specs
            .iter()
            .filter(|item| item.kind == "datasource")
            .count(),
        folder_count: specs.iter().filter(|item| item.kind == "folder").count(),
        alert_count: specs.iter().filter(|item| item.kind == "alert").count(),
    }
}

pub fn build_sync_summary_document(raw_specs: &[Value]) -> Result<Value> {
    let specs = normalize_resource_specs(raw_specs)?;
    let summary = summarize_resource_specs(&specs);
    Ok(serde_json::json!({
        "kind": SYNC_SUMMARY_KIND,
        "schemaVersion": SYNC_SUMMARY_SCHEMA_VERSION,
        "summary": {
            "resourceCount": summary.resource_count,
            "dashboardCount": summary.dashboard_count,
            "datasourceCount": summary.datasource_count,
            "folderCount": summary.folder_count,
            "alertCount": summary.alert_count,
        },
        "resources": specs.iter().map(|item| {
            serde_json::json!({
                "kind": item.kind,
                "identity": item.identity,
                "title": item.title,
                "managedFields": item.managed_fields,
                "bodyFieldCount": item.body.len(),
                "sourcePath": item.source_path,
            })
        }).collect::<Vec<_>>(),
    }))
}

pub fn build_sync_source_bundle_document(
    dashboards: &[Value],
    datasources: &[Value],
    folders: &[Value],
    alerts: &[Value],
    alerting: Option<&Value>,
    metadata: Option<&Value>,
) -> Result<Value> {
    let alerting = alerting
        .cloned()
        .unwrap_or_else(|| Value::Object(Map::new()));
    let alerting_contract_source = if alerting.get("alerting").is_some() {
        alerting.clone()
    } else {
        let mut root = Map::new();
        root.insert("alerting".to_string(), alerting.clone());
        Value::Object(root)
    };
    let metadata = metadata
        .cloned()
        .unwrap_or_else(|| Value::Object(Map::new()));
    let alerting_summary = alerting
        .get("summary")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let alert_contract = build_alert_bundle_contract_document(&alerting_contract_source);
    Ok(serde_json::json!({
        "kind": SYNC_SOURCE_BUNDLE_KIND,
        "schemaVersion": SYNC_SOURCE_BUNDLE_SCHEMA_VERSION,
        "summary": {
            "dashboardCount": dashboards.len(),
            "datasourceCount": datasources.len(),
            "folderCount": folders.len(),
            "alertRuleCount": alerting_summary.get("ruleCount").and_then(Value::as_i64).unwrap_or(0),
            "contactPointCount": alerting_summary.get("contactPointCount").and_then(Value::as_i64).unwrap_or(0),
            "muteTimingCount": alerting_summary.get("muteTimingCount").and_then(Value::as_i64).unwrap_or(0),
            "policyCount": alerting_summary.get("policyCount").and_then(Value::as_i64).unwrap_or(0),
            "templateCount": alerting_summary.get("templateCount").and_then(Value::as_i64).unwrap_or(0),
        },
        "dashboards": dashboards,
        "datasources": datasources,
        "folders": folders,
        "alerts": alerts,
        "alerting": alerting,
        "alertContract": alert_contract,
        "metadata": metadata,
    }))
}

pub fn render_sync_source_bundle_text(document: &Value) -> Result<Vec<String>> {
    if document.get("kind").and_then(Value::as_str) != Some(SYNC_SOURCE_BUNDLE_KIND) {
        return Err(message(
            "Sync source bundle document kind is not supported.",
        ));
    }
    let summary = document
        .get("summary")
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| message("Sync source bundle document is missing summary."))?;
    Ok(vec![
        "Sync source bundle".to_string(),
        format!(
            "Dashboards: {}",
            summary
                .get("dashboardCount")
                .and_then(Value::as_i64)
                .unwrap_or(0)
        ),
        format!(
            "Datasources: {}",
            summary
                .get("datasourceCount")
                .and_then(Value::as_i64)
                .unwrap_or(0)
        ),
        format!(
            "Folders: {}",
            summary
                .get("folderCount")
                .and_then(Value::as_i64)
                .unwrap_or(0)
        ),
        format!(
            "Alerting: rules={} contact-points={} mute-timings={} policies={} templates={}",
            summary
                .get("alertRuleCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("contactPointCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("muteTimingCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("policyCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("templateCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
        ),
    ])
}

fn build_index(
    specs: &[SyncResourceSpec],
) -> Result<std::collections::BTreeMap<(String, String), SyncResourceSpec>> {
    let mut index = std::collections::BTreeMap::new();
    for spec in specs {
        let key = (spec.kind.clone(), spec.identity.clone());
        if index.contains_key(&key) {
            return Err(message(format!(
                "Duplicate sync identity detected for {} {}.",
                spec.kind, spec.identity
            )));
        }
        index.insert(key, spec.clone());
    }
    Ok(index)
}

fn compare_body(desired: &SyncResourceSpec, live: &SyncResourceSpec) -> Vec<String> {
    let mut fields = std::collections::BTreeSet::new();
    for key in desired.body.keys() {
        fields.insert(key.clone());
    }
    let managed_filter = if desired.managed_fields.is_empty() {
        None
    } else {
        Some(
            desired
                .managed_fields
                .iter()
                .cloned()
                .collect::<std::collections::BTreeSet<String>>(),
        )
    };
    for key in live.body.keys() {
        if managed_filter
            .as_ref()
            .map(|set| set.contains(key))
            .unwrap_or(true)
        {
            fields.insert(key.clone());
        }
    }
    fields
        .into_iter()
        .filter(|field| desired.body.get(field) != live.body.get(field))
        .collect()
}

fn build_alert_assessment_document(operations: &[Value]) -> Value {
    let mut alerts = Vec::new();
    let mut candidate_count = 0i64;
    let mut plan_only_count = 0i64;
    let mut blocked_count = 0i64;
    for item in operations {
        let Some(object) = item.as_object() else {
            continue;
        };
        if object.get("kind").and_then(Value::as_str) != Some("alert") {
            continue;
        }
        let managed_fields = object
            .get("managedFields")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|value| value.as_str().map(str::to_string))
            .collect::<Vec<String>>();
        let desired = object
            .get("desired")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let has_condition = managed_fields.iter().any(|field| field == "condition");
        let has_plan_only_fields = managed_fields
            .iter()
            .any(|field| field == "contactPoints" || field == "annotations");
        let condition_text = desired
            .get("condition")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        let (status, live_apply_allowed, detail) = if !has_condition {
            (
                "blocked",
                false,
                "Alert sync must manage condition explicitly before live apply can be considered.",
            )
        } else if has_plan_only_fields {
            (
                "plan-only",
                false,
                "Alert sync includes linked routing or annotation fields and stays plan-only until mutation semantics settle.",
            )
        } else if condition_text.is_empty() {
            (
                "blocked",
                false,
                "Alert sync body must include a non-empty condition.",
            )
        } else {
            (
                "candidate",
                true,
                "Alert sync scope is narrow enough for future controlled live-apply experiments.",
            )
        };
        match status {
            "candidate" => candidate_count += 1,
            "plan-only" => plan_only_count += 1,
            _ => blocked_count += 1,
        }
        alerts.push(serde_json::json!({
            "identity": object.get("identity").cloned().unwrap_or(Value::Null),
            "title": object.get("title").cloned().unwrap_or(Value::Null),
            "managedFields": managed_fields,
            "status": status,
            "liveApplyAllowed": live_apply_allowed,
            "detail": detail,
        }));
    }
    serde_json::json!({
        "kind": "grafana-utils-alert-sync-plan",
        "schemaVersion": 1,
        "summary": {
            "alertCount": alerts.len(),
            "candidateCount": candidate_count,
            "planOnlyCount": plan_only_count,
            "blockedCount": blocked_count,
        },
        "alerts": alerts,
    })
}

pub fn build_sync_plan_document(
    desired_specs: &[Value],
    live_specs: &[Value],
    allow_prune: bool,
) -> Result<Value> {
    let desired = normalize_resource_specs(desired_specs)?;
    let live = normalize_resource_specs(live_specs)?;
    let desired_index = build_index(&desired)?;
    let live_index = build_index(&live)?;
    let mut operations = Vec::new();
    let mut would_create = 0i64;
    let mut would_update = 0i64;
    let mut would_delete = 0i64;
    let mut noop = 0i64;
    let mut unmanaged = 0i64;

    for (key, desired_spec) in &desired_index {
        if let Some(live_spec) = live_index.get(key) {
            let changed_fields = compare_body(desired_spec, live_spec);
            let action = if changed_fields.is_empty() {
                noop += 1;
                "noop"
            } else {
                would_update += 1;
                "would-update"
            };
            operations.push(serde_json::json!({
                "kind": desired_spec.kind,
                "identity": desired_spec.identity,
                "title": desired_spec.title,
                "action": action,
                "reason": if action == "noop" { "in-sync" } else { "drift-detected" },
                "changedFields": changed_fields,
                "managedFields": desired_spec.managed_fields,
                "desired": desired_spec.body,
                "live": live_spec.body,
                "sourcePath": desired_spec.source_path,
            }));
        } else {
            would_create += 1;
            operations.push(serde_json::json!({
                "kind": desired_spec.kind,
                "identity": desired_spec.identity,
                "title": desired_spec.title,
                "action": "would-create",
                "reason": "missing-live",
                "changedFields": desired_spec.body.keys().cloned().collect::<Vec<String>>(),
                "managedFields": desired_spec.managed_fields,
                "desired": desired_spec.body,
                "live": Value::Null,
                "sourcePath": desired_spec.source_path,
            }));
        }
    }

    for (key, live_spec) in &live_index {
        if desired_index.contains_key(key) {
            continue;
        }
        let action = if allow_prune {
            would_delete += 1;
            "would-delete"
        } else {
            unmanaged += 1;
            "unmanaged"
        };
        operations.push(serde_json::json!({
            "kind": live_spec.kind,
            "identity": live_spec.identity,
            "title": live_spec.title,
            "action": action,
            "reason": if allow_prune { "missing-from-desired-state" } else { "prune-disabled" },
            "changedFields": Vec::<String>::new(),
            "managedFields": Vec::<String>::new(),
            "desired": Value::Null,
            "live": live_spec.body,
            "sourcePath": live_spec.source_path,
        }));
    }

    let alert_assessment = build_alert_assessment_document(&operations);
    Ok(serde_json::json!({
        "kind": SYNC_PLAN_KIND,
        "schemaVersion": SYNC_PLAN_SCHEMA_VERSION,
        "dryRun": true,
        "reviewRequired": true,
        "reviewed": false,
        "allowPrune": allow_prune,
        "summary": {
            "would_create": would_create,
            "would_update": would_update,
            "would_delete": would_delete,
            "noop": noop,
            "unmanaged": unmanaged,
            "alert_candidate": alert_assessment["summary"]["candidateCount"],
            "alert_plan_only": alert_assessment["summary"]["planOnlyCount"],
            "alert_blocked": alert_assessment["summary"]["blockedCount"],
        },
        "alertAssessment": alert_assessment,
        "operations": operations,
    }))
}

pub fn build_sync_apply_intent_document(plan_document: &Value, approve: bool) -> Result<Value> {
    let plan = plan_document
        .as_object()
        .ok_or_else(|| message("Sync plan document must be a JSON object."))?;
    if plan.get("kind").and_then(Value::as_str) != Some(SYNC_PLAN_KIND) {
        return Err(message("Sync plan document kind is not supported."));
    }
    if plan
        .get("reviewRequired")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && !plan
            .get("reviewed")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    {
        return Err(message(
            "Refusing local sync apply intent before the reviewable plan is marked reviewed.",
        ));
    }
    if !approve {
        return Err(message(
            "Refusing local sync apply intent without explicit approval.",
        ));
    }
    let operations = plan
        .get("operations")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| message("Sync plan document is missing operations."))?;
    let executable_operations = operations
        .into_iter()
        .filter(|item| {
            matches!(
                item.get("action").and_then(Value::as_str),
                Some("would-create" | "would-update" | "would-delete")
            )
        })
        .collect::<Vec<Value>>();
    Ok(serde_json::json!({
        "kind": SYNC_APPLY_INTENT_KIND,
        "schemaVersion": SYNC_APPLY_INTENT_SCHEMA_VERSION,
        "mode": "apply",
        "reviewed": plan.get("reviewed").cloned().unwrap_or(Value::Bool(false)),
        "reviewRequired": plan.get("reviewRequired").cloned().unwrap_or(Value::Bool(true)),
        "allowPrune": plan.get("allowPrune").cloned().unwrap_or(Value::Bool(false)),
        "approved": true,
        "summary": plan.get("summary").cloned().unwrap_or(Value::Null),
        "alertAssessment": plan.get("alertAssessment").cloned().unwrap_or(Value::Null),
        "operations": executable_operations,
    }))
}
