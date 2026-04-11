//! Sync staged document review, apply, and preflight gating helpers.
//!
//! Maintainer note:
//! - This module owns the staged contract checks between plan, review, and
//!   apply intent generation.
//! - Preflight inputs are bridged into compact summaries here so apply intents
//!   carry only the gate outcome needed for audit and execution decisions.
//! - Audit fields such as `reviewedBy` and `appliedAt` are attached here,
//!   while lineage sequencing stays owned by `staged_documents_lineage.rs`.
#![cfg_attr(not(any(feature = "tui", test)), allow(dead_code))]

use crate::common::{message, Result};
use crate::sync::DEFAULT_REVIEW_TOKEN;
use serde_json::{Map, Value};

use super::super::blocked_reasons::{
    collect_blocking_reasons, format_blocking_rejection_message, BlockingReasonSource,
};
use super::super::bundle_preflight::SYNC_BUNDLE_PREFLIGHT_KIND;
use super::super::bundle_preflight::{
    alert_artifact_assessment_summary_or_default, require_sync_bundle_preflight_summary,
};
use super::super::json::require_json_object;
use super::super::preflight::{require_sync_preflight_summary, SYNC_PREFLIGHT_KIND};
use super::{deterministic_stage_marker, normalize_optional_text, require_trace_id};

const PREFLIGHT_REASON_SOURCES: &[BlockingReasonSource] = &[BlockingReasonSource {
    label: "",
    path: &["checks"],
}];

const BUNDLE_PREFLIGHT_REASON_SOURCES: &[BlockingReasonSource] = &[
    BlockingReasonSource {
        label: "syncPreflight",
        path: &["syncPreflight", "checks"],
    },
    BlockingReasonSource {
        label: "providerAssessment",
        path: &["providerAssessment", "checks"],
    },
    BlockingReasonSource {
        label: "secretPlaceholderAssessment",
        path: &["secretPlaceholderAssessment", "checks"],
    },
    BlockingReasonSource {
        label: "alertArtifactAssessment",
        path: &["alertArtifactAssessment", "checks"],
    },
];

pub(crate) fn mark_plan_reviewed(document: &Value, review_token: &str) -> Result<Value> {
    let mut object = require_json_object(document, "Sync plan document")?.clone();
    if object.get("kind").and_then(Value::as_str) != Some("grafana-utils-sync-plan") {
        return Err(message("Sync plan document kind is not supported."));
    }
    if review_token.trim() != DEFAULT_REVIEW_TOKEN {
        return Err(message("Sync plan review token rejected."));
    }
    // Review is a staged state transition, not a plan rebuild. Preserve the
    // existing document and stamp only the minimum review gate fields here.
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
        SYNC_PREFLIGHT_KIND => {
            let summary = require_sync_preflight_summary(document)?;
            let check_count = summary.check_count;
            let ok_count = summary.ok_count;
            let blocking_count = summary.blocking_count;
            // Apply intent only needs the gating summary, not the full
            // preflight check list. Keep this bridge intentionally narrow so
            // apply-document shape stays stable if preflight detail evolves.
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
                "Workspace package-test document is not supported via --input-test-file; use --package-test-file.",
            ))
        }
        _ => return Err(message("Sync preflight document kind is not supported.")),
    };
    if blocking > 0 {
        let reasons = collect_blocking_reasons(document, PREFLIGHT_REASON_SOURCES);
        return Err(message(format_blocking_rejection_message(
            "preflight",
            blocking,
            &reasons,
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
    // Bundle preflight already aggregates several staged assessments. Re-bridge
    // that result into one apply-facing summary instead of embedding the full
    // source documents into the apply intent.
    let blocking_count = summary.sync_blocking_count
        + summary.provider_blocking_count
        + summary.secret_placeholder_blocking_count
        + alert_artifact_summary.blocked_count;
    if blocking_count > 0 {
        let reasons = collect_blocking_reasons(document, BUNDLE_PREFLIGHT_REASON_SOURCES);
        return Err(message(format_blocking_rejection_message(
            "bundle preflight",
            blocking_count,
            &reasons,
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
        "secretPlaceholderBlockingCount": summary.secret_placeholder_blocking_count,
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
    // Audit ownership lives here: actor, timestamp, and freeform note describe
    // who approved the staged transition, while lineage markers are attached by
    // the separate lineage helper.
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
    // Keep apply audit metadata distinct from live execution results. This
    // staged document records approval context; Grafana responses belong in the
    // later live-apply audit payload.
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
