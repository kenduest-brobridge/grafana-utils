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
use serde_json::{Map, Value};

use crate::common::Result;
use crate::grafana_api::{project_status_live as project_status_live_support, AccessResourceClient};
use crate::http::JsonHttpClient;
use crate::project_status::{
    status_finding, ProjectDomainStatus, ProjectStatusFinding, PROJECT_STATUS_PARTIAL,
    PROJECT_STATUS_READY,
};

use super::render::{normalize_org_role, scalar_text, value_bool};
use super::{request_object_list_field, DEFAULT_PAGE_SIZE};
use super::{
    team::iter_teams_with_request,
    user::{iter_global_users_with_request, list_org_users_with_request},
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

fn is_admin_user(user: &Map<String, Value>) -> bool {
    normalize_org_role(user.get("role").or_else(|| user.get("orgRole"))) == "Admin"
        || value_bool(user.get("isGrafanaAdmin"))
            .or_else(|| value_bool(user.get("isAdmin")))
            .unwrap_or(false)
}

fn build_user_review_signals(users: &[Map<String, Value>]) -> Vec<LiveScopeReviewSignal> {
    let identity_gap_count = users
        .iter()
        .filter(|user| {
            scalar_text(user.get("login")).trim().is_empty()
                || scalar_text(user.get("email")).trim().is_empty()
        })
        .count();
    let admin_count = users.iter().filter(|user| is_admin_user(user)).count();
    let mut review_signals = Vec::new();
    if identity_gap_count > 0 {
        review_signals.push(LiveScopeReviewSignal::new(
            LiveReviewSignalGroup::ImportReview,
            "users missing login or email",
            "live.users.identityGapCount",
            ACCESS_FINDING_KIND_USERS_IDENTITY_GAP,
            identity_gap_count,
        ));
    }
    if admin_count > 0 {
        review_signals.push(LiveScopeReviewSignal::new(
            LiveReviewSignalGroup::DriftSeverity,
            "admin users",
            "live.users.adminCount",
            ACCESS_FINDING_KIND_USERS_ADMIN_COUNT,
            admin_count,
        ));
    }
    review_signals
}

fn build_team_review_signals(teams: &[Map<String, Value>]) -> Vec<LiveScopeReviewSignal> {
    let email_gap_count = teams
        .iter()
        .filter(|team| scalar_text(team.get("email")).trim().is_empty())
        .count();
    let empty_count = teams
        .iter()
        .filter(|team| {
            scalar_text(team.get("memberCount"))
                .parse::<usize>()
                .unwrap_or(0)
                == 0
        })
        .count();
    let mut review_signals = Vec::new();
    if email_gap_count > 0 {
        review_signals.push(LiveScopeReviewSignal::new(
            LiveReviewSignalGroup::ImportReview,
            "teams missing email",
            "live.teams.emailGapCount",
            ACCESS_FINDING_KIND_TEAMS_EMAIL_GAP,
            email_gap_count,
        ));
    }
    if empty_count > 0 {
        review_signals.push(LiveScopeReviewSignal::new(
            LiveReviewSignalGroup::DriftSeverity,
            "empty teams",
            "live.teams.emptyCount",
            ACCESS_FINDING_KIND_TEAMS_EMPTY_COUNT,
            empty_count,
        ));
    }
    review_signals
}

fn build_service_account_review_signals(
    service_accounts: &[Map<String, Value>],
) -> Vec<LiveScopeReviewSignal> {
    let role_gap_count = service_accounts
        .iter()
        .filter(|service_account| scalar_text(service_account.get("role")).trim().is_empty())
        .count();
    let disabled_count = service_accounts
        .iter()
        .filter(|service_account| {
            value_bool(service_account.get("disabled"))
                .or_else(|| value_bool(service_account.get("isDisabled")))
                .unwrap_or(false)
        })
        .count();
    let tokenless_count = service_accounts
        .iter()
        .filter(|service_account| scalar_text(service_account.get("tokens")) == "0")
        .count();
    let mut review_signals = Vec::new();
    if role_gap_count > 0 {
        review_signals.push(LiveScopeReviewSignal::new(
            LiveReviewSignalGroup::ImportReview,
            "service accounts missing role",
            "live.serviceAccounts.roleGapCount",
            ACCESS_FINDING_KIND_SERVICE_ACCOUNTS_ROLE_GAP,
            role_gap_count,
        ));
    }
    if disabled_count > 0 {
        review_signals.push(LiveScopeReviewSignal::new(
            LiveReviewSignalGroup::DriftSeverity,
            "disabled service accounts",
            "live.serviceAccounts.disabledCount",
            ACCESS_FINDING_KIND_SERVICE_ACCOUNTS_DISABLED_COUNT,
            disabled_count,
        ));
    }
    if tokenless_count > 0 {
        review_signals.push(LiveScopeReviewSignal::new(
            LiveReviewSignalGroup::DriftSeverity,
            "tokenless service accounts",
            "live.serviceAccounts.tokenlessCount",
            ACCESS_FINDING_KIND_SERVICE_ACCOUNTS_TOKENLESS_COUNT,
            tokenless_count,
        ));
    }
    review_signals
}

fn list_live_service_accounts_with_request<F>(
    mut request_json: F,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut rows = Vec::new();
    let mut page = 1usize;
    loop {
        let params = vec![
            ("query".to_string(), String::new()),
            ("page".to_string(), page.to_string()),
            ("perpage".to_string(), DEFAULT_PAGE_SIZE.to_string()),
        ];
        let batch = request_object_list_field(
            &mut request_json,
            Method::GET,
            "/api/serviceaccounts/search",
            &params,
            None,
            "serviceAccounts",
            (
                "Unexpected service-account list response from Grafana.",
                "Unexpected service-account list response from Grafana.",
            ),
        )?;
        let batch_len = batch.len();
        rows.extend(batch);
        if batch_len < DEFAULT_PAGE_SIZE {
            break;
        }
        page += 1;
    }
    Ok(rows)
}

fn read_live_users_with_request<F>(request_json: &mut F) -> LiveScopeReading
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if let Ok(users) = list_org_users_with_request(&mut *request_json) {
        return LiveScopeReading::readable(
            "users",
            ACCESS_SOURCE_KIND_LIVE_ORG_USERS,
            "live.users.count",
            ACCESS_FINDING_KIND_USERS_COUNT,
            users.len(),
            build_user_review_signals(&users),
        );
    }
    if let Ok(users) = iter_global_users_with_request(&mut *request_json, DEFAULT_PAGE_SIZE) {
        return LiveScopeReading::readable(
            "users",
            ACCESS_SOURCE_KIND_LIVE_GLOBAL_USERS,
            "live.users.count",
            ACCESS_FINDING_KIND_USERS_COUNT,
            users.len(),
            build_user_review_signals(&users),
        );
    }
    LiveScopeReading::unreadable(
        "users",
        "live.users.count",
        ACCESS_FINDING_KIND_USERS_UNREADABLE,
    )
}

fn read_live_users(client: &AccessResourceClient<'_>) -> LiveScopeReading {
    if let Ok(users) = client.list_org_users() {
        return LiveScopeReading::readable(
            "users",
            ACCESS_SOURCE_KIND_LIVE_ORG_USERS,
            "live.users.count",
            ACCESS_FINDING_KIND_USERS_COUNT,
            users.len(),
            build_user_review_signals(&users),
        );
    }
    if let Ok(users) = client.iter_global_users(DEFAULT_PAGE_SIZE) {
        return LiveScopeReading::readable(
            "users",
            ACCESS_SOURCE_KIND_LIVE_GLOBAL_USERS,
            "live.users.count",
            ACCESS_FINDING_KIND_USERS_COUNT,
            users.len(),
            build_user_review_signals(&users),
        );
    }
    LiveScopeReading::unreadable(
        "users",
        "live.users.count",
        ACCESS_FINDING_KIND_USERS_UNREADABLE,
    )
}

fn read_live_teams_with_request<F>(request_json: &mut F) -> LiveScopeReading
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match iter_teams_with_request(&mut *request_json, None) {
        Ok(teams) => LiveScopeReading::readable(
            "teams",
            ACCESS_SOURCE_KIND_LIVE_TEAMS,
            "live.teams.count",
            ACCESS_FINDING_KIND_TEAMS_COUNT,
            teams.len(),
            build_team_review_signals(&teams),
        ),
        Err(_error) => LiveScopeReading::unreadable(
            "teams",
            "live.teams.count",
            ACCESS_FINDING_KIND_TEAMS_UNREADABLE,
        ),
    }
}

fn read_live_teams(client: &AccessResourceClient<'_>) -> LiveScopeReading {
    match client.iter_teams(None, DEFAULT_PAGE_SIZE) {
        Ok(teams) => LiveScopeReading::readable(
            "teams",
            ACCESS_SOURCE_KIND_LIVE_TEAMS,
            "live.teams.count",
            ACCESS_FINDING_KIND_TEAMS_COUNT,
            teams.len(),
            build_team_review_signals(&teams),
        ),
        Err(_error) => LiveScopeReading::unreadable(
            "teams",
            "live.teams.count",
            ACCESS_FINDING_KIND_TEAMS_UNREADABLE,
        ),
    }
}

fn read_live_orgs_with_request<F>(request_json: &mut F) -> LiveScopeReading
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match project_status_live_support::list_visible_orgs_with_request(request_json) {
        Ok(orgs) => {
            let orgs: Vec<Map<String, Value>> = orgs;
            LiveScopeReading::readable(
                "orgs",
                ACCESS_SOURCE_KIND_LIVE_ORGS,
                "live.orgs.count",
                ACCESS_FINDING_KIND_ORGS_COUNT,
                orgs.len(),
                Vec::new(),
            )
        }
        Err(_error) => LiveScopeReading::unreadable(
            "orgs",
            "live.orgs.count",
            ACCESS_FINDING_KIND_ORGS_UNREADABLE,
        ),
    }
}

fn read_live_service_accounts_with_request<F>(request_json: &mut F) -> LiveScopeReading
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match list_live_service_accounts_with_request(&mut *request_json) {
        Ok(service_accounts) => LiveScopeReading::readable(
            "service accounts",
            ACCESS_SOURCE_KIND_LIVE_SERVICE_ACCOUNTS,
            "live.serviceAccounts.count",
            ACCESS_FINDING_KIND_SERVICE_ACCOUNTS_COUNT,
            service_accounts.len(),
            build_service_account_review_signals(&service_accounts),
        ),
        Err(_error) => LiveScopeReading::unreadable(
            "service accounts",
            "live.serviceAccounts.count",
            ACCESS_FINDING_KIND_SERVICE_ACCOUNTS_UNREADABLE,
        ),
    }
}

fn read_live_service_accounts(client: &AccessResourceClient<'_>) -> LiveScopeReading {
    match client.list_service_accounts(DEFAULT_PAGE_SIZE) {
        Ok(service_accounts) => LiveScopeReading::readable(
            "service accounts",
            ACCESS_SOURCE_KIND_LIVE_SERVICE_ACCOUNTS,
            "live.serviceAccounts.count",
            ACCESS_FINDING_KIND_SERVICE_ACCOUNTS_COUNT,
            service_accounts.len(),
            build_service_account_review_signals(&service_accounts),
        ),
        Err(_error) => LiveScopeReading::unreadable(
            "service accounts",
            "live.serviceAccounts.count",
            ACCESS_FINDING_KIND_SERVICE_ACCOUNTS_UNREADABLE,
        ),
    }
}

fn build_review_next_actions(readings: &[LiveScopeReading]) -> Vec<String> {
    let import_review_labels = readings
        .iter()
        .flat_map(|reading| reading.review_signals.iter())
        .filter(|signal| matches!(signal.group, LiveReviewSignalGroup::ImportReview))
        .map(|signal| signal.label)
        .collect::<Vec<&str>>();
    let drift_severity_labels = readings
        .iter()
        .flat_map(|reading| reading.review_signals.iter())
        .filter(|signal| matches!(signal.group, LiveReviewSignalGroup::DriftSeverity))
        .map(|signal| signal.label)
        .collect::<Vec<&str>>();
    let mut next_actions = Vec::new();
    if !import_review_labels.is_empty() {
        next_actions.push(format!(
            "review live access import-review signals: {}",
            import_review_labels.join(", ")
        ));
    }
    if !drift_severity_labels.is_empty() {
        next_actions.push(format!(
            "review live access drift-severity signals: {}",
            drift_severity_labels.join(", ")
        ));
    }
    next_actions
}

fn build_next_actions(readings: &[LiveScopeReading], total_count: usize) -> Vec<String> {
    let unreadable_labels = readings
        .iter()
        .filter(|reading| !reading.is_readable())
        .map(|reading| reading.label)
        .collect::<Vec<&str>>();
    let mut next_actions = Vec::new();
    if !unreadable_labels.is_empty() {
        next_actions.push(format!(
            "restore access to unreadable live scopes: {}",
            unreadable_labels.join(", ")
        ));
    }
    next_actions.extend(build_review_next_actions(readings));
    if total_count == 0 {
        next_actions.extend(
            ACCESS_NO_DATA_NEXT_ACTIONS
                .iter()
                .map(|item| (*item).to_string()),
        );
    } else if unreadable_labels.is_empty() {
        next_actions.extend(
            ACCESS_READY_NEXT_ACTIONS
                .iter()
                .map(|item| (*item).to_string()),
        );
    }
    next_actions
}

pub(crate) fn build_access_live_domain_status_with_request<F>(
    mut request_json: F,
) -> Option<ProjectDomainStatus>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let readings = [
        read_live_users_with_request(&mut request_json),
        read_live_teams_with_request(&mut request_json),
        read_live_orgs_with_request(&mut request_json),
        read_live_service_accounts_with_request(&mut request_json),
    ];

    let mut source_kinds = Vec::new();
    let mut warnings = Vec::new();
    let mut total_count = 0usize;
    let mut unreadable_count = 0usize;

    for reading in &readings {
        if let Some(source_kind) = reading.source_kind {
            source_kinds.push(source_kind.to_string());
            total_count += reading.count;
        } else {
            unreadable_count += 1;
        }
        warnings.push(reading.finding());
        warnings.extend(
            reading
                .review_signals
                .iter()
                .map(LiveScopeReviewSignal::finding),
        );
    }

    let (status, reason_code) = if unreadable_count > 0 {
        (PROJECT_STATUS_PARTIAL, ACCESS_REASON_PARTIAL_LIVE_SCOPES)
    } else if total_count == 0 {
        (PROJECT_STATUS_PARTIAL, ACCESS_REASON_PARTIAL_NO_DATA)
    } else {
        (PROJECT_STATUS_READY, ACCESS_REASON_READY)
    };

    Some(ProjectDomainStatus {
        id: ACCESS_DOMAIN_ID.to_string(),
        scope: ACCESS_SCOPE.to_string(),
        mode: ACCESS_MODE.to_string(),
        status: status.to_string(),
        reason_code: reason_code.to_string(),
        primary_count: total_count,
        blocker_count: 0,
        warning_count: warnings.iter().map(|item| item.count).sum(),
        source_kinds,
        signal_keys: ACCESS_SIGNAL_KEYS
            .iter()
            .map(|item| (*item).to_string())
            .collect(),
        blockers: Vec::new(),
        warnings,
        next_actions: build_next_actions(&readings, total_count),
        freshness: Default::default(),
    })
}

pub(crate) fn build_access_live_domain_status(
    client: &JsonHttpClient,
) -> Option<ProjectDomainStatus> {
    let access_client = AccessResourceClient::new(client);
    let readings = [
        read_live_users(&access_client),
        read_live_teams(&access_client),
        match project_status_live_support::list_visible_orgs(client) {
            Ok(orgs) => {
                let orgs: Vec<Map<String, Value>> = orgs;
                LiveScopeReading::readable(
                    "orgs",
                    ACCESS_SOURCE_KIND_LIVE_ORGS,
                    "live.orgs.count",
                    ACCESS_FINDING_KIND_ORGS_COUNT,
                    orgs.len(),
                    Vec::new(),
                )
            }
            Err(_error) => LiveScopeReading::unreadable(
                "orgs",
                "live.orgs.count",
                ACCESS_FINDING_KIND_ORGS_UNREADABLE,
            ),
        },
        read_live_service_accounts(&access_client),
    ];

    let mut source_kinds = Vec::new();
    let mut warnings = Vec::new();
    let mut total_count = 0usize;
    let mut unreadable_count = 0usize;

    for reading in &readings {
        if let Some(source_kind) = reading.source_kind {
            source_kinds.push(source_kind.to_string());
            total_count += reading.count;
        } else {
            unreadable_count += 1;
        }
        warnings.push(reading.finding());
        warnings.extend(
            reading
                .review_signals
                .iter()
                .map(LiveScopeReviewSignal::finding),
        );
    }

    let (status, reason_code) = if unreadable_count > 0 {
        (PROJECT_STATUS_PARTIAL, ACCESS_REASON_PARTIAL_LIVE_SCOPES)
    } else if total_count == 0 {
        (PROJECT_STATUS_PARTIAL, ACCESS_REASON_PARTIAL_NO_DATA)
    } else {
        (PROJECT_STATUS_READY, ACCESS_REASON_READY)
    };

    Some(ProjectDomainStatus {
        id: ACCESS_DOMAIN_ID.to_string(),
        scope: ACCESS_SCOPE.to_string(),
        mode: ACCESS_MODE.to_string(),
        status: status.to_string(),
        reason_code: reason_code.to_string(),
        primary_count: total_count,
        blocker_count: 0,
        warning_count: warnings.iter().map(|item| item.count).sum(),
        source_kinds,
        signal_keys: ACCESS_SIGNAL_KEYS
            .iter()
            .map(|item| (*item).to_string())
            .collect(),
        blockers: Vec::new(),
        warnings,
        next_actions: build_next_actions(&readings, total_count),
        freshness: Default::default(),
    })
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
