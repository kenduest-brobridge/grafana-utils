//! Shared team runtime helpers for list, modify, and mutation flows.

use reqwest::Method;
use serde_json::{Map, Value};
use std::fmt::Write as _;

use crate::common::{message, string_field, Result};

use super::render::{map_get_text, scalar_text, value_bool};
use super::user::lookup_org_user_by_identity;
use super::{
    request_array, request_object, request_object_list_field, TeamModifyArgs, DEFAULT_PAGE_SIZE,
};

pub(super) fn normalize_access_identity(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

pub(super) fn user_id_from_record(record: &Map<String, Value>) -> String {
    let user_id = scalar_text(record.get("userId"));
    if user_id.is_empty() {
        scalar_text(record.get("id"))
    } else {
        user_id
    }
}

fn user_id_json_value(user_id: &str) -> Value {
    match user_id.trim().parse::<u64>() {
        Ok(value) => Value::Number(value.into()),
        Err(_) => Value::String(user_id.to_string()),
    }
}

pub(crate) fn list_teams_with_request<F>(
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

pub(crate) fn list_team_members_with_request<F>(
    mut request_json: F,
    team_id: &str,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_array(
        &mut request_json,
        Method::GET,
        &format!("/api/teams/{team_id}/members"),
        &[],
        None,
        &format!("Unexpected member list response for Grafana team {team_id}."),
    )
}

pub(super) fn get_team_with_request<F>(
    mut request_json: F,
    team_id: &str,
) -> Result<Map<String, Value>>
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

pub(super) fn create_team_with_request<F>(
    mut request_json: F,
    payload: &Value,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::POST,
        "/api/teams",
        &[],
        Some(payload),
        "Unexpected team create response from Grafana.",
    )
}

pub(super) fn add_team_member_with_request<F>(
    mut request_json: F,
    team_id: &str,
    user_id: &str,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::POST,
        &format!("/api/teams/{team_id}/members"),
        &[],
        Some(&Value::Object(Map::from_iter(vec![(
            "userId".to_string(),
            user_id_json_value(user_id),
        )]))),
        &format!("Unexpected add-member response for Grafana team {team_id}."),
    )
}

pub(super) fn remove_team_member_with_request<F>(
    mut request_json: F,
    team_id: &str,
    user_id: &str,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::DELETE,
        &format!("/api/teams/{team_id}/members/{user_id}"),
        &[],
        None,
        &format!("Unexpected remove-member response for Grafana team {team_id}."),
    )
}

pub(super) fn update_team_members_with_request<F>(
    mut request_json: F,
    team_id: &str,
    members: Vec<String>,
    admins: Vec<String>,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::PUT,
        &format!("/api/teams/{team_id}/members"),
        &[],
        Some(&Value::Object(Map::from_iter(vec![
            (
                "members".to_string(),
                Value::Array(members.into_iter().map(Value::String).collect()),
            ),
            (
                "admins".to_string(),
                Value::Array(admins.into_iter().map(Value::String).collect()),
            ),
        ]))),
        &format!("Unexpected team member update response for Grafana team {team_id}."),
    )
}

pub(crate) fn lookup_team_by_name<F>(mut request_json: F, name: &str) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let teams = list_teams_with_request(&mut request_json, Some(name), 1, DEFAULT_PAGE_SIZE)?;
    teams
        .into_iter()
        .find(|team| string_field(team, "name", "") == name)
        .ok_or_else(|| message(format!("Grafana team lookup did not find {name}.")))
}

pub(crate) fn iter_teams_with_request<F>(
    mut request_json: F,
    query: Option<&str>,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut teams = Vec::new();
    let mut page = 1usize;
    loop {
        let batch = list_teams_with_request(&mut request_json, query, page, DEFAULT_PAGE_SIZE)?;
        let batch_len = batch.len();
        teams.extend(batch);
        if batch_len < DEFAULT_PAGE_SIZE {
            break;
        }
        page += 1;
    }
    Ok(teams)
}

pub(crate) fn validate_team_modify_args(args: &TeamModifyArgs) -> Result<()> {
    if args.team_id.is_none() && args.name.is_none() {
        return Err(message("Team modify requires one of --team-id or --name."));
    }
    if args.add_member.is_empty()
        && args.remove_member.is_empty()
        && args.add_admin.is_empty()
        && args.remove_admin.is_empty()
    {
        return Err(message(
            "Team modify requires at least one of --add-member, --remove-member, --add-admin, or --remove-admin.",
        ));
    }
    Ok(())
}

pub(crate) fn team_member_identity(member: &Map<String, Value>) -> String {
    let email = string_field(member, "email", "");
    if !email.is_empty() {
        email
    } else {
        string_field(member, "login", "")
    }
}

pub(super) fn team_member_is_admin(member: &Map<String, Value>) -> bool {
    value_bool(member.get("isAdmin"))
        .unwrap_or_else(|| value_bool(member.get("admin")).unwrap_or(false))
}

pub(super) fn add_or_remove_member<F>(
    request_json: &mut F,
    team_id: &str,
    identity: &str,
    add: bool,
) -> Result<String>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let user = lookup_org_user_by_identity(&mut *request_json, identity)?;
    let user_id = {
        let found = user_id_from_record(&user);
        if found.is_empty() {
            return Err(message(format!(
                "Team member lookup did not return an id: {identity}"
            )));
        }
        found
    };
    if add {
        let _ = add_team_member_with_request(&mut *request_json, team_id, &user_id)?;
    } else {
        let _ = remove_team_member_with_request(&mut *request_json, team_id, &user_id)?;
    }
    Ok(string_field(
        &user,
        "email",
        &string_field(&user, "login", identity),
    ))
}

pub(super) fn team_modify_result(
    team_id: &str,
    team_name: &str,
    added_members: Vec<String>,
    removed_members: Vec<String>,
    added_admins: Vec<String>,
    removed_admins: Vec<String>,
    email: String,
) -> Map<String, Value> {
    Map::from_iter(vec![
        ("teamId".to_string(), Value::String(team_id.to_string())),
        ("name".to_string(), Value::String(team_name.to_string())),
        ("email".to_string(), Value::String(email)),
        (
            "addedMembers".to_string(),
            Value::Array(added_members.into_iter().map(Value::String).collect()),
        ),
        (
            "removedMembers".to_string(),
            Value::Array(removed_members.into_iter().map(Value::String).collect()),
        ),
        (
            "addedAdmins".to_string(),
            Value::Array(added_admins.into_iter().map(Value::String).collect()),
        ),
        (
            "removedAdmins".to_string(),
            Value::Array(removed_admins.into_iter().map(Value::String).collect()),
        ),
    ])
}

pub(super) fn team_modify_summary_line(result: &Map<String, Value>) -> String {
    let mut text = format!(
        "teamId={} name={}",
        map_get_text(result, "teamId"),
        map_get_text(result, "name")
    );
    for key in [
        "addedMembers",
        "removedMembers",
        "addedAdmins",
        "removedAdmins",
    ] {
        let value = map_get_text(result, key);
        if !value.is_empty() {
            let _ = write!(&mut text, " {}={}", key, value);
        }
    }
    text
}
