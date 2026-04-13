//! Promotion-preflight summary, check shaping, and document building helpers.

use super::super::bundle_preflight::{
    build_sync_bundle_preflight_document, require_sync_bundle_preflight_summary,
};
use super::super::json::{require_json_object, require_json_object_field};
use super::promotion_preflight_mapping::{
    dashboard_folder_checks, datasource_reference_checks, nested_mapping,
    parse_promotion_mapping_document, PromotionMappingSummaryDocument,
    ALERT_DATASOURCE_NAME_REMAP_KIND, ALERT_DATASOURCE_UID_REMAP_KIND, DATASOURCE_NAME_REMAP_KIND,
    DATASOURCE_UID_REMAP_KIND, FOLDER_REMAP_KIND,
};
use crate::common::{message, tool_version, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const SYNC_PROMOTION_PREFLIGHT_KIND: &str = "grafana-utils-sync-promotion-preflight";
pub const SYNC_PROMOTION_PREFLIGHT_SCHEMA_VERSION: i64 = 1;
pub const SYNC_PROMOTION_MAPPING_KIND: &str = "grafana-utils-sync-promotion-mapping";
pub const SYNC_PROMOTION_MAPPING_SCHEMA_VERSION: i64 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct SyncPromotionPreflightSummary {
    pub resource_count: i64,
    pub direct_match_count: i64,
    pub mapped_count: i64,
    pub missing_mapping_count: i64,
    pub bundle_blocking_count: i64,
    pub blocking_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct PromotionCheckDocument {
    kind: String,
    identity: String,
    source_value: String,
    target_value: String,
    resolution: String,
    mapping_source: String,
    status: String,
    detail: String,
    blocking: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct PromotionCheckSummary {
    folder_remap_count: i64,
    datasource_uid_remap_count: i64,
    datasource_name_remap_count: i64,
    resolved_count: i64,
    direct_count: i64,
    mapped_count: i64,
    missing_target_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct PromotionHandoffSummary {
    review_required: bool,
    ready_for_review: bool,
    next_stage: String,
    blocking_count: i64,
    review_instruction: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct PromotionContinuationSummary {
    staged_only: bool,
    live_mutation_allowed: bool,
    ready_for_continuation: bool,
    next_stage: String,
    resolved_count: i64,
    blocking_count: i64,
    continuation_instruction: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromotionCheck {
    pub(crate) kind: String,
    pub(crate) identity: String,
    pub(crate) source_value: String,
    pub(crate) target_value: String,
    pub(crate) resolution: String,
    pub(crate) mapping_source: String,
    pub(crate) status: String,
    pub(crate) detail: String,
    pub(crate) blocking: bool,
}

impl From<&PromotionCheck> for PromotionCheckDocument {
    fn from(check: &PromotionCheck) -> Self {
        Self {
            kind: check.kind.clone(),
            identity: check.identity.clone(),
            source_value: check.source_value.clone(),
            target_value: check.target_value.clone(),
            resolution: check.resolution.clone(),
            mapping_source: check.mapping_source.clone(),
            status: check.status.clone(),
            detail: check.detail.clone(),
            blocking: check.blocking,
        }
    }
}

fn promotion_check_documents(checks: &[PromotionCheck]) -> Vec<PromotionCheckDocument> {
    checks.iter().map(PromotionCheckDocument::from).collect()
}

fn promotion_check_documents_with_filter<F>(
    checks: &[PromotionCheck],
    predicate: F,
) -> Vec<PromotionCheckDocument>
where
    F: Fn(&PromotionCheck) -> bool,
{
    checks
        .iter()
        .filter(|check| predicate(check))
        .map(PromotionCheckDocument::from)
        .collect()
}

pub(crate) fn normalize_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.trim().to_string(),
        Some(Value::Number(number)) => number.to_string(),
        _ => String::new(),
    }
}

impl SyncPromotionPreflightSummary {
    pub(crate) fn from_document(document: &Value) -> Result<Self> {
        let object = require_json_object(document, "Sync promotion preflight document")?;
        let summary =
            require_json_object_field(object, "summary", "Sync promotion preflight document")?;
        serde_json::from_value(Value::Object(summary.clone())).map_err(|error| {
            message(format!(
                "Sync promotion preflight summary is invalid: {error}"
            ))
        })
    }
}

fn summarize_promotion_checks(checks: &[PromotionCheck]) -> PromotionCheckSummary {
    PromotionCheckSummary {
        folder_remap_count: checks
            .iter()
            .filter(|item| item.kind == FOLDER_REMAP_KIND)
            .count() as i64,
        datasource_uid_remap_count: checks
            .iter()
            .filter(|item| {
                item.kind == DATASOURCE_UID_REMAP_KIND
                    || item.kind == ALERT_DATASOURCE_UID_REMAP_KIND
            })
            .count() as i64,
        datasource_name_remap_count: checks
            .iter()
            .filter(|item| {
                item.kind == DATASOURCE_NAME_REMAP_KIND
                    || item.kind == ALERT_DATASOURCE_NAME_REMAP_KIND
            })
            .count() as i64,
        resolved_count: checks.iter().filter(|item| !item.blocking).count() as i64,
        direct_count: checks.iter().filter(|item| item.status == "direct").count() as i64,
        mapped_count: checks.iter().filter(|item| item.status == "mapped").count() as i64,
        missing_target_count: checks
            .iter()
            .filter(|item| item.status == "missing-target")
            .count() as i64,
    }
}

fn partition_promotion_checks(checks: &[Value]) -> (Vec<&Value>, Vec<&Value>) {
    let mut resolved_checks = Vec::new();
    let mut blocking_checks = Vec::new();
    for check in checks {
        if check
            .get("blocking")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            blocking_checks.push(check);
        } else {
            resolved_checks.push(check);
        }
    }
    (resolved_checks, blocking_checks)
}

pub(crate) fn render_promotion_check_arrays(
    document: &Value,
) -> Result<(Vec<&Value>, Vec<&Value>)> {
    // Accept both the newer resolved/blocking split and the older flat checks
    // array so historical artifacts keep rendering without regeneration.
    if let Some(resolved_checks) = document.get("resolvedChecks").and_then(Value::as_array) {
        Ok((
            resolved_checks.iter().collect(),
            document
                .get("blockingChecks")
                .and_then(Value::as_array)
                .map(|checks| checks.iter().collect())
                .unwrap_or_default(),
        ))
    } else if let Some(checks) = document.get("checks").and_then(Value::as_array) {
        Ok(partition_promotion_checks(checks))
    } else {
        Err(message(
            "Sync promotion preflight document is missing resolvedChecks.",
        ))
    }
}

fn summarize_promotion_handoff(blocking_count: i64) -> PromotionHandoffSummary {
    let ready_for_review = blocking_count == 0;
    // Handoff ends at review: blockers keep the next stage on resolve-blockers
    // until every remap and bundle issue is cleared.
    PromotionHandoffSummary {
        review_required: true,
        ready_for_review,
        next_stage: if ready_for_review {
            "review".to_string()
        } else {
            "resolve-blockers".to_string()
        },
        blocking_count,
        review_instruction: if ready_for_review {
            "promotion handoff is ready to move into review".to_string()
        } else {
            "promotion handoff is blocked until the listed remaps and bundle issues are cleared"
                .to_string()
        },
    }
}

fn summarize_promotion_continuation(
    resolved_count: i64,
    blocking_count: i64,
) -> PromotionContinuationSummary {
    let ready_for_continuation = blocking_count == 0;
    // Continuation is intentionally staged-only. It can move reviewed remaps
    // forward, but it never turns on live mutation inside this document.
    PromotionContinuationSummary {
        staged_only: true,
        live_mutation_allowed: false,
        ready_for_continuation,
        next_stage: if ready_for_continuation {
            "staged-apply-continuation".to_string()
        } else {
            "resolve-blockers".to_string()
        },
        resolved_count,
        blocking_count,
        continuation_instruction: if ready_for_continuation {
            "reviewed remaps can continue into a staged apply continuation without enabling live mutation"
                .to_string()
        } else {
            "keep the promotion staged until blockers clear; do not enter the apply continuation"
                .to_string()
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncPromotionPreflightDocument {
    kind: &'static str,
    schema_version: i64,
    tool_version: &'static str,
    summary: SyncPromotionPreflightSummary,
    #[serde(rename = "bundlePreflight")]
    bundle_preflight: Value,
    #[serde(rename = "mappingSummary")]
    mapping_summary: PromotionMappingSummaryDocument,
    #[serde(rename = "checkSummary")]
    check_summary: PromotionCheckSummary,
    #[serde(rename = "handoffSummary")]
    handoff_summary: PromotionHandoffSummary,
    #[serde(rename = "continuationSummary")]
    continuation_summary: PromotionContinuationSummary,
    checks: Vec<PromotionCheckDocument>,
    #[serde(rename = "resolvedChecks")]
    resolved_checks: Vec<PromotionCheckDocument>,
    #[serde(rename = "blockingChecks")]
    blocking_checks: Vec<PromotionCheckDocument>,
}

pub(crate) fn build_sync_promotion_preflight_document(
    source_bundle: &Value,
    target_inventory: &Value,
    availability: Option<&Value>,
    mapping: Option<&Value>,
) -> Result<Value> {
    let source_bundle = require_json_object(source_bundle, "Workspace package input")?;
    let target_inventory = require_json_object(target_inventory, "Sync target inventory input")?;
    // Build on bundle-preflight first so the promotion summary reuses the same
    // base blocking semantics before adding remap-specific checks.
    let bundle_preflight = build_sync_bundle_preflight_document(
        &Value::Object(source_bundle.clone()),
        &Value::Object(target_inventory.clone()),
        availability,
    )?;
    let (mapping, mapping_document) = parse_promotion_mapping_document(mapping)?;
    let folder_mapping = nested_mapping(&mapping, "folders", None);
    let datasource_uid_mapping = nested_mapping(&mapping, "datasources", Some("uids"));
    let datasource_name_mapping = nested_mapping(&mapping, "datasources", Some("names"));

    let mut checks = dashboard_folder_checks(source_bundle, target_inventory, &folder_mapping);
    checks.extend(datasource_reference_checks(
        source_bundle,
        target_inventory,
        &datasource_uid_mapping,
        &datasource_name_mapping,
    ));

    let direct_match_count = checks.iter().filter(|item| item.status == "direct").count() as i64;
    let mapped_count = checks.iter().filter(|item| item.status == "mapped").count() as i64;
    let missing_mapping_count = checks.iter().filter(|item| item.blocking).count() as i64;
    let check_summary = summarize_promotion_checks(&checks);
    let bundle_summary = require_sync_bundle_preflight_summary(&bundle_preflight)?;
    let bundle_blocking_count = bundle_summary.sync_blocking_count
        + bundle_summary.provider_blocking_count
        + bundle_summary.secret_placeholder_blocking_count
        + bundle_summary.alert_artifact_blocked_count;
    let blocking_count = bundle_blocking_count + missing_mapping_count;
    let continuation_summary =
        summarize_promotion_continuation(check_summary.resolved_count, blocking_count);
    let resource_count = source_bundle
        .get("summary")
        .and_then(Value::as_object)
        .map(|summary| summary.values().filter_map(Value::as_i64).sum::<i64>())
        .unwrap_or(0);

    Ok(serde_json::to_value(SyncPromotionPreflightDocument {
        kind: SYNC_PROMOTION_PREFLIGHT_KIND,
        schema_version: SYNC_PROMOTION_PREFLIGHT_SCHEMA_VERSION,
        tool_version: tool_version(),
        summary: SyncPromotionPreflightSummary {
            resource_count,
            direct_match_count,
            mapped_count,
            missing_mapping_count,
            bundle_blocking_count,
            blocking_count,
        },
        bundle_preflight,
        mapping_summary: PromotionMappingSummaryDocument {
            mapping_kind: mapping_document.get("kind").cloned().unwrap_or(Value::Null),
            mapping_schema_version: mapping_document
                .get("schemaVersion")
                .cloned()
                .unwrap_or(Value::Null),
            source_environment: mapping_document
                .get("sourceEnvironment")
                .cloned()
                .unwrap_or(Value::Null),
            target_environment: mapping_document
                .get("targetEnvironment")
                .cloned()
                .unwrap_or(Value::Null),
            folder_mapping_count: folder_mapping.len(),
            datasource_uid_mapping_count: datasource_uid_mapping.len(),
            datasource_name_mapping_count: datasource_name_mapping.len(),
        },
        check_summary,
        handoff_summary: summarize_promotion_handoff(blocking_count),
        continuation_summary,
        checks: promotion_check_documents(&checks),
        resolved_checks: promotion_check_documents_with_filter(&checks, |check| !check.blocking),
        blocking_checks: promotion_check_documents_with_filter(&checks, |check| check.blocking),
    })?)
}
