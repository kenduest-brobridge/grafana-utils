//! Live sync domain-status producer.
//!
//! Maintainer note:
//! - This module derives one sync-owned domain-status row from staged sync
//!   summary and package-test surfaces.
//! - Keep the producer conservative: package-test data is preferred when it
//!   exists, staged summary data is only a fallback, and missing surfaces stay
//!   explicit.

#![allow(dead_code)]

use serde_json::Value;

use crate::project_status::{
    status_finding, ProjectDomainStatus, PROJECT_STATUS_BLOCKED, PROJECT_STATUS_PARTIAL,
    PROJECT_STATUS_READY,
};

const SYNC_DOMAIN_ID: &str = "sync";
const SYNC_SCOPE: &str = "live";
const SYNC_MODE: &str = "live-sync-surfaces";

const SYNC_REASON_READY: &str = PROJECT_STATUS_READY;
const SYNC_REASON_PARTIAL_NO_DATA: &str = "partial-no-data";
const SYNC_REASON_PARTIAL_MISSING_SURFACES: &str = "partial-missing-surfaces";
const SYNC_REASON_BLOCKED_BY_BLOCKERS: &str = "blocked-by-blockers";

const SYNC_SOURCE_KIND_SUMMARY: &str = "sync-summary";
const SYNC_SOURCE_KIND_BUNDLE_PREFLIGHT: &str = "package-test";

const SYNC_SIGNAL_KEYS_SUMMARY: &[&str] = &["summary.resourceCount"];
const SYNC_SIGNAL_KEYS_BUNDLE_PREFLIGHT: &[&str] = &[
    "summary.resourceCount",
    "summary.syncBlockingCount",
    "summary.providerBlockingCount",
    "summary.secretPlaceholderBlockingCount",
    "summary.alertArtifactCount",
    "summary.alertArtifactBlockedCount",
    "summary.alertArtifactPlanOnlyCount",
    "summary.blockedCount",
    "summary.planOnlyCount",
];

const SYNC_BLOCKER_SYNC_BLOCKING: &str = "sync-blocking";
const SYNC_BLOCKER_PROVIDER_BLOCKING: &str = "provider-blocking";
const SYNC_BLOCKER_SECRET_PLACEHOLDER_BLOCKING: &str = "secret-placeholder-blocking";
const SYNC_BLOCKER_ALERT_ARTIFACT_BLOCKING: &str = "alert-artifact-blocking";
const SYNC_BLOCKER_BUNDLE_BLOCKING: &str = "bundle-blocking";
const SYNC_WARNING_ALERT_ARTIFACT_PLAN_ONLY: &str = "alert-artifact-plan-only";
const SYNC_WARNING_BUNDLE_PLAN_ONLY: &str = "bundle-plan-only";

const SYNC_RESOLVE_BLOCKERS_ACTIONS: &[&str] = &[
    "resolve sync workflow blockers in the fixed order: sync, provider, secret-placeholder, alert-artifact",
];
const SYNC_STAGE_AT_LEAST_ONE_ACTIONS: &[&str] =
    &["stage at least one dashboard, datasource, or alert resource"];
const SYNC_PROVIDE_PREFLIGHT_ACTIONS: &[&str] =
    &["provide a staged package-test document before interpreting live sync readiness"];
const SYNC_REVIEW_NON_BLOCKING_ACTIONS: &[&str] =
    &["review non-blocking sync findings before promotion or apply"];
const SYNC_REEXPORT_AFTER_CHANGES_ACTIONS: &[&str] = &["re-run sync summary after staged changes"];

#[derive(Debug, Clone, Default)]
pub struct SyncLiveProjectStatusInputs<'a> {
    pub summary_document: Option<&'a Value>,
    pub bundle_preflight_document: Option<&'a Value>,
}

fn summary_number(document: &Value, key: &str) -> usize {
    document
        .get("summary")
        .and_then(|value| value.get(key))
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize
}

fn next_actions_for_partial(resources: usize, has_bundle_preflight: bool) -> Vec<String> {
    if resources == 0 {
        SYNC_STAGE_AT_LEAST_ONE_ACTIONS
            .iter()
            .map(|item| (*item).to_string())
            .collect()
    } else if has_bundle_preflight {
        SYNC_REEXPORT_AFTER_CHANGES_ACTIONS
            .iter()
            .map(|item| (*item).to_string())
            .collect()
    } else {
        SYNC_PROVIDE_PREFLIGHT_ACTIONS
            .iter()
            .map(|item| (*item).to_string())
            .collect()
    }
}

fn next_actions_for_ready(warnings_present: bool) -> Vec<String> {
    if warnings_present {
        SYNC_REVIEW_NON_BLOCKING_ACTIONS
            .iter()
            .map(|item| (*item).to_string())
            .collect()
    } else {
        SYNC_REEXPORT_AFTER_CHANGES_ACTIONS
            .iter()
            .map(|item| (*item).to_string())
            .collect()
    }
}

pub(crate) fn build_live_sync_domain_status(
    inputs: SyncLiveProjectStatusInputs<'_>,
) -> Option<ProjectDomainStatus> {
    if inputs.summary_document.is_none() && inputs.bundle_preflight_document.is_none() {
        return None;
    }

    let mut source_kinds = Vec::new();
    let mut signal_keys = Vec::new();
    if inputs.summary_document.is_some() {
        source_kinds.push(SYNC_SOURCE_KIND_SUMMARY.to_string());
    }

    let mut blockers = Vec::new();
    let mut warnings = Vec::new();

    let (resources, status, reason_code, next_actions) =
        if let Some(document) = inputs.bundle_preflight_document {
            source_kinds.push(SYNC_SOURCE_KIND_BUNDLE_PREFLIGHT.to_string());
            signal_keys.extend(
                SYNC_SIGNAL_KEYS_BUNDLE_PREFLIGHT
                    .iter()
                    .map(|item| (*item).to_string()),
            );

            let resources = summary_number(document, "resourceCount");
            let sync_blocking = summary_number(document, "syncBlockingCount");
            let provider_blocking = summary_number(document, "providerBlockingCount");
            let secret_blocking = summary_number(document, "secretPlaceholderBlockingCount");
            let alert_blocking = summary_number(document, "alertArtifactBlockedCount");
            let alert_plan_only = summary_number(document, "alertArtifactPlanOnlyCount");
            let bundle_blocking = summary_number(document, "blockedCount");
            let bundle_plan_only = summary_number(document, "planOnlyCount");

            // Keep the bundle-preflight handoff signals visible even when a
            // subset of them does not change the final status.
            if sync_blocking > 0 {
                blockers.push(status_finding(
                    SYNC_BLOCKER_SYNC_BLOCKING,
                    sync_blocking,
                    "summary.syncBlockingCount",
                ));
            }
            if provider_blocking > 0 {
                blockers.push(status_finding(
                    SYNC_BLOCKER_PROVIDER_BLOCKING,
                    provider_blocking,
                    "summary.providerBlockingCount",
                ));
            }
            if secret_blocking > 0 {
                blockers.push(status_finding(
                    SYNC_BLOCKER_SECRET_PLACEHOLDER_BLOCKING,
                    secret_blocking,
                    "summary.secretPlaceholderBlockingCount",
                ));
            }
            if alert_blocking > 0 {
                blockers.push(status_finding(
                    SYNC_BLOCKER_ALERT_ARTIFACT_BLOCKING,
                    alert_blocking,
                    "summary.alertArtifactBlockedCount",
                ));
            }
            if blockers.is_empty() && bundle_blocking > 0 {
                blockers.push(status_finding(
                    SYNC_BLOCKER_BUNDLE_BLOCKING,
                    bundle_blocking,
                    "summary.blockedCount",
                ));
            }
            if alert_plan_only > 0 {
                warnings.push(status_finding(
                    SYNC_WARNING_ALERT_ARTIFACT_PLAN_ONLY,
                    alert_plan_only,
                    "summary.alertArtifactPlanOnlyCount",
                ));
            } else if bundle_plan_only > 0 {
                warnings.push(status_finding(
                    SYNC_WARNING_BUNDLE_PLAN_ONLY,
                    bundle_plan_only,
                    "summary.planOnlyCount",
                ));
            }

            let has_blockers = !blockers.is_empty();
            let has_warnings = !warnings.is_empty();
            let next_actions = if has_blockers {
                SYNC_RESOLVE_BLOCKERS_ACTIONS
                    .iter()
                    .map(|item| (*item).to_string())
                    .collect()
            } else if resources == 0 {
                next_actions_for_partial(resources, true)
            } else {
                next_actions_for_ready(has_warnings)
            };
            let status = if has_blockers {
                PROJECT_STATUS_BLOCKED
            } else if resources == 0 {
                PROJECT_STATUS_PARTIAL
            } else {
                PROJECT_STATUS_READY
            };
            let reason_code = if has_blockers {
                SYNC_REASON_BLOCKED_BY_BLOCKERS
            } else if resources == 0 {
                SYNC_REASON_PARTIAL_NO_DATA
            } else {
                SYNC_REASON_READY
            };
            (resources, status, reason_code, next_actions)
        } else {
            let resources = inputs
                .summary_document
                .map(|document| summary_number(document, "resourceCount"))
                .unwrap_or(0);
            signal_keys.extend(
                SYNC_SIGNAL_KEYS_SUMMARY
                    .iter()
                    .map(|item| (*item).to_string()),
            );

            let next_actions = next_actions_for_partial(resources, false);
            let reason_code = if resources == 0 {
                SYNC_REASON_PARTIAL_NO_DATA
            } else {
                SYNC_REASON_PARTIAL_MISSING_SURFACES
            };
            (resources, PROJECT_STATUS_PARTIAL, reason_code, next_actions)
        };

    Some(ProjectDomainStatus {
        id: SYNC_DOMAIN_ID.to_string(),
        scope: SYNC_SCOPE.to_string(),
        mode: SYNC_MODE.to_string(),
        status: status.to_string(),
        reason_code: reason_code.to_string(),
        primary_count: resources,
        blocker_count: blockers.iter().map(|item| item.count).sum(),
        warning_count: warnings.iter().map(|item| item.count).sum(),
        source_kinds,
        signal_keys,
        blockers,
        warnings,
        next_actions,
        freshness: Default::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::{build_live_sync_domain_status, SyncLiveProjectStatusInputs};
    use serde_json::json;

    #[test]
    fn build_live_sync_domain_status_returns_none_without_any_surfaces() {
        assert!(build_live_sync_domain_status(SyncLiveProjectStatusInputs::default()).is_none());
    }

    #[test]
    fn build_live_sync_domain_status_reports_blockers_from_bundle_preflight() {
        let bundle_preflight = json!({
            "kind": "grafana-utils-sync-bundle-preflight",
            "summary": {
                "resourceCount": 4,
                "syncBlockingCount": 1,
                "providerBlockingCount": 2,
                "secretPlaceholderBlockingCount": 0,
                "alertArtifactCount": 4,
                "alertArtifactBlockedCount": 3,
                "alertArtifactPlanOnlyCount": 1,
                "blockedCount": 2,
                "planOnlyCount": 1,
            }
        });

        let domain = build_live_sync_domain_status(SyncLiveProjectStatusInputs {
            summary_document: None,
            bundle_preflight_document: Some(&bundle_preflight),
        })
        .unwrap();
        let value = serde_json::to_value(domain).unwrap();

        assert_eq!(value["id"], json!("sync"));
        assert_eq!(value["scope"], json!("live"));
        assert_eq!(value["mode"], json!("live-sync-surfaces"));
        assert_eq!(value["status"], json!("blocked"));
        assert_eq!(value["reasonCode"], json!("blocked-by-blockers"));
        assert_eq!(value["primaryCount"], json!(4));
        assert_eq!(value["blockerCount"], json!(6));
        assert_eq!(value["warningCount"], json!(1));
        assert_eq!(value["sourceKinds"], json!(["package-test"]));
        assert_eq!(
            value["signalKeys"],
            json!([
                "summary.resourceCount",
                "summary.syncBlockingCount",
                "summary.providerBlockingCount",
                "summary.secretPlaceholderBlockingCount",
                "summary.alertArtifactCount",
                "summary.alertArtifactBlockedCount",
                "summary.alertArtifactPlanOnlyCount",
                "summary.blockedCount",
                "summary.planOnlyCount",
            ])
        );
        assert_eq!(value["blockers"].as_array().unwrap().len(), 3);
        assert_eq!(
            value["warnings"],
            json!([{
                "kind": "alert-artifact-plan-only",
                "count": 1,
                "source": "summary.alertArtifactPlanOnlyCount"
            }])
        );
        assert!(value["nextActions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item == "resolve sync workflow blockers in the fixed order: sync, provider, secret-placeholder, alert-artifact"));
    }

    #[test]
    fn build_live_sync_domain_status_is_partial_when_only_summary_exists() {
        let summary = json!({
            "kind": "grafana-utils-sync-summary",
            "summary": {
                "resourceCount": 2,
            }
        });

        let domain = build_live_sync_domain_status(SyncLiveProjectStatusInputs {
            summary_document: Some(&summary),
            bundle_preflight_document: None,
        })
        .unwrap();
        let value = serde_json::to_value(domain).unwrap();

        assert_eq!(value["status"], json!("partial"));
        assert_eq!(value["reasonCode"], json!("partial-missing-surfaces"));
        assert_eq!(value["primaryCount"], json!(2));
        assert_eq!(value["sourceKinds"], json!(["sync-summary"]));
        assert_eq!(value["signalKeys"], json!(["summary.resourceCount"]));
        assert_eq!(
            value["nextActions"],
            json!([
                "provide a staged package-test document before interpreting live sync readiness"
            ])
        );
    }

    #[test]
    fn build_live_sync_domain_status_keeps_summary_and_bundle_sources_additive() {
        let summary = json!({
            "kind": "grafana-utils-sync-summary",
            "summary": {
                "resourceCount": 2,
            }
        });
        let bundle_preflight = json!({
            "kind": "grafana-utils-sync-bundle-preflight",
            "summary": {
                "resourceCount": 2,
                "syncBlockingCount": 0,
                "providerBlockingCount": 0,
                "secretPlaceholderBlockingCount": 0,
                "alertArtifactCount": 1,
                "alertArtifactBlockedCount": 0,
                "alertArtifactPlanOnlyCount": 0,
                "blockedCount": 0,
                "planOnlyCount": 0,
            }
        });

        let domain = build_live_sync_domain_status(SyncLiveProjectStatusInputs {
            summary_document: Some(&summary),
            bundle_preflight_document: Some(&bundle_preflight),
        })
        .unwrap();
        let value = serde_json::to_value(domain).unwrap();

        assert_eq!(value["status"], json!("ready"));
        assert_eq!(
            value["sourceKinds"],
            json!(["sync-summary", "package-test"])
        );
        assert_eq!(
            value["signalKeys"],
            json!([
                "summary.resourceCount",
                "summary.syncBlockingCount",
                "summary.providerBlockingCount",
                "summary.secretPlaceholderBlockingCount",
                "summary.alertArtifactCount",
                "summary.alertArtifactBlockedCount",
                "summary.alertArtifactPlanOnlyCount",
                "summary.blockedCount",
                "summary.planOnlyCount",
            ])
        );
        assert_eq!(value["warningCount"], json!(0));
        assert_eq!(
            value["nextActions"],
            json!(["re-run sync summary after staged changes"])
        );
    }

    #[test]
    fn build_live_sync_domain_status_is_partial_when_bundle_preflight_has_no_resources() {
        let bundle_preflight = json!({
            "kind": "grafana-utils-sync-bundle-preflight",
            "summary": {
                "resourceCount": 0,
                "syncBlockingCount": 0,
                "providerBlockingCount": 0,
                "secretPlaceholderBlockingCount": 0,
                "alertArtifactBlockedCount": 0,
                "alertArtifactPlanOnlyCount": 0,
            }
        });

        let domain = build_live_sync_domain_status(SyncLiveProjectStatusInputs {
            summary_document: None,
            bundle_preflight_document: Some(&bundle_preflight),
        })
        .unwrap();

        assert_eq!(domain.status, "partial");
        assert_eq!(domain.reason_code, "partial-no-data");
        assert_eq!(domain.primary_count, 0);
        assert_eq!(
            domain.next_actions,
            vec!["stage at least one dashboard, datasource, or alert resource".to_string()]
        );
    }

    #[test]
    fn build_live_sync_domain_status_falls_back_to_generic_bundle_summary_keys() {
        let bundle_preflight = json!({
            "kind": "grafana-utils-sync-bundle-preflight",
            "summary": {
                "resourceCount": 3,
                "blockedCount": 2,
                "planOnlyCount": 1
            }
        });

        let domain = build_live_sync_domain_status(SyncLiveProjectStatusInputs {
            summary_document: None,
            bundle_preflight_document: Some(&bundle_preflight),
        })
        .unwrap();
        let value = serde_json::to_value(domain).unwrap();

        assert_eq!(value["status"], json!("blocked"));
        assert_eq!(value["reasonCode"], json!("blocked-by-blockers"));
        assert_eq!(
            value["blockers"],
            json!([{
                "kind": "bundle-blocking",
                "count": 2,
                "source": "summary.blockedCount"
            }])
        );
    }
}
