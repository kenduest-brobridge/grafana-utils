use reqwest::Method;
use serde_json::{Map, Value};
use std::fmt::Write as _;

use crate::common::{message, string_field, value_as_object, Result};

use super::access_render::{
    format_table, map_get_text, normalize_team_row, render_csv, render_objects_json,
    scalar_text, team_summary_line, team_table_rows, value_bool,
};
use super::access_user::lookup_org_user_by_identity;
use super::{request_array, request_object, TeamAddArgs, TeamListArgs, TeamModifyArgs, DEFAULT_PAGE_SIZE};

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
            .map(|value| Ok(value_as_object(value, "Unexpected team list response from Grafana.")?.clone()))
            .collect(),
        _ => Err(message("Unexpected team list response from Grafana.")),
    }
}

fn list_team_members_with_request<F>(mut request_json: F, team_id: &str) -> Result<Vec<Map<String, Value>>>
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

fn create_team_with_request<F>(mut request_json: F, payload: &Value) -> Result<Map<String, Value>>
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

fn add_team_member_with_request<F>(
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
            Value::String(user_id.to_string()),
        )]))),
        &format!("Unexpected add-member response for Grafana team {team_id}."),
    )
}

fn remove_team_member_with_request<F>(
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

fn update_team_members_with_request<F>(
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

fn validate_team_modify_args(args: &TeamModifyArgs) -> Result<()> {
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

fn team_member_identity(member: &Map<String, Value>) -> String {
    let email = string_field(member, "email", "");
    if !email.is_empty() {
        email
    } else {
        string_field(member, "login", "")
    }
}

fn team_member_is_admin(member: &Map<String, Value>) -> bool {
    value_bool(member.get("isAdmin")).unwrap_or_else(|| value_bool(member.get("admin")).unwrap_or(false))
}

fn add_or_remove_member<F>(request_json: &mut F, team_id: &str, identity: &str, add: bool) -> Result<String>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let user = lookup_org_user_by_identity(&mut *request_json, identity)?;
    let user_id = string_field(&user, "userId", &string_field(&user, "id", ""));
    if add {
        let _ = add_team_member_with_request(&mut *request_json, team_id, &user_id)?;
    } else {
        let _ = remove_team_member_with_request(&mut *request_json, team_id, &user_id)?;
    }
    Ok(string_field(&user, "email", &string_field(&user, "login", identity)))
}

pub(crate) fn list_teams_command_with_request<F>(
    mut request_json: F,
    args: &TeamListArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut rows = list_teams_with_request(&mut request_json, args.query.as_deref(), args.page, args.per_page)?
        .into_iter()
        .map(|team| normalize_team_row(&team))
        .collect::<Vec<Map<String, Value>>>();
    if let Some(name) = &args.name {
        rows.retain(|row| map_get_text(row, "name") == *name);
    }
    if args.with_members {
        for row in &mut rows {
            let team_id = map_get_text(row, "id");
            let members = list_team_members_with_request(&mut request_json, &team_id)?
                .into_iter()
                .map(|member| team_member_identity(&member))
                .filter(|identity| !identity.is_empty())
                .map(Value::String)
                .collect::<Vec<Value>>();
            row.insert("members".to_string(), Value::Array(members));
        }
    }
    if args.json {
        println!("{}", render_objects_json(&rows)?);
    } else if args.csv {
        for line in render_csv(
            &["id", "name", "email", "memberCount", "members"],
            &team_table_rows(&rows),
        ) {
            println!("{line}");
        }
    } else if args.table {
        for line in format_table(
            &["ID", "NAME", "EMAIL", "MEMBER_COUNT", "MEMBERS"],
            &team_table_rows(&rows),
        ) {
            println!("{line}");
        }
        println!();
        println!("Listed {} team(s) at {}", rows.len(), args.common.url);
    } else {
        for row in &rows {
            println!("{}", team_summary_line(row));
        }
        println!();
        println!("Listed {} team(s) at {}", rows.len(), args.common.url);
    }
    Ok(rows.len())
}

fn team_modify_result(
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

fn team_modify_summary_line(result: &Map<String, Value>) -> String {
    let mut text = format!(
        "teamId={} name={}",
        map_get_text(result, "teamId"),
        map_get_text(result, "name")
    );
    for key in ["addedMembers", "removedMembers", "addedAdmins", "removedAdmins"] {
        let value = map_get_text(result, key);
        if !value.is_empty() {
            let _ = write!(&mut text, " {}={}", key, value);
        }
    }
    text
}

pub(crate) fn modify_team_with_request<F>(
    mut request_json: F,
    args: &TeamModifyArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    validate_team_modify_args(args)?;
    let team = if let Some(team_id) = &args.team_id {
        get_team_with_request(&mut request_json, team_id)?
    } else {
        lookup_team_by_name(&mut request_json, args.name.as_deref().unwrap_or(""))?
    };
    let team_id = scalar_text(team.get("id"));
    let team_name = string_field(&team, "name", "");
    let mut added_members = Vec::new();
    let mut removed_members = Vec::new();
    for identity in &args.add_member {
        added_members.push(add_or_remove_member(&mut request_json, &team_id, identity, true)?);
    }
    for identity in &args.remove_member {
        removed_members.push(add_or_remove_member(&mut request_json, &team_id, identity, false)?);
    }
    let existing_members = list_team_members_with_request(&mut request_json, &team_id)?;
    let mut member_identities = existing_members
        .iter()
        .map(team_member_identity)
        .collect::<Vec<String>>();
    let mut admin_identities = existing_members
        .iter()
        .filter(|member| team_member_is_admin(member))
        .map(team_member_identity)
        .collect::<Vec<String>>();
    let mut added_admins = Vec::new();
    let mut removed_admins = Vec::new();
    if !args.add_admin.is_empty() || !args.remove_admin.is_empty() {
        for identity in &args.add_admin {
            let user = lookup_org_user_by_identity(&mut request_json, identity)?;
            let resolved = string_field(&user, "email", &string_field(&user, "login", identity));
            if !member_identities.contains(&resolved) {
                member_identities.push(resolved.clone());
            }
            if !admin_identities.contains(&resolved) {
                admin_identities.push(resolved.clone());
                added_admins.push(resolved);
            }
        }
        for identity in &args.remove_admin {
            let user = lookup_org_user_by_identity(&mut request_json, identity)?;
            let resolved = string_field(&user, "email", &string_field(&user, "login", identity));
            if let Some(index) = admin_identities.iter().position(|value| value == &resolved) {
                admin_identities.remove(index);
                removed_admins.push(resolved);
            }
        }
        member_identities.sort();
        member_identities.dedup();
        admin_identities.sort();
        admin_identities.dedup();
        let _ = update_team_members_with_request(
            &mut request_json,
            &team_id,
            member_identities.clone(),
            admin_identities.clone(),
        )?;
    }
    let result = team_modify_result(
        &team_id,
        &team_name,
        added_members,
        removed_members,
        added_admins,
        removed_admins,
        string_field(&team, "email", ""),
    );
    if args.json {
        println!("{}", render_objects_json(&[result])?);
    } else {
        println!("{}", team_modify_summary_line(&result));
    }
    Ok(0)
}

pub(crate) fn add_team_with_request<F>(mut request_json: F, args: &TeamAddArgs) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut payload = Map::from_iter(vec![("name".to_string(), Value::String(args.name.clone()))]);
    if let Some(email) = &args.email {
        payload.insert("email".to_string(), Value::String(email.clone()));
    }
    let created = create_team_with_request(&mut request_json, &Value::Object(payload))?;
    let team_id = {
        let team_id = scalar_text(created.get("teamId"));
        if team_id.is_empty() {
            scalar_text(created.get("id"))
        } else {
            team_id
        }
    };
    let team = get_team_with_request(&mut request_json, &team_id)?;
    let modify = TeamModifyArgs {
        common: args.common.clone(),
        team_id: Some(team_id.clone()),
        name: None,
        add_member: args.members.clone(),
        remove_member: Vec::new(),
        add_admin: args.admins.clone(),
        remove_admin: Vec::new(),
        json: true,
    };
    let _ = modify_team_with_request(&mut request_json, &modify)?;
    let result = team_modify_result(
        &team_id,
        &string_field(&team, "name", &args.name),
        args.members.clone(),
        Vec::new(),
        args.admins.clone(),
        Vec::new(),
        string_field(&team, "email", args.email.as_deref().unwrap_or("")),
    );
    if args.json {
        println!("{}", render_objects_json(&[result])?);
    } else {
        println!("{}", team_modify_summary_line(&result));
    }
    Ok(0)
}
