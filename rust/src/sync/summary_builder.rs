//! Build normalized sync summaries and resource specs for review output.
//! This module reduces raw sync documents into the compact summary shape used by the
//! bundle and plan renderers, while preserving resource identity, counts, and alert
//! kind classification. It is presentation-oriented data shaping, not API transport.

use super::json::require_json_object;
use super::workbench::{
    SyncResourceSpec, SyncSummary, RESOURCE_KINDS, SYNC_SUMMARY_KIND, SYNC_SUMMARY_SCHEMA_VERSION,
};
use crate::common::{message, tool_version, Result};
use serde_json::{Map, Value};

pub(super) fn is_alert_sync_kind(kind: &str) -> bool {
    matches!(
        kind,
        "alert" | "alert-contact-point" | "alert-mute-timing" | "alert-policy" | "alert-template"
    )
}

fn normalize_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.trim().to_string(),
        Some(Value::Number(number)) => number.to_string(),
        Some(Value::Bool(flag)) => {
            if *flag {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        _ => String::new(),
    }
}

fn normalize_string_list(value: Option<&Value>, label: &str) -> Result<Vec<String>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let items = value
        .as_array()
        .ok_or_else(|| message(format!("{label} must be a list.")))?;
    let mut normalized = Vec::with_capacity(items.len());
    for item in items {
        let text = normalize_text(Some(item));
        if text.is_empty() {
            return Err(message(format!("{label} cannot contain empty values.")));
        }
        normalized.push(text);
    }
    Ok(normalized)
}

fn extract_identity(spec: &Map<String, Value>) -> String {
    for field in ["uid", "name", "title", "path"] {
        let value = normalize_text(spec.get(field));
        if !value.is_empty() {
            return value;
        }
    }
    String::new()
}

fn extract_title(spec: &Map<String, Value>, fallback_identity: &str) -> String {
    for field in ["title", "name", "uid", "path"] {
        let value = normalize_text(spec.get(field));
        if !value.is_empty() {
            return value;
        }
    }
    fallback_identity.to_string()
}

fn extract_body(spec: &Map<String, Value>) -> Result<Map<String, Value>> {
    if let Some(body) = spec.get("body") {
        return Ok(require_json_object(body, "body")?.clone());
    }
    if let Some(body) = spec.get("spec") {
        return Ok(require_json_object(body, "spec")?.clone());
    }
    Ok(Map::new())
}

pub fn normalize_resource_spec(raw_spec: &Value) -> Result<SyncResourceSpec> {
    let spec = require_json_object(raw_spec, "Sync resource spec")?;
    let kind = normalize_text(spec.get("kind")).to_lowercase();
    if !RESOURCE_KINDS.contains(&kind.as_str()) {
        return Err(message(format!(
            "Unsupported sync resource kind {:?}. Expected one of {}.",
            kind,
            RESOURCE_KINDS.join(", ")
        )));
    }
    let identity = extract_identity(spec);
    if identity.is_empty() {
        return Err(message(
            "Sync resource spec requires uid, name, title, or path.",
        ));
    }
    let managed_fields = normalize_string_list(spec.get("managedFields"), "managedFields")?;
    if is_alert_sync_kind(&kind) && managed_fields.is_empty() {
        return Err(message(
            "Alert sync specs must declare managedFields to keep partial ownership explicit.",
        ));
    }
    Ok(SyncResourceSpec {
        kind,
        identity: identity.clone(),
        title: extract_title(spec, &identity),
        body: extract_body(spec)?,
        managed_fields,
        source_path: normalize_text(spec.get("sourcePath")),
    })
}

pub fn normalize_resource_specs(raw_specs: &[Value]) -> Result<Vec<SyncResourceSpec>> {
    raw_specs
        .iter()
        .map(normalize_resource_spec)
        .collect::<Result<Vec<_>>>()
}

pub fn summarize_resource_specs(specs: &[SyncResourceSpec]) -> SyncSummary {
    SyncSummary {
        resource_count: specs.len(),
        dashboard_count: specs.iter().filter(|item| item.kind == "dashboard").count(),
        datasource_count: specs
            .iter()
            .filter(|item| item.kind == "datasource")
            .count(),
        folder_count: specs.iter().filter(|item| item.kind == "folder").count(),
        alert_count: specs
            .iter()
            .filter(|item| is_alert_sync_kind(&item.kind))
            .count(),
    }
}

pub fn build_sync_summary_document(raw_specs: &[Value]) -> Result<Value> {
    let specs = normalize_resource_specs(raw_specs)?;
    let summary = summarize_resource_specs(&specs);
    Ok(serde_json::json!({
        "kind": SYNC_SUMMARY_KIND,
        "schemaVersion": SYNC_SUMMARY_SCHEMA_VERSION,
        "toolVersion": tool_version(),
        "summary": {
            "resourceCount": summary.resource_count,
            "dashboardCount": summary.dashboard_count,
            "datasourceCount": summary.datasource_count,
            "folderCount": summary.folder_count,
            "alertCount": summary.alert_count,
        },
        "resources": specs.iter().map(|item| {
            serde_json::json!({
                "kind": item.kind,
                "identity": item.identity,
                "title": item.title,
                "managedFields": item.managed_fields,
                "bodyFieldCount": item.body.len(),
                "sourcePath": item.source_path,
            })
        }).collect::<Vec<_>>(),
    }))
}
