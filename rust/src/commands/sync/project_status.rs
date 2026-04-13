//! Shared sync domain-status producer.
//!
//! Maintainer note:
//! - This module derives one sync-owned domain-status row from existing staged
//!   sync documents.
//! - Keep this document-driven and reusable by multiple consumers.

use serde_json::Value;

use crate::project_status::{
    status_finding, ProjectDomainStatus, PROJECT_STATUS_BLOCKED, PROJECT_STATUS_PARTIAL,
    PROJECT_STATUS_READY,
};

const SYNC_DOMAIN_ID: &str = "sync";
const SYNC_SCOPE: &str = "staged";
const SYNC_MODE: &str = "staged-documents";
const SYNC_REASON_READY: &str = PROJECT_STATUS_READY;
const SYNC_REASON_PARTIAL_NO_DATA: &str = "partial-no-data";
const SYNC_REASON_BLOCKED_BY_BLOCKERS: &str = "blocked-by-blockers";

const SYNC_BLOCKER_SYNC_BLOCKING: &str = "sync-blocking";
const SYNC_BLOCKER_PROVIDER_BLOCKING: &str = "provider-blocking";
const SYNC_BLOCKER_SECRET_PLACEHOLDER_BLOCKING: &str = "secret-placeholder-blocking";
const SYNC_BLOCKER_ALERT_ARTIFACT_BLOCKING: &str = "alert-artifact-blocking";
const SYNC_WARNING_PROVIDER_REVIEW: &str = "provider-review";
const SYNC_WARNING_SECRET_PLACEHOLDER_REVIEW: &str = "secret-placeholder-review";
const SYNC_WARNING_ALERT_ARTIFACT_PLAN_ONLY: &str = "alert-artifact-plan-only";
const SYNC_WARNING_ALERT_ARTIFACT_REVIEW: &str = "alert-artifact-review";
const SYNC_PROVIDER_ASSESSMENT_SIGNAL_KEY: &str = "providerAssessment.summary.blockingCount";
const SYNC_SECRET_PLACEHOLDER_ASSESSMENT_SIGNAL_KEY: &str =
    "secretPlaceholderAssessment.summary.blockingCount";
const SYNC_PROVIDER_ASSESSMENT_PLAN_SIGNAL_KEY: &str = "providerAssessment.plans";
const SYNC_SECRET_PLACEHOLDER_ASSESSMENT_PLAN_SIGNAL_KEY: &str =
    "secretPlaceholderAssessment.plans";
const SYNC_BUNDLE_PREFLIGHT_SIGNAL_KEYS: &[&str] = &[
    "summary.syncBlockingCount",
    "summary.providerBlockingCount",
    "summary.secretPlaceholderBlockingCount",
    "summary.alertArtifactBlockedCount",
    "summary.alertArtifactPlanOnlyCount",
    "summary.alertArtifactCount",
];

const SYNC_RESOLVE_BLOCKERS_ACTIONS: &[&str] = &[
    "resolve sync workflow blockers in the fixed order: sync, provider, secret-placeholder, alert-artifact",
];
const SYNC_STAGE_AT_LEAST_ONE_ACTIONS: &[&str] =
    &["stage at least one dashboard, datasource, or alert resource"];
const SYNC_REVIEW_NON_BLOCKING_ACTIONS: &[&str] =
    &["review non-blocking sync findings before promotion or apply"];
const SYNC_REEXPORT_AFTER_CHANGES_ACTIONS: &[&str] = &["re-run sync summary after staged changes"];

#[derive(Debug, Clone, Default)]
pub struct SyncDomainStatusInputs<'a> {
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

fn nested_summary_number(document: &Value, section: &str, key: &str) -> usize {
    document
        .get(section)
        .and_then(Value::as_object)
        .and_then(|value| value.get("summary"))
        .and_then(Value::as_object)
        .and_then(|value| value.get(key))
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize
}

fn nested_array_count(document: &Value, section: &str, key: &str) -> usize {
    document
        .get(section)
        .and_then(Value::as_object)
        .and_then(|value| value.get(key))
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0)
}

pub(crate) fn build_sync_domain_status(
    inputs: SyncDomainStatusInputs<'_>,
) -> Option<ProjectDomainStatus> {
    let summary_document = inputs.summary_document;
    let bundle_preflight_document = inputs.bundle_preflight_document;
    if summary_document.is_none() && bundle_preflight_document.is_none() {
        return None;
    }

    let resources = summary_document
        .map(|document| summary_number(document, "resourceCount"))
        .or_else(|| {
            bundle_preflight_document.map(|document| summary_number(document, "resourceCount"))
        })
        .unwrap_or(0);

    let mut source_kinds = Vec::new();
    let mut signal_keys = Vec::new();
    if summary_document.is_some() {
        source_kinds.push("sync-summary".to_string());
        signal_keys.push("summary.resourceCount".to_string());
    } else if bundle_preflight_document.is_some() {
        signal_keys.push("summary.resourceCount".to_string());
    }

    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    if let Some(document) = bundle_preflight_document {
        source_kinds.push("package-test".to_string());
        signal_keys.extend(
            SYNC_BUNDLE_PREFLIGHT_SIGNAL_KEYS
                .iter()
                .map(|item| (*item).to_string()),
        );
        let sync_blocking = summary_number(document, "syncBlockingCount");
        let provider_blocking = summary_number(document, "providerBlockingCount").max(
            nested_summary_number(document, "providerAssessment", "blockingCount"),
        );
        let provider_plan_count = nested_array_count(document, "providerAssessment", "plans");
        let secret_blocking = summary_number(document, "secretPlaceholderBlockingCount").max(
            nested_summary_number(document, "secretPlaceholderAssessment", "blockingCount"),
        );
        let secret_placeholder_plan_count =
            nested_array_count(document, "secretPlaceholderAssessment", "plans");
        let alert_blocking = summary_number(document, "alertArtifactBlockedCount").max(
            nested_summary_number(document, "alertArtifactAssessment", "blockedCount"),
        );
        let alert_plan_only = summary_number(document, "alertArtifactPlanOnlyCount").max(
            nested_summary_number(document, "alertArtifactAssessment", "planOnlyCount"),
        );
        let alert_artifact_count = summary_number(document, "alertArtifactCount").max(
            nested_summary_number(document, "alertArtifactAssessment", "resourceCount"),
        );
        let provider_assessment_present = document
            .get("providerAssessment")
            .and_then(Value::as_object)
            .is_some();
        let secret_placeholder_assessment_present = document
            .get("secretPlaceholderAssessment")
            .and_then(Value::as_object)
            .is_some();

        if provider_assessment_present {
            signal_keys.push(SYNC_PROVIDER_ASSESSMENT_SIGNAL_KEY.to_string());
            signal_keys.push(SYNC_PROVIDER_ASSESSMENT_PLAN_SIGNAL_KEY.to_string());
        }
        if secret_placeholder_assessment_present {
            signal_keys.push(SYNC_SECRET_PLACEHOLDER_ASSESSMENT_SIGNAL_KEY.to_string());
            signal_keys.push(SYNC_SECRET_PLACEHOLDER_ASSESSMENT_PLAN_SIGNAL_KEY.to_string());
        }

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
                if summary_number(document, "providerBlockingCount") > 0 {
                    "summary.providerBlockingCount"
                } else {
                    SYNC_PROVIDER_ASSESSMENT_SIGNAL_KEY
                },
            ));
        }
        if secret_blocking > 0 {
            blockers.push(status_finding(
                SYNC_BLOCKER_SECRET_PLACEHOLDER_BLOCKING,
                secret_blocking,
                if summary_number(document, "secretPlaceholderBlockingCount") > 0 {
                    "summary.secretPlaceholderBlockingCount"
                } else {
                    SYNC_SECRET_PLACEHOLDER_ASSESSMENT_SIGNAL_KEY
                },
            ));
        }
        if provider_plan_count > 0 && provider_blocking == 0 {
            warnings.push(status_finding(
                SYNC_WARNING_PROVIDER_REVIEW,
                provider_plan_count,
                SYNC_PROVIDER_ASSESSMENT_PLAN_SIGNAL_KEY,
            ));
        }
        if secret_placeholder_plan_count > 0 && secret_blocking == 0 {
            warnings.push(status_finding(
                SYNC_WARNING_SECRET_PLACEHOLDER_REVIEW,
                secret_placeholder_plan_count,
                SYNC_SECRET_PLACEHOLDER_ASSESSMENT_PLAN_SIGNAL_KEY,
            ));
        }
        if alert_blocking > 0 {
            blockers.push(status_finding(
                SYNC_BLOCKER_ALERT_ARTIFACT_BLOCKING,
                alert_blocking,
                if summary_number(document, "alertArtifactBlockedCount") > 0 {
                    "summary.alertArtifactBlockedCount"
                } else {
                    "alertArtifactAssessment.summary.blockedCount"
                },
            ));
        }
        if alert_plan_only > 0 {
            warnings.push(status_finding(
                SYNC_WARNING_ALERT_ARTIFACT_PLAN_ONLY,
                alert_plan_only,
                if summary_number(document, "alertArtifactPlanOnlyCount") > 0 {
                    "summary.alertArtifactPlanOnlyCount"
                } else {
                    "alertArtifactAssessment.summary.planOnlyCount"
                },
            ));
        }
        if alert_artifact_count > 0 && alert_blocking == 0 && alert_plan_only == 0 {
            warnings.push(status_finding(
                SYNC_WARNING_ALERT_ARTIFACT_REVIEW,
                alert_artifact_count,
                if summary_number(document, "alertArtifactCount") > 0 {
                    "summary.alertArtifactCount"
                } else {
                    "alertArtifactAssessment.summary.resourceCount"
                },
            ));
        }
    }

    let has_blockers = !blockers.is_empty();
    let is_partial = resources == 0;
    let (status, reason_code, next_actions) = if has_blockers {
        (
            PROJECT_STATUS_BLOCKED,
            SYNC_REASON_BLOCKED_BY_BLOCKERS,
            SYNC_RESOLVE_BLOCKERS_ACTIONS,
        )
    } else if is_partial {
        (
            PROJECT_STATUS_PARTIAL,
            SYNC_REASON_PARTIAL_NO_DATA,
            SYNC_STAGE_AT_LEAST_ONE_ACTIONS,
        )
    } else if !warnings.is_empty() {
        (
            PROJECT_STATUS_READY,
            SYNC_REASON_READY,
            SYNC_REVIEW_NON_BLOCKING_ACTIONS,
        )
    } else {
        (
            PROJECT_STATUS_READY,
            SYNC_REASON_READY,
            SYNC_REEXPORT_AFTER_CHANGES_ACTIONS,
        )
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
        next_actions: next_actions
            .iter()
            .map(|item| (*item).to_string())
            .collect(),
        freshness: Default::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::{build_sync_domain_status, SyncDomainStatusInputs};
    use crate::project_status::status_finding;
    use serde_json::json;

    #[test]
    fn build_sync_domain_status_reports_bundle_preflight_artifact_evidence() {
        let summary = json!({
            "kind": "grafana-utils-sync-summary",
            "summary": {
                "resourceCount": 4,
            }
        });
        let bundle_preflight = json!({
            "kind": "grafana-utils-sync-bundle-preflight",
            "summary": {
                "resourceCount": 4,
                "syncBlockingCount": 1,
                "providerBlockingCount": 0,
                "secretPlaceholderBlockingCount": 0,
                "alertArtifactBlockedCount": 2,
                "alertArtifactPlanOnlyCount": 3,
                "alertArtifactCount": 5,
            }
        });

        let status = build_sync_domain_status(SyncDomainStatusInputs {
            summary_document: Some(&summary),
            bundle_preflight_document: Some(&bundle_preflight),
        })
        .unwrap();

        assert_eq!(status.id, "sync");
        assert_eq!(status.scope, "staged");
        assert_eq!(status.mode, "staged-documents");
        assert_eq!(status.status, "blocked");
        assert_eq!(status.reason_code, "blocked-by-blockers");
        assert_eq!(status.primary_count, 4);
        assert_eq!(
            status.source_kinds,
            vec!["sync-summary".to_string(), "package-test".to_string()]
        );
        assert_eq!(
            status.signal_keys,
            vec![
                "summary.resourceCount".to_string(),
                "summary.syncBlockingCount".to_string(),
                "summary.providerBlockingCount".to_string(),
                "summary.secretPlaceholderBlockingCount".to_string(),
                "summary.alertArtifactBlockedCount".to_string(),
                "summary.alertArtifactPlanOnlyCount".to_string(),
                "summary.alertArtifactCount".to_string(),
            ]
        );
        assert_eq!(status.blockers.len(), 2);
        assert_eq!(status.blockers[0].kind, "sync-blocking");
        assert_eq!(status.blockers[0].count, 1);
        assert_eq!(status.blockers[0].source, "summary.syncBlockingCount");
        assert_eq!(status.blockers[1].kind, "alert-artifact-blocking");
        assert_eq!(status.blockers[1].count, 2);
        assert_eq!(
            status.blockers[1].source,
            "summary.alertArtifactBlockedCount"
        );
        assert_eq!(status.warnings.len(), 1);
        assert_eq!(status.warnings[0].kind, "alert-artifact-plan-only");
        assert_eq!(status.warnings[0].count, 3);
        assert_eq!(
            status.warnings[0].source,
            "summary.alertArtifactPlanOnlyCount"
        );
        assert_eq!(
            status.next_actions,
            vec!["resolve sync workflow blockers in the fixed order: sync, provider, secret-placeholder, alert-artifact".to_string()]
        );
    }

    #[test]
    fn build_sync_domain_status_reports_bundle_preflight_resource_source_when_summary_is_missing() {
        let bundle_preflight = json!({
            "kind": "grafana-utils-sync-bundle-preflight",
            "summary": {
                "resourceCount": 4,
                "syncBlockingCount": 0,
                "providerBlockingCount": 0,
                "secretPlaceholderBlockingCount": 0,
                "alertArtifactBlockedCount": 0,
                "alertArtifactPlanOnlyCount": 0,
                "alertArtifactCount": 0,
            }
        });

        let status = build_sync_domain_status(SyncDomainStatusInputs {
            summary_document: None,
            bundle_preflight_document: Some(&bundle_preflight),
        })
        .unwrap();

        assert_eq!(status.id, "sync");
        assert_eq!(status.scope, "staged");
        assert_eq!(status.mode, "staged-documents");
        assert_eq!(status.status, "ready");
        assert_eq!(status.reason_code, "ready");
        assert_eq!(status.primary_count, 4);
        assert_eq!(status.source_kinds, vec!["package-test".to_string()]);
        assert_eq!(
            status.signal_keys,
            vec![
                "summary.resourceCount".to_string(),
                "summary.syncBlockingCount".to_string(),
                "summary.providerBlockingCount".to_string(),
                "summary.secretPlaceholderBlockingCount".to_string(),
                "summary.alertArtifactBlockedCount".to_string(),
                "summary.alertArtifactPlanOnlyCount".to_string(),
                "summary.alertArtifactCount".to_string(),
            ]
        );
        assert_eq!(status.blockers.len(), 0);
        assert_eq!(status.warnings.len(), 0);
        assert_eq!(
            status.next_actions,
            vec!["re-run sync summary after staged changes".to_string()]
        );
    }

    #[test]
    fn build_sync_domain_status_surfaces_alert_artifact_review_evidence_without_blocking_or_plan_only(
    ) {
        let bundle_preflight = json!({
            "kind": "grafana-utils-sync-bundle-preflight",
            "summary": {
                "resourceCount": 4,
                "syncBlockingCount": 0,
                "providerBlockingCount": 0,
                "secretPlaceholderBlockingCount": 0,
                "alertArtifactBlockedCount": 0,
                "alertArtifactPlanOnlyCount": 0,
                "alertArtifactCount": 3,
            }
        });

        let status = build_sync_domain_status(SyncDomainStatusInputs {
            summary_document: None,
            bundle_preflight_document: Some(&bundle_preflight),
        })
        .unwrap();

        assert_eq!(status.status, "ready");
        assert_eq!(status.reason_code, "ready");
        assert_eq!(status.blocker_count, 0);
        assert_eq!(status.warning_count, 3);
        assert_eq!(status.warnings.len(), 1);
        assert_eq!(status.warnings[0].kind, "alert-artifact-review");
        assert_eq!(status.warnings[0].count, 3);
        assert_eq!(status.warnings[0].source, "summary.alertArtifactCount");
        assert_eq!(
            status.next_actions,
            vec!["review non-blocking sync findings before promotion or apply".to_string()]
        );
    }

    #[test]
    fn build_sync_domain_status_uses_nested_alert_artifact_evidence_when_summary_counts_are_missing(
    ) {
        let bundle_preflight = json!({
            "kind": "grafana-utils-sync-bundle-preflight",
            "summary": {
                "resourceCount": 2,
                "syncBlockingCount": 0,
                "providerBlockingCount": 0,
                "secretPlaceholderBlockingCount": 0,
                "alertArtifactBlockedCount": 0,
                "alertArtifactPlanOnlyCount": 0,
                "alertArtifactCount": 0,
            },
            "alertArtifactAssessment": {
                "summary": {
                    "resourceCount": 2,
                    "blockedCount": 1,
                    "planOnlyCount": 2,
                }
            }
        });

        let status = build_sync_domain_status(SyncDomainStatusInputs {
            summary_document: None,
            bundle_preflight_document: Some(&bundle_preflight),
        })
        .unwrap();

        assert_eq!(status.status, "blocked");
        assert_eq!(status.reason_code, "blocked-by-blockers");
        assert_eq!(status.blocker_count, 1);
        assert_eq!(status.warning_count, 2);
        assert_eq!(status.blockers.len(), 1);
        assert_eq!(status.blockers[0].kind, "alert-artifact-blocking");
        assert_eq!(status.blockers[0].count, 1);
        assert_eq!(
            status.blockers[0].source,
            "alertArtifactAssessment.summary.blockedCount"
        );
        assert_eq!(status.warnings.len(), 1);
        assert_eq!(status.warnings[0].kind, "alert-artifact-plan-only");
        assert_eq!(status.warnings[0].count, 2);
        assert_eq!(
            status.warnings[0].source,
            "alertArtifactAssessment.summary.planOnlyCount"
        );
        assert_eq!(
            status.next_actions,
            vec!["resolve sync workflow blockers in the fixed order: sync, provider, secret-placeholder, alert-artifact".to_string()]
        );
    }

    #[test]
    fn build_sync_domain_status_uses_nested_provider_assessment_evidence_when_summary_counts_are_missing(
    ) {
        let bundle_preflight = json!({
            "kind": "grafana-utils-sync-bundle-preflight",
            "summary": {
                "resourceCount": 2,
                "syncBlockingCount": 0,
                "providerBlockingCount": 0,
                "secretPlaceholderBlockingCount": 0,
                "alertArtifactBlockedCount": 0,
                "alertArtifactPlanOnlyCount": 0,
                "alertArtifactCount": 0,
            },
            "providerAssessment": {
                "summary": {
                    "blockingCount": 3
                }
            }
        });

        let status = build_sync_domain_status(SyncDomainStatusInputs {
            summary_document: None,
            bundle_preflight_document: Some(&bundle_preflight),
        })
        .unwrap();

        assert_eq!(status.status, "blocked");
        assert_eq!(status.reason_code, "blocked-by-blockers");
        assert_eq!(status.blocker_count, 3);
        assert_eq!(status.blockers.len(), 1);
        assert_eq!(status.blockers[0].kind, "provider-blocking");
        assert_eq!(status.blockers[0].count, 3);
        assert_eq!(
            status.blockers[0].source,
            "providerAssessment.summary.blockingCount"
        );
        assert_eq!(
            status.signal_keys,
            vec![
                "summary.resourceCount".to_string(),
                "summary.syncBlockingCount".to_string(),
                "summary.providerBlockingCount".to_string(),
                "summary.secretPlaceholderBlockingCount".to_string(),
                "summary.alertArtifactBlockedCount".to_string(),
                "summary.alertArtifactPlanOnlyCount".to_string(),
                "summary.alertArtifactCount".to_string(),
                "providerAssessment.summary.blockingCount".to_string(),
                "providerAssessment.plans".to_string(),
            ]
        );
        assert_eq!(status.warnings.len(), 0);
        assert_eq!(
            status.next_actions,
            vec!["resolve sync workflow blockers in the fixed order: sync, provider, secret-placeholder, alert-artifact".to_string()]
        );
    }

    #[test]
    fn build_sync_domain_status_surfaces_provider_and_secret_review_evidence_without_blocking() {
        let bundle_preflight = json!({
            "kind": "grafana-utils-sync-bundle-preflight",
            "summary": {
                "resourceCount": 2,
                "syncBlockingCount": 0,
                "providerBlockingCount": 0,
                "secretPlaceholderBlockingCount": 0,
                "alertArtifactBlockedCount": 0,
                "alertArtifactPlanOnlyCount": 0,
                "alertArtifactCount": 0,
            },
            "providerAssessment": {
                "summary": {
                    "blockingCount": 0
                },
                "plans": [
                    {"providerKind": "vault"},
                    {"providerKind": "aws-secrets-manager"}
                ]
            },
            "secretPlaceholderAssessment": {
                "summary": {
                    "blockingCount": 0
                },
                "plans": [
                    {"providerKind": "vault"}
                ]
            }
        });

        let status = build_sync_domain_status(SyncDomainStatusInputs {
            summary_document: None,
            bundle_preflight_document: Some(&bundle_preflight),
        })
        .unwrap();

        assert_eq!(status.status, "ready");
        assert_eq!(status.reason_code, "ready");
        assert_eq!(status.blocker_count, 0);
        assert_eq!(status.warning_count, 3);
        assert_eq!(
            status.warnings,
            vec![
                status_finding("provider-review", 2, "providerAssessment.plans"),
                status_finding(
                    "secret-placeholder-review",
                    1,
                    "secretPlaceholderAssessment.plans"
                ),
            ]
        );
        assert_eq!(
            status.next_actions,
            vec!["review non-blocking sync findings before promotion or apply".to_string()]
        );
    }

    #[test]
    fn build_sync_domain_status_uses_nested_secret_placeholder_assessment_evidence_when_summary_counts_are_missing(
    ) {
        let bundle_preflight = json!({
            "kind": "grafana-utils-sync-bundle-preflight",
            "summary": {
                "resourceCount": 2,
                "syncBlockingCount": 0,
                "providerBlockingCount": 0,
                "secretPlaceholderBlockingCount": 0,
                "alertArtifactBlockedCount": 0,
                "alertArtifactPlanOnlyCount": 0,
                "alertArtifactCount": 0,
            },
            "secretPlaceholderAssessment": {
                "summary": {
                    "blockingCount": 2
                }
            }
        });

        let status = build_sync_domain_status(SyncDomainStatusInputs {
            summary_document: None,
            bundle_preflight_document: Some(&bundle_preflight),
        })
        .unwrap();

        assert_eq!(status.status, "blocked");
        assert_eq!(status.reason_code, "blocked-by-blockers");
        assert_eq!(status.blocker_count, 2);
        assert_eq!(status.blockers.len(), 1);
        assert_eq!(status.blockers[0].kind, "secret-placeholder-blocking");
        assert_eq!(status.blockers[0].count, 2);
        assert_eq!(
            status.blockers[0].source,
            "secretPlaceholderAssessment.summary.blockingCount"
        );
        assert_eq!(
            status.signal_keys,
            vec![
                "summary.resourceCount".to_string(),
                "summary.syncBlockingCount".to_string(),
                "summary.providerBlockingCount".to_string(),
                "summary.secretPlaceholderBlockingCount".to_string(),
                "summary.alertArtifactBlockedCount".to_string(),
                "summary.alertArtifactPlanOnlyCount".to_string(),
                "summary.alertArtifactCount".to_string(),
                "secretPlaceholderAssessment.summary.blockingCount".to_string(),
                "secretPlaceholderAssessment.plans".to_string(),
            ]
        );
        assert_eq!(status.warnings.len(), 0);
        assert_eq!(
            status.next_actions,
            vec!["resolve sync workflow blockers in the fixed order: sync, provider, secret-placeholder, alert-artifact".to_string()]
        );
    }
}
