//! Build alert compare/import/delete plan documents from live Grafana payloads.
//!
//! Responsibilities:
//! - Gather and normalize alert resources through shared request helpers.
//! - Produce plan and delete-preview documents used by diff/import execution flows.
//! - Preserve request semantics so CLI/runtime callers receive a stable sync-ready
//!   shape across execution paths.

use crate::common::{message, tool_version, value_as_object, Result};
use reqwest::Method;
use serde_json::{json, Map, Value};

use crate::common::{build_shared_diff_document, SharedDiffSummary};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::alert_support::{
    build_contact_point_scaffold_document, build_managed_policy_route_preview,
    normalize_compare_payload, remove_managed_policy_subtree, upsert_managed_policy_subtree,
};
#[allow(unused_imports)]
use super::{
    build_compare_document, build_contact_point_import_payload, build_import_operation,
    build_mute_timing_import_payload, build_new_contact_point_scaffold_document,
    build_new_rule_scaffold_document, build_new_template_scaffold_document,
    build_policies_import_payload, build_resource_identity, build_rule_import_payload,
    build_template_import_payload, discover_alert_resource_files, init_alert_managed_dir,
    load_alert_resource_file, resource_subdir_by_kind, strip_server_managed_fields,
    write_alert_resource_file, CONTACT_POINT_KIND, MUTE_TIMING_KIND, POLICIES_KIND, RULE_KIND,
    TEMPLATE_KIND,
};

pub const ALERT_PLAN_KIND: &str = "grafana-util-alert-plan";
pub const ALERT_PLAN_SCHEMA_VERSION: i64 = 1;
pub const ALERT_DELETE_PREVIEW_KIND: &str = "grafana-util-alert-delete-preview";
pub const ALERT_DELETE_PREVIEW_SCHEMA_VERSION: i64 = 1;
pub const ALERT_IMPORT_DRY_RUN_KIND: &str = "grafana-util-alert-import-dry-run";
pub const ALERT_IMPORT_DRY_RUN_SCHEMA_VERSION: i64 = 1;
pub const ALERT_MANAGED_POLICY_PREVIEW_SCHEMA_VERSION: i64 = 1;
type AlertDesiredOperation = (PathBuf, String, Map<String, Value>);

fn row_object<'a>(row: &'a Value, label: &str) -> Result<&'a Map<String, Value>> {
    value_as_object(row, label)
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[allow(unused_imports)]
use crate::grafana_api::alert_live::{
    apply_create_with_request, apply_delete_with_request, apply_update_with_request,
    fetch_live_compare_document_with_request, request_array_with_request,
    request_live_resources_by_kind_with_request, request_optional_object_with_request,
};

fn plan_summary(rows: &[Value]) -> Value {
    let count = |action: &str| {
        rows.iter()
            .filter(|row| row.get("action").and_then(Value::as_str) == Some(action))
            .count()
    };
    let warning = rows
        .iter()
        .filter(|row| row.get("status").and_then(Value::as_str) == Some("warning"))
        .count();
    json!({
        "processed": rows.len(),
        "create": count("create"),
        "update": count("update"),
        "noop": count("noop"),
        "delete": count("delete"),
        "blocked": count("blocked"),
        "warning": warning,
    })
}

fn plan_action_id(kind: &str, identity: &str, action: &str) -> String {
    format!("{kind}::{identity}::{action}")
}

fn field_change(field: &str, before: Option<&Value>, after: Option<&Value>) -> Value {
    json!({
        "field": field,
        "before": before.cloned().unwrap_or(Value::Null),
        "after": after.cloned().unwrap_or(Value::Null),
    })
}

fn compare_field_changes(
    desired: Option<&Map<String, Value>>,
    live: Option<&Map<String, Value>>,
) -> (Vec<String>, Vec<Value>) {
    let mut fields = BTreeSet::new();
    if let Some(object) = desired {
        fields.extend(object.keys().cloned());
    }
    if let Some(object) = live {
        fields.extend(object.keys().cloned());
    }

    let mut changed_fields = Vec::new();
    let mut changes = Vec::new();
    for field in fields {
        let desired_value = desired.and_then(|object| object.get(&field));
        let live_value = live.and_then(|object| object.get(&field));
        if desired_value != live_value {
            changed_fields.push(field.clone());
            changes.push(field_change(&field, live_value, desired_value));
        }
    }
    (changed_fields, changes)
}

fn review_hint(code: &str, field: &str, before: Option<&str>, after: Option<&str>) -> Value {
    json!({
        "code": code,
        "field": field,
        "before": before.unwrap_or(""),
        "after": after.unwrap_or(""),
    })
}

fn build_rule_review_hints(payload: &Map<String, Value>) -> Vec<Value> {
    let mut hints = Vec::new();
    let annotations = payload.get("annotations").and_then(Value::as_object);
    let dashboard_uid = annotations
        .and_then(|value| value.get("__dashboardUid__"))
        .and_then(Value::as_str);
    if let Some(dashboard_uid) = dashboard_uid {
        hints.push(review_hint(
            "linked-dashboard-reference",
            "annotations.__dashboardUid__",
            Some(dashboard_uid),
            Some(dashboard_uid),
        ));
    }

    let panel_id =
        annotations
            .and_then(|value| value.get("__panelId__"))
            .map(|value| match value {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            });
    if let Some(panel_id) = panel_id {
        hints.push(review_hint(
            "linked-panel-reference",
            "annotations.__panelId__",
            Some(&panel_id),
            Some(&panel_id),
        ));
    }

    hints
}

fn plan_status(action: &str, blocked_reason: Option<&str>, review_hints: &[Value]) -> &'static str {
    if blocked_reason.is_some() {
        "blocked"
    } else if !review_hints.is_empty() {
        "warning"
    } else if action == "noop" {
        "same"
    } else {
        "ready"
    }
}

struct PlanRowInput<'a> {
    kind: &'a str,
    identity: &'a str,
    path: Option<&'a Path>,
    action: &'a str,
    reason: &'a str,
    blocked_reason: Option<&'a str>,
    desired: Option<&'a Map<String, Value>>,
    live: Option<&'a Map<String, Value>>,
    review_hints: Vec<Value>,
}

fn build_plan_row(input: PlanRowInput<'_>) -> Value {
    let (changed_fields, changes) = if input.action == "noop" {
        (Vec::new(), Vec::new())
    } else {
        compare_field_changes(input.desired, input.live)
    };
    let status = plan_status(input.action, input.blocked_reason, &input.review_hints);
    json!({
        "domain": "alert",
        "resourceKind": input.kind,
        "kind": input.kind,
        "identity": input.identity,
        "actionId": plan_action_id(input.kind, input.identity, input.action),
        "action": input.action,
        "status": status,
        "reason": input.reason,
        "blockedReason": input.blocked_reason,
        "reviewHints": input.review_hints,
        "changedFields": changed_fields,
        "changes": changes,
        "path": input.path.map(path_string).map(Value::String).unwrap_or(Value::Null),
        "desired": input.desired.map(|value| Value::Object(value.clone())).unwrap_or(Value::Null),
        "live": input.live.map(|value| Value::Object(value.clone())).unwrap_or(Value::Null),
    })
}

pub fn build_alert_plan_document(rows: &[Value], allow_prune: bool) -> Value {
    json!({
        "kind": ALERT_PLAN_KIND,
        "schemaVersion": ALERT_PLAN_SCHEMA_VERSION,
        "toolVersion": tool_version(),
        "reviewRequired": true,
        "reviewed": false,
        "allowPrune": allow_prune,
        "summary": plan_summary(rows),
        "rows": rows,
    })
}

pub fn build_alert_delete_preview_document(rows: &[Value], allow_policy_reset: bool) -> Value {
    let count = |action: &str| {
        rows.iter()
            .filter(|row| row.get("action").and_then(Value::as_str) == Some(action))
            .count()
    };
    json!({
        "kind": ALERT_DELETE_PREVIEW_KIND,
        "schemaVersion": ALERT_DELETE_PREVIEW_SCHEMA_VERSION,
        "toolVersion": tool_version(),
        "reviewRequired": true,
        "reviewed": false,
        "allowPolicyReset": allow_policy_reset,
        "summary": {
            "processed": rows.len(),
            "delete": count("delete"),
            "blocked": count("blocked"),
        },
        "rows": rows,
    })
}

pub fn load_alert_desired_operations(dir: &Path) -> Result<Vec<AlertDesiredOperation>> {
    let resource_files = discover_alert_resource_files(dir)?;
    let mut operations = Vec::new();
    for path in resource_files {
        let document = load_alert_resource_file(&path, "Alerting resource")?;
        let (kind, payload) = build_import_operation(&document)?;
        operations.push((path, kind, payload));
    }
    Ok(operations)
}

pub fn build_alert_plan_with_request<F>(
    mut request_json: F,
    desired_dir: &Path,
    allow_prune: bool,
) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let desired_operations = load_alert_desired_operations(desired_dir)?;
    let mut rows = Vec::new();
    let mut desired_keys = BTreeSet::new();

    for (path, kind, payload) in desired_operations {
        let identity = build_resource_identity(&kind, &payload);
        let key = (kind.clone(), identity.clone());
        if !desired_keys.insert(key.clone()) {
            return Err(message(format!(
                "Duplicate alert desired identity detected for kind={} id={}.",
                kind, identity
            )));
        }
        let review_hints = if kind == RULE_KIND {
            build_rule_review_hints(&payload)
        } else {
            Vec::new()
        };
        let desired_compare =
            build_compare_document(&kind, &normalize_compare_payload(&kind, &payload));
        let live_compare =
            fetch_live_compare_document_with_request(&mut request_json, &kind, &payload)?;
        let action = match live_compare.as_ref() {
            None => "create",
            Some(live) if live == &desired_compare => "noop",
            Some(_) => "update",
        };
        let row_reason = match action {
            "create" => "missing-live",
            "noop" => "in-sync",
            "update" => "drift-detected",
            _ => unreachable!(),
        };
        let live_payload = live_compare.as_ref().and_then(Value::as_object);
        rows.push(build_plan_row(PlanRowInput {
            kind: &kind,
            identity: &identity,
            path: Some(&path),
            action,
            reason: row_reason,
            blocked_reason: None,
            desired: Some(&payload),
            live: live_payload,
            review_hints,
        }));
    }

    for kind in resource_subdir_by_kind().keys() {
        let mut live_items = request_live_resources_by_kind_with_request(&mut request_json, kind)?
            .into_iter()
            .map(|item| {
                let payload = strip_server_managed_fields(kind, &item);
                let identity = build_resource_identity(kind, &payload);
                (identity, payload)
            })
            .collect::<Vec<(String, Map<String, Value>)>>();
        live_items.sort_by(|left, right| left.0.cmp(&right.0));
        for (identity, payload) in live_items {
            if desired_keys.contains(&(kind.to_string(), identity.clone())) {
                continue;
            }
            let action = if allow_prune { "delete" } else { "blocked" };
            let reason = if allow_prune {
                "missing-from-desired-state"
            } else {
                "prune-required"
            };
            let blocked_reason = if allow_prune {
                None
            } else {
                Some("prune-required")
            };
            rows.push(build_plan_row(PlanRowInput {
                kind,
                identity: &identity,
                path: None,
                action,
                reason,
                blocked_reason,
                desired: None,
                live: Some(&payload),
                review_hints: Vec::new(),
            }));
        }
    }

    Ok(build_alert_plan_document(&rows, allow_prune))
}

pub fn build_alert_delete_preview_from_files(
    resource_files: &[PathBuf],
    allow_policy_reset: bool,
) -> Result<Value> {
    let mut rows = Vec::new();
    for path in resource_files {
        let document = load_alert_resource_file(path, "Alerting delete target")?;
        let (kind, payload) = build_import_operation(&document)?;
        let identity = build_resource_identity(&kind, &payload);
        let blocked = kind == POLICIES_KIND && !allow_policy_reset;
        rows.push(json!({
            "path": path_string(path),
            "kind": kind,
            "identity": identity,
            "action": if blocked { "blocked" } else { "delete" },
            "reason": if blocked {
                "policy-reset-requires-allow-policy-reset"
            } else {
                "explicit-delete-request"
            },
            "desired": Value::Object(payload),
        }));
    }
    Ok(build_alert_delete_preview_document(
        &rows,
        allow_policy_reset,
    ))
}

pub fn build_alert_delete_preview_from_dir(
    desired_dir: &Path,
    allow_policy_reset: bool,
) -> Result<Value> {
    build_alert_delete_preview_from_files(
        &discover_alert_resource_files(desired_dir)?,
        allow_policy_reset,
    )
}

fn payload_object_from_row<'a>(
    row: &'a Map<String, Value>,
    field: &str,
) -> Result<&'a Map<String, Value>> {
    row.get(field)
        .ok_or_else(|| message(format!("Alert plan row is missing {field}.")))
        .and_then(|value| value_as_object(value, &format!("Alert plan row field {field}")))
}

pub fn execute_alert_plan_with_request<F>(
    mut request_json: F,
    plan_document: &Value,
    allow_policy_reset: bool,
) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let plan = value_as_object(plan_document, "Alert plan document")?;
    if plan.get("kind").and_then(Value::as_str) != Some(ALERT_PLAN_KIND) {
        return Err(message("Alert plan document kind is not supported."));
    }
    let rows = plan
        .get("rows")
        .and_then(Value::as_array)
        .ok_or_else(|| message("Alert plan document is missing rows."))?;

    let mut results = Vec::new();
    let mut applied_count = 0usize;
    for row in rows {
        let row = row_object(row, "Alert plan row")?;
        let action = row.get("action").and_then(Value::as_str).unwrap_or("");
        if !matches!(action, "create" | "update" | "delete") {
            continue;
        }
        let kind = row
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(|| message("Alert plan row is missing kind."))?;
        let identity = row
            .get("identity")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let response = match action {
            "create" => {
                let desired = payload_object_from_row(row, "desired")?;
                apply_create_with_request(&mut request_json, kind, desired)?
            }
            "update" => {
                let desired = payload_object_from_row(row, "desired")?;
                apply_update_with_request(&mut request_json, kind, identity, desired)?
            }
            "delete" => {
                apply_delete_with_request(&mut request_json, kind, identity, allow_policy_reset)?
            }
            _ => unreachable!(),
        };
        applied_count += 1;
        results.push(json!({
            "kind": kind,
            "identity": identity,
            "action": action,
            "response": response,
        }));
    }

    Ok(json!({
        "kind": "grafana-util-alert-apply-result",
        "mode": "apply",
        "allowPolicyReset": allow_policy_reset,
        "appliedCount": applied_count,
        "results": results,
    }))
}

pub fn init_alert_runtime_layout(root: &Path) -> Result<Value> {
    let created = init_alert_managed_dir(root)?
        .into_iter()
        .map(|path| Value::String(path_string(&path)))
        .collect::<Vec<Value>>();
    Ok(json!({
        "kind": "grafana-util-alert-init",
        "root": path_string(root),
        "created": created,
    }))
}

pub fn write_new_rule_scaffold(path: &Path, name: &str, overwrite: bool) -> Result<Value> {
    let document = build_new_rule_scaffold_document(name);
    write_alert_resource_file(path, &document, overwrite)?;
    Ok(document)
}

pub fn write_new_contact_point_scaffold(path: &Path, name: &str, overwrite: bool) -> Result<Value> {
    let document = build_new_contact_point_scaffold_document(name);
    write_alert_resource_file(path, &document, overwrite)?;
    Ok(document)
}

#[allow(dead_code)]
pub fn write_contact_point_scaffold(
    path: &Path,
    name: &str,
    channel_type: &str,
    overwrite: bool,
) -> Result<Value> {
    let document = build_contact_point_scaffold_document(name, channel_type);
    write_alert_resource_file(path, &document, overwrite)?;
    Ok(document)
}

pub fn write_new_template_scaffold(path: &Path, name: &str, overwrite: bool) -> Result<Value> {
    let document = build_new_template_scaffold_document(name);
    write_alert_resource_file(path, &document, overwrite)?;
    Ok(document)
}

#[allow(dead_code)]
pub fn build_managed_policy_edit_preview_document(
    current_policy_document: &Value,
    route_name: &str,
    desired_route_document: Option<&Value>,
) -> Result<Value> {
    let current_policy = value_as_object(current_policy_document, "Current notification policies")?;
    let desired_route = match desired_route_document {
        Some(value) => Some(value_as_object(value, "Desired managed route")?),
        None => None,
    };
    Ok(json!({
        "kind": "grafana-util-alert-managed-policy-preview",
        "schemaVersion": ALERT_MANAGED_POLICY_PREVIEW_SCHEMA_VERSION,
        "toolVersion": tool_version(),
        "reviewRequired": true,
        "reviewed": false,
        "routeName": route_name,
        "preview": build_managed_policy_route_preview(current_policy, route_name, desired_route)?,
    }))
}

#[allow(dead_code)]
pub fn apply_managed_policy_subtree_edit_document(
    current_policy_document: &Value,
    route_name: &str,
    desired_route_document: Option<&Value>,
) -> Result<Value> {
    let current_policy = value_as_object(current_policy_document, "Current notification policies")?;
    let (next_policy, action) = match desired_route_document {
        Some(value) => upsert_managed_policy_subtree(
            current_policy,
            route_name,
            value_as_object(value, "Desired managed route")?,
        )?,
        None => remove_managed_policy_subtree(current_policy, route_name)?,
    };
    Ok(json!({
        "kind": POLICIES_KIND,
        "action": action,
        "spec": Value::Object(next_policy),
    }))
}

pub fn build_alert_import_dry_run_document(rows: &[Value]) -> Value {
    let processed = rows.len();
    let would_create = rows
        .iter()
        .filter(|row| row.get("action").and_then(Value::as_str) == Some("would-create"))
        .count();
    let would_update = rows
        .iter()
        .filter(|row| row.get("action").and_then(Value::as_str) == Some("would-update"))
        .count();
    let would_fail_existing = rows
        .iter()
        .filter(|row| row.get("action").and_then(Value::as_str) == Some("would-fail-existing"))
        .count();

    json!({
        "kind": ALERT_IMPORT_DRY_RUN_KIND,
        "schemaVersion": ALERT_IMPORT_DRY_RUN_SCHEMA_VERSION,
        "toolVersion": tool_version(),
        "reviewRequired": true,
        "reviewed": false,
        "summary": {
            "processed": processed,
            "wouldCreate": would_create,
            "wouldUpdate": would_update,
            "wouldFailExisting": would_fail_existing,
        },
        "rows": rows,
    })
}

pub fn build_alert_diff_document(rows: &[Value]) -> Value {
    let checked = rows.len();
    let same = rows
        .iter()
        .filter(|row| {
            row.get("status")
                .and_then(Value::as_str)
                .or_else(|| row.get("action").and_then(Value::as_str))
                == Some("same")
        })
        .count();
    let different = rows
        .iter()
        .filter(|row| {
            row.get("status")
                .and_then(Value::as_str)
                .or_else(|| row.get("action").and_then(Value::as_str))
                == Some("different")
        })
        .count();
    let missing_remote = rows
        .iter()
        .filter(|row| {
            row.get("status")
                .and_then(Value::as_str)
                .or_else(|| row.get("action").and_then(Value::as_str))
                == Some("missing-remote")
        })
        .count();

    let mut document = match build_shared_diff_document(
        "grafana-util-alert-diff",
        1,
        SharedDiffSummary {
            checked,
            same,
            different,
            missing_remote,
            extra_remote: 0,
            ambiguous: 0,
        },
        rows,
    ) {
        Value::Object(map) => map,
        _ => unreachable!("shared diff document must be an object"),
    };
    document.insert("reviewRequired".to_string(), Value::Bool(true));
    document.insert("reviewed".to_string(), Value::Bool(false));
    Value::Object(document)
}
