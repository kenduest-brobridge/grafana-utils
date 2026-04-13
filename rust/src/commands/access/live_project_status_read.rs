use reqwest::Method;
use serde_json::{Map, Value};

use crate::common::Result;
use crate::grafana_api::{
    project_status_live as project_status_live_support, AccessResourceClient,
};
use crate::http::JsonHttpClient;

use super::build_service_account_review_signals;
use super::build_team_review_signals;
use super::build_user_review_signals;
use super::{
    request_object_list_field, LiveScopeReading, ACCESS_FINDING_KIND_ORGS_COUNT,
    ACCESS_FINDING_KIND_ORGS_UNREADABLE, ACCESS_FINDING_KIND_SERVICE_ACCOUNTS_COUNT,
    ACCESS_FINDING_KIND_SERVICE_ACCOUNTS_UNREADABLE, ACCESS_FINDING_KIND_TEAMS_COUNT,
    ACCESS_FINDING_KIND_TEAMS_UNREADABLE, ACCESS_FINDING_KIND_USERS_COUNT,
    ACCESS_FINDING_KIND_USERS_UNREADABLE, ACCESS_SOURCE_KIND_LIVE_GLOBAL_USERS,
    ACCESS_SOURCE_KIND_LIVE_ORGS, ACCESS_SOURCE_KIND_LIVE_ORG_USERS,
    ACCESS_SOURCE_KIND_LIVE_SERVICE_ACCOUNTS, ACCESS_SOURCE_KIND_LIVE_TEAMS, DEFAULT_PAGE_SIZE,
};

pub(super) fn list_live_service_accounts_with_request<F>(
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

pub(super) fn read_live_users_with_request<F>(request_json: &mut F) -> LiveScopeReading
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if let Ok(users) = super::list_org_users_with_request(&mut *request_json) {
        return LiveScopeReading::readable(
            "users",
            ACCESS_SOURCE_KIND_LIVE_ORG_USERS,
            "live.users.count",
            ACCESS_FINDING_KIND_USERS_COUNT,
            users.len(),
            build_user_review_signals(&users),
        );
    }
    if let Ok(users) = super::iter_global_users_with_request(&mut *request_json, DEFAULT_PAGE_SIZE)
    {
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

pub(super) fn read_live_users(client: &AccessResourceClient<'_>) -> LiveScopeReading {
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

pub(super) fn read_live_teams_with_request<F>(request_json: &mut F) -> LiveScopeReading
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match super::iter_teams_with_request(&mut *request_json, None) {
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

pub(super) fn read_live_teams(client: &AccessResourceClient<'_>) -> LiveScopeReading {
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

pub(super) fn read_live_orgs_with_request<F>(request_json: &mut F) -> LiveScopeReading
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

pub(super) fn read_live_orgs(client: &JsonHttpClient) -> LiveScopeReading {
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
    }
}

pub(super) fn read_live_service_accounts_with_request<F>(request_json: &mut F) -> LiveScopeReading
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

pub(super) fn read_live_service_accounts(client: &AccessResourceClient<'_>) -> LiveScopeReading {
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
