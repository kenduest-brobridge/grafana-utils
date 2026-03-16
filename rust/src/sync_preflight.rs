//! Staged sync preflight helpers.
//!
//! Purpose:
//! - Build one pure preflight document from desired sync resources and explicit
//!   availability hints.
//! - Keep staged dependency and alert live-apply policy checks isolated from
//!   any Rust CLI or Grafana transport wiring.

use crate::common::{message, Result};
use crate::sync_workbench::{normalize_resource_specs, SyncResourceSpec};
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

fn is_builtin_alert_datasource_ref(value: &str) -> bool {
    matches!(value, "__expr__" | "__dashboard__")
}

fn collect_alert_datasource_uids(body: &Map<String, Value>) -> Result<Vec<String>> {
    let mut datasource_uids = BTreeSet::new();
    let direct_uid = normalize_text(body.get("datasourceUid"));
    if !direct_uid.is_empty() && !is_builtin_alert_datasource_ref(&direct_uid) {
        datasource_uids.insert(direct_uid);
    }
    for datasource_uid in require_string_list(body.get("datasourceUids"), "alert datasourceUids")? {
        if !is_builtin_alert_datasource_ref(&datasource_uid) {
            datasource_uids.insert(datasource_uid);
        }
    }
    for item in body
        .get("data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(object) = item.as_object() else {
            continue;
        };
        let datasource_uid = normalize_text(object.get("datasourceUid"));
        if !datasource_uid.is_empty() && !is_builtin_alert_datasource_ref(&datasource_uid) {
            datasource_uids.insert(datasource_uid);
        }
    }
    Ok(datasource_uids.into_iter().collect())
}

fn build_alert_checks(
    spec: &SyncResourceSpec,
    availability: &Map<String, Value>,
) -> Result<Vec<SyncPreflightCheck>> {
    let available_datasource_uids =
        require_string_list(availability.get("datasourceUids"), "datasourceUids")?
            .into_iter()
            .collect::<BTreeSet<String>>();
    let available_contact_points =
        require_string_list(availability.get("contactPoints"), "contactPoints")?
            .into_iter()
            .collect::<BTreeSet<String>>();
    let body = require_object(Some(&Value::Object(spec.body.clone())), "alert body")?;
    let mut checks = vec![SyncPreflightCheck {
        kind: "alert-live-apply".to_string(),
        identity: spec.identity.clone(),
        status: "blocked".to_string(),
        detail:
            "Alert sync stays plan-only until partial ownership and live-apply semantics are explicitly wired."
                .to_string(),
        blocking: true,
    }];
    for datasource_uid in collect_alert_datasource_uids(&body)? {
        let available = available_datasource_uids.contains(&datasource_uid);
        checks.push(SyncPreflightCheck {
            kind: "alert-datasource".to_string(),
            identity: format!("{}->{}", spec.identity, datasource_uid),
            status: if available { "ok" } else { "missing" }.to_string(),
            detail: if available {
                "Alert datasource is available."
            } else {
                "Alert datasource is missing."
            }
            .to_string(),
            blocking: !available,
        });
    }
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
    Ok(serde_json::json!({
        "kind": SYNC_PREFLIGHT_KIND,
        "schemaVersion": SYNC_PREFLIGHT_SCHEMA_VERSION,
        "summary": {
            "checkCount": checks.len(),
            "okCount": checks.iter().filter(|item| item.status == "ok").count(),
            "blockingCount": checks.iter().filter(|item| item.blocking).count(),
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
            "Checks: {} total, {} ok, {} blocking",
            summary
                .get("checkCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary.get("okCount").and_then(Value::as_i64).unwrap_or(0),
            summary
                .get("blockingCount")
                .and_then(Value::as_i64)
                .unwrap_or(0)
        ),
        String::new(),
        "# Checks".to_string(),
    ];
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
