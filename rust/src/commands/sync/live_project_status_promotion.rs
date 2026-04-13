//! Richer live promotion domain-status producer.
//!
//! Maintainer note:
//! - This module is intentionally more informative than the old transport-only
//!   stub, but it still stays conservative about readiness.
//! - It only promotes to `ready` when the staged promotion summary, mapping,
//!   and availability inputs all supply explicit evidence.
//! - When the staged promotion summary already carries handoff and
//!   apply-continuation summaries, those are surfaced as warnings and next
//!   actions instead of being inferred from transport state.

#![allow(dead_code)]

use serde_json::Value;

use crate::project_status::{
    status_finding, ProjectDomainStatus, ProjectStatusFinding, PROJECT_STATUS_BLOCKED,
    PROJECT_STATUS_PARTIAL, PROJECT_STATUS_READY,
};

const LIVE_PROMOTION_DOMAIN_ID: &str = "promotion";
const LIVE_PROMOTION_SCOPE: &str = "live";
const LIVE_PROMOTION_MODE: &str = "live-promotion-surfaces";
const LIVE_PROMOTION_REASON_READY: &str = PROJECT_STATUS_READY;
const LIVE_PROMOTION_REASON_PARTIAL_NO_DATA: &str = "partial-no-data";
const LIVE_PROMOTION_REASON_BLOCKED_BY_BLOCKERS: &str = "blocked-by-blockers";

const LIVE_PROMOTION_SOURCE_KINDS: &[&str] = &[
    "live-promotion-summary",
    "live-promotion-mapping",
    "live-promotion-availability",
];

const LIVE_PROMOTION_SIGNAL_KEYS: &[&str] = &[
    "summary.resourceCount",
    "summary.missingMappingCount",
    "summary.bundleBlockingCount",
    "summary.blockingCount",
    "mapping.entryCount",
    "availability.entryCount",
    "handoffSummary.reviewRequired",
    "handoffSummary.readyForReview",
    "handoffSummary.nextStage",
    "handoffSummary.blockingCount",
    "handoffSummary.reviewInstruction",
    "continuationSummary.stagedOnly",
    "continuationSummary.liveMutationAllowed",
    "continuationSummary.readyForContinuation",
    "continuationSummary.nextStage",
    "continuationSummary.blockingCount",
    "continuationSummary.continuationInstruction",
];

const LIVE_PROMOTION_BLOCKER_MISSING_MAPPINGS: &str = "missing-mappings";
const LIVE_PROMOTION_BLOCKER_BUNDLE_BLOCKING: &str = "bundle-blocking";
const LIVE_PROMOTION_BLOCKER_BLOCKING: &str = "blocking";
const LIVE_PROMOTION_WARNING_REVIEW_HANDOFF: &str = "review-handoff";
const LIVE_PROMOTION_WARNING_APPLY_CONTINUATION: &str = "apply-continuation";

const LIVE_PROMOTION_RESOLVE_BLOCKERS_ACTIONS: &[&str] =
    &["resolve promotion blockers in the fixed order: missing-mapping, bundle-blocking, blocking"];
const LIVE_PROMOTION_STAGE_AT_LEAST_ONE_ACTIONS: &[&str] =
    &["stage at least one promotable resource before promotion"];
const LIVE_PROMOTION_PROVIDE_SUMMARY_ACTIONS: &[&str] =
    &["provide a staged promotion summary before interpreting live promotion readiness"];
const LIVE_PROMOTION_PROVIDE_MAPPING_ACTIONS: &[&str] =
    &["provide explicit promotion mappings before promotion"];
const LIVE_PROMOTION_PROVIDE_AVAILABILITY_ACTIONS: &[&str] =
    &["provide live availability hints before promotion"];
const LIVE_PROMOTION_REVIEW_HANOFF_ACTIONS: &[&str] =
    &["resolve the staged promotion handoff before review"];
const LIVE_PROMOTION_REVIEW_READY_ACTIONS: &[&str] = &["promotion handoff is review-ready"];
const LIVE_PROMOTION_APPLY_CONTINUATION_ACTIONS: &[&str] =
    &["keep the promotion staged until the apply continuation is ready"];
const LIVE_PROMOTION_APPLY_READY_ACTIONS: &[&str] =
    &["promotion is apply-ready in the staged continuation"];

#[derive(Debug, Clone, Default)]
pub(crate) struct LivePromotionProjectStatusInputs<'a> {
    pub promotion_summary_document: Option<&'a Value>,
    pub promotion_mapping_document: Option<&'a Value>,
    pub availability_document: Option<&'a Value>,
}

fn summary_number(document: &Value, key: &str) -> usize {
    document
        .get("summary")
        .and_then(|value| value.get(key))
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize
}

fn array_count(document: Option<&Value>) -> usize {
    document
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0)
}

fn push_unique(next_actions: &mut Vec<String>, action: &str) {
    if !next_actions.iter().any(|item| item == action) {
        next_actions.push(action.to_string());
    }
}

fn mapping_entry_count(document: Option<&Value>) -> usize {
    let Some(object) = document.and_then(Value::as_object) else {
        return 0;
    };

    let folders = object
        .get("folders")
        .and_then(Value::as_object)
        .map(|value| value.len())
        .unwrap_or(0);
    let datasource_uid_mappings = object
        .get("datasources")
        .and_then(Value::as_object)
        .and_then(|value| value.get("uids"))
        .and_then(Value::as_object)
        .map(|value| value.len())
        .unwrap_or(0);
    let datasource_name_mappings = object
        .get("datasources")
        .and_then(Value::as_object)
        .and_then(|value| value.get("names"))
        .and_then(Value::as_object)
        .map(|value| value.len())
        .unwrap_or(0);

    folders + datasource_uid_mappings + datasource_name_mappings
}

fn availability_entry_count(document: Option<&Value>) -> usize {
    let Some(object) = document.and_then(Value::as_object) else {
        return 0;
    };

    [
        "pluginIds",
        "datasourceUids",
        "datasourceNames",
        "contactPoints",
        "providerNames",
        "secretPlaceholderNames",
    ]
    .into_iter()
    .map(|key| array_count(object.get(key)))
    .sum()
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

fn add_handoff_evidence(
    document: Option<&Value>,
    warnings: &mut Vec<ProjectStatusFinding>,
    next_actions: &mut Vec<String>,
) {
    if nested_summary_object(document, "handoffSummary").is_none() {
        return;
    }

    warnings.push(status_finding(
        LIVE_PROMOTION_WARNING_REVIEW_HANDOFF,
        1,
        "handoffSummary.reviewRequired",
    ));

    if summary_bool(document, "handoffSummary", "readyForReview") {
        for action in LIVE_PROMOTION_REVIEW_READY_ACTIONS {
            push_unique(next_actions, action);
        }
    } else {
        let action = summary_text(document, "handoffSummary", "reviewInstruction")
            .unwrap_or_else(|| LIVE_PROMOTION_REVIEW_HANOFF_ACTIONS[0].to_string());
        push_unique(next_actions, &action);
    }
}

fn add_continuation_evidence(
    document: Option<&Value>,
    warnings: &mut Vec<ProjectStatusFinding>,
    next_actions: &mut Vec<String>,
) {
    if nested_summary_object(document, "continuationSummary").is_none() {
        return;
    }

    warnings.push(status_finding(
        LIVE_PROMOTION_WARNING_APPLY_CONTINUATION,
        1,
        "continuationSummary.liveMutationAllowed",
    ));

    if summary_bool(document, "continuationSummary", "readyForContinuation") {
        for action in LIVE_PROMOTION_APPLY_READY_ACTIONS {
            push_unique(next_actions, action);
        }
    } else {
        let action = summary_text(document, "continuationSummary", "continuationInstruction")
            .unwrap_or_else(|| LIVE_PROMOTION_APPLY_CONTINUATION_ACTIONS[0].to_string());
        push_unique(next_actions, &action);
    }
}

fn build_next_actions(
    summary_present: bool,
    mapping_present: bool,
    availability_present: bool,
    resource_count: usize,
    mapping_count: usize,
    availability_count: usize,
) -> Vec<String> {
    let mut next_actions = Vec::new();

    if !summary_present {
        for action in LIVE_PROMOTION_PROVIDE_SUMMARY_ACTIONS {
            push_unique(&mut next_actions, action);
        }
    }
    if resource_count == 0 {
        for action in LIVE_PROMOTION_STAGE_AT_LEAST_ONE_ACTIONS {
            push_unique(&mut next_actions, action);
        }
    }
    if !mapping_present || mapping_count == 0 {
        for action in LIVE_PROMOTION_PROVIDE_MAPPING_ACTIONS {
            push_unique(&mut next_actions, action);
        }
    }
    if !availability_present || availability_count == 0 {
        for action in LIVE_PROMOTION_PROVIDE_AVAILABILITY_ACTIONS {
            push_unique(&mut next_actions, action);
        }
    }

    next_actions
}

pub(crate) fn build_live_promotion_project_status(
    inputs: LivePromotionProjectStatusInputs<'_>,
) -> Option<ProjectDomainStatus> {
    if inputs.promotion_summary_document.is_none()
        && inputs.promotion_mapping_document.is_none()
        && inputs.availability_document.is_none()
    {
        return None;
    }

    let resource_count = inputs
        .promotion_summary_document
        .map(|document| summary_number(document, "resourceCount"))
        .unwrap_or(0);
    let summary_present = inputs.promotion_summary_document.is_some();
    let missing_mapping_count = inputs
        .promotion_summary_document
        .map(|document| summary_number(document, "missingMappingCount"))
        .unwrap_or(0);
    let bundle_blocking_count = inputs
        .promotion_summary_document
        .map(|document| summary_number(document, "bundleBlockingCount"))
        .unwrap_or(0);
    let blocking_count = inputs
        .promotion_summary_document
        .map(|document| summary_number(document, "blockingCount"))
        .unwrap_or(0);
    let mapping_count = mapping_entry_count(inputs.promotion_mapping_document);
    let availability_count = availability_entry_count(inputs.availability_document);

    let mut source_kinds = Vec::new();
    if inputs.promotion_summary_document.is_some() {
        source_kinds.push(LIVE_PROMOTION_SOURCE_KINDS[0].to_string());
    }
    if inputs.promotion_mapping_document.is_some() {
        source_kinds.push(LIVE_PROMOTION_SOURCE_KINDS[1].to_string());
    }
    if inputs.availability_document.is_some() {
        source_kinds.push(LIVE_PROMOTION_SOURCE_KINDS[2].to_string());
    }

    let mut blockers = Vec::new();
    if missing_mapping_count > 0 {
        blockers.push(status_finding(
            LIVE_PROMOTION_BLOCKER_MISSING_MAPPINGS,
            missing_mapping_count,
            "summary.missingMappingCount",
        ));
    }
    if bundle_blocking_count > 0 {
        blockers.push(status_finding(
            LIVE_PROMOTION_BLOCKER_BUNDLE_BLOCKING,
            bundle_blocking_count,
            "summary.bundleBlockingCount",
        ));
    }
    if blockers.is_empty() && blocking_count > 0 {
        blockers.push(status_finding(
            LIVE_PROMOTION_BLOCKER_BLOCKING,
            blocking_count,
            "summary.blockingCount",
        ));
    }

    let mut warnings = Vec::new();
    let mut evidence_actions = Vec::new();
    add_handoff_evidence(
        inputs.promotion_summary_document,
        &mut warnings,
        &mut evidence_actions,
    );
    add_continuation_evidence(
        inputs.promotion_summary_document,
        &mut warnings,
        &mut evidence_actions,
    );

    let (status, reason_code, next_actions) = if !blockers.is_empty() {
        (
            PROJECT_STATUS_BLOCKED,
            LIVE_PROMOTION_REASON_BLOCKED_BY_BLOCKERS,
            LIVE_PROMOTION_RESOLVE_BLOCKERS_ACTIONS
                .iter()
                .map(|item| (*item).to_string())
                .collect::<Vec<String>>(),
        )
    } else if !summary_present
        || resource_count == 0
        || mapping_count == 0
        || availability_count == 0
    {
        let mut next_actions = build_next_actions(
            summary_present,
            inputs.promotion_mapping_document.is_some(),
            inputs.availability_document.is_some(),
            resource_count,
            mapping_count,
            availability_count,
        );
        next_actions.extend(evidence_actions);
        (
            PROJECT_STATUS_PARTIAL,
            LIVE_PROMOTION_REASON_PARTIAL_NO_DATA,
            next_actions,
        )
    } else {
        (
            PROJECT_STATUS_READY,
            LIVE_PROMOTION_REASON_READY,
            evidence_actions,
        )
    };

    Some(ProjectDomainStatus {
        id: LIVE_PROMOTION_DOMAIN_ID.to_string(),
        scope: LIVE_PROMOTION_SCOPE.to_string(),
        mode: LIVE_PROMOTION_MODE.to_string(),
        status: status.to_string(),
        reason_code: reason_code.to_string(),
        primary_count: resource_count,
        blocker_count: blockers.iter().map(|item| item.count).sum(),
        warning_count: warnings.iter().map(|item| item.count).sum(),
        source_kinds,
        signal_keys: LIVE_PROMOTION_SIGNAL_KEYS
            .iter()
            .map(|item| (*item).to_string())
            .collect(),
        blockers,
        warnings,
        next_actions,
        freshness: Default::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::{build_live_promotion_project_status, LivePromotionProjectStatusInputs};
    use serde_json::json;

    #[test]
    fn build_live_promotion_project_status_returns_none_without_inputs() {
        assert!(
            build_live_promotion_project_status(LivePromotionProjectStatusInputs::default())
                .is_none()
        );
    }

    #[test]
    fn build_live_promotion_project_status_reports_blocked_when_summary_has_blockers() {
        let summary = json!({
            "kind": "grafana-utils-sync-promotion-summary",
            "summary": {
                "resourceCount": 4,
                "missingMappingCount": 2,
                "bundleBlockingCount": 3,
                "blockingCount": 5,
            }
        });
        let mapping = json!({
            "kind": "grafana-utils-sync-promotion-mapping",
            "folders": {"ops-src": "ops-prod"},
            "datasources": {
                "uids": {"prom-src": "prom-prod"},
                "names": {"Prometheus Source": "Prometheus Prod"}
            }
        });
        let availability = json!({
            "pluginIds": ["prometheus"],
            "datasourceUids": ["prom-prod"],
            "contactPoints": ["pagerduty-primary"]
        });

        let status = build_live_promotion_project_status(LivePromotionProjectStatusInputs {
            promotion_summary_document: Some(&summary),
            promotion_mapping_document: Some(&mapping),
            availability_document: Some(&availability),
        })
        .unwrap();

        assert_eq!(status.id, "promotion");
        assert_eq!(status.scope, "live");
        assert_eq!(status.mode, "live-promotion-surfaces");
        assert_eq!(status.status, "blocked");
        assert_eq!(status.reason_code, "blocked-by-blockers");
        assert_eq!(status.primary_count, 4);
        assert_eq!(status.blocker_count, 5);
        assert_eq!(
            status.source_kinds,
            vec![
                "live-promotion-summary".to_string(),
                "live-promotion-mapping".to_string(),
                "live-promotion-availability".to_string(),
            ]
        );
        assert_eq!(
            status.signal_keys,
            vec![
                "summary.resourceCount".to_string(),
                "summary.missingMappingCount".to_string(),
                "summary.bundleBlockingCount".to_string(),
                "summary.blockingCount".to_string(),
                "mapping.entryCount".to_string(),
                "availability.entryCount".to_string(),
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
                "continuationSummary.continuationInstruction".to_string(),
            ]
        );
        assert_eq!(status.blockers.len(), 2);
        assert_eq!(status.blockers[0].kind, "missing-mappings");
        assert_eq!(status.blockers[0].count, 2);
        assert_eq!(status.blockers[0].source, "summary.missingMappingCount");
        assert_eq!(status.blockers[1].kind, "bundle-blocking");
        assert_eq!(status.blockers[1].count, 3);
        assert_eq!(status.blockers[1].source, "summary.bundleBlockingCount");
        assert_eq!(
            status.next_actions,
            vec!["resolve promotion blockers in the fixed order: missing-mapping, bundle-blocking, blocking".to_string()]
        );
    }

    #[test]
    fn build_live_promotion_project_status_reports_partial_when_inputs_are_incomplete() {
        let summary = json!({
            "kind": "grafana-utils-sync-promotion-summary",
            "summary": {
                "resourceCount": 0,
                "missingMappingCount": 0,
                "bundleBlockingCount": 0,
                "blockingCount": 0,
            }
        });
        let mapping = json!({
            "kind": "grafana-utils-sync-promotion-mapping",
            "folders": {},
            "datasources": {
                "uids": {},
                "names": {}
            }
        });

        let status = build_live_promotion_project_status(LivePromotionProjectStatusInputs {
            promotion_summary_document: Some(&summary),
            promotion_mapping_document: Some(&mapping),
            availability_document: None,
        })
        .unwrap();

        assert_eq!(status.status, "partial");
        assert_eq!(status.reason_code, "partial-no-data");
        assert_eq!(status.primary_count, 0);
        assert_eq!(status.blocker_count, 0);
        assert_eq!(
            status.next_actions,
            vec![
                "stage at least one promotable resource before promotion".to_string(),
                "provide explicit promotion mappings before promotion".to_string(),
                "provide live availability hints before promotion".to_string(),
            ]
        );
    }

    #[test]
    fn build_live_promotion_project_status_reports_ready_from_consistent_inputs() {
        let summary = json!({
            "kind": "grafana-utils-sync-promotion-summary",
            "summary": {
                "resourceCount": 3,
                "missingMappingCount": 0,
                "bundleBlockingCount": 0,
                "blockingCount": 0,
            }
        });
        let mapping = json!({
            "kind": "grafana-utils-sync-promotion-mapping",
            "folders": {"ops-src": "ops-prod"},
            "datasources": {
                "uids": {"prom-src": "prom-prod"},
                "names": {"Prometheus Source": "Prometheus Prod"}
            }
        });
        let availability = json!({
            "pluginIds": ["prometheus", "timeseries"],
            "datasourceUids": ["prom-prod"],
            "datasourceNames": ["Prometheus Prod"],
            "contactPoints": ["pagerduty-primary"],
            "providerNames": ["vault"],
            "secretPlaceholderNames": ["prom-basic-auth"]
        });

        let status = build_live_promotion_project_status(LivePromotionProjectStatusInputs {
            promotion_summary_document: Some(&summary),
            promotion_mapping_document: Some(&mapping),
            availability_document: Some(&availability),
        })
        .unwrap();

        assert_eq!(status.status, "ready");
        assert_eq!(status.reason_code, "ready");
        assert_eq!(status.primary_count, 3);
        assert!(status.blockers.is_empty());
        assert!(status.next_actions.is_empty());
    }

    #[test]
    fn build_live_promotion_project_status_reports_review_ready_handoff_and_apply_waiting_continuation(
    ) {
        let summary = json!({
            "kind": "grafana-utils-sync-promotion-summary",
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
                "readyForContinuation": false,
                "nextStage": "resolve-blockers",
                "blockingCount": 0,
                "continuationInstruction": "keep the promotion staged until the apply continuation is ready",
            }
        });
        let mapping = json!({
            "kind": "grafana-utils-sync-promotion-mapping",
            "folders": {"ops-src": "ops-prod"},
            "datasources": {
                "uids": {"prom-src": "prom-prod"},
                "names": {"Prometheus Source": "Prometheus Prod"}
            }
        });
        let availability = json!({
            "pluginIds": ["prometheus", "timeseries"],
            "datasourceUids": ["prom-prod"],
            "datasourceNames": ["Prometheus Prod"],
            "contactPoints": ["pagerduty-primary"],
            "providerNames": ["vault"],
            "secretPlaceholderNames": ["prom-basic-auth"]
        });

        let status = build_live_promotion_project_status(LivePromotionProjectStatusInputs {
            promotion_summary_document: Some(&summary),
            promotion_mapping_document: Some(&mapping),
            availability_document: Some(&availability),
        })
        .unwrap();

        assert_eq!(status.status, "ready");
        assert_eq!(status.reason_code, "ready");
        assert_eq!(status.warning_count, 2);
        assert_eq!(
            status.next_actions,
            vec![
                "promotion handoff is review-ready".to_string(),
                "keep the promotion staged until the apply continuation is ready".to_string(),
            ]
        );
    }

    #[test]
    fn build_live_promotion_project_status_reports_apply_ready_continuation_after_review_ready_handoff(
    ) {
        let summary = json!({
            "kind": "grafana-utils-sync-promotion-summary",
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
                "resolvedCount": 1,
                "blockingCount": 0,
                "continuationInstruction": "reviewed remaps can continue into a staged apply continuation without enabling live mutation",
            }
        });
        let mapping = json!({
            "kind": "grafana-utils-sync-promotion-mapping",
            "folders": {"ops-src": "ops-prod"},
            "datasources": {
                "uids": {"prom-src": "prom-prod"},
                "names": {"Prometheus Source": "Prometheus Prod"}
            }
        });
        let availability = json!({
            "pluginIds": ["prometheus", "timeseries"],
            "datasourceUids": ["prom-prod"],
            "datasourceNames": ["Prometheus Prod"],
            "contactPoints": ["pagerduty-primary"],
            "providerNames": ["vault"],
            "secretPlaceholderNames": ["prom-basic-auth"]
        });

        let status = build_live_promotion_project_status(LivePromotionProjectStatusInputs {
            promotion_summary_document: Some(&summary),
            promotion_mapping_document: Some(&mapping),
            availability_document: Some(&availability),
        })
        .unwrap();

        assert_eq!(status.status, "ready");
        assert_eq!(status.reason_code, "ready");
        assert_eq!(status.warning_count, 2);
        assert_eq!(
            status.next_actions,
            vec![
                "promotion handoff is review-ready".to_string(),
                "promotion is apply-ready in the staged continuation".to_string(),
            ]
        );
    }
}
