//! Live access domain-status producer.
//!
//! Maintainer note:
//! - This module derives one access-owned domain-status row from live request
//!   surfaces instead of staged export bundles.
//! - Keep the producer conservative: it should only report scope readability,
//!   record counts, and a small set of review-oriented drift signals from the
//!   same live surfaces.
#![allow(dead_code)]

use reqwest::Method;
use serde_json::Value;

use crate::common::Result;
use crate::grafana_api::AccessResourceClient;
use crate::http::JsonHttpClient;
use crate::project_status::{
    status_finding, ProjectDomainStatus, ProjectStatusFinding, PROJECT_STATUS_READY,
};

use super::render::{normalize_org_role, scalar_text, value_bool};
use super::{request_object_list_field, DEFAULT_PAGE_SIZE};
use super::{
    team::iter_teams_with_request,
    user::{iter_global_users_with_request, list_org_users_with_request},
};

#[path = "live_project_status_build.rs"]
mod live_project_status_build;
#[path = "live_project_status_read.rs"]
mod live_project_status_read;
#[path = "live_project_status_review.rs"]
mod live_project_status_review;

use live_project_status_review::{
    build_service_account_review_signals, build_team_review_signals, build_user_review_signals,
};

const ACCESS_DOMAIN_ID: &str = "access";
const ACCESS_SCOPE: &str = "live";
const ACCESS_MODE: &str = "live-list-surfaces";
const ACCESS_REASON_READY: &str = PROJECT_STATUS_READY;
const ACCESS_REASON_PARTIAL_NO_DATA: &str = "partial-no-data";
const ACCESS_REASON_PARTIAL_LIVE_SCOPES: &str = "partial-live-scopes";

const ACCESS_SIGNAL_KEYS: &[&str] = &[
    "live.users.count",
    "live.users.identityGapCount",
    "live.users.adminCount",
    "live.teams.count",
    "live.teams.emailGapCount",
    "live.teams.emptyCount",
    "live.orgs.count",
    "live.serviceAccounts.count",
    "live.serviceAccounts.roleGapCount",
    "live.serviceAccounts.disabledCount",
    "live.serviceAccounts.tokenlessCount",
];

const ACCESS_SOURCE_KIND_LIVE_ORG_USERS: &str = "grafana-utils-access-live-org-users";
const ACCESS_SOURCE_KIND_LIVE_GLOBAL_USERS: &str = "grafana-utils-access-live-global-users";
const ACCESS_SOURCE_KIND_LIVE_TEAMS: &str = "grafana-utils-access-live-teams";
const ACCESS_SOURCE_KIND_LIVE_ORGS: &str = "grafana-utils-access-live-orgs";
const ACCESS_SOURCE_KIND_LIVE_SERVICE_ACCOUNTS: &str = "grafana-utils-access-live-service-accounts";

const ACCESS_FINDING_KIND_USERS_COUNT: &str = "live-users-count";
const ACCESS_FINDING_KIND_USERS_IDENTITY_GAP: &str = "live-users-identity-gap";
const ACCESS_FINDING_KIND_USERS_UNREADABLE: &str = "live-users-unreadable";
const ACCESS_FINDING_KIND_USERS_ADMIN_COUNT: &str = "live-users-admin-count";
const ACCESS_FINDING_KIND_TEAMS_COUNT: &str = "live-teams-count";
const ACCESS_FINDING_KIND_TEAMS_EMAIL_GAP: &str = "live-teams-email-gap";
const ACCESS_FINDING_KIND_TEAMS_UNREADABLE: &str = "live-teams-unreadable";
const ACCESS_FINDING_KIND_TEAMS_EMPTY_COUNT: &str = "live-teams-empty-count";
const ACCESS_FINDING_KIND_ORGS_COUNT: &str = "live-orgs-count";
const ACCESS_FINDING_KIND_ORGS_UNREADABLE: &str = "live-orgs-unreadable";
const ACCESS_FINDING_KIND_SERVICE_ACCOUNTS_COUNT: &str = "live-service-accounts-count";
const ACCESS_FINDING_KIND_SERVICE_ACCOUNTS_ROLE_GAP: &str = "live-service-accounts-role-gap";
const ACCESS_FINDING_KIND_SERVICE_ACCOUNTS_UNREADABLE: &str = "live-service-accounts-unreadable";
const ACCESS_FINDING_KIND_SERVICE_ACCOUNTS_DISABLED_COUNT: &str =
    "live-service-accounts-disabled-count";
const ACCESS_FINDING_KIND_SERVICE_ACCOUNTS_TOKENLESS_COUNT: &str =
    "live-service-accounts-tokenless-count";

const ACCESS_READY_NEXT_ACTIONS: &[&str] = &["re-run live access status after access changes"];
const ACCESS_NO_DATA_NEXT_ACTIONS: &[&str] =
    &["read at least one live access record before re-running live access status"];

#[derive(Debug, Clone, Copy)]
enum LiveReviewSignalGroup {
    ImportReview,
    DriftSeverity,
}

#[derive(Debug, Clone)]
struct LiveScopeReviewSignal {
    group: LiveReviewSignalGroup,
    label: &'static str,
    signal_key: &'static str,
    finding_kind: &'static str,
    count: usize,
}

impl LiveScopeReviewSignal {
    fn new(
        group: LiveReviewSignalGroup,
        label: &'static str,
        signal_key: &'static str,
        finding_kind: &'static str,
        count: usize,
    ) -> Self {
        Self {
            group,
            label,
            signal_key,
            finding_kind,
            count,
        }
    }

    fn finding(&self) -> ProjectStatusFinding {
        status_finding(self.finding_kind, self.count, self.signal_key)
    }
}

#[derive(Debug, Clone)]
struct LiveScopeReading {
    label: &'static str,
    source_kind: Option<&'static str>,
    signal_key: &'static str,
    readable_finding_kind: &'static str,
    unreadable_finding_kind: &'static str,
    count: usize,
    review_signals: Vec<LiveScopeReviewSignal>,
}

impl LiveScopeReading {
    fn readable(
        label: &'static str,
        source_kind: &'static str,
        signal_key: &'static str,
        readable_finding_kind: &'static str,
        count: usize,
        review_signals: Vec<LiveScopeReviewSignal>,
    ) -> Self {
        Self {
            label,
            source_kind: Some(source_kind),
            signal_key,
            readable_finding_kind,
            unreadable_finding_kind: "",
            count,
            review_signals,
        }
    }

    fn unreadable(
        label: &'static str,
        signal_key: &'static str,
        unreadable_finding_kind: &'static str,
    ) -> Self {
        Self {
            label,
            source_kind: None,
            signal_key,
            readable_finding_kind: "",
            unreadable_finding_kind,
            count: 0,
            review_signals: Vec::new(),
        }
    }

    fn is_readable(&self) -> bool {
        self.source_kind.is_some()
    }

    fn finding(&self) -> ProjectStatusFinding {
        if self.is_readable() {
            status_finding(self.readable_finding_kind, self.count, self.signal_key)
        } else {
            status_finding(self.unreadable_finding_kind, 1, self.signal_key)
        }
    }
}

pub(crate) fn build_access_live_domain_status_with_request<F>(
    mut request_json: F,
) -> Option<ProjectDomainStatus>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let readings = [
        live_project_status_read::read_live_users_with_request(&mut request_json),
        live_project_status_read::read_live_teams_with_request(&mut request_json),
        live_project_status_read::read_live_orgs_with_request(&mut request_json),
        live_project_status_read::read_live_service_accounts_with_request(&mut request_json),
    ];
    live_project_status_build::build_access_live_domain_status_from_readings(&readings)
}

pub(crate) fn build_access_live_domain_status(
    client: &JsonHttpClient,
) -> Option<ProjectDomainStatus> {
    let access_client = AccessResourceClient::new(client);
    let readings = [
        live_project_status_read::read_live_users(&access_client),
        live_project_status_read::read_live_teams(&access_client),
        live_project_status_read::read_live_orgs(client),
        live_project_status_read::read_live_service_accounts(&access_client),
    ];
    live_project_status_build::build_access_live_domain_status_from_readings(&readings)
}

#[cfg(test)]
mod tests {
    use super::build_access_live_domain_status_with_request;
    use crate::common::message;
    use crate::project_status::{PROJECT_STATUS_PARTIAL, PROJECT_STATUS_READY};
    use reqwest::Method;
    use serde_json::json;

    #[test]
    fn build_access_live_domain_status_reports_readable_scopes_and_counts() {
        let domain =
            build_access_live_domain_status_with_request(|method, path, params, _payload| {
                match (method, path) {
                    (Method::GET, "/api/org/users") => Ok(Some(json!([
                        {"id": 1, "login": "alice", "email": "alice@example.com", "role": "Viewer"},
                        {"id": 2, "login": "bob", "email": "bob@example.com", "role": "Viewer"}
                    ]))),
                    (Method::GET, "/api/teams/search") => {
                        assert!(params
                            .iter()
                            .any(|(key, value)| key == "page" && value == "1"));
                        Ok(Some(json!({"teams": [{"id": 11, "name": "Ops", "email": "ops@example.com", "memberCount": 1}]})))
                    }
                    (Method::GET, "/api/orgs") => Ok(Some(json!([
                        {"id": 101},
                        {"id": 102},
                        {"id": 103}
                    ]))),
                    (Method::GET, "/api/serviceaccounts/search") => {
                        assert!(params
                            .iter()
                            .any(|(key, value)| key == "page" && value == "1"));
                        Ok(Some(json!({
                            "serviceAccounts": [
                                {"id": 21, "name": "ci", "login": "sa-ci", "role": "Viewer", "isDisabled": false, "tokens": 1},
                                {"id": 22, "name": "bot", "login": "sa-bot", "role": "Viewer", "isDisabled": false, "tokens": 1},
                                {"id": 23, "name": "deploy", "login": "sa-deploy", "role": "Viewer", "isDisabled": false, "tokens": 2},
                                {"id": 24, "name": "ops", "login": "sa-ops", "role": "Viewer", "isDisabled": false, "tokens": 3}
                            ]
                        })))
                    }
                    _ => panic!("unexpected path {path}"),
                }
            })
            .unwrap();

        assert_eq!(domain.id, "access");
        assert_eq!(domain.scope, "live");
        assert_eq!(domain.mode, "live-list-surfaces");
        assert_eq!(domain.status, "ready");
        assert_eq!(domain.reason_code, "ready");
        assert_eq!(domain.primary_count, 10);
        assert_eq!(domain.blocker_count, 0);
        assert_eq!(domain.warning_count, 10);
        assert_eq!(
            domain.source_kinds,
            vec![
                "grafana-utils-access-live-org-users".to_string(),
                "grafana-utils-access-live-teams".to_string(),
                "grafana-utils-access-live-orgs".to_string(),
                "grafana-utils-access-live-service-accounts".to_string(),
            ]
        );
        assert_eq!(
            domain.signal_keys,
            vec![
                "live.users.count".to_string(),
                "live.users.identityGapCount".to_string(),
                "live.users.adminCount".to_string(),
                "live.teams.count".to_string(),
                "live.teams.emailGapCount".to_string(),
                "live.teams.emptyCount".to_string(),
                "live.orgs.count".to_string(),
                "live.serviceAccounts.count".to_string(),
                "live.serviceAccounts.roleGapCount".to_string(),
                "live.serviceAccounts.disabledCount".to_string(),
                "live.serviceAccounts.tokenlessCount".to_string(),
            ]
        );
        assert_eq!(
            domain.next_actions,
            vec!["re-run live access status after access changes".to_string()]
        );
        assert_eq!(domain.warnings.len(), 4);
        assert_eq!(domain.warnings[0].kind, "live-users-count");
        assert_eq!(domain.warnings[0].count, 2);
        assert_eq!(domain.warnings[0].source, "live.users.count");
        assert_eq!(domain.warnings[1].kind, "live-teams-count");
        assert_eq!(domain.warnings[1].count, 1);
        assert_eq!(domain.warnings[1].source, "live.teams.count");
        assert_eq!(domain.warnings[2].kind, "live-orgs-count");
        assert_eq!(domain.warnings[2].count, 3);
        assert_eq!(domain.warnings[2].source, "live.orgs.count");
        assert_eq!(domain.warnings[3].kind, "live-service-accounts-count");
        assert_eq!(domain.warnings[3].count, 4);
        assert_eq!(domain.warnings[3].source, "live.serviceAccounts.count");
    }

    #[test]
    fn build_access_live_domain_status_reports_review_signals_from_live_surfaces() {
        let domain = build_access_live_domain_status_with_request(
            |method, path, _params, _payload| match (method, path) {
                (Method::GET, "/api/org/users") => Ok(Some(json!([
                    {"id": 1, "login": "alice", "email": "alice@example.com", "role": "Admin"},
                    {"id": 2, "login": "bob", "email": "", "role": "Viewer"}
                ]))),
                (Method::GET, "/api/teams/search") => Ok(Some(json!({"teams": [
                    {"id": 11, "name": "Ops", "email": "", "memberCount": 0},
                    {"id": 12, "name": "Platform", "email": "platform@example.com", "memberCount": 3}
                ]}))),
                (Method::GET, "/api/orgs") => Ok(Some(json!([
                    {"id": 101}
                ]))),
                (Method::GET, "/api/serviceaccounts/search") => Ok(Some(json!({
                    "serviceAccounts": [
                        {"id": 21, "name": "ci", "login": "sa-ci", "role": "Viewer", "isDisabled": true, "tokens": 1},
                        {"id": 22, "name": "bot", "login": "sa-bot", "role": "", "isDisabled": false, "tokens": 0},
                        {"id": 23, "name": "active", "login": "sa-active", "role": "Viewer", "isDisabled": false, "tokens": 2}
                    ]
                }))),
                _ => panic!("unexpected path {path}"),
            },
        )
        .unwrap();

        assert_eq!(domain.status, PROJECT_STATUS_READY);
        assert_eq!(domain.reason_code, "ready");
        assert_eq!(domain.primary_count, 8);
        assert_eq!(domain.warning_count, 15);
        assert_eq!(
            domain.signal_keys,
            vec![
                "live.users.count".to_string(),
                "live.users.identityGapCount".to_string(),
                "live.users.adminCount".to_string(),
                "live.teams.count".to_string(),
                "live.teams.emailGapCount".to_string(),
                "live.teams.emptyCount".to_string(),
                "live.orgs.count".to_string(),
                "live.serviceAccounts.count".to_string(),
                "live.serviceAccounts.roleGapCount".to_string(),
                "live.serviceAccounts.disabledCount".to_string(),
                "live.serviceAccounts.tokenlessCount".to_string(),
            ]
        );
        assert_eq!(
            domain.next_actions,
            vec![
                "review live access import-review signals: users missing login or email, teams missing email, service accounts missing role".to_string(),
                "review live access drift-severity signals: admin users, empty teams, disabled service accounts, tokenless service accounts".to_string(),
                "re-run live access status after access changes".to_string(),
            ]
        );
        assert_eq!(domain.warnings.len(), 11);
        assert_eq!(domain.warnings[0].kind, "live-users-count");
        assert_eq!(domain.warnings[0].count, 2);
        assert_eq!(domain.warnings[0].source, "live.users.count");
        assert_eq!(domain.warnings[1].kind, "live-users-identity-gap");
        assert_eq!(domain.warnings[1].count, 1);
        assert_eq!(domain.warnings[1].source, "live.users.identityGapCount");
        assert_eq!(domain.warnings[2].kind, "live-users-admin-count");
        assert_eq!(domain.warnings[2].count, 1);
        assert_eq!(domain.warnings[2].source, "live.users.adminCount");
        assert_eq!(domain.warnings[3].kind, "live-teams-count");
        assert_eq!(domain.warnings[3].count, 2);
        assert_eq!(domain.warnings[3].source, "live.teams.count");
        assert_eq!(domain.warnings[4].kind, "live-teams-email-gap");
        assert_eq!(domain.warnings[4].count, 1);
        assert_eq!(domain.warnings[4].source, "live.teams.emailGapCount");
        assert_eq!(domain.warnings[5].kind, "live-teams-empty-count");
        assert_eq!(domain.warnings[5].count, 1);
        assert_eq!(domain.warnings[5].source, "live.teams.emptyCount");
        assert_eq!(domain.warnings[6].kind, "live-orgs-count");
        assert_eq!(domain.warnings[6].count, 1);
        assert_eq!(domain.warnings[6].source, "live.orgs.count");
        assert_eq!(domain.warnings[7].kind, "live-service-accounts-count");
        assert_eq!(domain.warnings[7].count, 3);
        assert_eq!(domain.warnings[7].source, "live.serviceAccounts.count");
        assert_eq!(domain.warnings[8].kind, "live-service-accounts-role-gap");
        assert_eq!(domain.warnings[8].count, 1);
        assert_eq!(
            domain.warnings[8].source,
            "live.serviceAccounts.roleGapCount"
        );
        assert_eq!(
            domain.warnings[9].kind,
            "live-service-accounts-disabled-count"
        );
        assert_eq!(domain.warnings[9].count, 1);
        assert_eq!(
            domain.warnings[9].source,
            "live.serviceAccounts.disabledCount"
        );
        assert_eq!(
            domain.warnings[10].kind,
            "live-service-accounts-tokenless-count"
        );
        assert_eq!(domain.warnings[10].count, 1);
        assert_eq!(
            domain.warnings[10].source,
            "live.serviceAccounts.tokenlessCount"
        );
    }

    #[test]
    fn build_access_live_domain_status_reports_partial_when_some_scopes_are_unreadable() {
        let domain = build_access_live_domain_status_with_request(
            |method, path, _params, _payload| match (method, path) {
                (Method::GET, "/api/org/users") => Err(message("org users forbidden")),
                (Method::GET, "/api/users") => Ok(Some(json!([
                    {"id": 7, "login": "alice", "email": "alice@example.com", "role": "Viewer"},
                    {"id": 8, "login": "bob", "email": "bob@example.com", "role": "Viewer"},
                    {"id": 9, "login": "carol", "email": "carol@example.com", "role": "Viewer"}
                ]))),
                (Method::GET, "/api/teams/search") => Err(message("team search forbidden")),
                (Method::GET, "/api/orgs") => Ok(Some(json!([
                    {"id": 101}
                ]))),
                (Method::GET, "/api/serviceaccounts/search") => Ok(Some(json!({
                    "serviceAccounts": [
                        {"id": 31, "name": "ci", "login": "sa-ci", "role": "Viewer", "isDisabled": false, "tokens": 1},
                        {"id": 32, "name": "bot", "login": "sa-bot", "role": "Viewer", "isDisabled": false, "tokens": 1}
                    ]
                }))),
                _ => panic!("unexpected path {path}"),
            },
        )
        .unwrap();

        assert_eq!(domain.status, PROJECT_STATUS_PARTIAL);
        assert_eq!(domain.reason_code, "partial-live-scopes");
        assert_eq!(domain.primary_count, 6);
        assert_eq!(
            domain.source_kinds,
            vec![
                "grafana-utils-access-live-global-users".to_string(),
                "grafana-utils-access-live-orgs".to_string(),
                "grafana-utils-access-live-service-accounts".to_string(),
            ]
        );
        assert_eq!(
            domain.next_actions,
            vec!["restore access to unreadable live scopes: teams".to_string()]
        );
        assert_eq!(domain.warnings.len(), 4);
        assert_eq!(domain.warnings[0].kind, "live-users-count");
        assert_eq!(domain.warnings[0].count, 3);
        assert_eq!(domain.warnings[0].source, "live.users.count");
        assert_eq!(domain.warnings[1].kind, "live-teams-unreadable");
        assert_eq!(domain.warnings[1].count, 1);
        assert_eq!(domain.warnings[1].source, "live.teams.count");
        assert_eq!(domain.warnings[2].kind, "live-orgs-count");
        assert_eq!(domain.warnings[2].count, 1);
        assert_eq!(domain.warnings[2].source, "live.orgs.count");
        assert_eq!(domain.warnings[3].kind, "live-service-accounts-count");
        assert_eq!(domain.warnings[3].count, 2);
        assert_eq!(domain.warnings[3].source, "live.serviceAccounts.count");
    }

    #[test]
    fn build_access_live_domain_status_reports_partial_no_data_when_counts_are_zero() {
        let domain = build_access_live_domain_status_with_request(
            |method, path, _params, _payload| match (method, path) {
                (Method::GET, "/api/org/users") => Ok(Some(json!([]))),
                (Method::GET, "/api/teams/search") => Ok(Some(json!({"teams": []}))),
                (Method::GET, "/api/orgs") => Ok(Some(json!([]))),
                (Method::GET, "/api/serviceaccounts/search") => {
                    Ok(Some(json!({"serviceAccounts": []})))
                }
                _ => panic!("unexpected path {path}"),
            },
        )
        .unwrap();

        assert_eq!(domain.status, PROJECT_STATUS_PARTIAL);
        assert_eq!(domain.reason_code, "partial-no-data");
        assert_eq!(domain.primary_count, 0);
        assert_eq!(
            domain.next_actions,
            vec![
                "read at least one live access record before re-running live access status"
                    .to_string()
            ]
        );
        assert_eq!(domain.warnings.len(), 4);
        assert_eq!(domain.warnings[0].kind, "live-users-count");
        assert_eq!(domain.warnings[0].count, 0);
        assert_eq!(domain.warnings[0].source, "live.users.count");
        assert_eq!(domain.warnings[1].kind, "live-teams-count");
        assert_eq!(domain.warnings[1].count, 0);
        assert_eq!(domain.warnings[1].source, "live.teams.count");
        assert_eq!(domain.warnings[2].kind, "live-orgs-count");
        assert_eq!(domain.warnings[2].count, 0);
        assert_eq!(domain.warnings[2].source, "live.orgs.count");
        assert_eq!(domain.warnings[3].kind, "live-service-accounts-count");
        assert_eq!(domain.warnings[3].count, 0);
        assert_eq!(domain.warnings[3].source, "live.serviceAccounts.count");
    }
}
