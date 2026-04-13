//! Promotion-preflight text rendering helpers.

use super::super::bundle_preflight::render_sync_bundle_preflight_text;
use super::super::json::{require_json_object, require_json_object_field};
use super::promotion_preflight_checks::{
    normalize_text, render_promotion_check_arrays, SyncPromotionPreflightSummary,
};
use crate::common::{message, Result};
use serde_json::Value;

fn render_promotion_check_section(
    lines: &mut Vec<String>,
    heading: &str,
    checks: &[&Value],
    empty_line: &str,
) {
    lines.push(heading.to_string());
    if checks.is_empty() {
        lines.push(empty_line.to_string());
        return;
    }
    for check in checks {
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

pub(crate) fn render_sync_promotion_preflight_text(document: &Value) -> Result<Vec<String>> {
    if document.get("kind").and_then(Value::as_str)
        != Some(super::promotion_preflight_checks::SYNC_PROMOTION_PREFLIGHT_KIND)
    {
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
    ];
    render_promotion_check_section(
        &mut lines,
        "# Resolved remaps",
        &resolved_checks,
        "- none status=ok detail=No resolved remaps to review.",
    );
    lines.push(String::new());
    render_promotion_check_section(
        &mut lines,
        "# Blocking remaps",
        &blocking_checks,
        "- none status=ok detail=No blocking remaps remain.",
    );
    lines.push(String::new());
    lines.extend(render_sync_bundle_preflight_text(bundle_preflight)?);
    Ok(lines)
}
