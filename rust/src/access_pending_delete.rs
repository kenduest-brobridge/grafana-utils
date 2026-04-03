//! Access delete staging surface (parser-only, not yet wired to runtime handlers).
//! Kept adjacent to access CLI modules to avoid command-surface churn while handler work is staged.
use clap::{Args, Subcommand};
use reqwest::Method;
use serde_json::{Map, Value};

use crate::common::{message, string_field, value_as_object, Result};

use super::access_render::{map_get_text, normalize_service_account_row, scalar_text};
use super::{
    request_object, CommonCliArgs, TeamAddArgs, TeamListArgs, TeamModifyArgs, DEFAULT_PAGE_SIZE,
};

// Staging module for the remaining access-management delete surface.
//
// This file intentionally contains only argument shapes and parser-level plumbing so
// the incomplete delete surface can be reviewed incrementally without forcing runtime
// behavior changes. Keep it aligned with `access.rs` as command handlers are re-enabled.
//
// Intended future wiring:
// - declare this module from `access.rs`
// - re-export the staged arg types from `access.rs` and `access_cli_defs.rs`
// - extend `TeamCommand`, `ServiceAccountCommand`, and `ServiceAccountTokenCommand`
// - dispatch the new handlers from `run_access_cli_with_request` and `run_access_cli`
// - optionally materialize `GroupCommandStage` as a compatibility alias for `team`

#[derive(Debug, Clone, Args)]
pub struct TeamDeleteArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help_heading = "Target Selection",
        conflicts_with = "name",
        help = "Target one team by numeric Grafana team id."
    )]
    pub team_id: Option<String>,
    #[arg(
        long,
        help_heading = "Target Selection",
        conflicts_with = "team_id",
        help = "Target one team by exact name."
    )]
    pub name: Option<String>,
    #[arg(
        long,
        help_heading = "Safety",
        default_value_t = false,
        help = "Skip the interactive confirmation prompt."
    )]
    pub yes: bool,
    #[arg(long, default_value_t = false, help_heading = "Output Options")]
    pub json: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ServiceAccountDeleteArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long = "service-account-id",
        help_heading = "Target Selection",
        conflicts_with = "name",
        help = "Target one service account by numeric id."
    )]
    pub service_account_id: Option<String>,
    #[arg(
        long,
        help_heading = "Target Selection",
        conflicts_with = "service_account_id",
        help = "Target one service account by exact name."
    )]
    pub name: Option<String>,
    #[arg(
        long,
        help_heading = "Safety",
        default_value_t = false,
        help = "Skip the interactive confirmation prompt."
    )]
    pub yes: bool,
    #[arg(long, default_value_t = false, help_heading = "Output Options")]
    pub json: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ServiceAccountTokenDeleteArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long = "service-account-id",
        help_heading = "Target Selection",
        conflicts_with = "name",
        help = "Target one service account by numeric id."
    )]
    pub service_account_id: Option<String>,
    #[arg(
        long,
        help_heading = "Target Selection",
        conflicts_with = "service_account_id",
        help = "Target one service account by exact name."
    )]
    pub name: Option<String>,
    #[arg(
        long = "token-id",
        help_heading = "Target Selection",
        conflicts_with = "token_name",
        help = "Target token by numeric token id."
    )]
    pub token_id: Option<String>,
    #[arg(
        long = "token-name",
        help_heading = "Target Selection",
        conflicts_with = "token_id",
        help = "Target token by exact token name."
    )]
    pub token_name: Option<String>,
    #[arg(
        long,
        help_heading = "Safety",
        default_value_t = false,
        help = "Skip the interactive confirmation prompt."
    )]
    pub yes: bool,
    #[arg(long, default_value_t = false, help_heading = "Output Options")]
    pub json: bool,
}

#[derive(Debug, Clone, Subcommand)]
pub enum GroupCommandStage {
    List(TeamListArgs),
    Add(TeamAddArgs),
    Modify(TeamModifyArgs),
    Delete(TeamDeleteArgs),
}

fn validate_confirmation(yes: bool, noun: &str) -> Result<()> {
    if yes {
        Ok(())
    } else {
        Err(message(format!("{noun} delete requires --yes.")))
    }
}

fn render_single_object_json(object: &Map<String, Value>) -> Result<String> {
    serde_json::to_string_pretty(&Value::Object(object.clone())).map_err(Into::into)
}

fn validate_exactly_one_identity(
    id_present: bool,
    name_present: bool,
    noun: &str,
    id_flag: &str,
) -> Result<()> {
    match (id_present, name_present) {
        (true, false) | (false, true) => Ok(()),
        (false, false) => Err(message(format!(
            "{noun} delete requires one of {id_flag} or --name."
        ))),
        (true, true) => Err(message(format!(
            "{noun} delete accepts either {id_flag} or --name, not both."
        ))),
    }
}

fn validate_token_identity(args: &ServiceAccountTokenDeleteArgs) -> Result<()> {
    validate_exactly_one_identity(
        args.service_account_id.is_some(),
        args.name.is_some(),
        "Service-account token",
        "--service-account-id",
    )?;
    match (args.token_id.is_some(), args.token_name.is_some()) {
        (true, false) | (false, true) => Ok(()),
        (false, false) => Err(message(
            "Service-account token delete requires one of --token-id or --token-name.",
        )),
        (true, true) => Err(message(
            "Service-account token delete accepts either --token-id or --token-name, not both.",
        )),
    }
}

fn list_teams_with_request<F>(
    mut request_json: F,
    query: Option<&str>,
    page: usize,
    per_page: usize,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let params = vec![
        ("query".to_string(), query.unwrap_or("").to_string()),
        ("page".to_string(), page.to_string()),
        ("perpage".to_string(), per_page.to_string()),
    ];
    let object = request_object(
        &mut request_json,
        Method::GET,
        "/api/teams/search",
        &params,
        None,
        "Unexpected team list response from Grafana.",
    )?;
    match object.get("teams") {
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| {
                Ok(value_as_object(value, "Unexpected team list response from Grafana.")?.clone())
            })
            .collect(),
        _ => Err(message("Unexpected team list response from Grafana.")),
    }
}

fn lookup_team_by_name<F>(mut request_json: F, name: &str) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let teams = list_teams_with_request(&mut request_json, Some(name), 1, DEFAULT_PAGE_SIZE)?;
    teams
        .into_iter()
        .find(|team| string_field(team, "name", "") == name)
        .ok_or_else(|| message(format!("Grafana team lookup did not find {name}.")))
}

fn get_team_with_request<F>(mut request_json: F, team_id: &str) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::GET,
        &format!("/api/teams/{team_id}"),
        &[],
        None,
        &format!("Unexpected team lookup response for Grafana team {team_id}."),
    )
}

fn delete_team_api_with_request<F>(mut request_json: F, team_id: &str) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::DELETE,
        &format!("/api/teams/{team_id}"),
        &[],
        None,
        &format!("Unexpected team delete response for Grafana team {team_id}."),
    )
}

fn team_delete_result(
    team: &Map<String, Value>,
    response: &Map<String, Value>,
) -> Map<String, Value> {
    Map::from_iter(vec![
        (
            "teamId".to_string(),
            Value::String({
                let id = scalar_text(team.get("id"));
                if id.is_empty() {
                    scalar_text(response.get("teamId"))
                } else {
                    id
                }
            }),
        ),
        (
            "name".to_string(),
            Value::String(string_field(team, "name", "")),
        ),
        (
            "email".to_string(),
            Value::String(string_field(team, "email", "")),
        ),
        (
            "message".to_string(),
            Value::String(string_field(response, "message", "Team deleted.")),
        ),
    ])
}

fn team_delete_summary_line(result: &Map<String, Value>) -> String {
    let mut parts = vec![
        format!("teamId={}", map_get_text(result, "teamId")),
        format!("name={}", map_get_text(result, "name")),
    ];
    let email = map_get_text(result, "email");
    if !email.is_empty() {
        parts.push(format!("email={email}"));
    }
    let message = map_get_text(result, "message");
    if !message.is_empty() {
        parts.push(format!("message={message}"));
    }
    parts.join(" ")
}

pub(crate) fn delete_team_with_request<F>(
    mut request_json: F,
    args: &TeamDeleteArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    validate_exactly_one_identity(
        args.team_id.is_some(),
        args.name.is_some(),
        "Team",
        "--team-id",
    )?;
    validate_confirmation(args.yes, "Team")?;
    let team = if let Some(team_id) = &args.team_id {
        get_team_with_request(&mut request_json, team_id)?
    } else {
        lookup_team_by_name(&mut request_json, args.name.as_deref().unwrap_or(""))?
    };
    let team_id = scalar_text(team.get("id"));
    let response = delete_team_api_with_request(&mut request_json, &team_id)?;
    let result = team_delete_result(&team, &response);
    if args.json {
        println!("{}", serde_json::to_string_pretty(&Value::Object(result))?);
    } else {
        println!("{}", team_delete_summary_line(&result));
    }
    Ok(0)
}

fn list_service_accounts_with_request<F>(
    mut request_json: F,
    query: Option<&str>,
    page: usize,
    per_page: usize,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let params = vec![
        ("query".to_string(), query.unwrap_or("").to_string()),
        ("page".to_string(), page.to_string()),
        ("perpage".to_string(), per_page.to_string()),
    ];
    let object = request_object(
        &mut request_json,
        Method::GET,
        "/api/serviceaccounts/search",
        &params,
        None,
        "Unexpected service-account list response from Grafana.",
    )?;
    match object.get("serviceAccounts") {
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| {
                Ok(value_as_object(
                    value,
                    "Unexpected service-account list response from Grafana.",
                )?
                .clone())
            })
            .collect(),
        _ => Err(message(
            "Unexpected service-account list response from Grafana.",
        )),
    }
}

fn lookup_service_account_by_name<F>(mut request_json: F, name: &str) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let accounts =
        list_service_accounts_with_request(&mut request_json, Some(name), 1, DEFAULT_PAGE_SIZE)?;
    accounts
        .into_iter()
        .find(|item| string_field(item, "name", "") == name)
        .ok_or_else(|| {
            message(format!(
                "Grafana service-account lookup did not find {name}."
            ))
        })
}

fn get_service_account_with_request<F>(
    mut request_json: F,
    service_account_id: &str,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::GET,
        &format!("/api/serviceaccounts/{service_account_id}"),
        &[],
        None,
        &format!(
            "Unexpected service-account lookup response for Grafana service account {service_account_id}."
        ),
    )
}

fn delete_service_account_api_with_request<F>(
    mut request_json: F,
    service_account_id: &str,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::DELETE,
        &format!("/api/serviceaccounts/{service_account_id}"),
        &[],
        None,
        &format!(
            "Unexpected service-account delete response for Grafana service account {service_account_id}."
        ),
    )
}

fn service_account_delete_result(
    service_account: &Map<String, Value>,
    response: &Map<String, Value>,
) -> Map<String, Value> {
    let mut row = normalize_service_account_row(service_account);
    row.insert(
        "serviceAccountId".to_string(),
        Value::String({
            let id = map_get_text(&row, "id");
            if id.is_empty() {
                scalar_text(response.get("id"))
            } else {
                id
            }
        }),
    );
    row.insert(
        "message".to_string(),
        Value::String(string_field(
            response,
            "message",
            "Service account deleted.",
        )),
    );
    row
}

fn service_account_delete_summary_line(result: &Map<String, Value>) -> String {
    let mut parts = vec![
        format!(
            "serviceAccountId={}",
            map_get_text(result, "serviceAccountId")
        ),
        format!("name={}", map_get_text(result, "name")),
    ];
    let login = map_get_text(result, "login");
    if !login.is_empty() {
        parts.push(format!("login={login}"));
    }
    let role = map_get_text(result, "role");
    if !role.is_empty() {
        parts.push(format!("role={role}"));
    }
    let message = map_get_text(result, "message");
    if !message.is_empty() {
        parts.push(format!("message={message}"));
    }
    parts.join(" ")
}

pub(crate) fn delete_service_account_with_request<F>(
    mut request_json: F,
    args: &ServiceAccountDeleteArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    validate_exactly_one_identity(
        args.service_account_id.is_some(),
        args.name.is_some(),
        "Service-account",
        "--service-account-id",
    )?;
    validate_confirmation(args.yes, "Service-account")?;
    let service_account = if let Some(service_account_id) = &args.service_account_id {
        get_service_account_with_request(&mut request_json, service_account_id)?
    } else {
        lookup_service_account_by_name(&mut request_json, args.name.as_deref().unwrap_or(""))?
    };
    let service_account_id = {
        let id = scalar_text(service_account.get("id"));
        if id.is_empty() {
            scalar_text(service_account.get("serviceAccountId"))
        } else {
            id
        }
    };
    let response = delete_service_account_api_with_request(&mut request_json, &service_account_id)?;
    let result = service_account_delete_result(&service_account, &response);
    if args.json {
        println!("{}", render_single_object_json(&result)?);
    } else {
        println!("{}", service_account_delete_summary_line(&result));
    }
    Ok(0)
}

fn list_service_account_tokens_with_request<F>(
    mut request_json: F,
    service_account_id: &str,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match request_json(
        Method::GET,
        &format!("/api/serviceaccounts/{service_account_id}/tokens"),
        &[],
        None,
    )? {
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| {
                Ok(value_as_object(
                    item,
                    "Unexpected service-account token list response from Grafana.",
                )?
                .clone())
            })
            .collect(),
        Some(_) => Err(message(
            "Unexpected service-account token list response from Grafana.",
        )),
        None => Ok(Vec::new()),
    }
}

fn lookup_service_account_token_by_name<F>(
    mut request_json: F,
    service_account_id: &str,
    token_name: &str,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let tokens = list_service_account_tokens_with_request(&mut request_json, service_account_id)?;
    tokens
        .into_iter()
        .find(|token| string_field(token, "name", "") == token_name)
        .ok_or_else(|| {
            message(format!(
                "Grafana service-account token lookup did not find {token_name}."
            ))
        })
}

fn delete_service_account_token_api_with_request<F>(
    mut request_json: F,
    service_account_id: &str,
    token_id: &str,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::DELETE,
        &format!("/api/serviceaccounts/{service_account_id}/tokens/{token_id}"),
        &[],
        None,
        &format!(
            "Unexpected service-account token delete response for Grafana service account {service_account_id} token {token_id}."
        ),
    )
}

fn service_account_token_delete_result(
    service_account: &Map<String, Value>,
    token: &Map<String, Value>,
    response: &Map<String, Value>,
) -> Map<String, Value> {
    Map::from_iter(vec![
        (
            "serviceAccountId".to_string(),
            Value::String({
                let id = scalar_text(service_account.get("id"));
                if id.is_empty() {
                    scalar_text(service_account.get("serviceAccountId"))
                } else {
                    id
                }
            }),
        ),
        (
            "serviceAccountName".to_string(),
            Value::String(string_field(service_account, "name", "")),
        ),
        (
            "tokenId".to_string(),
            Value::String({
                let id = scalar_text(token.get("id"));
                if id.is_empty() {
                    scalar_text(response.get("tokenId"))
                } else {
                    id
                }
            }),
        ),
        (
            "tokenName".to_string(),
            Value::String(string_field(token, "name", "")),
        ),
        (
            "message".to_string(),
            Value::String(string_field(
                response,
                "message",
                "Service-account token deleted.",
            )),
        ),
    ])
}

fn service_account_token_delete_summary_line(result: &Map<String, Value>) -> String {
    [
        format!(
            "serviceAccountId={}",
            map_get_text(result, "serviceAccountId")
        ),
        format!(
            "serviceAccountName={}",
            map_get_text(result, "serviceAccountName")
        ),
        format!("tokenId={}", map_get_text(result, "tokenId")),
        format!("tokenName={}", map_get_text(result, "tokenName")),
        format!("message={}", map_get_text(result, "message")),
    ]
    .join(" ")
}

pub(crate) fn delete_service_account_token_with_request<F>(
    mut request_json: F,
    args: &ServiceAccountTokenDeleteArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    validate_token_identity(args)?;
    validate_confirmation(args.yes, "Service-account token")?;
    let service_account = if let Some(service_account_id) = &args.service_account_id {
        get_service_account_with_request(&mut request_json, service_account_id)?
    } else {
        lookup_service_account_by_name(&mut request_json, args.name.as_deref().unwrap_or(""))?
    };
    let service_account_id = {
        let id = scalar_text(service_account.get("id"));
        if id.is_empty() {
            scalar_text(service_account.get("serviceAccountId"))
        } else {
            id
        }
    };
    let token = if let Some(token_id) = &args.token_id {
        Map::from_iter(vec![
            ("id".to_string(), Value::String(token_id.clone())),
            ("name".to_string(), Value::String(String::new())),
        ])
    } else {
        lookup_service_account_token_by_name(
            &mut request_json,
            &service_account_id,
            args.token_name.as_deref().unwrap_or(""),
        )?
    };
    let token_id = scalar_text(token.get("id"));
    let response = delete_service_account_token_api_with_request(
        &mut request_json,
        &service_account_id,
        &token_id,
    )?;
    let result = service_account_token_delete_result(&service_account, &token, &response);
    if args.json {
        println!("{}", render_single_object_json(&result)?);
    } else {
        println!("{}", service_account_token_delete_summary_line(&result));
    }
    Ok(0)
}

#[cfg(test)]
mod access_pending_delete_rust_tests {
    use super::*;
    use crate::access::access_cli_defs::{DEFAULT_TIMEOUT, DEFAULT_URL};
    use crate::access::CommonCliArgs;

    fn common_args() -> CommonCliArgs {
        CommonCliArgs {
            url: DEFAULT_URL.to_string(),
            api_token: Some("token".to_string()),
            username: None,
            password: None,
            prompt_password: false,
            prompt_token: false,
            org_id: None,
            timeout: DEFAULT_TIMEOUT,
            verify_ssl: false,
        }
    }

    #[test]
    fn validate_team_delete_requires_confirmation() {
        let error = delete_team_with_request(
            |_method, _path, _params, _payload| Ok(None),
            &TeamDeleteArgs {
                common: common_args(),
                team_id: Some("7".to_string()),
                name: None,
                yes: false,
                json: false,
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("Team delete requires --yes."));
    }

    #[test]
    fn validate_service_account_delete_requires_identity() {
        let error = delete_service_account_with_request(
            |_method, _path, _params, _payload| Ok(None),
            &ServiceAccountDeleteArgs {
                common: common_args(),
                service_account_id: None,
                name: None,
                yes: true,
                json: false,
            },
        )
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("Service-account delete requires one of --service-account-id or --name."));
    }

    #[test]
    fn validate_service_account_token_delete_requires_token_selector() {
        let error = delete_service_account_token_with_request(
            |_method, _path, _params, _payload| Ok(None),
            &ServiceAccountTokenDeleteArgs {
                common: common_args(),
                service_account_id: Some("4".to_string()),
                name: None,
                token_id: None,
                token_name: None,
                yes: true,
                json: false,
            },
        )
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("Service-account token delete requires one of --token-id or --token-name."));
    }

    #[test]
    fn render_single_object_json_returns_object_payload() {
        let payload = Map::from_iter(vec![
            (
                "serviceAccountId".to_string(),
                Value::String("4".to_string()),
            ),
            ("message".to_string(), Value::String("deleted".to_string())),
        ]);
        let rendered = render_single_object_json(&payload).unwrap();
        assert!(rendered.trim_start().starts_with('{'));
        assert!(!rendered.trim_start().starts_with('['));
        assert!(rendered.contains("\"serviceAccountId\": \"4\""));
    }
}
