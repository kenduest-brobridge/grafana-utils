//! Workspace preview contract adapter.
//!
//! This module turns the staged sync preview into a richer review document
//! without reimplementing per-domain logic. It keeps the legacy `operations`
//! array for apply/review compatibility and adds a newer `actions` contract
//! plus aggregated domain/blocker metadata for future TUI consumers.

use std::collections::{BTreeMap, BTreeSet};

use crate::common::{message, Result};
use serde_json::{Map, Value};

fn action_rank(action: &str) -> usize {
    match action {
        "would-create" => 0,
        "would-update" => 1,
        "would-delete" => 2,
        "same" => 3,
        "extra-remote" => 4,
        "unmanaged" => 5,
        _ => 6,
    }
}

fn create_update_domain_rank(domain: &str) -> usize {
    match domain {
        "folder" => 0,
        "datasource" => 1,
        "dashboard" => 2,
        "alert" => 3,
        "access" => 4,
        _ => 5,
    }
}

fn delete_domain_rank(domain: &str) -> usize {
    match domain {
        "alert" => 0,
        "dashboard" => 1,
        "datasource" => 2,
        "folder" | "access" => 3,
        _ => 4,
    }
}

fn operation_kind_rank(domain: &str, action: &str) -> usize {
    if action == "would-delete" {
        delete_domain_rank(domain)
    } else {
        create_update_domain_rank(domain)
    }
}

fn action_group(action: &str) -> &'static str {
    match action {
        "would-delete" => "delete",
        "would-create" | "would-update" => "create-update",
        "same" => "read-only",
        "extra-remote" => "warning",
        "unmanaged" => "blocked",
        _ => "review",
    }
}

fn derive_domain(resource_kind: &str) -> String {
    match resource_kind {
        "dashboard" | "datasource" | "folder" => resource_kind.to_string(),
        "alert"
        | "alert-policy"
        | "alert-contact-point"
        | "alert-mute-timing"
        | "alert-template" => "alert".to_string(),
        "user" | "team" | "org" | "service-account" => "access".to_string(),
        other if other.contains("access") => "access".to_string(),
        _ => "workspace".to_string(),
    }
}

fn derive_resource_kind(action: &Map<String, Value>) -> String {
    action
        .get("resourceKind")
        .and_then(Value::as_str)
        .or_else(|| action.get("kind").and_then(Value::as_str))
        .unwrap_or("workspace")
        .to_string()
}

fn derive_identity(action: &Map<String, Value>) -> String {
    action
        .get("identity")
        .and_then(Value::as_str)
        .or_else(|| action.get("uid").and_then(Value::as_str))
        .or_else(|| action.get("name").and_then(Value::as_str))
        .or_else(|| action.get("sourcePath").and_then(Value::as_str))
        .unwrap_or("unknown")
        .to_string()
}

fn derive_action_id(
    action: &Map<String, Value>,
    domain: &str,
    resource_kind: &str,
    identity: &str,
) -> String {
    action
        .get("actionId")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| {
            let kind = if resource_kind.is_empty() {
                "unknown"
            } else {
                resource_kind
            };
            let identity_kind = if action
                .get("uid")
                .and_then(Value::as_str)
                .map(|text| !text.trim().is_empty())
                .unwrap_or(false)
            {
                "uid"
            } else {
                "identity"
            };
            format!("{domain}:{kind}:{identity_kind}:{identity}")
        })
}

fn derive_status(action: &str, existing: Option<&str>) -> String {
    existing
        .map(str::to_string)
        .unwrap_or_else(|| match action {
            "same" => "same".to_string(),
            "would-create" | "would-update" | "would-delete" => "ready".to_string(),
            "extra-remote" => "warning".to_string(),
            "blocked-read-only"
            | "blocked-ambiguous"
            | "blocked-uid-mismatch"
            | "blocked-missing-org" => "blocked".to_string(),
            "unmanaged" => "blocked".to_string(),
            _ => "warning".to_string(),
        })
}

fn normalize_action(action: &Value) -> Result<Value> {
    let Some(object) = action.as_object() else {
        return Err(message("Workspace preview action is not a JSON object."));
    };
    let mut normalized = object.clone();
    let resource_kind = derive_resource_kind(&normalized);
    let domain = normalized
        .get("domain")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| derive_domain(&resource_kind));
    let identity = derive_identity(&normalized);
    let action_name = normalized
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    if !normalized.contains_key("resourceKind") {
        normalized.insert(
            "resourceKind".to_string(),
            Value::String(resource_kind.clone()),
        );
    }
    if !normalized.contains_key("domain") {
        normalized.insert("domain".to_string(), Value::String(domain.clone()));
    }
    if !normalized.contains_key("kind") {
        normalized.insert("kind".to_string(), Value::String(resource_kind.clone()));
    }
    if !normalized.contains_key("identity") {
        normalized.insert("identity".to_string(), Value::String(identity.clone()));
    }
    let status = derive_status(
        &action_name,
        normalized.get("status").and_then(Value::as_str),
    );
    normalized.insert("status".to_string(), Value::String(status));
    let action_id = derive_action_id(&normalized, &domain, &resource_kind, &identity);
    if !normalized.contains_key("actionId") {
        normalized.insert("actionId".to_string(), Value::String(action_id));
    }
    if !normalized.contains_key("orderGroup") {
        normalized.insert(
            "orderGroup".to_string(),
            Value::String(action_group(&action_name).to_string()),
        );
    }
    if !normalized.contains_key("kindOrder") {
        normalized.insert(
            "kindOrder".to_string(),
            Value::Number(operation_kind_rank(&domain, &action_name).into()),
        );
    }
    if !normalized.contains_key("reviewHints") {
        normalized.insert("reviewHints".to_string(), Value::Array(Vec::new()));
    }
    if !normalized.contains_key("blockedReason") {
        if let Some(reason) = normalized.get("reason").and_then(Value::as_str) {
            normalized.insert(
                "blockedReason".to_string(),
                Value::String(reason.to_string()),
            );
        }
    }
    if !normalized.contains_key("details") {
        if let Some(detail) = normalized.get("detail").and_then(Value::as_str) {
            normalized.insert("details".to_string(), Value::String(detail.to_string()));
        }
    }
    Ok(Value::Object(normalized))
}

fn collect_actions(document: &Map<String, Value>) -> Result<Vec<Value>> {
    let source = document
        .get("actions")
        .or_else(|| document.get("operations"))
        .and_then(Value::as_array)
        .ok_or_else(|| message("Sync plan document is missing actions or operations."))?;
    let mut actions = source
        .iter()
        .map(normalize_action)
        .collect::<Result<Vec<Value>>>()?;
    actions.sort_by(|left, right| {
        let left_object = left.as_object().expect("normalized action");
        let right_object = right.as_object().expect("normalized action");
        let left_action = left_object
            .get("action")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let right_action = right_object
            .get("action")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let left_domain = left_object
            .get("domain")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let right_domain = right_object
            .get("domain")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let left_kind_order = left_object
            .get("kindOrder")
            .and_then(Value::as_i64)
            .unwrap_or_else(|| operation_kind_rank(left_domain, left_action) as i64);
        let right_kind_order = right_object
            .get("kindOrder")
            .and_then(Value::as_i64)
            .unwrap_or_else(|| operation_kind_rank(right_domain, right_action) as i64);
        let left_identity = left_object
            .get("identity")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let right_identity = right_object
            .get("identity")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let left_order = left_object
            .get("orderIndex")
            .and_then(Value::as_i64)
            .unwrap_or(i64::MAX);
        let right_order = right_object
            .get("orderIndex")
            .and_then(Value::as_i64)
            .unwrap_or(i64::MAX);
        left_order
            .cmp(&right_order)
            .then_with(|| action_rank(left_action).cmp(&action_rank(right_action)))
            .then_with(|| left_kind_order.cmp(&right_kind_order))
            .then_with(|| left_domain.cmp(right_domain))
            .then_with(|| left_identity.cmp(right_identity))
    });
    Ok(actions)
}

fn collect_blocked_reasons(actions: &[Value]) -> Vec<String> {
    let mut reasons = BTreeSet::new();
    for action in actions {
        let Some(object) = action.as_object() else {
            continue;
        };
        let action_name = object
            .get("action")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let status = object
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if status != "blocked" && !action_name.starts_with("blocked-") && action_name != "unmanaged"
        {
            continue;
        }
        if let Some(reason) = object
            .get("blockedReason")
            .and_then(Value::as_str)
            .or_else(|| object.get("reason").and_then(Value::as_str))
        {
            let reason = reason.trim();
            if !reason.is_empty() {
                reasons.insert(reason.to_string());
            }
        }
    }
    reasons.into_iter().take(5).collect()
}

fn count_actions(actions: &[Value], predicate: impl Fn(&str, &str) -> bool) -> usize {
    actions
        .iter()
        .filter_map(Value::as_object)
        .filter(|object| {
            let action = object
                .get("action")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let status = object
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or_default();
            predicate(action, status)
        })
        .count()
}

fn domain_summary(actions: &[Value]) -> Vec<Value> {
    let mut grouped: BTreeMap<String, Vec<&Map<String, Value>>> = BTreeMap::new();
    for action in actions.iter().filter_map(Value::as_object) {
        let domain = action
            .get("domain")
            .and_then(Value::as_str)
            .unwrap_or("workspace")
            .to_string();
        grouped.entry(domain).or_default().push(action);
    }
    let mut domains = grouped
        .into_iter()
        .map(|(domain, items)| {
            let checked = items.len();
            let same = items
                .iter()
                .filter(|item| item.get("action").and_then(Value::as_str) == Some("same"))
                .count();
            let create = items
                .iter()
                .filter(|item| item.get("action").and_then(Value::as_str) == Some("would-create"))
                .count();
            let update = items
                .iter()
                .filter(|item| item.get("action").and_then(Value::as_str) == Some("would-update"))
                .count();
            let delete = items
                .iter()
                .filter(|item| item.get("action").and_then(Value::as_str) == Some("would-delete"))
                .count();
            let warning = items
                .iter()
                .filter(|item| {
                    matches!(item.get("status").and_then(Value::as_str), Some("warning"))
                })
                .count();
            let blocked = items
                .iter()
                .filter(|item| {
                    matches!(item.get("status").and_then(Value::as_str), Some("blocked"))
                })
                .count();
            Value::Object(Map::from_iter(vec![
                ("id".to_string(), Value::String(domain)),
                (
                    "checked".to_string(),
                    Value::Number((checked as i64).into()),
                ),
                ("same".to_string(), Value::Number((same as i64).into())),
                ("create".to_string(), Value::Number((create as i64).into())),
                ("update".to_string(), Value::Number((update as i64).into())),
                ("delete".to_string(), Value::Number((delete as i64).into())),
                (
                    "warning".to_string(),
                    Value::Number((warning as i64).into()),
                ),
                (
                    "blocked".to_string(),
                    Value::Number((blocked as i64).into()),
                ),
                (
                    "actionCount".to_string(),
                    Value::Number((checked as i64).into()),
                ),
            ]))
        })
        .collect::<Vec<_>>();
    for domain in ["dashboard", "datasource", "alert", "access"] {
        if !domains
            .iter()
            .any(|value| value.get("id").and_then(Value::as_str) == Some(domain))
        {
            domains.push(Value::Object(Map::from_iter(vec![
                ("id".to_string(), Value::String(domain.to_string())),
                ("checked".to_string(), Value::Number(0.into())),
                ("same".to_string(), Value::Number(0.into())),
                ("create".to_string(), Value::Number(0.into())),
                ("update".to_string(), Value::Number(0.into())),
                ("delete".to_string(), Value::Number(0.into())),
                ("warning".to_string(), Value::Number(0.into())),
                ("blocked".to_string(), Value::Number(0.into())),
                ("actionCount".to_string(), Value::Number(0.into())),
            ])));
        }
    }
    domains.sort_by(|left, right| {
        let left_id = left.get("id").and_then(Value::as_str).unwrap_or_default();
        let right_id = right.get("id").and_then(Value::as_str).unwrap_or_default();
        create_update_domain_rank(left_id).cmp(&create_update_domain_rank(right_id))
    });
    domains
}

fn enrich_summary(summary: &mut Map<String, Value>, actions: &[Value], domains: &[Value]) {
    let same = count_actions(actions, |action, _| action == "same");
    let blocked = count_actions(actions, |_, status| status == "blocked");
    let warning = count_actions(actions, |_, status| status == "warning");
    summary
        .entry("actionCount".to_string())
        .or_insert(Value::Number((actions.len() as i64).into()));
    summary
        .entry("domainCount".to_string())
        .or_insert(Value::Number((domains.len() as i64).into()));
    summary
        .entry("sameCount".to_string())
        .or_insert(Value::Number((same as i64).into()));
    summary
        .entry("blockedCount".to_string())
        .or_insert(Value::Number((blocked as i64).into()));
    summary
        .entry("warningCount".to_string())
        .or_insert(Value::Number((warning as i64).into()));
    if !summary.contains_key("blocked_reasons") {
        summary.insert(
            "blocked_reasons".to_string(),
            Value::Array(
                collect_blocked_reasons(actions)
                    .into_iter()
                    .map(Value::String)
                    .collect(),
            ),
        );
    }
}

pub(crate) fn enrich_workspace_preview_document(document: &Value) -> Result<Value> {
    let object = document
        .as_object()
        .ok_or_else(|| message("Sync plan document is not a JSON object."))?;
    if object.get("kind").and_then(Value::as_str) != Some("grafana-utils-sync-plan") {
        return Err(message("Sync plan document kind is not supported."));
    }
    let mut enriched = object.clone();
    let actions = collect_actions(&enriched)?;
    let domains = domain_summary(&actions);
    if !enriched.contains_key("actions") {
        enriched.insert("actions".to_string(), Value::Array(actions.clone()));
    }
    if !enriched.contains_key("operations") {
        enriched.insert("operations".to_string(), Value::Array(actions.clone()));
    }
    enriched.insert("domains".to_string(), Value::Array(domains.clone()));
    enriched.insert(
        "blockedReasons".to_string(),
        Value::Array(
            collect_blocked_reasons(&actions)
                .into_iter()
                .map(Value::String)
                .collect(),
        ),
    );
    if let Some(summary) = enriched.get_mut("summary").and_then(Value::as_object_mut) {
        enrich_summary(summary, &actions, &domains);
    }
    Ok(Value::Object(enriched))
}
