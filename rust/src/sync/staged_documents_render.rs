//! Sync staged document rendering and drift display helpers.
#![cfg_attr(not(any(feature = "tui", test)), allow(dead_code))]

use crate::alert_sync::ALERT_SYNC_KIND;
use crate::common::{message, Result};
use serde_json::Value;

use super::super::live::load_apply_intent_operations;

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
        "Plan-only alert items stay review-only until this plan is approved and applied."
            .to_string(),
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
    if let Some(ordering_mode) = document
        .get("ordering")
        .and_then(Value::as_object)
        .and_then(|ordering| ordering.get("mode"))
        .and_then(Value::as_str)
    {
        lines.insert(5, format!("Ordering: {ordering_mode}"));
    }
    if let Some(reviewed_by) = document.get("reviewedBy").and_then(Value::as_str) {
        lines.push(format!("Reviewed by: {reviewed_by}"));
    }
    if let Some(reviewed_at) = document.get("reviewedAt").and_then(Value::as_str) {
        lines.push(format!("Reviewed at: {reviewed_at}"));
    }
    if let Some(review_note) = document.get("reviewNote").and_then(Value::as_str) {
        lines.push(format!("Review note: {review_note}"));
    }
    if let Some(reasons) = summary.get("blocked_reasons").and_then(Value::as_array) {
        for reason in reasons.iter().filter_map(Value::as_str) {
            lines.push(format!("Blocked reason: {reason}"));
        }
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
            "Bundle preflight: resources={} sync-blocking={} provider-blocking={} secret-placeholder-blocking={} alert-artifacts={} plan-only={} blocking={}",
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
                .get("secretPlaceholderBlockingCount")
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
        lines.push(
            "Reason: preflight and bundle-preflight blocking must be 0 before apply; missing provider or secret placeholder availability blocks apply; plan-only alert artifacts stay staged."
                .to_string(),
        );
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
