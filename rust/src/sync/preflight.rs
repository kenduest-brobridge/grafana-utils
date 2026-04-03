//! Staged sync preflight helpers.
//!
//! Build one pure preflight document from desired sync resources and explicit
//! availability hints.
//! Keep staged dependency and alert live-apply policy checks isolated from
//! Rust CLI and Grafana transport wiring.

use super::json::{require_json_object, require_json_object_field};
use super::workbench::{normalize_resource_specs, SyncResourceSpec};
use crate::common::{message, tool_version, Result};
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SyncPreflightSummary {
    pub check_count: i64,
    pub ok_count: i64,
    pub blocking_count: i64,
}

impl SyncPreflightSummary {
    pub(crate) fn from_document(document: &Value) -> Result<Self> {
        let object = require_json_object(document, "Sync preflight document")?;
        let summary = require_json_object_field(object, "summary", "Sync preflight document")?;
        serde_json::from_value(Value::Object(summary.clone()))
            .map_err(|error| message(format!("Sync preflight summary is invalid: {error}")))
    }
}

pub(crate) fn require_sync_preflight_summary(document: &Value) -> Result<SyncPreflightSummary> {
    let document = require_json_object(document, "Sync preflight document")?;
    let summary = require_json_object_field(document, "summary", "Sync preflight document")?;
    let check_count = summary
        .get("checkCount")
        .and_then(Value::as_i64)
        .ok_or_else(|| message("Sync preflight summary is missing checkCount."))?;
    let ok_count = summary
        .get("okCount")
        .and_then(Value::as_i64)
        .ok_or_else(|| message("Sync preflight summary is missing okCount."))?;
    let blocking_count = summary
        .get("blockingCount")
        .and_then(Value::as_i64)
        .ok_or_else(|| message("Sync preflight summary is missing blockingCount."))?;
    Ok(SyncPreflightSummary {
        check_count,
        ok_count,
        blocking_count,
    })
}

fn normalize_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.trim().to_string(),
        Some(Value::Number(number)) => number.to_string(),
        _ => String::new(),
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
    let available_names =
        require_string_list(availability.get("datasourceNames"), "datasourceNames")?
            .into_iter()
            .collect::<BTreeSet<String>>();
    let datasource_uids =
        require_string_list(spec.body.get("datasourceUids"), "dashboard datasourceUids")?;
    let datasource_names = require_string_list(
        spec.body.get("datasourceNames"),
        "dashboard datasourceNames",
    )?;
    let available_plugin_ids = require_string_list(availability.get("pluginIds"), "pluginIds")?
        .into_iter()
        .collect::<BTreeSet<String>>();
    let plugin_ids = require_string_list(spec.body.get("pluginIds"), "dashboard pluginIds")?;
    let mut checks = datasource_uids
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
        .collect::<Vec<_>>();
    checks.extend(datasource_names.into_iter().map(|datasource_name| {
        let available = available_names.contains(&datasource_name);
        SyncPreflightCheck {
            kind: "dashboard-datasource-name".to_string(),
            identity: format!("{}->{}", spec.identity, datasource_name),
            status: if available { "ok" } else { "missing" }.to_string(),
            detail: if available {
                "Referenced datasource name is available for dashboard sync."
            } else {
                "Referenced datasource name is missing for dashboard sync."
            }
            .to_string(),
            blocking: !available,
        }
    }));
    checks.extend(plugin_ids.into_iter().map(|plugin_id| {
        let available = available_plugin_ids.contains(&plugin_id);
        SyncPreflightCheck {
            kind: "dashboard-plugin".to_string(),
            identity: format!("{}->{}", spec.identity, plugin_id),
            status: if available { "ok" } else { "missing" }.to_string(),
            detail: if available {
                "Dashboard plugin dependency is available."
            } else {
                "Dashboard plugin dependency is missing."
            }
            .to_string(),
            blocking: !available,
        }
    }));
    Ok(checks)
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

fn collect_alert_datasource_names(body: &Map<String, Value>) -> Result<Vec<String>> {
    let mut datasource_names = BTreeSet::new();
    let direct_name = normalize_text(body.get("datasourceName"));
    if !direct_name.is_empty() {
        datasource_names.insert(direct_name);
    }
    for datasource_name in
        require_string_list(body.get("datasourceNames"), "alert datasourceNames")?
    {
        datasource_names.insert(datasource_name);
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
        let datasource_name = normalize_text(object.get("datasourceName"));
        if !datasource_name.is_empty() {
            datasource_names.insert(datasource_name);
        }
    }
    Ok(datasource_names.into_iter().collect())
}

fn collect_alert_contact_points(body: &Map<String, Value>) -> Result<Vec<String>> {
    let mut contact_points = require_string_list(body.get("contactPoints"), "alert contactPoints")?
        .into_iter()
        .collect::<BTreeSet<String>>();
    let receiver = normalize_text(body.get("receiver"));
    if !receiver.is_empty() {
        contact_points.insert(receiver);
    }
    if let Some(notification_settings) = body.get("notificationSettings").and_then(Value::as_object)
    {
        let receiver = normalize_text(notification_settings.get("receiver"));
        if !receiver.is_empty() {
            contact_points.insert(receiver);
        }
    }
    Ok(contact_points.into_iter().collect())
}

fn build_alert_checks(
    spec: &SyncResourceSpec,
    availability: &Map<String, Value>,
) -> Result<Vec<SyncPreflightCheck>> {
    if spec.kind != "alert" {
        return Ok(vec![SyncPreflightCheck {
            kind: "alert-live-apply".to_string(),
            identity: spec.identity.clone(),
            status: "ok".to_string(),
            detail: "Alert provisioning resource is eligible for live apply.".to_string(),
            blocking: false,
        }]);
    }
    let available_datasource_uids =
        require_string_list(availability.get("datasourceUids"), "datasourceUids")?
            .into_iter()
            .collect::<BTreeSet<String>>();
    let available_datasource_names =
        require_string_list(availability.get("datasourceNames"), "datasourceNames")?
            .into_iter()
            .collect::<BTreeSet<String>>();
    let available_contact_points =
        require_string_list(availability.get("contactPoints"), "contactPoints")?
            .into_iter()
            .collect::<BTreeSet<String>>();
    let available_plugin_ids = require_string_list(availability.get("pluginIds"), "pluginIds")?
        .into_iter()
        .collect::<BTreeSet<String>>();
    let body_value = Value::Object(spec.body.clone());
    let body = require_json_object(&body_value, "alert body")?;
    let plugin_ids = require_string_list(body.get("pluginIds"), "alert pluginIds")?;
    let mut checks = vec![SyncPreflightCheck {
        kind: "alert-live-apply".to_string(),
        identity: spec.identity.clone(),
        status: "blocked".to_string(),
        detail:
            "Alert sync stays plan-only until partial ownership and live-apply semantics are explicitly wired."
                .to_string(),
        blocking: true,
    }];
    for datasource_uid in collect_alert_datasource_uids(body)? {
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
    for datasource_name in collect_alert_datasource_names(body)? {
        let available = available_datasource_names.contains(&datasource_name);
        checks.push(SyncPreflightCheck {
            kind: "alert-datasource-name".to_string(),
            identity: format!("{}->{}", spec.identity, datasource_name),
            status: if available { "ok" } else { "missing" }.to_string(),
            detail: if available {
                "Alert datasource name is available."
            } else {
                "Alert datasource name is missing."
            }
            .to_string(),
            blocking: !available,
        });
    }
    for plugin_id in plugin_ids {
        let available = available_plugin_ids.contains(&plugin_id);
        checks.push(SyncPreflightCheck {
            kind: "alert-plugin".to_string(),
            identity: format!("{}->{}", spec.identity, plugin_id),
            status: if available { "ok" } else { "missing" }.to_string(),
            detail: if available {
                "Alert plugin dependency is available."
            } else {
                "Alert plugin dependency is missing."
            }
            .to_string(),
            blocking: !available,
        });
    }
    for contact_point in collect_alert_contact_points(body)? {
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
    let availability = match availability {
        None => Map::new(),
        Some(value) => require_json_object(value, "availability")?.clone(),
    };
    let mut checks = Vec::new();
    for spec in &specs {
        match spec.kind.as_str() {
            "datasource" => checks.extend(build_datasource_checks(spec, &availability)?),
            "dashboard" => checks.extend(build_dashboard_checks(spec, &availability)?),
            "alert"
            | "alert-contact-point"
            | "alert-mute-timing"
            | "alert-policy"
            | "alert-template" => checks.extend(build_alert_checks(spec, &availability)?),
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
        "toolVersion": tool_version(),
        "summary": serde_json::to_value(SyncPreflightSummary {
            check_count: checks.len() as i64,
            ok_count: checks.iter().filter(|item| item.status == "ok").count() as i64,
            blocking_count: checks.iter().filter(|item| item.blocking).count() as i64,
        })?,
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
    let summary = SyncPreflightSummary::from_document(document)?;
    let mut lines = vec![
        "Sync preflight summary".to_string(),
        format!(
            "Checks: {} total, {} ok, {} blocking",
            summary.check_count, summary.ok_count, summary.blocking_count
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
