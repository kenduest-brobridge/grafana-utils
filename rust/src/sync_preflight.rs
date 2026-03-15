//! Staged sync preflight helpers.
//!
//! Purpose:
//! - Build one pure preflight document from desired sync resources and explicit
//!   availability hints.
//! - Keep staged dependency and alert live-apply policy checks isolated from
//!   any Rust CLI or Grafana transport wiring.

use crate::common::{message, Result};
use crate::sync_contracts::{normalize_resource_specs, SyncResourceSpec};
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::BTreeSet;

pub const SYNC_PREFLIGHT_KIND: &str = "grafana-utils-sync-preflight";
pub const SYNC_PREFLIGHT_SCHEMA_VERSION: i64 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SyncPreflightCheck {
    pub kind: String,
    pub identity: String,
    pub status: String,
    pub detail: String,
    pub blocking: bool,
}

fn normalize_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.trim().to_string(),
        Some(Value::Number(number)) => number.to_string(),
        _ => String::new(),
    }
}

fn require_object(value: Option<&Value>, label: &str) -> Result<Map<String, Value>> {
    match value {
        None => Ok(Map::new()),
        Some(Value::Object(object)) => Ok(object.clone()),
        Some(_) => Err(message(format!("{label} must be a JSON object."))),
    }
}

fn require_string_list(value: Option<&Value>, label: &str) -> Result<Vec<String>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let items = value
        .as_array()
        .ok_or_else(|| message(format!("{label} must be a list.")))?;
    let mut result = Vec::new();
    for item in items {
        let text = normalize_text(Some(item));
        if !text.is_empty() {
            result.push(text);
        }
    }
    Ok(result)
}

fn build_datasource_checks(
    spec: &SyncResourceSpec,
    availability: &Map<String, Value>,
) -> Result<Vec<SyncPreflightCheck>> {
    let available_uids = require_string_list(availability.get("datasourceUids"), "datasourceUids")?
        .into_iter()
        .collect::<BTreeSet<String>>();
    let plugin_ids = require_string_list(availability.get("pluginIds"), "pluginIds")?
        .into_iter()
        .collect::<BTreeSet<String>>();
    let datasource_type = normalize_text(spec.body.get("type"));
    let mut checks = vec![if available_uids.contains(&spec.identity) {
        SyncPreflightCheck {
            kind: "datasource".to_string(),
            identity: spec.identity.clone(),
            status: "ok".to_string(),
            detail: "Datasource already exists in the destination inventory.".to_string(),
            blocking: false,
        }
    } else {
        SyncPreflightCheck {
            kind: "datasource".to_string(),
            identity: spec.identity.clone(),
            status: "create-planned".to_string(),
            detail: "Datasource is absent and would be created by sync.".to_string(),
            blocking: false,
        }
    }];
    if datasource_type.is_empty() || plugin_ids.contains(&datasource_type) {
        checks.push(SyncPreflightCheck {
            kind: "plugin".to_string(),
            identity: if datasource_type.is_empty() {
                "unknown".to_string()
            } else {
                datasource_type
            },
            status: "ok".to_string(),
            detail: "Datasource plugin type is available.".to_string(),
            blocking: false,
        });
    } else {
        checks.push(SyncPreflightCheck {
            kind: "plugin".to_string(),
            identity: datasource_type,
            status: "missing".to_string(),
            detail: "Datasource plugin type is not listed in destination plugin availability."
                .to_string(),
            blocking: true,
        });
    }
    Ok(checks)
}

fn build_dashboard_checks(
    spec: &SyncResourceSpec,
    availability: &Map<String, Value>,
) -> Result<Vec<SyncPreflightCheck>> {
    let available_uids = require_string_list(availability.get("datasourceUids"), "datasourceUids")?
        .into_iter()
        .collect::<BTreeSet<String>>();
    let datasource_uids =
        require_string_list(spec.body.get("datasourceUids"), "dashboard datasourceUids")?;
    Ok(datasource_uids
        .into_iter()
        .map(|datasource_uid| {
            let available = available_uids.contains(&datasource_uid);
            SyncPreflightCheck {
                kind: "dashboard-datasource".to_string(),
                identity: format!("{}->{}", spec.identity, datasource_uid),
                status: if available { "ok" } else { "missing" }.to_string(),
                detail: if available {
                    "Referenced datasource is available for dashboard sync."
                } else {
                    "Referenced datasource is missing for dashboard sync."
                }
                .to_string(),
                blocking: !available,
            }
        })
        .collect())
}

fn build_alert_checks(
    spec: &SyncResourceSpec,
    availability: &Map<String, Value>,
) -> Result<Vec<SyncPreflightCheck>> {
    let available_contact_points =
        require_string_list(availability.get("contactPoints"), "contactPoints")?
            .into_iter()
            .collect::<BTreeSet<String>>();
    let mut checks = vec![SyncPreflightCheck {
        kind: "alert-live-apply".to_string(),
        identity: spec.identity.clone(),
        status: "blocked".to_string(),
        detail:
            "Alert sync stays plan-only until partial ownership and live-apply semantics are explicitly wired."
                .to_string(),
        blocking: true,
    }];
    for contact_point in require_string_list(spec.body.get("contactPoints"), "alert contactPoints")?
    {
        let available = available_contact_points.contains(&contact_point);
        checks.push(SyncPreflightCheck {
            kind: "alert-contact-point".to_string(),
            identity: format!("{}->{}", spec.identity, contact_point),
            status: if available { "ok" } else { "missing" }.to_string(),
            detail: if available {
                "Alert contact point is available."
            } else {
                "Alert contact point is missing."
            }
            .to_string(),
            blocking: !available,
        });
    }
    Ok(checks)
}

pub fn build_sync_preflight_document(
    desired_specs: &[Value],
    availability: Option<&Value>,
) -> Result<Value> {
    let specs = normalize_resource_specs(desired_specs)?;
    let availability = require_object(availability, "availability")?;
    let mut checks = Vec::new();
    for spec in &specs {
        match spec.kind.as_str() {
            "datasource" => checks.extend(build_datasource_checks(spec, &availability)?),
            "dashboard" => checks.extend(build_dashboard_checks(spec, &availability)?),
            "alert" => checks.extend(build_alert_checks(spec, &availability)?),
            "folder" => checks.push(SyncPreflightCheck {
                kind: "folder".to_string(),
                identity: spec.identity.clone(),
                status: "ok".to_string(),
                detail: "Folder sync does not require extra staged preflight checks.".to_string(),
                blocking: false,
            }),
            other => return Err(message(format!("Unsupported sync preflight kind {other}."))),
        }
    }
    let check_count = checks.len();
    let ok_count = checks.iter().filter(|item| item.status == "ok").count();
    let create_planned_count = checks
        .iter()
        .filter(|item| item.status == "create-planned")
        .count();
    let missing_count = checks.iter().filter(|item| item.status == "missing").count();
    let blocking_count = checks.iter().filter(|item| item.blocking).count();
    let policy_blocking_count = checks
        .iter()
        .filter(|item| item.blocking && item.kind == "alert-live-apply")
        .count();
    let dependency_blocking_count = blocking_count.saturating_sub(policy_blocking_count);
    let alert_policy_count = checks
        .iter()
        .filter(|item| item.kind == "alert-live-apply")
        .count();
    Ok(serde_json::json!({
        "kind": SYNC_PREFLIGHT_KIND,
        "schemaVersion": SYNC_PREFLIGHT_SCHEMA_VERSION,
        "summary": {
            "resourceCount": specs.len(),
            "checkCount": check_count,
            "okCount": ok_count,
            "createPlannedCount": create_planned_count,
            "missingCount": missing_count,
            "blockingCount": blocking_count,
            "dependencyBlockingCount": dependency_blocking_count,
            "policyBlockingCount": policy_blocking_count,
            "alertPolicyCount": alert_policy_count,
        },
        "checks": checks.iter().map(|item| {
            serde_json::json!({
                "kind": item.kind,
                "identity": item.identity,
                "status": item.status,
                "detail": item.detail,
                "blocking": item.blocking,
            })
        }).collect::<Vec<_>>(),
    }))
}

pub fn render_sync_preflight_text(document: &Value) -> Result<Vec<String>> {
    let kind = normalize_text(document.get("kind"));
    if kind != SYNC_PREFLIGHT_KIND {
        return Err(message("Sync preflight document kind is not supported."));
    }
    let summary = require_object(document.get("summary"), "summary")?;
    let mut lines = vec![
        "Sync preflight summary".to_string(),
        format!(
            "Resources: {} total",
            summary
                .get("resourceCount")
                .and_then(Value::as_i64)
                .unwrap_or(0)
        ),
        format!(
            "Checks: {} total, {} ok, {} create-planned, {} blocking",
            summary
                .get("checkCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary.get("okCount").and_then(Value::as_i64).unwrap_or(0),
            summary
                .get("createPlannedCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("blockingCount")
                .and_then(Value::as_i64)
                .unwrap_or(0)
        )
    ];
    let dependency_blocking = summary
        .get("dependencyBlockingCount")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let policy_blocking = summary
        .get("policyBlockingCount")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    lines.push(format!(
        "Blocking split: dependency={} policy={}",
        dependency_blocking, policy_blocking
    ));
    let alert_policy_count = summary
        .get("alertPolicyCount")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    if alert_policy_count > 0 {
        lines.push(format!(
            "Alert policy: {} alert resources remain plan-only for live apply",
            alert_policy_count
        ));
    }
    lines.extend([
        String::new(),
        "# Checks".to_string(),
    ]);
    if let Some(items) = document.get("checks").and_then(Value::as_array) {
        for item in items {
            if let Some(object) = item.as_object() {
                lines.push(format!(
                    "- {} identity={} status={} detail={}",
                    normalize_text(object.get("kind")),
                    normalize_text(object.get("identity")),
                    normalize_text(object.get("status")),
                    normalize_text(object.get("detail")),
                ));
            }
        }
    }
    Ok(lines)
}
