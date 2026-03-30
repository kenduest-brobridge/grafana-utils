//! Promotion-preflight checks for staged sync handoff.
//!
//! This module owns promotion-specific remap checks and the review /
//! continuation summaries built on top of bundle-preflight. Base sync blocking
//! stays in the bundle document so promotion only layers on mapping policy.

use super::bundle_preflight::{
    build_sync_bundle_preflight_document, render_sync_bundle_preflight_text,
    require_sync_bundle_preflight_summary,
};
use super::json::{require_json_object, require_json_object_field};
use crate::common::{message, tool_version, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeSet;

pub const SYNC_PROMOTION_PREFLIGHT_KIND: &str = "grafana-utils-sync-promotion-preflight";
pub const SYNC_PROMOTION_PREFLIGHT_SCHEMA_VERSION: i64 = 1;
pub const SYNC_PROMOTION_MAPPING_KIND: &str = "grafana-utils-sync-promotion-mapping";
pub const SYNC_PROMOTION_MAPPING_SCHEMA_VERSION: i64 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SyncPromotionPreflightSummary {
    pub resource_count: i64,
    pub direct_match_count: i64,
    pub mapped_count: i64,
    pub missing_mapping_count: i64,
    pub bundle_blocking_count: i64,
    pub blocking_count: i64,
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
struct PromotionCheck {
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
            .filter(|item| item.kind == "folder-remap")
            .count() as i64,
        datasource_uid_remap_count: checks
            .iter()
            .filter(|item| {
                item.kind == "datasource-uid-remap" || item.kind == "alert-datasource-uid-remap"
            })
            .count() as i64,
        datasource_name_remap_count: checks
            .iter()
            .filter(|item| {
                item.kind == "datasource-name-remap" || item.kind == "alert-datasource-name-remap"
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

fn render_promotion_check_arrays(document: &Value) -> Result<(Vec<&Value>, Vec<&Value>)> {
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

fn normalize_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.trim().to_string(),
        Some(Value::Number(number)) => number.to_string(),
        _ => String::new(),
    }
}

fn nested_mapping(
    root: &Map<String, Value>,
    first: &str,
    second: Option<&str>,
) -> Map<String, Value> {
    match second {
        Some(second) => root
            .get(first)
            .and_then(Value::as_object)
            .and_then(|object| object.get(second))
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default(),
        None => root
            .get(first)
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default(),
    }
}

fn parse_promotion_mapping_document(value: Option<&Value>) -> Result<(Map<String, Value>, Value)> {
    let Some(value) = value else {
        return Ok((Map::new(), Value::Object(Map::new())));
    };
    let object = require_json_object(value, "Sync promotion mapping input")?;
    let kind = normalize_text(object.get("kind"));
    if !kind.is_empty() && kind != SYNC_PROMOTION_MAPPING_KIND {
        return Err(message(
            "Sync promotion mapping input kind is not supported.",
        ));
    }
    if let Some(schema_version) = object.get("schemaVersion").and_then(Value::as_i64) {
        if schema_version != SYNC_PROMOTION_MAPPING_SCHEMA_VERSION {
            return Err(message(format!(
                "Sync promotion mapping schemaVersion must be {}.",
                SYNC_PROMOTION_MAPPING_SCHEMA_VERSION
            )));
        }
    }
    let metadata = object
        .get("metadata")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let source_environment = metadata
        .get("sourceEnvironment")
        .cloned()
        .unwrap_or(Value::Null);
    let target_environment = metadata
        .get("targetEnvironment")
        .cloned()
        .unwrap_or(Value::Null);
    let mut mapping = object.clone();
    mapping.remove("kind");
    mapping.remove("schemaVersion");
    mapping.remove("metadata");
    Ok((
        mapping,
        serde_json::json!({
            "kind": if kind.is_empty() { Value::Null } else { Value::String(kind) },
            "schemaVersion": object.get("schemaVersion").cloned().unwrap_or(Value::Null),
            "sourceEnvironment": source_environment,
            "targetEnvironment": target_environment,
        }),
    ))
}

fn target_uids(document: &Map<String, Value>, key: &str) -> BTreeSet<String> {
    document
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_object)
        .map(|object| normalize_text(object.get("uid")))
        .filter(|value| !value.is_empty())
        .collect()
}

fn target_names(document: &Map<String, Value>, key: &str) -> BTreeSet<String> {
    document
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_object)
        .map(|object| {
            let name = normalize_text(object.get("name"));
            if name.is_empty() {
                normalize_text(object.get("title"))
            } else {
                name
            }
        })
        .filter(|value| !value.is_empty())
        .collect()
}

fn mapped_target(mapping: &Map<String, Value>, source_value: &str) -> String {
    mapping
        .get(source_value)
        .map(|value| normalize_text(Some(value)))
        .unwrap_or_default()
}

fn classify_mapping_check(
    kind: &str,
    identity: String,
    source_value: String,
    mapped_value: String,
    mapping_source: &str,
    target_values: &BTreeSet<String>,
    missing_detail: String,
) -> Option<PromotionCheck> {
    if source_value.is_empty() {
        return None;
    }
    if target_values.contains(&source_value) {
        return Some(PromotionCheck {
            kind: kind.to_string(),
            identity,
            source_value: source_value.clone(),
            target_value: source_value,
            resolution: "direct-match".to_string(),
            mapping_source: "inventory".to_string(),
            status: "direct".to_string(),
            detail: "Target inventory already contains the same identifier.".to_string(),
            blocking: false,
        });
    }
    if !mapped_value.is_empty() && target_values.contains(&mapped_value) {
        return Some(PromotionCheck {
            kind: kind.to_string(),
            identity,
            source_value,
            target_value: mapped_value,
            resolution: "explicit-map".to_string(),
            mapping_source: mapping_source.to_string(),
            status: "mapped".to_string(),
            detail: "Promotion mapping resolves this source identifier onto the target inventory."
                .to_string(),
            blocking: false,
        });
    }
    Some(PromotionCheck {
        kind: kind.to_string(),
        identity,
        source_value,
        target_value: mapped_value,
        resolution: "missing-map".to_string(),
        mapping_source: mapping_source.to_string(),
        status: "missing-target".to_string(),
        detail: missing_detail,
        blocking: true,
    })
}

fn dashboard_folder_checks(
    source_bundle: &Map<String, Value>,
    target_inventory: &Map<String, Value>,
    mapping: &Map<String, Value>,
) -> Vec<PromotionCheck> {
    let target_folder_uids = target_uids(target_inventory, "folders");
    source_bundle
        .get("dashboards")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_object)
        .filter_map(|dashboard| {
            let dashboard_uid = normalize_text(dashboard.get("uid"));
            let folder_uid = normalize_text(dashboard.get("folderUid"));
            classify_mapping_check(
                "folder-remap",
                if dashboard_uid.is_empty() {
                    "dashboard".to_string()
                } else {
                    dashboard_uid
                },
                folder_uid.clone(),
                mapped_target(mapping, &folder_uid),
                "folders",
                &target_folder_uids,
                "Dashboard folder UID is missing from the target inventory and has no valid promotion mapping."
                    .to_string(),
            )
        })
        .collect()
}

fn datasource_reference_checks(
    source_bundle: &Map<String, Value>,
    target_inventory: &Map<String, Value>,
    uid_mapping: &Map<String, Value>,
    name_mapping: &Map<String, Value>,
) -> Vec<PromotionCheck> {
    let target_datasource_uids = target_uids(target_inventory, "datasources");
    let target_datasource_names = target_names(target_inventory, "datasources");
    let mut checks = Vec::new();
    for dashboard in source_bundle
        .get("dashboards")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(object) = dashboard.as_object() else {
            continue;
        };
        let dashboard_uid = normalize_text(object.get("uid"));
        let body = object.get("body").and_then(Value::as_object);
        for datasource_uid in body
            .and_then(|body| body.get("datasourceUids"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let source_uid = normalize_text(Some(datasource_uid));
            if let Some(check) = classify_mapping_check(
                "datasource-uid-remap",
                dashboard_uid.clone(),
                source_uid.clone(),
                mapped_target(uid_mapping, &source_uid),
                "datasources.uids",
                &target_datasource_uids,
                "Datasource UID is missing from the target inventory and has no valid promotion mapping."
                    .to_string(),
            ) {
                checks.push(check);
            }
        }
        for datasource_name in body
            .and_then(|body| body.get("datasourceNames"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let source_name = normalize_text(Some(datasource_name));
            if let Some(check) = classify_mapping_check(
                "datasource-name-remap",
                dashboard_uid.clone(),
                source_name.clone(),
                mapped_target(name_mapping, &source_name),
                "datasources.names",
                &target_datasource_names,
                "Datasource name is missing from the target inventory and has no valid promotion mapping."
                    .to_string(),
            ) {
                checks.push(check);
            }
        }
    }
    for alert in source_bundle
        .get("alerts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(object) = alert.as_object() else {
            continue;
        };
        let alert_uid = normalize_text(object.get("uid"));
        let body = object.get("body").and_then(Value::as_object);
        for datasource_uid in body
            .and_then(|body| body.get("datasourceUids"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let source_uid = normalize_text(Some(datasource_uid));
            if source_uid.is_empty() || source_uid == "__expr__" || source_uid == "__dashboard__" {
                continue;
            }
            if let Some(check) = classify_mapping_check(
                "alert-datasource-uid-remap",
                alert_uid.clone(),
                source_uid.clone(),
                mapped_target(uid_mapping, &source_uid),
                "datasources.uids",
                &target_datasource_uids,
                "Alert datasource UID is missing from the target inventory and has no valid promotion mapping."
                    .to_string(),
            ) {
                checks.push(check);
            }
        }
        for datasource_name in body
            .and_then(|body| body.get("datasourceNames"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let source_name = normalize_text(Some(datasource_name));
            if let Some(check) = classify_mapping_check(
                "alert-datasource-name-remap",
                alert_uid.clone(),
                source_name.clone(),
                mapped_target(name_mapping, &source_name),
                "datasources.names",
                &target_datasource_names,
                "Alert datasource name is missing from the target inventory and has no valid promotion mapping."
                    .to_string(),
            ) {
                checks.push(check);
            }
        }
    }
    checks
}

pub fn build_sync_promotion_preflight_document(
    source_bundle: &Value,
    target_inventory: &Value,
    availability: Option<&Value>,
    mapping: Option<&Value>,
) -> Result<Value> {
    let source_bundle = require_json_object(source_bundle, "Sync source bundle input")?;
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

    Ok(serde_json::json!({
        "kind": SYNC_PROMOTION_PREFLIGHT_KIND,
        "schemaVersion": SYNC_PROMOTION_PREFLIGHT_SCHEMA_VERSION,
        "toolVersion": tool_version(),
        "summary": SyncPromotionPreflightSummary {
            resource_count,
            direct_match_count,
            mapped_count,
            missing_mapping_count,
            bundle_blocking_count,
            blocking_count,
        },
        "bundlePreflight": bundle_preflight,
        "mappingSummary": {
            "mappingKind": mapping_document.get("kind").cloned().unwrap_or(Value::Null),
            "mappingSchemaVersion": mapping_document.get("schemaVersion").cloned().unwrap_or(Value::Null),
            "sourceEnvironment": mapping_document.get("sourceEnvironment").cloned().unwrap_or(Value::Null),
            "targetEnvironment": mapping_document.get("targetEnvironment").cloned().unwrap_or(Value::Null),
            "folderMappingCount": folder_mapping.len(),
            "datasourceUidMappingCount": datasource_uid_mapping.len(),
            "datasourceNameMappingCount": datasource_name_mapping.len(),
        },
        "checkSummary": serde_json::to_value(check_summary)?,
        "handoffSummary": serde_json::to_value(summarize_promotion_handoff(blocking_count))?,
        "continuationSummary": serde_json::to_value(continuation_summary)?,
        "checks": checks.iter().map(|item| serde_json::json!({
            "kind": item.kind,
            "identity": item.identity,
            "sourceValue": item.source_value,
            "targetValue": item.target_value,
            "resolution": item.resolution,
            "mappingSource": item.mapping_source,
            "status": item.status,
            "detail": item.detail,
            "blocking": item.blocking,
        })).collect::<Vec<_>>(),
        "resolvedChecks": checks
            .iter()
            .filter(|item| !item.blocking)
            .map(|item| serde_json::json!({
                "kind": item.kind,
                "identity": item.identity,
                "sourceValue": item.source_value,
                "targetValue": item.target_value,
                "resolution": item.resolution,
                "mappingSource": item.mapping_source,
                "status": item.status,
                "detail": item.detail,
                "blocking": item.blocking,
            }))
            .collect::<Vec<_>>(),
        "blockingChecks": checks
            .iter()
            .filter(|item| item.blocking)
            .map(|item| serde_json::json!({
                "kind": item.kind,
                "identity": item.identity,
                "sourceValue": item.source_value,
                "targetValue": item.target_value,
                "resolution": item.resolution,
                "mappingSource": item.mapping_source,
                "status": item.status,
                "detail": item.detail,
                "blocking": item.blocking,
            }))
            .collect::<Vec<_>>(),
    }))
}

pub fn render_sync_promotion_preflight_text(document: &Value) -> Result<Vec<String>> {
    if document.get("kind").and_then(Value::as_str) != Some(SYNC_PROMOTION_PREFLIGHT_KIND) {
        return Err(message(
            "Sync promotion preflight document kind is not supported.",
        ));
    }
    let summary = SyncPromotionPreflightSummary::from_document(document)?;
    let mapping_summary = require_json_object_field(
        require_json_object(document, "Sync promotion preflight document")?,
        "mappingSummary",
        "Sync promotion preflight document",
    )?;
    let check_summary = require_json_object_field(
        require_json_object(document, "Sync promotion preflight document")?,
        "checkSummary",
        "Sync promotion preflight document",
    )?;
    let handoff_summary = require_json_object_field(
        require_json_object(document, "Sync promotion preflight document")?,
        "handoffSummary",
        "Sync promotion preflight document",
    )?;
    let continuation_summary = require_json_object_field(
        require_json_object(document, "Sync promotion preflight document")?,
        "continuationSummary",
        "Sync promotion preflight document",
    )?;
    let bundle_preflight = document
        .get("bundlePreflight")
        .ok_or_else(|| message("Sync promotion preflight document is missing bundlePreflight."))?;
    let (resolved_checks, blocking_checks) = render_promotion_check_arrays(document)?;
    let resolved_remap_count = check_summary
        .get("resolvedCount")
        .and_then(Value::as_i64)
        .unwrap_or(resolved_checks.len() as i64);
    let blocking_remap_count = check_summary
        .get("missingTargetCount")
        .and_then(Value::as_i64)
        .unwrap_or(blocking_checks.len() as i64);
    let mut lines = vec![
        "Sync promotion preflight".to_string(),
        format!(
            "Summary: resources={} direct={} mapped={} missing-mappings={} bundle-blocking={} blocking={}",
            summary.resource_count,
            summary.direct_match_count,
            summary.mapped_count,
            summary.missing_mapping_count,
            summary.bundle_blocking_count,
            summary.blocking_count,
        ),
        format!(
            "Mappings: kind={} schema={} source-env={} target-env={} folders={} datasource-uids={} datasource-names={}",
            normalize_text(mapping_summary.get("mappingKind")),
            normalize_text(mapping_summary.get("mappingSchemaVersion")),
            normalize_text(mapping_summary.get("sourceEnvironment")),
            normalize_text(mapping_summary.get("targetEnvironment")),
            mapping_summary
                .get("folderMappingCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            mapping_summary
                .get("datasourceUidMappingCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            mapping_summary
                .get("datasourceNameMappingCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
        ),
        format!(
            "Check buckets: folder-remaps={} datasource-uid-remaps={} datasource-name-remaps={} resolved-remaps={} blocking-remaps={} direct={} mapped={}",
            check_summary
                .get("folderRemapCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            check_summary
                .get("datasourceUidRemapCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            check_summary
                .get("datasourceNameRemapCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            resolved_remap_count,
            blocking_remap_count,
            check_summary
                .get("directCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            check_summary
                .get("mappedCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
        ),
        "Reason: promotion stays blocked until blocking checks are cleared; resolved remaps stay in the review handoff for traceability.".to_string(),
        format!(
            "Handoff: review-required={} ready-for-review={} next-stage={} blocking={} instruction={}",
            handoff_summary
                .get("reviewRequired")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            handoff_summary
                .get("readyForReview")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            normalize_text(handoff_summary.get("nextStage")),
            handoff_summary
                .get("blockingCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            normalize_text(handoff_summary.get("reviewInstruction")),
        ),
        String::new(),
        "# Controlled apply continuation".to_string(),
        format!(
            "- staged-only={} live-mutation-allowed={} ready-for-continuation={} next-stage={} resolved={} blocking={} instruction={}",
            continuation_summary
                .get("stagedOnly")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            continuation_summary
                .get("liveMutationAllowed")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            continuation_summary
                .get("readyForContinuation")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            normalize_text(continuation_summary.get("nextStage")),
            continuation_summary
                .get("resolvedCount")
                .and_then(Value::as_i64)
                .unwrap_or(resolved_remap_count),
            continuation_summary
                .get("blockingCount")
                .and_then(Value::as_i64)
                .unwrap_or(blocking_remap_count),
            normalize_text(continuation_summary.get("continuationInstruction")),
        ),
        String::new(),
        "# Resolved remaps".to_string(),
    ];
    if resolved_checks.is_empty() {
        lines.push("- none status=ok detail=No resolved remaps to review.".to_string());
    } else {
        for check in resolved_checks {
            if let Some(object) = check.as_object() {
                lines.push(format!(
                    "- {} identity={} source={} target={} resolution={} mapping-source={} status={} detail={}",
                    normalize_text(object.get("kind")),
                    normalize_text(object.get("identity")),
                    normalize_text(object.get("sourceValue")),
                    normalize_text(object.get("targetValue")),
                    normalize_text(object.get("resolution")),
                    normalize_text(object.get("mappingSource")),
                    normalize_text(object.get("status")),
                    normalize_text(object.get("detail")),
                ));
            }
        }
    }
    lines.push(String::new());
    lines.push("# Blocking remaps".to_string());
    if blocking_checks.is_empty() {
        lines.push("- none status=ok detail=No blocking remaps remain.".to_string());
    } else {
        for check in blocking_checks {
            if let Some(object) = check.as_object() {
                lines.push(format!(
                    "- {} identity={} source={} target={} resolution={} mapping-source={} status={} detail={}",
                    normalize_text(object.get("kind")),
                    normalize_text(object.get("identity")),
                    normalize_text(object.get("sourceValue")),
                    normalize_text(object.get("targetValue")),
                    normalize_text(object.get("resolution")),
                    normalize_text(object.get("mappingSource")),
                    normalize_text(object.get("status")),
                    normalize_text(object.get("detail")),
                ));
            }
        }
    }
    lines.push(String::new());
    lines.extend(render_sync_bundle_preflight_text(bundle_preflight)?);
    Ok(lines)
}
