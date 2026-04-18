//! Internal workspace review view model for preview/review helpers.
//!
//! This adapter normalizes the staged sync preview into a reusable
//! action/domain view without changing the public JSON contract. Callers can
//! reuse the typed fields for future TUI surfaces while still emitting the
//! legacy plan payload shape.

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkspaceReviewAction {
    pub action_id: String,
    pub action: String,
    pub domain: String,
    pub resource_kind: String,
    pub identity: String,
    pub status: String,
    pub order_group: String,
    pub kind_order: usize,
    pub blocked_reason: Option<String>,
    pub details: Option<String>,
    pub review_hints: Vec<String>,
    pub raw: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkspaceReviewDomain {
    pub id: String,
    pub checked: usize,
    pub same: usize,
    pub create: usize,
    pub update: usize,
    pub delete: usize,
    pub warning: usize,
    pub blocked: usize,
    pub action_count: usize,
    pub raw: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkspaceReviewSummary {
    pub action_count: usize,
    pub domain_count: usize,
    pub same_count: usize,
    pub blocked_count: usize,
    pub warning_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkspaceReviewView {
    pub actions: Vec<WorkspaceReviewAction>,
    pub domains: Vec<WorkspaceReviewDomain>,
    pub blocked_reasons: Vec<String>,
    pub summary: WorkspaceReviewSummary,
}

fn normalize_action(action: &Value) -> Result<WorkspaceReviewAction> {
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
    normalized.insert("status".to_string(), Value::String(status.clone()));
    let action_id = derive_action_id(&normalized, &domain, &resource_kind, &identity);
    if !normalized.contains_key("actionId") {
        normalized.insert("actionId".to_string(), Value::String(action_id.clone()));
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
    Ok(WorkspaceReviewAction {
        action_id,
        action: action_name,
        domain,
        resource_kind,
        identity,
        status,
        order_group: normalized
            .get("orderGroup")
            .and_then(Value::as_str)
            .unwrap_or("review")
            .to_string(),
        kind_order: normalized
            .get("kindOrder")
            .and_then(Value::as_i64)
            .unwrap_or(0) as usize,
        blocked_reason: normalized
            .get("blockedReason")
            .and_then(Value::as_str)
            .map(str::to_string),
        details: normalized
            .get("details")
            .and_then(Value::as_str)
            .map(str::to_string),
        review_hints: normalized
            .get("reviewHints")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        raw: Value::Object(normalized),
    })
}

fn collect_actions(document: &Map<String, Value>) -> Result<Vec<WorkspaceReviewAction>> {
    let source = document
        .get("actions")
        .or_else(|| document.get("operations"))
        .and_then(Value::as_array)
        .ok_or_else(|| message("Sync plan document is missing actions or operations."))?;
    let mut actions = source
        .iter()
        .map(normalize_action)
        .collect::<Result<Vec<WorkspaceReviewAction>>>()?;
    actions.sort_by(|left, right| {
        left.kind_order
            .cmp(&right.kind_order)
            .then_with(|| action_rank(&left.action).cmp(&action_rank(&right.action)))
            .then_with(|| left.domain.cmp(&right.domain))
            .then_with(|| left.identity.cmp(&right.identity))
            .then_with(|| left.action_id.cmp(&right.action_id))
    });
    Ok(actions)
}

fn collect_blocked_reasons(actions: &[WorkspaceReviewAction]) -> Vec<String> {
    let mut reasons = BTreeSet::new();
    for action in actions {
        if action.status != "blocked"
            && !action.action.starts_with("blocked-")
            && action.action != "unmanaged"
        {
            continue;
        }
        if let Some(reason) = action
            .blocked_reason
            .as_deref()
            .or_else(|| action.raw.get("reason").and_then(Value::as_str))
        {
            let reason = reason.trim();
            if !reason.is_empty() {
                reasons.insert(reason.to_string());
            }
        }
    }
    reasons.into_iter().take(5).collect()
}

fn domain_summary(actions: &[WorkspaceReviewAction]) -> Vec<WorkspaceReviewDomain> {
    let mut grouped: BTreeMap<String, Vec<&WorkspaceReviewAction>> = BTreeMap::new();
    for action in actions {
        grouped
            .entry(action.domain.clone())
            .or_default()
            .push(action);
    }
    let mut domains = grouped
        .into_iter()
        .map(|(domain, items)| {
            let checked = items.len();
            let same = items.iter().filter(|item| item.action == "same").count();
            let create = items
                .iter()
                .filter(|item| item.action == "would-create")
                .count();
            let update = items
                .iter()
                .filter(|item| item.action == "would-update")
                .count();
            let delete = items
                .iter()
                .filter(|item| item.action == "would-delete")
                .count();
            let warning = items.iter().filter(|item| item.status == "warning").count();
            let blocked = items.iter().filter(|item| item.status == "blocked").count();
            let raw = Value::Object(Map::from_iter(vec![
                ("id".to_string(), Value::String(domain.clone())),
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
            ]));
            WorkspaceReviewDomain {
                id: domain,
                checked,
                same,
                create,
                update,
                delete,
                warning,
                blocked,
                action_count: checked,
                raw,
            }
        })
        .collect::<Vec<_>>();
    for domain in ["dashboard", "datasource", "alert", "access"] {
        if !domains.iter().any(|value| value.id == domain) {
            domains.push(WorkspaceReviewDomain {
                id: domain.to_string(),
                checked: 0,
                same: 0,
                create: 0,
                update: 0,
                delete: 0,
                warning: 0,
                blocked: 0,
                action_count: 0,
                raw: Value::Object(Map::from_iter(vec![
                    ("id".to_string(), Value::String(domain.to_string())),
                    ("checked".to_string(), Value::Number(0.into())),
                    ("same".to_string(), Value::Number(0.into())),
                    ("create".to_string(), Value::Number(0.into())),
                    ("update".to_string(), Value::Number(0.into())),
                    ("delete".to_string(), Value::Number(0.into())),
                    ("warning".to_string(), Value::Number(0.into())),
                    ("blocked".to_string(), Value::Number(0.into())),
                    ("actionCount".to_string(), Value::Number(0.into())),
                ])),
            });
        }
    }
    domains.sort_by(|left, right| {
        let left_id = left.id.as_str();
        let right_id = right.id.as_str();
        create_update_domain_rank(left_id).cmp(&create_update_domain_rank(right_id))
    });
    domains
}

pub(crate) fn build_workspace_review_view(document: &Value) -> Result<WorkspaceReviewView> {
    let object = document
        .as_object()
        .ok_or_else(|| message("Sync plan document is not a JSON object."))?;
    if object.get("kind").and_then(Value::as_str) != Some("grafana-utils-sync-plan") {
        return Err(message("Sync plan document kind is not supported."));
    }
    let actions = collect_actions(object)?;
    let domains = domain_summary(&actions);
    let blocked_reasons = collect_blocked_reasons(&actions);
    let summary = WorkspaceReviewSummary {
        action_count: actions.len(),
        domain_count: domains.len(),
        same_count: actions
            .iter()
            .filter(|action| action.action == "same")
            .count(),
        blocked_count: actions
            .iter()
            .filter(|action| action.status == "blocked")
            .count(),
        warning_count: actions
            .iter()
            .filter(|action| action.status == "warning")
            .count(),
    };
    Ok(WorkspaceReviewView {
        actions,
        domains,
        blocked_reasons,
        summary,
    })
}
