//! Resolve and validate pending team deletes before destructive API calls.
//! This module looks up teams from search results, checks the caller's confirmation
//! flags, and prepares the delete target used by the final delete workflow. It is
//! intentionally narrow: it only handles resolution and validation, not the delete
//! request itself.

use reqwest::Method;
use serde_json::{Map, Value};

use crate::common::{message, string_field, Result};

use super::super::render::{map_get_text, scalar_text};
use super::super::{request_object, request_object_list_field, DEFAULT_PAGE_SIZE};
use super::pending_delete_support::{
    validate_confirmation, validate_exactly_one_identity, TeamDeleteArgs,
};

/// List one page of teams for delete resolution.
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
    request_object_list_field(
        &mut request_json,
        Method::GET,
        "/api/teams/search",
        &params,
        None,
        "teams",
        (
            "Unexpected team list response from Grafana.",
            "Unexpected team list response from Grafana.",
        ),
    )
}

/// Find a team by exact name.
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

/// Fetch one team record for delete confirmation output.
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

/// Call the team DELETE endpoint and return Grafana's response payload.
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

/// Merge input team data with API response text into a stable result row.
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

/// Build a stable summary line for a deleted team.
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

/// Delete one team after resolving identity and confirmation constraints.
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
