//! Sync plan construction and summary shaping.
//!
//! This module owns normalized plan diffs, managed-fields comparison rules,
//! and the stable summary buckets consumed by the sync CLI renderers.

use super::summary_builder::{is_alert_sync_kind, normalize_resource_specs};
use super::workbench::{SyncResourceSpec, SYNC_PLAN_KIND, SYNC_PLAN_SCHEMA_VERSION};
use crate::common::{message, tool_version, Result};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

fn create_update_kind_rank(kind: &str) -> usize {
    match kind {
        "folder" => 0,
        "datasource" => 1,
        "dashboard" => 2,
        kind if is_alert_sync_kind(kind) => 3,
        _ => 4,
    }
}

fn delete_kind_rank(kind: &str) -> usize {
    match kind {
        kind if is_alert_sync_kind(kind) => 0,
        "dashboard" => 1,
        "datasource" => 2,
        "folder" => 3,
        _ => 4,
    }
}

fn action_rank(action: &str) -> usize {
    match action {
        "would-create" => 0,
        "would-update" => 1,
        "would-delete" => 2,
        "unmanaged" => 3,
        "noop" => 4,
        _ => 5,
    }
}

fn operation_kind_rank(kind: &str, action: &str) -> usize {
    if action == "would-delete" {
        delete_kind_rank(kind)
    } else {
        create_update_kind_rank(kind)
    }
}

fn operation_group_label(action: &str) -> &'static str {
    match action {
        "would-delete" => "delete",
        "would-create" | "would-update" => "create-update",
        "unmanaged" => "blocked",
        _ => "read-only",
    }
}

fn annotate_and_sort_operations(operations: &mut [Value]) {
    operations.sort_by(|left, right| {
        let left_action = left
            .get("action")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let right_action = right
            .get("action")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let left_kind = left.get("kind").and_then(Value::as_str).unwrap_or_default();
        let right_kind = right
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let left_identity = left
            .get("identity")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let right_identity = right
            .get("identity")
            .and_then(Value::as_str)
            .unwrap_or_default();
        action_rank(left_action)
            .cmp(&action_rank(right_action))
            .then_with(|| {
                operation_kind_rank(left_kind, left_action)
                    .cmp(&operation_kind_rank(right_kind, right_action))
            })
            .then_with(|| left_kind.cmp(right_kind))
            .then_with(|| left_identity.cmp(right_identity))
    });

    for (index, item) in operations.iter_mut().enumerate() {
        let Some(object) = item.as_object_mut() else {
            continue;
        };
        let action = object
            .get("action")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let kind = object
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        object.insert("orderIndex".to_string(), serde_json::json!(index + 1));
        object.insert(
            "orderGroup".to_string(),
            serde_json::json!(operation_group_label(&action)),
        );
        object.insert(
            "kindOrder".to_string(),
            serde_json::json!(operation_kind_rank(&kind, &action)),
        );
    }
}

fn collect_plan_blocked_reasons(operations: &[Value]) -> Vec<String> {
    let mut reasons = Vec::new();
    for item in operations {
        let Some(object) = item.as_object() else {
            continue;
        };
        let action = object
            .get("action")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if action != "unmanaged" {
            continue;
        }
        let kind = object
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let identity = object
            .get("identity")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let reason = object
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("blocked");
        reasons.push(format!("kind={kind} identity={identity} reason={reason}"));
        if reasons.len() >= 3 {
            break;
        }
    }
    reasons
}

fn build_index(specs: &[SyncResourceSpec]) -> Result<BTreeMap<(String, String), SyncResourceSpec>> {
    let mut index = BTreeMap::new();
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
    let mut fields = BTreeSet::new();
    for key in desired.body.keys() {
        fields.insert(key.clone());
    }
    // Only compare live keys that the desired spec claims to manage; this keeps
    // out-of-band fields from becoming false drift when ownership is partial.
    let managed_filter = if desired.managed_fields.is_empty() {
        None
    } else {
        Some(
            desired
                .managed_fields
                .iter()
                .cloned()
                .collect::<BTreeSet<String>>(),
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

fn supports_prune_delete(_kind: &str) -> bool {
    true
}

pub(crate) fn build_sync_alert_assessment_document(operations: &[Value]) -> Value {
    let mut alerts = Vec::new();
    let mut candidate_count = 0i64;
    let mut plan_only_count = 0i64;
    let mut blocked_count = 0i64;
    for item in operations {
        let Some(object) = item.as_object() else {
            continue;
        };
        let kind = object.get("kind").and_then(Value::as_str).unwrap_or("");
        if !is_alert_sync_kind(kind) {
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
        let (status, live_apply_allowed, detail) = if kind == "alert" {
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
            if !has_condition {
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
            }
        } else {
            (
                "candidate",
                true,
                "Alert provisioning resource is narrow enough for controlled live apply.",
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
        "toolVersion": tool_version(),
        "summary": {
            "alertCount": alerts.len(),
            "candidateCount": candidate_count,
            "planOnlyCount": plan_only_count,
            "blockedCount": blocked_count,
        },
        "alerts": alerts,
    })
}

pub(crate) fn build_sync_plan_summary_document(operations: &[Value]) -> Value {
    let mut would_create = 0usize;
    let mut would_update = 0usize;
    let mut would_delete = 0usize;
    let mut noop = 0usize;
    let mut unmanaged = 0usize;
    for item in operations {
        match item
            .get("action")
            .and_then(Value::as_str)
            .unwrap_or_default()
        {
            "would-create" => would_create += 1,
            "would-update" => would_update += 1,
            "would-delete" => would_delete += 1,
            "noop" => noop += 1,
            "unmanaged" => unmanaged += 1,
            _ => {}
        }
    }
    let alert_assessment = build_sync_alert_assessment_document(operations);
    serde_json::json!({
        "would_create": would_create,
        "would_update": would_update,
        "would_delete": would_delete,
        "noop": noop,
        "unmanaged": unmanaged,
        "blocked_reasons": collect_plan_blocked_reasons(operations),
        // Keep these aggregate names stable so older renderers can continue to
        // read the same backward-compatible summary buckets.
        "alert_candidate": alert_assessment["summary"]["candidateCount"],
        "alert_plan_only": alert_assessment["summary"]["planOnlyCount"],
        "alert_blocked": alert_assessment["summary"]["blockedCount"],
    })
}

pub fn build_sync_plan_document(
    desired_specs: &[Value],
    live_specs: &[Value],
    allow_prune: bool,
) -> Result<Value> {
    // The planner only emits normalized operations; stage transitions and
    // transport concerns stay in the CLI orchestration layer.
    let desired = normalize_resource_specs(desired_specs)?;
    let live = normalize_resource_specs(live_specs)?;
    let desired_index = build_index(&desired)?;
    let live_index = build_index(&live)?;
    let mut operations = Vec::new();

    for (key, desired_spec) in &desired_index {
        if let Some(live_spec) = live_index.get(key) {
            let changed_fields = compare_body(desired_spec, live_spec);
            let action = if changed_fields.is_empty() {
                "noop"
            } else {
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
        let action = if allow_prune && supports_prune_delete(&live_spec.kind) {
            "would-delete"
        } else {
            "unmanaged"
        };
        operations.push(serde_json::json!({
            "kind": live_spec.kind,
            "identity": live_spec.identity,
            "title": live_spec.title,
            "action": action,
            "reason": if allow_prune && supports_prune_delete(&live_spec.kind) {
                "missing-from-desired-state"
            } else if allow_prune {
                "delete-not-supported"
            } else {
                "prune-disabled"
            },
            "changedFields": Vec::<String>::new(),
            "managedFields": Vec::<String>::new(),
            "desired": Value::Null,
            "live": live_spec.body,
            "sourcePath": live_spec.source_path,
        }));
    }

    annotate_and_sort_operations(&mut operations);
    let alert_assessment = build_sync_alert_assessment_document(&operations);
    Ok(serde_json::json!({
        "kind": SYNC_PLAN_KIND,
        "schemaVersion": SYNC_PLAN_SCHEMA_VERSION,
        "toolVersion": tool_version(),
        "dryRun": true,
        "reviewRequired": true,
        "reviewed": false,
        "allowPrune": allow_prune,
        "ordering": {
            "mode": "dependency-aware",
            "createUpdateKindOrder": ["folder", "datasource", "dashboard", "alert"],
            "deleteKindOrder": ["alert", "dashboard", "datasource", "folder"],
        },
        "summary": build_sync_plan_summary_document(&operations),
        "alertAssessment": alert_assessment,
        "operations": operations,
    }))
}
