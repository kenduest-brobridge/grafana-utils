//! Staged alert sync helpers.
//!
//! Purpose:
//! - Stage alert-specific sync ownership and mutation policy before live wiring.
//! - Keep partial alert ownership explicit and reviewable.

use crate::common::{message, Result};
use serde::Serialize;
use serde_json::{json, Map, Value};

/// Constant for alert sync kind.
pub const ALERT_SYNC_KIND: &str = "grafana-utils-alert-sync-plan";
/// Constant for alert sync schema version.
pub const ALERT_SYNC_SCHEMA_VERSION: i64 = 1;
/// Constant for alert allowed managed fields.
pub const ALERT_ALLOWED_MANAGED_FIELDS: &[&str] = &[
    "condition",
    "labels",
    "annotations",
    "contactPoints",
    "for",
    "noDataState",
    "execErrState",
];

/// Struct definition for AlertSyncAssessment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AlertSyncAssessment {
    pub identity: String,
    pub title: String,
    pub managed_fields: Vec<String>,
    pub status: String,
    pub live_apply_allowed: bool,
    pub detail: String,
}

fn normalize_text(value: Option<&Value>, default: &str) -> String {
    match value {
        Some(Value::String(text)) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                default.to_string()
            } else {
                trimmed.to_string()
            }
        }
        _ => default.to_string(),
    }
}

fn require_object<'a>(value: Option<&'a Value>, label: &str) -> Result<&'a Map<String, Value>> {
    match value {
        Some(Value::Object(object)) => Ok(object),
        Some(_) => Err(message(format!("{label} must be a JSON object."))),
        None => Err(message(format!("{label} must be a JSON object."))),
    }
}

fn normalize_managed_fields(value: Option<&Value>) -> Result<Vec<String>> {
    let items = value
        .and_then(Value::as_array)
        .ok_or_else(|| message("Alert managedFields must be a list."))?;
    let mut managed_fields = Vec::with_capacity(items.len());
    for item in items {
        let field = normalize_text(Some(item), "");
        if field.is_empty() {
            return Err(message("Alert managedFields cannot contain empty values."));
        }
        if !ALERT_ALLOWED_MANAGED_FIELDS.contains(&field.as_str()) {
            return Err(message(format!(
                "Unsupported alert managed field {:?}. Allowed values: {}.",
                field,
                ALERT_ALLOWED_MANAGED_FIELDS.join(", ")
            )));
        }
        managed_fields.push(field);
    }
    Ok(managed_fields)
}

/// assess alert sync specs.
pub fn assess_alert_sync_specs(alert_specs: &[Value]) -> Result<Value> {
    // Call graph (hierarchy): this function is used in related modules.
    // Upstream callers: bundle_preflight.rs:build_bundle_preflight_document, bundle_preflight_rust_tests.rs:assess_alert_sync_specs_reports_plan_only_and_blocked_states, sync.rs:run_sync_cli
    // Downstream callees: alert_sync.rs:normalize_managed_fields, alert_sync.rs:normalize_text, alert_sync.rs:require_object, common.rs:message

    let mut assessments = Vec::new();
    for raw_spec in alert_specs {
        let spec = require_object(Some(raw_spec), "Alert sync spec")?;
        let kind = normalize_text(spec.get("kind"), "").to_lowercase();
        if kind != "alert" {
            return Err(message("Alert sync assessment only supports kind=alert."));
        }
        let identity = ["uid", "name", "title"]
            .iter()
            .find_map(|field| {
                let value = normalize_text(spec.get(*field), "");
                if value.is_empty() {
                    None
                } else {
                    Some(value)
                }
            })
            .ok_or_else(|| message("Alert sync spec requires uid, name, or title."))?;
        let title = {
            let title = normalize_text(spec.get("title"), "");
            if title.is_empty() {
                let name = normalize_text(spec.get("name"), "");
                if name.is_empty() {
                    identity.clone()
                } else {
                    name
                }
            } else {
                title
            }
        };
        let managed_fields = normalize_managed_fields(spec.get("managedFields"))?;
        let body = if let Some(body) = spec.get("body") {
            require_object(Some(body), "Alert body")?
        } else if let Some(body) = spec.get("spec") {
            require_object(Some(body), "Alert body")?
        } else {
            return Err(message("Alert body must be a JSON object."));
        };

        let assessment = if !managed_fields.iter().any(|field| field == "condition") {
            AlertSyncAssessment {
                identity,
                title,
                managed_fields,
                status: "blocked".to_string(),
                live_apply_allowed: false,
                detail: "Alert sync must manage condition explicitly before live apply can be considered.".to_string(),
            }
        } else if managed_fields
            .iter()
            .any(|field| field == "contactPoints" || field == "annotations")
        {
            AlertSyncAssessment {
                identity,
                title,
                managed_fields,
                status: "plan-only".to_string(),
                live_apply_allowed: false,
                detail: "Alert sync includes linked routing or annotation fields and stays plan-only until mutation semantics settle.".to_string(),
            }
        } else if normalize_text(body.get("condition"), "").is_empty() {
            AlertSyncAssessment {
                identity,
                title,
                managed_fields,
                status: "blocked".to_string(),
                live_apply_allowed: false,
                detail: "Alert sync body must include a non-empty condition.".to_string(),
            }
        } else {
            AlertSyncAssessment {
                identity,
                title,
                managed_fields,
                status: "candidate".to_string(),
                live_apply_allowed: true,
                detail: "Alert sync scope is narrow enough for future controlled live-apply experiments.".to_string(),
            }
        };
        assessments.push(assessment);
    }

    Ok(json!({
        "kind": ALERT_SYNC_KIND,
        "schemaVersion": ALERT_SYNC_SCHEMA_VERSION,
        "summary": {
            "alertCount": assessments.len(),
            "candidateCount": assessments.iter().filter(|item| item.status == "candidate").count(),
            "planOnlyCount": assessments.iter().filter(|item| item.status == "plan-only").count(),
            "blockedCount": assessments.iter().filter(|item| item.status == "blocked").count(),
        },
        "alerts": assessments.iter().map(|item| {
            json!({
                "identity": item.identity,
                "title": item.title,
                "managedFields": item.managed_fields,
                "status": item.status,
                "liveApplyAllowed": item.live_apply_allowed,
                "detail": item.detail,
            })
        }).collect::<Vec<_>>(),
    }))
}
