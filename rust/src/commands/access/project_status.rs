//! Shared access domain-status producer.
//!
//! Maintainer note:
//! - This module derives one access-owned domain-status row from staged export
//!   bundle documents.
//! - Keep the producer conservative: it should only reason about bundle
//!   presence and record counts until later drift/import signals are added.

use serde_json::Value;

use crate::project_status::{
    status_finding, ProjectDomainStatus, PROJECT_STATUS_PARTIAL, PROJECT_STATUS_READY,
};
use crate::project_status_model::{StatusReading, StatusRecordCount};

use super::{
    ACCESS_EXPORT_KIND_ORGS, ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS, ACCESS_EXPORT_KIND_TEAMS,
    ACCESS_EXPORT_KIND_USERS,
};

const ACCESS_DOMAIN_ID: &str = "access";
const ACCESS_SCOPE: &str = "staged";
const ACCESS_MODE: &str = "staged-export-bundles";
const ACCESS_REASON_READY: &str = PROJECT_STATUS_READY;
const ACCESS_REASON_PARTIAL_NO_DATA: &str = "partial-no-data";
const ACCESS_REASON_PARTIAL_MISSING_BUNDLES: &str = "partial-missing-bundles";

const ACCESS_SIGNAL_KEYS: &[&str] = &[
    "summary.users.recordCount",
    "summary.teams.recordCount",
    "summary.orgs.recordCount",
    "summary.serviceAccounts.recordCount",
];

const ACCESS_EXPORT_USERS_LABEL: &str = "users";
const ACCESS_EXPORT_TEAMS_LABEL: &str = "teams";
const ACCESS_EXPORT_ORGS_LABEL: &str = "orgs";
const ACCESS_EXPORT_SERVICE_ACCOUNTS_LABEL: &str = "service accounts";

const ACCESS_MISSING_BUNDLE_KIND: &str = "missing-bundle-kind";
const ACCESS_READY_NEXT_ACTIONS: &[&str] = &["re-run access export after membership changes"];
const ACCESS_NO_DATA_NEXT_ACTIONS: &[&str] =
    &["export at least one access user, team, org, or service-account record"];
const ACCESS_MIXED_WORKSPACE_REVIEW_ACTIONS: &[&str] = &[
    "add the missing access export bundles before using this workspace as one mixed workspace handoff",
    "re-run workspace test after the missing access bundles are exported",
];

#[derive(Debug, Clone, Copy)]
struct AccessBundleSpec {
    kind: &'static str,
    signal_key: &'static str,
    label: &'static str,
}

const ACCESS_BUNDLE_SPECS: &[AccessBundleSpec] = &[
    AccessBundleSpec {
        kind: ACCESS_EXPORT_KIND_USERS,
        signal_key: "summary.users.recordCount",
        label: ACCESS_EXPORT_USERS_LABEL,
    },
    AccessBundleSpec {
        kind: ACCESS_EXPORT_KIND_TEAMS,
        signal_key: "summary.teams.recordCount",
        label: ACCESS_EXPORT_TEAMS_LABEL,
    },
    AccessBundleSpec {
        kind: ACCESS_EXPORT_KIND_ORGS,
        signal_key: "summary.orgs.recordCount",
        label: ACCESS_EXPORT_ORGS_LABEL,
    },
    AccessBundleSpec {
        kind: ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS,
        signal_key: "summary.serviceAccounts.recordCount",
        label: ACCESS_EXPORT_SERVICE_ACCOUNTS_LABEL,
    },
];

#[derive(Debug, Clone, Default)]
pub struct AccessDomainStatusInputs<'a> {
    pub user_export_document: Option<&'a Value>,
    pub team_export_document: Option<&'a Value>,
    pub org_export_document: Option<&'a Value>,
    pub service_account_export_document: Option<&'a Value>,
}

fn summary_number(document: &Value, key: &str) -> usize {
    document
        .get("summary")
        .and_then(|value| value.get(key))
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize
}

fn access_bundle_document<'a>(
    inputs: &'a AccessDomainStatusInputs<'a>,
    spec: AccessBundleSpec,
) -> Option<&'a Value> {
    match spec.kind {
        ACCESS_EXPORT_KIND_USERS => inputs.user_export_document,
        ACCESS_EXPORT_KIND_TEAMS => inputs.team_export_document,
        ACCESS_EXPORT_KIND_ORGS => inputs.org_export_document,
        ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS => inputs.service_account_export_document,
        _ => None,
    }
}

pub(crate) fn build_access_domain_status(
    inputs: AccessDomainStatusInputs<'_>,
) -> Option<ProjectDomainStatus> {
    let mut source_kinds = Vec::new();
    let mut warnings = Vec::new();
    let mut total_records = 0usize;
    let mut missing_labels = Vec::new();

    for spec in ACCESS_BUNDLE_SPECS {
        if let Some(document) = access_bundle_document(&inputs, *spec) {
            source_kinds.push(spec.kind.to_string());
            total_records += summary_number(document, "recordCount");
        } else {
            missing_labels.push(spec.label);
            warnings.push(status_finding(
                ACCESS_MISSING_BUNDLE_KIND,
                1,
                spec.signal_key,
            ));
        }
    }

    if source_kinds.is_empty() {
        return None;
    }

    let (status, reason_code, next_actions) = if !missing_labels.is_empty() {
        let mut next_actions = vec![format!(
            "export the missing access bundle kinds: {}",
            missing_labels.join(", ")
        )];
        next_actions.extend(
            ACCESS_MIXED_WORKSPACE_REVIEW_ACTIONS
                .iter()
                .map(|item| (*item).to_string()),
        );
        (
            PROJECT_STATUS_PARTIAL,
            ACCESS_REASON_PARTIAL_MISSING_BUNDLES,
            next_actions,
        )
    } else if total_records == 0 {
        (
            PROJECT_STATUS_PARTIAL,
            ACCESS_REASON_PARTIAL_NO_DATA,
            ACCESS_NO_DATA_NEXT_ACTIONS
                .iter()
                .map(|item| (*item).to_string())
                .collect(),
        )
    } else {
        (
            PROJECT_STATUS_READY,
            ACCESS_REASON_READY,
            ACCESS_READY_NEXT_ACTIONS
                .iter()
                .map(|item| (*item).to_string())
                .collect(),
        )
    };

    let reading = StatusReading {
        id: ACCESS_DOMAIN_ID.to_string(),
        scope: ACCESS_SCOPE.to_string(),
        mode: ACCESS_MODE.to_string(),
        status: status.to_string(),
        reason_code: reason_code.to_string(),
        primary_count: total_records,
        source_kinds,
        signal_keys: ACCESS_SIGNAL_KEYS
            .iter()
            .map(|item| (*item).to_string())
            .collect(),
        blockers: Vec::<StatusRecordCount>::new(),
        warnings: warnings.into_iter().map(Into::into).collect(),
        next_actions,
        freshness: Default::default(),
    };

    Some(reading.into_project_domain_status())
}

#[cfg(test)]
mod access_project_status_rust_tests {
    use super::{build_access_domain_status, AccessDomainStatusInputs};
    use crate::project_status::{PROJECT_STATUS_PARTIAL, PROJECT_STATUS_READY};
    use serde_json::json;

    #[test]
    fn build_access_domain_status_tracks_missing_bundle_kinds() {
        let user_document = json!({"summary": {"recordCount": 2}});
        let org_document = json!({"summary": {"recordCount": 1}});
        let domain = build_access_domain_status(AccessDomainStatusInputs {
            user_export_document: Some(&user_document),
            team_export_document: None,
            org_export_document: Some(&org_document),
            service_account_export_document: None,
        })
        .unwrap();

        assert_eq!(domain.id, "access");
        assert_eq!(domain.scope, "staged");
        assert_eq!(domain.mode, "staged-export-bundles");
        assert_eq!(domain.status, PROJECT_STATUS_PARTIAL);
        assert_eq!(domain.reason_code, "partial-missing-bundles");
        assert_eq!(domain.primary_count, 3);
        assert_eq!(domain.blocker_count, 0);
        assert_eq!(domain.warning_count, 2);
        assert_eq!(
            domain.source_kinds,
            vec![
                "grafana-utils-access-user-export-index".to_string(),
                "grafana-utils-access-org-export-index".to_string(),
            ]
        );
        assert_eq!(
            domain.signal_keys,
            vec![
                "summary.users.recordCount".to_string(),
                "summary.teams.recordCount".to_string(),
                "summary.orgs.recordCount".to_string(),
                "summary.serviceAccounts.recordCount".to_string(),
            ]
        );
        assert!(domain.blockers.is_empty());
        assert_eq!(domain.warnings.len(), 2);
        assert_eq!(
            domain.next_actions,
            vec![
                "export the missing access bundle kinds: teams, service accounts".to_string(),
                "add the missing access export bundles before using this workspace as one mixed workspace handoff".to_string(),
                "re-run workspace test after the missing access bundles are exported".to_string(),
            ]
        );
    }

    #[test]
    fn build_access_domain_status_reports_ready_when_all_bundles_have_records() {
        let user_document = json!({"summary": {"recordCount": 2}});
        let team_document = json!({"summary": {"recordCount": 3}});
        let org_document = json!({"summary": {"recordCount": 1}});
        let service_account_document = json!({"summary": {"recordCount": 4}});
        let domain = build_access_domain_status(AccessDomainStatusInputs {
            user_export_document: Some(&user_document),
            team_export_document: Some(&team_document),
            org_export_document: Some(&org_document),
            service_account_export_document: Some(&service_account_document),
        })
        .unwrap();

        assert_eq!(domain.status, PROJECT_STATUS_READY);
        assert_eq!(domain.reason_code, "ready");
        assert_eq!(domain.primary_count, 10);
        assert_eq!(domain.warning_count, 0);
        assert!(domain.warnings.is_empty());
        assert_eq!(
            domain.next_actions,
            vec!["re-run access export after membership changes".to_string()]
        );
    }
}
