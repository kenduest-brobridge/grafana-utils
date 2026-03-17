//! Staged bundle-level preflight helpers.
//!
//! Purpose:
//! - Stage one combined preflight view across dashboards, datasources, folders,
//!   and alerts.
//! - Reuse existing staged Rust contracts without wiring them into any CLI.

use crate::alert_sync::assess_alert_sync_specs;
use crate::common::{message, Result};
use crate::datasource_provider::{
    build_provider_plan, iter_provider_names, summarize_provider_plan,
};
use crate::sync_preflight::build_sync_preflight_document;
use crate::sync_workbench::build_sync_summary_document;
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;

/// Constant for bundle preflight kind.
pub const BUNDLE_PREFLIGHT_KIND: &str = "grafana-utils-bundle-preflight";
/// Constant for bundle preflight schema version.
pub const BUNDLE_PREFLIGHT_SCHEMA_VERSION: i64 = 1;

fn require_object<'a>(value: Option<&'a Value>, label: &str) -> Result<&'a Map<String, Value>> {
    match value {
        Some(Value::Object(object)) => Ok(object),
        Some(_) => Err(message(format!("{label} must be a JSON object."))),
        None => Err(message(format!("{label} must be a JSON object."))),
    }
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

fn require_string_list(value: Option<&Value>, label: &str) -> Result<Vec<String>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let items = value
        .as_array()
        .ok_or_else(|| message(format!("{label} must be a list.")))?;
    Ok(items
        .iter()
        .filter_map(|item| {
            let text = normalize_text(Some(item), "");
            if text.is_empty() {
                None
            } else {
                Some(text)
            }
        })
        .collect())
}

fn bundle_section_items(bundle: &Map<String, Value>, key: &str) -> Vec<Value> {
    bundle
        .get(key)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn build_sync_specs_from_bundle(bundle: &Map<String, Value>) -> Vec<Value> {
    let mut sync_specs = Vec::new();
    for item in bundle_section_items(bundle, "dashboards") {
        if let Some(object) = item.as_object() {
            sync_specs.push(json!({
                "kind": "dashboard",
                "uid": object.get("uid"),
                "title": object.get("title"),
                "body": Value::Object(object.clone()),
            }));
        }
    }
    for item in bundle_section_items(bundle, "datasources") {
        if let Some(object) = item.as_object() {
            sync_specs.push(json!({
                "kind": "datasource",
                "uid": object.get("uid"),
                "name": object.get("name"),
                "title": object.get("name").cloned().unwrap_or(Value::Null),
                "body": Value::Object(object.clone()),
            }));
        }
    }
    for item in bundle_section_items(bundle, "folders") {
        sync_specs.push(item);
    }
    for item in bundle_section_items(bundle, "alerts") {
        if let Some(object) = item.as_object() {
            sync_specs.push(json!({
                "kind": "alert",
                "uid": object.get("uid"),
                "title": object.get("title"),
                "managedFields": object.get("managedFields").cloned().unwrap_or(Value::Array(Vec::new())),
                "body": object.get("body").cloned().unwrap_or(Value::Object(Map::new())),
            }));
        }
    }
    sync_specs
}

fn build_provider_assessment(
    datasources: &[Value],
    availability: &Map<String, Value>,
) -> Result<Value> {
    let available_provider_names = require_string_list(
        availability
            .get("providerNames")
            .or_else(|| availability.get("secretProviderNames")),
        "providerNames",
    )?
    .into_iter()
    .collect::<BTreeSet<String>>();
    let mut plans = Vec::new();
    let mut checks = Vec::new();
    for datasource in datasources {
        let Some(object) = datasource.as_object() else {
            continue;
        };
        if !object.contains_key("secureJsonDataProviders") {
            continue;
        }
        let plan = build_provider_plan(object)?;
        let summary = summarize_provider_plan(&plan);
        for provider_name in iter_provider_names(&plan.references) {
            let blocking = !available_provider_names.contains(provider_name);
            checks.push(json!({
                "kind": "secret-provider",
                "datasourceName": plan.datasource_name,
                "identity": format!(
                    "{}->{}",
                    plan.datasource_uid.clone().unwrap_or_else(|| plan.datasource_name.clone()),
                    provider_name
                ),
                "providerName": provider_name,
                "status": if blocking { "missing" } else { "ok" },
                "blocking": blocking,
            }));
        }
        plans.push(summary);
    }
    Ok(json!({
        "summary": {
            "datasourceCount": plans.len(),
            "referenceCount": plans.iter().flat_map(|plan| plan.get("providers").and_then(Value::as_array).into_iter().flatten()).count(),
            "blockingCount": checks.iter().filter(|item| item.get("blocking") == Some(&Value::Bool(true))).count(),
        },
        "plans": plans,
        "checks": checks,
    }))
}

/// Purpose: implementation note.
///
/// Args: see function signature.
/// Returns: see implementation.
pub fn build_bundle_preflight_document(
    source_bundle: &Value,
    target_inventory: Option<&Value>,
    availability: Option<&Value>,
) -> Result<Value> {
    // Call graph (hierarchy): this function is used in related modules.
    // Upstream callers: bundle_preflight_rust_tests.rs:build_bundle_preflight_document_aggregates_sync_alert_and_provider_checks, bundle_preflight_rust_tests.rs:render_bundle_preflight_text_renders_summary
    // Downstream callees: alert_sync.rs:assess_alert_sync_specs, bundle_preflight.rs:build_provider_assessment, bundle_preflight.rs:build_sync_specs_from_bundle, bundle_preflight.rs:bundle_section_items, bundle_preflight.rs:require_object, sync_preflight.rs:build_sync_preflight_document, sync_workbench.rs:build_sync_summary_document

    let source_bundle = require_object(Some(source_bundle), "source bundle")?;
    let availability_map = match availability {
        Some(value) => require_object(Some(value), "availability")?.clone(),
        None => Map::new(),
    };
    let sync_specs = build_sync_specs_from_bundle(source_bundle);
    let sync_summary = build_sync_summary_document(&sync_specs)?;
    let sync_preflight =
        build_sync_preflight_document(&sync_specs, Some(&Value::Object(availability_map.clone())))?;
    let alert_assessment = assess_alert_sync_specs(&bundle_section_items(source_bundle, "alerts"))?;
    let provider_assessment = build_provider_assessment(
        &bundle_section_items(source_bundle, "datasources"),
        &availability_map,
    )?;

    let target_summary = if let Some(target_inventory) = target_inventory {
        let target_inventory = require_object(Some(target_inventory), "target inventory")?;
        let target_specs = build_sync_specs_from_bundle(target_inventory);
        Some(build_sync_summary_document(&target_specs)?)
    } else {
        None
    };

    Ok(json!({
        "kind": BUNDLE_PREFLIGHT_KIND,
        "schemaVersion": BUNDLE_PREFLIGHT_SCHEMA_VERSION,
        "summary": {
            "syncBlockingCount": sync_preflight["summary"]["blockingCount"].as_i64().unwrap_or(0),
            "alertBlockedCount": alert_assessment["summary"]["blockedCount"].as_i64().unwrap_or(0),
            "alertPlanOnlyCount": alert_assessment["summary"]["planOnlyCount"].as_i64().unwrap_or(0),
            "providerBlockingCount": provider_assessment["summary"]["blockingCount"].as_i64().unwrap_or(0),
        },
        "sourceSummary": sync_summary,
        "targetSummary": target_summary,
        "syncPreflight": sync_preflight,
        "alertAssessment": alert_assessment,
        "providerAssessment": provider_assessment,
    }))
}

/// Purpose: implementation note.
///
/// Args: see function signature.
/// Returns: see implementation.
pub fn render_bundle_preflight_text(document: &Value) -> Result<Vec<String>> {
    // Call graph (hierarchy): this function is used in related modules.
    // Upstream callers: bundle_preflight_rust_tests.rs:render_bundle_preflight_text_renders_summary
    // Downstream callees: bundle_preflight.rs:normalize_text, bundle_preflight.rs:require_object, common.rs:message

    if normalize_text(document.get("kind"), "") != BUNDLE_PREFLIGHT_KIND {
        return Err(message("Bundle preflight document kind is not supported."));
    }
    let summary = require_object(document.get("summary"), "summary")?;
    Ok(vec![
        "Bundle preflight summary".to_string(),
        format!(
            "Sync blocking: {}",
            summary
                .get("syncBlockingCount")
                .and_then(Value::as_i64)
                .unwrap_or(0)
        ),
        format!(
            "Alert blocked: {}",
            summary
                .get("alertBlockedCount")
                .and_then(Value::as_i64)
                .unwrap_or(0)
        ),
        format!(
            "Alert plan-only: {}",
            summary
                .get("alertPlanOnlyCount")
                .and_then(Value::as_i64)
                .unwrap_or(0)
        ),
        format!(
            "Provider blocking: {}",
            summary
                .get("providerBlockingCount")
                .and_then(Value::as_i64)
                .unwrap_or(0)
        ),
    ])
}
