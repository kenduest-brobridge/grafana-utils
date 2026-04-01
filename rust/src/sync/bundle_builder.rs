//! Build the local sync source bundle document from fetched resources.
//! This module assembles dashboards, datasources, folders, alerts, alerting state,
//! and metadata into the bundle schema consumed by sync export and review flows. It
//! only shapes the document; validation and live fetching happen in adjacent layers.

use super::bundle_alert_contracts::build_alert_bundle_contract_document;
use super::workbench::{SYNC_SOURCE_BUNDLE_KIND, SYNC_SOURCE_BUNDLE_SCHEMA_VERSION};
use crate::common::{message, tool_version, Result};
use serde_json::{Map, Value};

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
        "toolVersion": tool_version(),
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
