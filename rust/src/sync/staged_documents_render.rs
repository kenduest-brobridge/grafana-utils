//! Sync staged document rendering and audit helpers.
#![cfg_attr(not(any(feature = "tui", test)), allow(dead_code))]

use crate::alert_sync::ALERT_SYNC_KIND;
use crate::common::{message, Result};
use crate::sync::DEFAULT_REVIEW_TOKEN;
use serde_json::{Map, Value};

use super::super::bundle_preflight::SYNC_BUNDLE_PREFLIGHT_KIND;
use super::super::bundle_preflight::{
    alert_artifact_assessment_summary_or_default, require_sync_bundle_preflight_summary,
};
use super::super::json::require_json_object;
use super::super::live::load_apply_intent_operations;
use super::super::preflight::require_sync_preflight_summary;
use super::{deterministic_stage_marker, normalize_optional_text, require_trace_id};

#[cfg(feature = "tui")]
use std::cmp::Ordering;

pub fn render_sync_summary_text(document: &Value) -> Result<Vec<String>> {
    if document.get("kind").and_then(Value::as_str) != Some("grafana-utils-sync-summary") {
        return Err(message("Sync summary document kind is not supported."));
    }
    let summary = document
        .get("summary")
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| message("Sync summary document is missing summary."))?;
    Ok(vec![
        "Sync summary".to_string(),
        format!(
            "Resources: {} total, {} dashboards, {} datasources, {} folders, {} alerts",
            summary
                .get("resourceCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("dashboardCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("datasourceCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("folderCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("alertCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
        ),
    ])
}

pub fn render_alert_sync_assessment_text(document: &Value) -> Result<Vec<String>> {
    if document.get("kind").and_then(Value::as_str) != Some(ALERT_SYNC_KIND) {
        return Err(message(
            "Alert sync assessment document kind is not supported.",
        ));
    }
    let summary = document
        .get("summary")
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| message("Alert sync assessment document is missing summary."))?;
    let mut lines = vec![
        "Alert sync assessment".to_string(),
        format!(
            "Alerts: {} total, {} candidate, {} plan-only, {} blocked",
            summary
                .get("alertCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("candidateCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("planOnlyCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("blockedCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
        ),
        String::new(),
        "# Alerts".to_string(),
    ];
    if let Some(items) = document.get("alerts").and_then(Value::as_array) {
        for item in items {
            if let Some(object) = item.as_object() {
                lines.push(format!(
                    "- {} status={} liveApplyAllowed={} detail={}",
                    object
                        .get("identity")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown"),
                    object
                        .get("status")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown"),
                    if object
                        .get("liveApplyAllowed")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                    {
                        "true"
                    } else {
                        "false"
                    },
                    object.get("detail").and_then(Value::as_str).unwrap_or(""),
                ));
            }
        }
    }
    Ok(lines)
}

pub fn render_sync_plan_text(document: &Value) -> Result<Vec<String>> {
    if document.get("kind").and_then(Value::as_str) != Some("grafana-utils-sync-plan") {
        return Err(message("Sync plan document kind is not supported."));
    }
    let summary = document
        .get("summary")
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| message("Sync plan document is missing summary."))?;
    let mut lines = vec![
        "Sync plan".to_string(),
        format!(
            "Trace: {}",
            document
                .get("traceId")
                .and_then(Value::as_str)
                .unwrap_or("missing")
        ),
        format!(
            "Lineage: stage={} step={} parent={}",
            document
                .get("stage")
                .and_then(Value::as_str)
                .unwrap_or("missing"),
            document
                .get("stepIndex")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            document
                .get("parentTraceId")
                .and_then(Value::as_str)
                .unwrap_or("none")
        ),
        format!(
            "Summary: create={} update={} delete={} noop={} unmanaged={}",
            summary
                .get("would_create")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("would_update")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("would_delete")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary.get("noop").and_then(Value::as_i64).unwrap_or(0),
            summary
                .get("unmanaged")
                .and_then(Value::as_i64)
                .unwrap_or(0),
        ),
        format!(
            "Alerts: candidate={} plan-only={} blocked={}",
            summary
                .get("alert_candidate")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("alert_plan_only")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("alert_blocked")
                .and_then(Value::as_i64)
                .unwrap_or(0),
        ),
        format!(
            "Review: required={} reviewed={}",
            document
                .get("reviewRequired")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            document
                .get("reviewed")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        ),
    ];
    if let Some(reviewed_by) = document.get("reviewedBy").and_then(Value::as_str) {
        lines.push(format!("Reviewed by: {reviewed_by}"));
    }
    if let Some(reviewed_at) = document.get("reviewedAt").and_then(Value::as_str) {
        lines.push(format!("Reviewed at: {reviewed_at}"));
    }
    if let Some(review_note) = document.get("reviewNote").and_then(Value::as_str) {
        lines.push(format!("Review note: {review_note}"));
    }
    Ok(lines)
}

pub fn render_sync_apply_intent_text(document: &Value) -> Result<Vec<String>> {
    if document.get("kind").and_then(Value::as_str) != Some("grafana-utils-sync-apply-intent") {
        return Err(message("Sync apply intent document kind is not supported."));
    }
    let summary = document
        .get("summary")
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| message("Sync apply intent document is missing summary."))?;
    let operations = load_apply_intent_operations(document)?;
    let mut lines = vec![
        "Sync apply intent".to_string(),
        format!(
            "Trace: {}",
            document
                .get("traceId")
                .and_then(Value::as_str)
                .unwrap_or("missing")
        ),
        format!(
            "Lineage: stage={} step={} parent={}",
            document
                .get("stage")
                .and_then(Value::as_str)
                .unwrap_or("missing"),
            document
                .get("stepIndex")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            document
                .get("parentTraceId")
                .and_then(Value::as_str)
                .unwrap_or("none")
        ),
        format!(
            "Summary: create={} update={} delete={} executable={}",
            summary
                .get("would_create")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("would_update")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("would_delete")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            operations.len(),
        ),
        format!(
            "Review: required={} reviewed={} approved={}",
            document
                .get("reviewRequired")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            document
                .get("reviewed")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            document
                .get("approved")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        ),
    ];
    if let Some(preflight_summary) = document.get("preflightSummary").and_then(Value::as_object) {
        lines.push(format!(
            "Preflight: kind={} checks={} ok={} blocking={}",
            preflight_summary
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
            preflight_summary
                .get("checkCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            preflight_summary
                .get("okCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            preflight_summary
                .get("blockingCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
        ));
    }
    if let Some(bundle_summary) = document
        .get("bundlePreflightSummary")
        .and_then(Value::as_object)
    {
        lines.push(format!(
            "Bundle preflight: resources={} sync-blocking={} provider-blocking={} alert-artifacts={} plan-only={} blocking={}",
            bundle_summary
                .get("resourceCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            bundle_summary
                .get("syncBlockingCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            bundle_summary
                .get("providerBlockingCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            bundle_summary
                .get("alertArtifactCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            bundle_summary
                .get("alertArtifactPlanOnlyCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            bundle_summary
                .get("alertArtifactBlockingCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
        ));
    }
    if let Some(applied_by) = document.get("appliedBy").and_then(Value::as_str) {
        lines.push(format!("Applied by: {applied_by}"));
    }
    if let Some(applied_at) = document.get("appliedAt").and_then(Value::as_str) {
        lines.push(format!("Applied at: {applied_at}"));
    }
    if let Some(approval_reason) = document.get("approvalReason").and_then(Value::as_str) {
        lines.push(format!("Approval reason: {approval_reason}"));
    }
    if let Some(apply_note) = document.get("applyNote").and_then(Value::as_str) {
        lines.push(format!("Apply note: {apply_note}"));
    }
    Ok(lines)
}

pub(crate) fn mark_plan_reviewed(document: &Value, review_token: &str) -> Result<Value> {
    let mut object = require_json_object(document, "Sync plan document")?.clone();
    if object.get("kind").and_then(Value::as_str) != Some("grafana-utils-sync-plan") {
        return Err(message("Sync plan document kind is not supported."));
    }
    if review_token.trim() != DEFAULT_REVIEW_TOKEN {
        return Err(message("Sync plan review token rejected."));
    }
    let trace_id = require_trace_id(document, "Sync plan document")?;
    object.insert("reviewed".to_string(), Value::Bool(true));
    object.insert("traceId".to_string(), Value::String(trace_id));
    Ok(Value::Object(object))
}

pub(crate) fn validate_apply_preflight(document: &Value) -> Result<Value> {
    let object = require_json_object(document, "Sync preflight document")?;
    let kind = object
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| message("Sync preflight document is missing kind."))?;
    let mut bridged = Map::new();
    let blocking = match kind {
        super::super::preflight::SYNC_PREFLIGHT_KIND => {
            let summary = require_sync_preflight_summary(document)?;
            let check_count = summary.check_count;
            let ok_count = summary.ok_count;
            let blocking_count = summary.blocking_count;
            bridged.insert("kind".to_string(), Value::String(kind.to_string()));
            bridged.insert("checkCount".to_string(), Value::Number(check_count.into()));
            bridged.insert("okCount".to_string(), Value::Number(ok_count.into()));
            bridged.insert(
                "blockingCount".to_string(),
                Value::Number(blocking_count.into()),
            );
            blocking_count
        }
        SYNC_BUNDLE_PREFLIGHT_KIND => {
            return Err(message(
                "Sync bundle preflight document is not supported via --preflight-file; use --bundle-preflight-file.",
            ))
        }
        _ => return Err(message("Sync preflight document kind is not supported.")),
    };
    if blocking > 0 {
        return Err(message(format!(
            "Refusing local sync apply intent because preflight reports {blocking} blocking checks."
        )));
    }
    Ok(Value::Object(bridged))
}

pub(crate) fn validate_apply_bundle_preflight(document: &Value) -> Result<Value> {
    let object = require_json_object(document, "Sync bundle preflight document")?;
    if object.get("kind").and_then(Value::as_str) != Some(SYNC_BUNDLE_PREFLIGHT_KIND) {
        return Err(message(
            "Sync bundle preflight document kind is not supported.",
        ));
    }
    let summary = require_sync_bundle_preflight_summary(document)?;
    let alert_artifact_summary = object
        .get("alertArtifactAssessment")
        .map(alert_artifact_assessment_summary_or_default)
        .unwrap_or_default();
    let blocking_count = summary.sync_blocking_count
        + summary.provider_blocking_count
        + alert_artifact_summary.blocked_count;
    if blocking_count > 0 {
        return Err(message(format!(
            "Refusing local sync apply intent because bundle preflight reports {blocking_count} blocking checks."
        )));
    }
    Ok(serde_json::json!({
        "kind": SYNC_BUNDLE_PREFLIGHT_KIND,
        "resourceCount": summary.resource_count,
        "checkCount": summary.resource_count,
        "okCount": (summary.resource_count - blocking_count).max(0),
        "blockingCount": blocking_count,
        "syncBlockingCount": summary.sync_blocking_count,
        "providerBlockingCount": summary.provider_blocking_count,
        "alertArtifactBlockingCount": alert_artifact_summary.blocked_count,
        "alertArtifactPlanOnlyCount": alert_artifact_summary.plan_only_count,
        "alertArtifactCount": alert_artifact_summary.resource_count,
    }))
}

pub(crate) fn attach_preflight_summary(
    intent: &Value,
    preflight_summary: Option<Value>,
) -> Result<Value> {
    let mut object = require_json_object(intent, "Sync apply intent document")?.clone();
    if let Some(summary) = preflight_summary {
        object.insert("preflightSummary".to_string(), summary);
    }
    Ok(Value::Object(object))
}

pub(crate) fn attach_bundle_preflight_summary(
    intent: &Value,
    bundle_preflight_summary: Option<Value>,
) -> Result<Value> {
    let mut object = require_json_object(intent, "Sync apply intent document")?.clone();
    if let Some(summary) = bundle_preflight_summary {
        object.insert("bundlePreflightSummary".to_string(), summary);
    }
    Ok(Value::Object(object))
}

pub(crate) fn attach_review_audit(
    document: &Value,
    trace_id: &str,
    reviewed_by: Option<&str>,
    reviewed_at: Option<&str>,
    review_note: Option<&str>,
) -> Result<Value> {
    let mut object = require_json_object(document, "Sync reviewed plan document")?.clone();
    if let Some(actor) = normalize_optional_text(reviewed_by) {
        object.insert("reviewedBy".to_string(), Value::String(actor));
    }
    object.insert(
        "reviewedAt".to_string(),
        Value::String(
            normalize_optional_text(reviewed_at)
                .unwrap_or_else(|| deterministic_stage_marker(trace_id, "reviewed")),
        ),
    );
    if let Some(note) = normalize_optional_text(review_note) {
        object.insert("reviewNote".to_string(), Value::String(note));
    }
    Ok(Value::Object(object))
}

pub(crate) fn attach_apply_audit(
    document: &Value,
    trace_id: &str,
    applied_by: Option<&str>,
    applied_at: Option<&str>,
    approval_reason: Option<&str>,
    apply_note: Option<&str>,
) -> Result<Value> {
    let mut object = require_json_object(document, "Sync apply intent document")?.clone();
    if let Some(actor) = normalize_optional_text(applied_by) {
        object.insert("appliedBy".to_string(), Value::String(actor));
    }
    object.insert(
        "appliedAt".to_string(),
        Value::String(
            normalize_optional_text(applied_at)
                .unwrap_or_else(|| deterministic_stage_marker(trace_id, "applied")),
        ),
    );
    if let Some(reason) = normalize_optional_text(approval_reason) {
        object.insert("approvalReason".to_string(), Value::String(reason));
    }
    if let Some(note) = normalize_optional_text(apply_note) {
        object.insert("applyNote".to_string(), Value::String(note));
    }
    Ok(Value::Object(object))
}

#[cfg(feature = "tui")]
fn sync_audit_field<'a>(row: &'a Value, key: &str) -> &'a str {
    row.get(key).and_then(Value::as_str).unwrap_or("")
}

#[cfg(feature = "tui")]
fn sync_audit_display<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.is_empty() {
        fallback
    } else {
        value
    }
}

#[cfg(feature = "tui")]
fn sync_audit_status_rank(status: &str) -> u8 {
    match status {
        "missing-live" => 0,
        "missing-lock" => 1,
        "drift-detected" => 2,
        _ => 3,
    }
}

#[cfg(feature = "tui")]
pub(crate) fn sync_audit_drift_cmp(left: &Value, right: &Value) -> Ordering {
    sync_audit_status_rank(sync_audit_field(left, "status"))
        .cmp(&sync_audit_status_rank(sync_audit_field(right, "status")))
        .then_with(|| sync_audit_field(left, "kind").cmp(sync_audit_field(right, "kind")))
        .then_with(|| sync_audit_field(left, "identity").cmp(sync_audit_field(right, "identity")))
        .then_with(|| sync_audit_field(left, "title").cmp(sync_audit_field(right, "title")))
        .then_with(|| {
            sync_audit_field(left, "sourcePath").cmp(sync_audit_field(right, "sourcePath"))
        })
}

#[cfg(feature = "tui")]
pub(crate) fn sync_audit_drift_title(drift: &Value) -> String {
    format!(
        "{} {}",
        sync_audit_display(sync_audit_field(drift, "kind"), "unknown"),
        sync_audit_display(sync_audit_field(drift, "identity"), "unknown"),
    )
}

#[cfg(feature = "tui")]
pub(crate) fn sync_audit_drift_meta(drift: &Value) -> String {
    let baseline_status = sync_audit_display(sync_audit_field(drift, "baselineStatus"), "unknown");
    let current_status = sync_audit_display(sync_audit_field(drift, "currentStatus"), "unknown");
    format!(
        "{} | base={} cur={}",
        sync_audit_display(sync_audit_field(drift, "status"), "unknown"),
        baseline_status,
        current_status
    )
}

#[cfg(feature = "tui")]
pub(crate) fn sync_audit_drift_details(drift: &Value) -> Vec<String> {
    let mut details = vec![
        format!(
            "Triage: {}",
            sync_audit_display(sync_audit_field(drift, "status"), "(unknown)")
        ),
        format!(
            "Baseline/current: {} -> {}",
            sync_audit_display(sync_audit_field(drift, "baselineStatus"), "(unknown)"),
            sync_audit_display(sync_audit_field(drift, "currentStatus"), "(unknown)")
        ),
        format!(
            "Source: {}",
            sync_audit_display(sync_audit_field(drift, "sourcePath"), "(not set)")
        ),
    ];

    let drifted_fields = drift
        .get("driftedFields")
        .and_then(Value::as_array)
        .map(|values| values.iter().filter_map(Value::as_str).collect::<Vec<_>>())
        .unwrap_or_default();
    details.push(format!(
        "Fields: {}",
        if drifted_fields.is_empty() {
            "none".to_string()
        } else {
            drifted_fields.join(", ")
        }
    ));
    let baseline_checksum = drift
        .get("baselineChecksum")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .unwrap_or("(none)");
    let current_checksum = drift
        .get("currentChecksum")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .unwrap_or("(none)");
    if baseline_checksum != "(none)" || current_checksum != "(none)" {
        details.push(format!(
            "Checksums: baseline={} current={}",
            baseline_checksum, current_checksum
        ));
    }
    details
}
