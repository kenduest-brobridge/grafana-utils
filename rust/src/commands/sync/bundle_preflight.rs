//! Staged bundle-level sync preflight helpers.
//!
//! Purpose:
//! - Aggregate staged sync preflight checks and datasource provider assessments
//!   into one reviewable bundle document.
//! - Keep Rust-side bundle planning pure and import-safe before any CLI wiring.

pub(crate) use super::bundle_preflight_assessments::alert_artifact_assessment_summary_or_default;
use super::bundle_preflight_assessments::{
    alert_artifact_assessment_summary, build_alert_artifact_assessment, build_provider_assessment,
    build_secret_placeholder_assessment, collect_alert_specs, provider_assessment_summary,
    secret_placeholder_assessment_summary,
};
use super::json::{require_json_array, require_json_object, require_json_object_field};
use super::preflight::{build_sync_preflight_document, SyncPreflightSummary};
use crate::common::{message, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Constant for sync bundle preflight kind.
pub const SYNC_BUNDLE_PREFLIGHT_KIND: &str = "grafana-utils-sync-bundle-preflight";
/// Constant for sync bundle preflight schema version.
pub const SYNC_BUNDLE_PREFLIGHT_SCHEMA_VERSION: i64 = 1;

/// Struct definition for SyncBundlePreflightSummary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SyncBundlePreflightSummary {
    pub resource_count: i64,
    pub sync_blocking_count: i64,
    pub provider_blocking_count: i64,
    pub secret_placeholder_blocking_count: i64,
    pub alert_artifact_count: i64,
    pub alert_artifact_blocked_count: i64,
    pub alert_artifact_plan_only_count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncBundlePreflightDocument {
    kind: &'static str,
    schema_version: i64,
    summary: SyncBundlePreflightSummary,
    sync_preflight: Value,
    alert_artifact_assessment: Value,
    secret_placeholder_assessment: Value,
    provider_assessment: Value,
}

impl SyncBundlePreflightSummary {
    pub(crate) fn from_document(document: &Value) -> Result<Self> {
        let object = require_json_object(document, "Sync bundle preflight document")?;
        let summary =
            require_json_object_field(object, "summary", "Sync bundle preflight document")?;
        serde_json::from_value(Value::Object(summary.clone()))
            .map_err(|error| message(format!("Sync bundle preflight summary is invalid: {error}")))
    }
}

pub(crate) fn require_sync_bundle_preflight_summary(
    document: &Value,
) -> Result<SyncBundlePreflightSummary> {
    let document = require_json_object(document, "Sync bundle preflight document")?;
    let summary = require_json_object_field(document, "summary", "Sync bundle preflight document")?;
    let resource_count = summary
        .get("resourceCount")
        .and_then(Value::as_i64)
        .ok_or_else(|| message("Sync bundle preflight summary is missing resourceCount."))?;
    let sync_blocking_count = summary
        .get("syncBlockingCount")
        .and_then(Value::as_i64)
        .ok_or_else(|| message("Sync bundle preflight summary is missing syncBlockingCount."))?;
    let provider_blocking_count = summary
        .get("providerBlockingCount")
        .and_then(Value::as_i64)
        .ok_or_else(|| {
            message("Sync bundle preflight summary is missing providerBlockingCount.")
        })?;
    let secret_placeholder_blocking_count = summary
        .get("secretPlaceholderBlockingCount")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    Ok(SyncBundlePreflightSummary {
        resource_count,
        sync_blocking_count,
        provider_blocking_count,
        secret_placeholder_blocking_count,
        alert_artifact_count: 0,
        alert_artifact_blocked_count: 0,
        alert_artifact_plan_only_count: 0,
    })
}

pub fn build_sync_bundle_preflight_document(
    source_bundle: &Value,
    target_inventory: &Value,
    availability: Option<&Value>,
) -> Result<Value> {
    let source_bundle = require_json_object(source_bundle, "source bundle")?.clone();
    let _target_inventory = require_json_object(target_inventory, "target inventory")?.clone();
    let availability = match availability {
        None => Map::new(),
        Some(value) => require_json_object(value, "availability")?.clone(),
    };
    let mut desired_specs = Vec::new();
    for key in ["dashboards", "datasources", "folders"] {
        let Some(items) = source_bundle.get(key) else {
            continue;
        };
        for item in require_json_array(items, key)? {
            desired_specs.push(item.clone());
        }
    }
    desired_specs.extend(collect_alert_specs(&source_bundle)?);

    let sync_preflight =
        build_sync_preflight_document(&desired_specs, Some(&Value::Object(availability.clone())))?;
    let alert_artifact_assessment = build_alert_artifact_assessment(&source_bundle);
    let datasource_specs = source_bundle
        .get("datasources")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let provider_assessment = build_provider_assessment(datasource_specs, &availability)?;
    let secret_placeholder_assessment =
        build_secret_placeholder_assessment(datasource_specs, &availability)?;

    let sync_preflight_summary = SyncPreflightSummary::from_document(&sync_preflight)?;
    let provider_summary = provider_assessment_summary(&provider_assessment)?;
    let secret_placeholder_summary =
        secret_placeholder_assessment_summary(&secret_placeholder_assessment)?;
    let alert_artifact_summary = alert_artifact_assessment_summary(&alert_artifact_assessment)?;
    let summary = SyncBundlePreflightSummary {
        resource_count: desired_specs.len() as i64,
        sync_blocking_count: sync_preflight_summary.blocking_count,
        provider_blocking_count: provider_summary.blocking_count,
        secret_placeholder_blocking_count: secret_placeholder_summary.blocking_count,
        alert_artifact_count: alert_artifact_summary.resource_count,
        alert_artifact_blocked_count: alert_artifact_summary.blocked_count,
        alert_artifact_plan_only_count: alert_artifact_summary.plan_only_count,
    };
    let document = SyncBundlePreflightDocument {
        kind: SYNC_BUNDLE_PREFLIGHT_KIND,
        schema_version: SYNC_BUNDLE_PREFLIGHT_SCHEMA_VERSION,
        summary,
        sync_preflight,
        alert_artifact_assessment,
        secret_placeholder_assessment,
        provider_assessment,
    };
    serde_json::to_value(document).map_err(|error| {
        message(format!(
            "Sync bundle preflight document serialization failed: {error}"
        ))
    })
}

fn document_kind(document: &Value) -> &str {
    document
        .get("kind")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("")
}

fn secret_placeholder_summary_counts(document: &Value) -> (i64, i64, i64) {
    let Some(summary) = document
        .get("secretPlaceholderAssessment")
        .and_then(Value::as_object)
        .and_then(|assessment| assessment.get("summary"))
        .and_then(Value::as_object)
    else {
        return (0, 0, 0);
    };
    (
        summary
            .get("datasourceCount")
            .and_then(Value::as_i64)
            .unwrap_or(0),
        summary
            .get("referenceCount")
            .and_then(Value::as_i64)
            .unwrap_or(0),
        summary
            .get("blockingCount")
            .and_then(Value::as_i64)
            .unwrap_or(0),
    )
}

pub fn render_sync_bundle_preflight_text(document: &Value) -> Result<Vec<String>> {
    if document_kind(document) != SYNC_BUNDLE_PREFLIGHT_KIND {
        return Err(message(
            "Sync bundle preflight document kind is not supported.",
        ));
    }
    let summary = SyncBundlePreflightSummary::from_document(document)?;
    let (
        secret_placeholder_datasource_count,
        secret_placeholder_reference_count,
        secret_placeholder_blocking_count,
    ) = secret_placeholder_summary_counts(document);
    Ok(vec![
        "Sync bundle preflight summary".to_string(),
        format!("Resources: {} total", summary.resource_count),
        format!("Sync blocking: {}", summary.sync_blocking_count),
        format!("Provider blocking: {}", summary.provider_blocking_count),
        format!(
            "Secret placeholders: {} datasources, {} references, {} blocking",
            secret_placeholder_datasource_count,
            secret_placeholder_reference_count,
            secret_placeholder_blocking_count
        ),
        format!(
            "Secret placeholders blocking: {}",
            summary.secret_placeholder_blocking_count
        ),
        format!("Alert artifacts: {} total", summary.alert_artifact_count),
        "Reason: missing provider or secret placeholder availability blocks apply; plan-only alert artifacts stay staged; blocked artifacts prevent apply."
            .to_string(),
    ])
}

#[cfg(test)]
#[path = "bundle_rust_tests.rs"]
mod sync_bundle_rust_tests;
