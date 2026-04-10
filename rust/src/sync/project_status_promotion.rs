//! Shared promotion domain-status producer.
//!
//! Maintainer note:
//! - This module derives one promotion-owned domain-status row from the staged
//!   promotion preflight document.
//! - Keep the producer document-driven and conservative; missing mappings,
//!   bundle blocking, and staged blocking should remain explicit signals.

use serde_json::Value;

use crate::project_status::{
    status_finding, ProjectDomainStatus, PROJECT_STATUS_BLOCKED, PROJECT_STATUS_PARTIAL,
    PROJECT_STATUS_READY,
};

const PROMOTION_DOMAIN_ID: &str = "promotion";
const PROMOTION_SCOPE: &str = "staged";
const PROMOTION_MODE: &str = "artifact-summary";
const PROMOTION_REASON_READY: &str = PROJECT_STATUS_READY;
const PROMOTION_REASON_PARTIAL_NO_DATA: &str = "partial-no-data";
const PROMOTION_REASON_BLOCKED_BY_BLOCKERS: &str = "blocked-by-blockers";

const PROMOTION_SOURCE_KINDS: &[&str] = &["promotion-preflight"];
const PROMOTION_SIGNAL_KEYS: &[&str] = &[
    "summary.resourceCount",
    "summary.missingMappingCount",
    "summary.bundleBlockingCount",
    "summary.blockingCount",
];
const PROMOTION_HANDOFF_SIGNAL_KEYS: &[&str] = &[
    "handoffSummary.reviewRequired",
    "handoffSummary.readyForReview",
    "handoffSummary.nextStage",
    "handoffSummary.blockingCount",
    "handoffSummary.reviewInstruction",
];
const PROMOTION_CONTINUATION_SIGNAL_KEYS: &[&str] = &[
    "continuationSummary.stagedOnly",
    "continuationSummary.liveMutationAllowed",
    "continuationSummary.readyForContinuation",
    "continuationSummary.nextStage",
    "continuationSummary.blockingCount",
    "continuationSummary.resolvedCount",
    "continuationSummary.continuationInstruction",
];
const PROMOTION_CHECK_SUMMARY_SIGNAL_KEYS: &[&str] = &[
    "checkSummary.folderRemapCount",
    "checkSummary.datasourceUidRemapCount",
    "checkSummary.datasourceNameRemapCount",
];

const PROMOTION_BLOCKER_MISSING_MAPPINGS: &str = "missing-mappings";
const PROMOTION_BLOCKER_BUNDLE_BLOCKING: &str = "bundle-blocking";
const PROMOTION_BLOCKER_BLOCKING: &str = "blocking";
const PROMOTION_WARNING_REVIEW_HANDOFF: &str = "review-handoff";
const PROMOTION_WARNING_APPLY_CONTINUATION: &str = "apply-continuation";
const PROMOTION_WARNING_FOLDER_REMAPS: &str = "folder-remaps";
const PROMOTION_WARNING_DATASOURCE_UID_REMAPS: &str = "datasource-uid-remaps";
const PROMOTION_WARNING_DATASOURCE_NAME_REMAPS: &str = "datasource-name-remaps";

const PROMOTION_RESOLVE_BLOCKERS_ACTIONS: &[&str] =
    &["resolve promotion blockers in the fixed order: missing-mapping, bundle-blocking, blocking"];
const PROMOTION_STAGE_AT_LEAST_ONE_ACTIONS: &[&str] =
    &["stage at least one promotable resource before promotion"];
const PROMOTION_REVIEW_READY_ACTIONS: &[&str] = &["promotion handoff is review-ready"];
const PROMOTION_REVIEW_HANDOFF_ACTIONS: &[&str] =
    &["resolve the staged promotion handoff before review"];
const PROMOTION_APPLY_READY_ACTIONS: &[&str] =
    &["promotion is apply-ready in the staged continuation"];
const PROMOTION_APPLY_CONTINUATION_ACTIONS: &[&str] =
    &["keep the promotion staged until the apply continuation is ready"];
const PROMOTION_REVIEW_FOLDER_REMAPS_ACTIONS: &[&str] =
    &["review folder remaps before promotion review"];
const PROMOTION_REVIEW_DATASOURCE_REMAPS_ACTIONS: &[&str] =
    &["review datasource remaps before promotion review"];

fn summary_number(document: &Value, key: &str) -> usize {
    document
        .get("summary")
        .and_then(|value| value.get(key))
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize
}

fn nested_summary_object<'a>(document: Option<&'a Value>, key: &str) -> Option<&'a Value> {
    document
        .and_then(Value::as_object)
        .and_then(|object| object.get(key))
}

fn summary_bool(document: Option<&Value>, section: &str, key: &str) -> bool {
    nested_summary_object(document, section)
        .and_then(|value| value.get(key))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn summary_text(document: Option<&Value>, section: &str, key: &str) -> Option<String> {
    nested_summary_object(document, section)
        .and_then(|value| value.get(key))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn nested_number(document: Option<&Value>, section: &str, key: &str) -> usize {
    nested_summary_object(document, section)
        .and_then(|value| value.get(key))
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize
}

fn push_unique(next_actions: &mut Vec<String>, action: &str) {
    if !next_actions.iter().any(|item| item == action) {
        next_actions.push(action.to_string());
    }
}

fn handoff_warning_source(document: Option<&Value>) -> &'static str {
    if summary_bool(document, "handoffSummary", "readyForReview")
        && summary_text(document, "handoffSummary", "reviewInstruction").is_none()
    {
        "handoffSummary.nextStage"
    } else if summary_bool(document, "handoffSummary", "readyForReview") {
        "handoffSummary.readyForReview"
    } else if summary_text(document, "handoffSummary", "reviewInstruction").is_some() {
        "handoffSummary.reviewInstruction"
    } else {
        "handoffSummary.reviewRequired"
    }
}

fn continuation_warning_source(document: Option<&Value>) -> &'static str {
    if summary_bool(document, "continuationSummary", "readyForContinuation")
        && nested_number(document, "continuationSummary", "resolvedCount") > 0
    {
        "continuationSummary.resolvedCount"
    } else if summary_bool(document, "continuationSummary", "readyForContinuation")
        && summary_text(document, "continuationSummary", "continuationInstruction").is_none()
    {
        "continuationSummary.nextStage"
    } else if summary_bool(document, "continuationSummary", "readyForContinuation") {
        "continuationSummary.readyForContinuation"
    } else if summary_text(document, "continuationSummary", "continuationInstruction").is_some() {
        "continuationSummary.continuationInstruction"
    } else {
        "continuationSummary.liveMutationAllowed"
    }
}

fn continuation_warning_count(document: Option<&Value>) -> usize {
    let resolved = nested_number(document, "continuationSummary", "resolvedCount");
    if continuation_warning_source(document) == "continuationSummary.resolvedCount" {
        resolved.max(1)
    } else {
        1
    }
}

pub(crate) fn build_promotion_domain_status(
    promotion_preflight_document: Option<&Value>,
) -> Option<ProjectDomainStatus> {
    let document = promotion_preflight_document?;
    let resources = summary_number(document, "resourceCount");
    let missing_mappings = summary_number(document, "missingMappingCount");
    let bundle_blocking = summary_number(document, "bundleBlockingCount");
    let summary_blocking = summary_number(document, "blockingCount");
    let handoff_blocking = nested_number(Some(document), "handoffSummary", "blockingCount");
    let continuation_blocking =
        nested_number(Some(document), "continuationSummary", "blockingCount");
    let blocking = summary_blocking
        .max(handoff_blocking)
        .max(continuation_blocking);
    let blocking_source = if summary_blocking > 0 {
        "summary.blockingCount"
    } else if handoff_blocking > 0 {
        "handoffSummary.blockingCount"
    } else {
        "continuationSummary.blockingCount"
    };

    let mut blockers = Vec::new();
    if missing_mappings > 0 {
        blockers.push(status_finding(
            PROMOTION_BLOCKER_MISSING_MAPPINGS,
            missing_mappings,
            "summary.missingMappingCount",
        ));
    }
    if bundle_blocking > 0 {
        blockers.push(status_finding(
            PROMOTION_BLOCKER_BUNDLE_BLOCKING,
            bundle_blocking,
            "summary.bundleBlockingCount",
        ));
    }
    if blockers.is_empty() && blocking > 0 {
        blockers.push(status_finding(
            PROMOTION_BLOCKER_BLOCKING,
            blocking,
            blocking_source,
        ));
    }

    let mut signal_keys = PROMOTION_SIGNAL_KEYS
        .iter()
        .map(|item| (*item).to_string())
        .collect::<Vec<_>>();
    if nested_summary_object(Some(document), "handoffSummary").is_some() {
        signal_keys.extend(
            PROMOTION_HANDOFF_SIGNAL_KEYS
                .iter()
                .map(|item| (*item).to_string()),
        );
    }
    if nested_summary_object(Some(document), "continuationSummary").is_some() {
        signal_keys.extend(
            PROMOTION_CONTINUATION_SIGNAL_KEYS
                .iter()
                .map(|item| (*item).to_string()),
        );
    }
    if nested_summary_object(Some(document), "checkSummary").is_some() {
        signal_keys.extend(
            PROMOTION_CHECK_SUMMARY_SIGNAL_KEYS
                .iter()
                .map(|item| (*item).to_string()),
        );
    }

    let has_blockers = !blockers.is_empty();
    let is_partial = resources == 0;
    let (status, reason_code) = if has_blockers {
        (PROJECT_STATUS_BLOCKED, PROMOTION_REASON_BLOCKED_BY_BLOCKERS)
    } else if is_partial {
        (PROJECT_STATUS_PARTIAL, PROMOTION_REASON_PARTIAL_NO_DATA)
    } else {
        (PROJECT_STATUS_READY, PROMOTION_REASON_READY)
    };
    let mut warnings = Vec::new();
    let folder_remaps = nested_number(Some(document), "checkSummary", "folderRemapCount");
    let datasource_uid_remaps =
        nested_number(Some(document), "checkSummary", "datasourceUidRemapCount");
    let datasource_name_remaps =
        nested_number(Some(document), "checkSummary", "datasourceNameRemapCount");
    if folder_remaps > 0 {
        warnings.push(status_finding(
            PROMOTION_WARNING_FOLDER_REMAPS,
            folder_remaps,
            "checkSummary.folderRemapCount",
        ));
    }
    if datasource_uid_remaps > 0 {
        warnings.push(status_finding(
            PROMOTION_WARNING_DATASOURCE_UID_REMAPS,
            datasource_uid_remaps,
            "checkSummary.datasourceUidRemapCount",
        ));
    }
    if datasource_name_remaps > 0 {
        warnings.push(status_finding(
            PROMOTION_WARNING_DATASOURCE_NAME_REMAPS,
            datasource_name_remaps,
            "checkSummary.datasourceNameRemapCount",
        ));
    }
    let next_actions = if has_blockers {
        PROMOTION_RESOLVE_BLOCKERS_ACTIONS
            .iter()
            .map(|item| (*item).to_string())
            .collect::<Vec<_>>()
    } else if is_partial {
        PROMOTION_STAGE_AT_LEAST_ONE_ACTIONS
            .iter()
            .map(|item| (*item).to_string())
            .collect::<Vec<_>>()
    } else {
        let mut actions = Vec::new();
        if nested_summary_object(Some(document), "handoffSummary").is_some() {
            warnings.push(status_finding(
                PROMOTION_WARNING_REVIEW_HANDOFF,
                1,
                handoff_warning_source(Some(document)),
            ));
            let action = if summary_bool(Some(document), "handoffSummary", "readyForReview") {
                PROMOTION_REVIEW_READY_ACTIONS[0].to_string()
            } else {
                summary_text(Some(document), "handoffSummary", "reviewInstruction")
                    .unwrap_or_else(|| PROMOTION_REVIEW_HANDOFF_ACTIONS[0].to_string())
            };
            push_unique(&mut actions, &action);
        }
        if folder_remaps > 0 {
            push_unique(&mut actions, PROMOTION_REVIEW_FOLDER_REMAPS_ACTIONS[0]);
        }
        if datasource_uid_remaps > 0 || datasource_name_remaps > 0 {
            push_unique(&mut actions, PROMOTION_REVIEW_DATASOURCE_REMAPS_ACTIONS[0]);
        }
        if nested_summary_object(Some(document), "continuationSummary").is_some() {
            warnings.push(status_finding(
                PROMOTION_WARNING_APPLY_CONTINUATION,
                continuation_warning_count(Some(document)),
                continuation_warning_source(Some(document)),
            ));
            let action = if summary_bool(
                Some(document),
                "continuationSummary",
                "readyForContinuation",
            ) {
                PROMOTION_APPLY_READY_ACTIONS[0].to_string()
            } else {
                summary_text(
                    Some(document),
                    "continuationSummary",
                    "continuationInstruction",
                )
                .unwrap_or_else(|| PROMOTION_APPLY_CONTINUATION_ACTIONS[0].to_string())
            };
            push_unique(&mut actions, &action);
        }
        actions
    };

    Some(ProjectDomainStatus {
        id: PROMOTION_DOMAIN_ID.to_string(),
        scope: PROMOTION_SCOPE.to_string(),
        mode: PROMOTION_MODE.to_string(),
        status: status.to_string(),
        reason_code: reason_code.to_string(),
        primary_count: resources,
        blocker_count: blockers.iter().map(|item| item.count).sum(),
        warning_count: warnings.iter().map(|item| item.count).sum(),
        source_kinds: PROMOTION_SOURCE_KINDS
            .iter()
            .map(|item| (*item).to_string())
            .collect(),
        signal_keys,
        blockers,
        warnings,
        next_actions,
        freshness: Default::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::build_promotion_domain_status;
    use serde_json::json;

    #[test]
    fn build_promotion_domain_status_reports_blockers_and_explicit_signals() {
        let document = json!({
            "kind": "grafana-utils-sync-promotion-preflight",
            "summary": {
                "resourceCount": 3,
                "missingMappingCount": 2,
                "bundleBlockingCount": 5,
                "blockingCount": 7,
            }
        });

        let status = build_promotion_domain_status(Some(&document)).unwrap();

        assert_eq!(status.id, "promotion");
        assert_eq!(status.scope, "staged");
        assert_eq!(status.mode, "artifact-summary");
        assert_eq!(status.status, "blocked");
        assert_eq!(status.reason_code, "blocked-by-blockers");
        assert_eq!(status.primary_count, 3);
        assert_eq!(status.blocker_count, 7);
        assert_eq!(status.source_kinds, vec!["promotion-preflight".to_string()]);
        assert_eq!(
            status.signal_keys,
            vec![
                "summary.resourceCount".to_string(),
                "summary.missingMappingCount".to_string(),
                "summary.bundleBlockingCount".to_string(),
                "summary.blockingCount".to_string(),
            ]
        );
        assert_eq!(status.blockers.len(), 2);
        assert_eq!(status.blockers[0].kind, "missing-mappings");
        assert_eq!(status.blockers[0].count, 2);
        assert_eq!(status.blockers[0].source, "summary.missingMappingCount");
        assert_eq!(status.blockers[1].kind, "bundle-blocking");
        assert_eq!(status.blockers[1].count, 5);
        assert_eq!(status.blockers[1].source, "summary.bundleBlockingCount");
        assert!(status.next_actions.contains(&"resolve promotion blockers in the fixed order: missing-mapping, bundle-blocking, blocking".to_string()));
    }

    #[test]
    fn build_promotion_domain_status_reports_partial_when_no_resources() {
        let document = json!({
            "kind": "grafana-utils-sync-promotion-preflight",
            "summary": {
                "resourceCount": 0,
                "missingMappingCount": 0,
                "bundleBlockingCount": 0,
                "blockingCount": 0,
            }
        });

        let status = build_promotion_domain_status(Some(&document)).unwrap();

        assert_eq!(status.status, "partial");
        assert_eq!(status.reason_code, "partial-no-data");
        assert_eq!(status.primary_count, 0);
        assert_eq!(status.blocker_count, 0);
        assert_eq!(
            status.next_actions,
            vec!["stage at least one promotable resource before promotion".to_string()]
        );
    }

    #[test]
    fn build_promotion_domain_status_reports_review_and_apply_evidence_when_ready() {
        let document = json!({
            "kind": "grafana-utils-sync-promotion-preflight",
            "summary": {
                "resourceCount": 3,
                "missingMappingCount": 0,
                "bundleBlockingCount": 0,
                "blockingCount": 0,
            },
            "handoffSummary": {
                "reviewRequired": true,
                "readyForReview": true,
                "nextStage": "review",
                "blockingCount": 0,
                "reviewInstruction": "promotion handoff is ready to move into review",
            },
            "continuationSummary": {
                "stagedOnly": true,
                "liveMutationAllowed": false,
                "readyForContinuation": true,
                "nextStage": "staged-apply-continuation",
                "blockingCount": 0,
                "continuationInstruction": "reviewed remaps can continue into a staged apply continuation without enabling live mutation",
            }
        });

        let status = build_promotion_domain_status(Some(&document)).unwrap();

        assert_eq!(status.id, "promotion");
        assert_eq!(status.scope, "staged");
        assert_eq!(status.mode, "artifact-summary");
        assert_eq!(status.status, "ready");
        assert_eq!(status.reason_code, "ready");
        assert_eq!(status.primary_count, 3);
        assert_eq!(status.blocker_count, 0);
        assert_eq!(status.warning_count, 2);
        assert_eq!(status.source_kinds, vec!["promotion-preflight".to_string()]);
        assert_eq!(
            status.signal_keys,
            vec![
                "summary.resourceCount".to_string(),
                "summary.missingMappingCount".to_string(),
                "summary.bundleBlockingCount".to_string(),
                "summary.blockingCount".to_string(),
                "handoffSummary.reviewRequired".to_string(),
                "handoffSummary.readyForReview".to_string(),
                "handoffSummary.nextStage".to_string(),
                "handoffSummary.blockingCount".to_string(),
                "handoffSummary.reviewInstruction".to_string(),
                "continuationSummary.stagedOnly".to_string(),
                "continuationSummary.liveMutationAllowed".to_string(),
                "continuationSummary.readyForContinuation".to_string(),
                "continuationSummary.nextStage".to_string(),
                "continuationSummary.blockingCount".to_string(),
                "continuationSummary.resolvedCount".to_string(),
                "continuationSummary.continuationInstruction".to_string(),
            ]
        );
        assert_eq!(status.blockers.len(), 0);
        assert_eq!(status.warnings.len(), 2);
        assert_eq!(status.warnings[0].kind, "review-handoff");
        assert_eq!(status.warnings[0].count, 1);
        assert_eq!(status.warnings[0].source, "handoffSummary.readyForReview");
        assert_eq!(status.warnings[1].kind, "apply-continuation");
        assert_eq!(status.warnings[1].count, 1);
        assert_eq!(
            status.warnings[1].source,
            "continuationSummary.readyForContinuation"
        );
        assert_eq!(
            status.next_actions,
            vec![
                "promotion handoff is review-ready".to_string(),
                "promotion is apply-ready in the staged continuation".to_string(),
            ]
        );
    }

    #[test]
    fn build_promotion_domain_status_prefers_resolved_apply_evidence_when_continuation_reports_it()
    {
        let document = json!({
            "kind": "grafana-utils-sync-promotion-preflight",
            "summary": {
                "resourceCount": 3,
                "missingMappingCount": 0,
                "bundleBlockingCount": 0,
                "blockingCount": 0,
            },
            "handoffSummary": {
                "reviewRequired": true,
                "readyForReview": true,
                "nextStage": "review",
                "blockingCount": 0,
                "reviewInstruction": "promotion handoff is ready to move into review",
            },
            "continuationSummary": {
                "stagedOnly": true,
                "liveMutationAllowed": false,
                "readyForContinuation": true,
                "nextStage": "staged-apply-continuation",
                "resolvedCount": 4,
                "blockingCount": 0,
                "continuationInstruction": "reviewed remaps can continue into a staged apply continuation without enabling live mutation",
            }
        });

        let status = build_promotion_domain_status(Some(&document)).unwrap();

        assert_eq!(status.status, "ready");
        assert_eq!(status.reason_code, "ready");
        assert_eq!(status.warning_count, 5);
        assert_eq!(status.warnings[1].kind, "apply-continuation");
        assert_eq!(status.warnings[1].count, 4);
        assert_eq!(
            status.warnings[1].source,
            "continuationSummary.resolvedCount"
        );
        assert_eq!(
            status.signal_keys,
            vec![
                "summary.resourceCount".to_string(),
                "summary.missingMappingCount".to_string(),
                "summary.bundleBlockingCount".to_string(),
                "summary.blockingCount".to_string(),
                "handoffSummary.reviewRequired".to_string(),
                "handoffSummary.readyForReview".to_string(),
                "handoffSummary.nextStage".to_string(),
                "handoffSummary.blockingCount".to_string(),
                "handoffSummary.reviewInstruction".to_string(),
                "continuationSummary.stagedOnly".to_string(),
                "continuationSummary.liveMutationAllowed".to_string(),
                "continuationSummary.readyForContinuation".to_string(),
                "continuationSummary.nextStage".to_string(),
                "continuationSummary.blockingCount".to_string(),
                "continuationSummary.resolvedCount".to_string(),
                "continuationSummary.continuationInstruction".to_string(),
            ]
        );
        assert_eq!(
            status.next_actions,
            vec![
                "promotion handoff is review-ready".to_string(),
                "promotion is apply-ready in the staged continuation".to_string(),
            ]
        );
    }

    #[test]
    fn build_promotion_domain_status_uses_next_stage_evidence_when_instructions_are_missing() {
        let document = json!({
            "kind": "grafana-utils-sync-promotion-preflight",
            "summary": {
                "resourceCount": 3,
                "missingMappingCount": 0,
                "bundleBlockingCount": 0,
                "blockingCount": 0,
            },
            "handoffSummary": {
                "reviewRequired": true,
                "readyForReview": true,
                "nextStage": "review",
                "blockingCount": 0,
            },
            "continuationSummary": {
                "stagedOnly": true,
                "liveMutationAllowed": false,
                "readyForContinuation": true,
                "nextStage": "staged-apply-continuation",
                "blockingCount": 0,
            }
        });

        let status = build_promotion_domain_status(Some(&document)).unwrap();

        assert_eq!(status.status, "ready");
        assert_eq!(status.reason_code, "ready");
        assert_eq!(status.warning_count, 2);
        assert_eq!(status.warnings[0].source, "handoffSummary.nextStage");
        assert_eq!(status.warnings[1].source, "continuationSummary.nextStage");
        assert_eq!(
            status.next_actions,
            vec![
                "promotion handoff is review-ready".to_string(),
                "promotion is apply-ready in the staged continuation".to_string(),
            ]
        );
    }

    #[test]
    fn build_promotion_domain_status_reports_instruction_sources_when_not_ready() {
        let document = json!({
            "kind": "grafana-utils-sync-promotion-preflight",
            "summary": {
                "resourceCount": 3,
                "missingMappingCount": 0,
                "bundleBlockingCount": 0,
                "blockingCount": 0,
            },
            "handoffSummary": {
                "reviewRequired": true,
                "readyForReview": false,
                "nextStage": "review",
                "blockingCount": 0,
                "reviewInstruction": "promotion handoff is blocked until the staged review step is ready",
            },
            "continuationSummary": {
                "stagedOnly": true,
                "liveMutationAllowed": false,
                "readyForContinuation": false,
                "nextStage": "staged-apply-continuation",
                "blockingCount": 0,
                "continuationInstruction": "keep the promotion staged until the apply continuation is ready",
            }
        });

        let status = build_promotion_domain_status(Some(&document)).unwrap();

        assert_eq!(status.status, "ready");
        assert_eq!(status.reason_code, "ready");
        assert_eq!(status.warning_count, 2);
        assert_eq!(
            status.warnings[0].source,
            "handoffSummary.reviewInstruction"
        );
        assert_eq!(
            status.warnings[1].source,
            "continuationSummary.continuationInstruction"
        );
        assert_eq!(
            status.next_actions,
            vec![
                "promotion handoff is blocked until the staged review step is ready".to_string(),
                "keep the promotion staged until the apply continuation is ready".to_string(),
            ]
        );
    }

    #[test]
    fn build_promotion_domain_status_uses_nested_blocking_evidence_when_summary_is_clear() {
        let document = json!({
            "kind": "grafana-utils-sync-promotion-preflight",
            "summary": {
                "resourceCount": 3,
                "missingMappingCount": 0,
                "bundleBlockingCount": 0,
                "blockingCount": 0,
            },
            "handoffSummary": {
                "reviewRequired": true,
                "readyForReview": false,
                "nextStage": "review",
                "blockingCount": 0,
                "reviewInstruction": "promotion handoff is blocked until the staged review step is ready",
            },
            "continuationSummary": {
                "stagedOnly": true,
                "liveMutationAllowed": false,
                "readyForContinuation": false,
                "nextStage": "staged-apply-continuation",
                "blockingCount": 2,
                "continuationInstruction": "keep the promotion staged until the apply continuation is ready",
            }
        });

        let status = build_promotion_domain_status(Some(&document)).unwrap();

        assert_eq!(status.status, "blocked");
        assert_eq!(status.reason_code, "blocked-by-blockers");
        assert_eq!(status.blocker_count, 2);
        assert_eq!(status.blockers.len(), 1);
        assert_eq!(status.blockers[0].kind, "blocking");
        assert_eq!(status.blockers[0].count, 2);
        assert_eq!(
            status.blockers[0].source,
            "continuationSummary.blockingCount"
        );
        assert_eq!(status.warning_count, 0);
        assert!(status.warnings.is_empty());
        assert_eq!(
            status.next_actions,
            vec!["resolve promotion blockers in the fixed order: missing-mapping, bundle-blocking, blocking".to_string()]
        );
    }

    #[test]
    fn build_promotion_domain_status_surfaces_remap_complexity_signals_from_check_summary() {
        let document = json!({
            "kind": "grafana-utils-sync-promotion-preflight",
            "summary": {
                "resourceCount": 4,
                "missingMappingCount": 0,
                "bundleBlockingCount": 0,
                "blockingCount": 0,
            },
            "checkSummary": {
                "folderRemapCount": 1,
                "datasourceUidRemapCount": 2,
                "datasourceNameRemapCount": 3,
            },
            "handoffSummary": {
                "reviewRequired": true,
                "readyForReview": true,
                "nextStage": "review",
                "blockingCount": 0,
                "reviewInstruction": "promotion handoff is ready to move into review",
            },
            "continuationSummary": {
                "stagedOnly": true,
                "liveMutationAllowed": false,
                "readyForContinuation": true,
                "nextStage": "staged-apply-continuation",
                "blockingCount": 0,
                "continuationInstruction": "reviewed remaps can continue into a staged apply continuation without enabling live mutation",
            }
        });

        let status = build_promotion_domain_status(Some(&document)).unwrap();

        assert_eq!(status.status, "ready");
        assert_eq!(status.reason_code, "ready");
        assert_eq!(status.blocker_count, 0);
        assert_eq!(status.warning_count, 8);
        assert!(status.signal_keys.contains(&"checkSummary.folderRemapCount".to_string()));
        assert!(status
            .signal_keys
            .contains(&"checkSummary.datasourceUidRemapCount".to_string()));
        assert!(status
            .signal_keys
            .contains(&"checkSummary.datasourceNameRemapCount".to_string()));
        assert!(status.warnings.iter().any(|warning| {
            warning.kind == "folder-remaps"
                && warning.count == 1
                && warning.source == "checkSummary.folderRemapCount"
        }));
        assert!(status.warnings.iter().any(|warning| {
            warning.kind == "datasource-uid-remaps"
                && warning.count == 2
                && warning.source == "checkSummary.datasourceUidRemapCount"
        }));
        assert!(status.warnings.iter().any(|warning| {
            warning.kind == "datasource-name-remaps"
                && warning.count == 3
                && warning.source == "checkSummary.datasourceNameRemapCount"
        }));
        assert_eq!(
            status.next_actions,
            vec![
                "promotion handoff is review-ready".to_string(),
                "review folder remaps before promotion review".to_string(),
                "review datasource remaps before promotion review".to_string(),
                "promotion is apply-ready in the staged continuation".to_string(),
            ]
        );
    }
}
