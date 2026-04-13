//! Resolve and validate pending team deletes before destructive API calls.
//! This module looks up teams from search results, checks the caller's confirmation
//! flags, and prepares the delete target used by the final delete workflow. It is
//! intentionally narrow: it only handles resolution and validation, not the delete
//! request itself.

use reqwest::Method;
use serde_json::{Map, Value};

use crate::common::{message, render_json_value, string_field, Result};

use super::super::render::access_delete_summary_line;
use super::super::render::{build_access_delete_review_document, map_get_text, scalar_text};
use super::super::{request_object, request_object_list_field, DEFAULT_PAGE_SIZE};
use super::pending_delete_support::{
    format_prompt_row, print_delete_confirmation_summary, prompt_confirm_delete,
    prompt_select_indexes, validate_confirmation, validate_delete_prompt,
    validate_exactly_one_identity, TeamDeleteArgs,
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
    access_delete_summary_line(
        "team",
        &map_get_text(result, "name"),
        &[
            ("teamId", map_get_text(result, "teamId")),
            ("email", map_get_text(result, "email")),
            ("message", map_get_text(result, "message")),
        ],
    )
}

fn team_delete_prompt_label(team: &Map<String, Value>) -> String {
    let name = string_field(team, "name", "-");
    let email = string_field(team, "email", "-");
    let id = scalar_text(team.get("id"));
    let member_count = string_field(team, "memberCount", "-");
    format_prompt_row(
        &[(&name, 24), (&email, 30)],
        &format!("id={id} members={member_count}"),
    )
}

/// Delete one team after resolving identity and confirmation constraints.
pub(crate) fn delete_team_with_request<F>(
    mut request_json: F,
    args: &TeamDeleteArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    validate_delete_prompt(args.prompt, args.json, "Team")?;
    if !args.prompt {
        validate_exactly_one_identity(
            args.team_id.is_some(),
            args.name.is_some(),
            "Team",
            "--team-id",
        )?;
        validate_confirmation(args.yes, "Team")?;
    }
    let teams = if args.prompt && args.team_id.is_none() && args.name.is_none() {
        let teams = list_teams_with_request(&mut request_json, None, 1, DEFAULT_PAGE_SIZE)?;
        if teams.is_empty() {
            return Err(message(
                "Team delete --prompt did not find any matching teams.",
            ));
        }
        let labels = teams
            .iter()
            .map(team_delete_prompt_label)
            .collect::<Vec<_>>();
        let Some(indexes) = prompt_select_indexes("Teams To Delete", &labels)? else {
            println!("Cancelled team delete.");
            return Ok(0);
        };
        indexes
            .into_iter()
            .filter_map(|index| teams.get(index).cloned())
            .collect::<Vec<_>>()
    } else if let Some(team_id) = &args.team_id {
        vec![get_team_with_request(&mut request_json, team_id)?]
    } else {
        vec![lookup_team_by_name(
            &mut request_json,
            args.name.as_deref().unwrap_or(""),
        )?]
    };
    if args.prompt {
        let labels = teams
            .iter()
            .map(team_delete_prompt_label)
            .collect::<Vec<_>>();
        print_delete_confirmation_summary("The following teams will be deleted:", &labels);
    }
    if args.prompt && !prompt_confirm_delete(&format!("Delete {} team(s)?", teams.len()))? {
        println!("Cancelled team delete.");
        return Ok(0);
    }
    let mut results = Vec::new();
    for team in &teams {
        let team_id = scalar_text(team.get("id"));
        let response = delete_team_api_with_request(&mut request_json, &team_id)?;
        results.push(team_delete_result(team, &response));
    }
    if args.json {
        println!(
            "{}",
            render_json_value(&build_access_delete_review_document(
                "team",
                "Grafana live teams",
                &results
                    .iter()
                    .cloned()
                    .map(Value::Object)
                    .collect::<Vec<_>>(),
            ))?
        );
    } else {
        for result in &results {
            println!("{}", team_delete_summary_line(result));
        }
        if results.len() > 1 {
            println!("Deleted {} team(s).", results.len());
        }
    }
    Ok(results.len())
}

#[cfg(test)]
mod pending_delete_team_tests {
    use super::*;

    #[test]
    fn team_delete_prompt_label_includes_member_count() {
        let team = Map::from_iter(vec![
            ("id".to_string(), Value::String("3".to_string())),
            ("name".to_string(), Value::String("Ops".to_string())),
            (
                "email".to_string(),
                Value::String("ops@example.com".to_string()),
            ),
            ("memberCount".to_string(), Value::String("2".to_string())),
        ]);

        let label = team_delete_prompt_label(&team);

        assert!(label.contains("Ops"));
        assert!(label.contains("ops@example.com"));
        assert!(label.contains("id=3 members=2"));
    }

    #[test]
    fn team_delete_summary_line_includes_identity_and_message() {
        let result = Map::from_iter(vec![
            ("teamId".to_string(), Value::String("3".to_string())),
            ("name".to_string(), Value::String("Ops".to_string())),
            (
                "email".to_string(),
                Value::String("ops@example.com".to_string()),
            ),
            (
                "message".to_string(),
                Value::String("Team deleted.".to_string()),
            ),
        ]);

        let line = super::team_delete_summary_line(&result);

        assert_eq!(
            line,
            "Deleted team Ops teamId=3 email=ops@example.com message=Team deleted."
        );
    }
}
