//! Build the local sync source bundle document from fetched resources.
//! This module assembles dashboards, datasources, folders, alerts, alerting state,
//! and metadata into the bundle schema consumed by sync export and review flows. It
//! only shapes the document; validation and live fetching happen in adjacent layers.

use super::bundle_alert_contracts::build_alert_bundle_contract_document;
use super::render_discovery_summary_from_value;
use super::workbench::{SYNC_SOURCE_BUNDLE_KIND, SYNC_SOURCE_BUNDLE_SCHEMA_VERSION};
use crate::common::{message, tool_version, Result};
use serde::Serialize;
use serde_json::{Map, Value};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncSourceBundleSummaryDocument {
    dashboard_count: usize,
    datasource_count: usize,
    folder_count: usize,
    alert_rule_count: i64,
    contact_point_count: i64,
    mute_timing_count: i64,
    policy_count: i64,
    template_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncSourceBundleDocument {
    kind: &'static str,
    schema_version: i64,
    tool_version: &'static str,
    summary: SyncSourceBundleSummaryDocument,
    dashboards: Vec<Value>,
    datasources: Vec<Value>,
    folders: Vec<Value>,
    alerts: Vec<Value>,
    alerting: Value,
    alert_contract: Value,
    metadata: Value,
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
    let document = SyncSourceBundleDocument {
        kind: SYNC_SOURCE_BUNDLE_KIND,
        schema_version: SYNC_SOURCE_BUNDLE_SCHEMA_VERSION,
        tool_version: tool_version(),
        summary: SyncSourceBundleSummaryDocument {
            dashboard_count: dashboards.len(),
            datasource_count: datasources.len(),
            folder_count: folders.len(),
            alert_rule_count: alerting_summary
                .get("ruleCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            contact_point_count: alerting_summary
                .get("contactPointCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            mute_timing_count: alerting_summary
                .get("muteTimingCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            policy_count: alerting_summary
                .get("policyCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            template_count: alerting_summary
                .get("templateCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
        },
        dashboards: dashboards.to_vec(),
        datasources: datasources.to_vec(),
        folders: folders.to_vec(),
        alerts: alerts.to_vec(),
        alerting,
        alert_contract,
        metadata,
    };
    Ok(serde_json::to_value(document)?)
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
    let mut lines = vec!["Sync source bundle".to_string()];
    if let Some(discovery) = document.get("discovery").and_then(Value::as_object) {
        if let Some(summary_line) = render_discovery_summary_from_value(discovery) {
            lines.push(summary_line);
        }
    }
    lines.extend([
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
    ]);
    Ok(lines)
}
