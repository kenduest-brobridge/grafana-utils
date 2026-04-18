//! User browse mutation actions and confirmation helpers.

use reqwest::Method;
use serde_json::{Map, Value};

use crate::access::render::map_get_text;
use crate::access::user::{delete_user_with_request, modify_user_with_request};
use crate::access::{request_object, UserDeleteArgs, UserModifyArgs};
use crate::common::{message, Result};

use super::user_browse_input::load_rows;
use super::user_browse_state::{row_kind, BrowserState};
use super::UserBrowseArgs;

pub(super) fn save_edit<F>(
    request_json: &mut F,
    args: &UserBrowseArgs,
    state: &mut BrowserState,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let edit = state
        .pending_edit
        .take()
        .ok_or_else(|| message("User browse edit state is missing."))?;
    let row = state
        .selected_row()
        .ok_or_else(|| message("User browse lost the selected row."))?;
    let current_login = map_get_text(row, "login");
    let current_email = map_get_text(row, "email");
    let current_name = map_get_text(row, "name");
    let current_role = map_get_text(row, "orgRole");
    let current_admin = map_get_text(row, "grafanaAdmin");
    let set_grafana_admin = if edit
        .grafana_admin
        .trim()
        .eq_ignore_ascii_case(&current_admin)
    {
        None
    } else {
        match edit.grafana_admin.trim().to_ascii_lowercase().as_str() {
            "" => None,
            "true" | "t" | "yes" | "y" | "1" => Some(true),
            "false" | "f" | "no" | "n" | "0" => Some(false),
            _ => return Err(message("Grafana Admin must be true or false.")),
        }
    };
    let modify = UserModifyArgs {
        common: args.common.clone(),
        user_id: Some(edit.id.clone()),
        login: None,
        email: None,
        set_login: (edit.login != current_login).then_some(edit.login.clone()),
        set_email: (edit.email != current_email).then_some(edit.email.clone()),
        set_name: (edit.name != current_name).then_some(edit.name.clone()),
        set_password: None,
        set_password_file: None,
        prompt_set_password: false,
        set_org_role: (edit.org_role != current_role && !edit.org_role.trim().is_empty())
            .then_some(edit.org_role.clone()),
        set_grafana_admin,
        json: false,
    };
    if modify.set_login.is_none()
        && modify.set_email.is_none()
        && modify.set_name.is_none()
        && modify.set_org_role.is_none()
        && modify.set_grafana_admin.is_none()
    {
        state.status = format!("No user changes detected for {}.", current_login);
        return Ok(());
    }
    let _ = modify_user_with_request(&mut *request_json, &modify)?;
    state.replace_rows(load_rows(&mut *request_json, args, state.display_mode)?);
    state.status = format!("Updated user {}.", edit.id);
    Ok(())
}

pub(super) fn confirm_delete<F>(
    request_json: &mut F,
    args: &UserBrowseArgs,
    state: &mut BrowserState,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let row = state
        .selected_row()
        .ok_or_else(|| message("User browse has no selected row to delete."))?
        .clone();
    let login = map_get_text(&row, "login");
    let delete = UserDeleteArgs {
        common: args.common.clone(),
        user_id: Some(map_get_text(&row, "id")),
        login: None,
        email: None,
        scope: Some(args.scope.clone()),
        prompt: false,
        yes: true,
        json: false,
    };
    let _ = delete_user_with_request(&mut *request_json, &delete)?;
    state.replace_rows(load_rows(&mut *request_json, args, state.display_mode)?);
    state.status = format!("Deleted user {}.", login);
    Ok(())
}

pub(super) fn confirm_member_remove<F>(request_json: &mut F, state: &mut BrowserState) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let row = state
        .selected_row()
        .ok_or_else(|| message("User browse has no selected team membership to remove."))?
        .clone();
    if row_kind(&row) != "team" {
        return Err(message(
            "Select a team membership row before removing a membership.",
        ));
    }
    let team_id = map_get_text(&row, "parentTeamId");
    let user_id = map_get_text(&row, "parentUserId");
    let team_name = map_get_text(&row, "teamName");
    let login = map_get_text(&row, "parentLogin");
    if team_id.is_empty() || user_id.is_empty() {
        return Err(message(
            "Team membership row is missing the team id or user id.",
        ));
    }
    let _ = request_object(
        &mut *request_json,
        Method::DELETE,
        &format!("/api/teams/{team_id}/members/{user_id}"),
        &[],
        None,
        &format!("Unexpected remove-member response for Grafana team {team_id}."),
    )?;
    let selected_parent_id = user_id.clone();
    let removed =
        remove_team_membership_from_rows(&mut state.base_rows, &user_id, &team_id, &team_name);
    if !removed {
        return Err(message(format!(
            "Removed team membership {team_id} for user {user_id}, but the user row was not found in memory."
        )));
    }
    state.pending_member_remove = false;
    state.replace_rows(state.base_rows.clone());
    if let Some(index) = state
        .rows
        .iter()
        .position(|candidate| map_get_text(candidate, "id") == selected_parent_id)
    {
        state.select_index(index);
    }
    state.status = if login.is_empty() {
        format!("Removed membership from team {}.", team_name)
    } else {
        format!("Removed membership from {}.", login)
    };
    Ok(())
}

fn remove_team_membership_from_rows(
    rows: &mut [Map<String, Value>],
    user_id: &str,
    team_id: &str,
    team_name: &str,
) -> bool {
    for row in rows {
        if map_get_text(row, "id") != user_id {
            continue;
        }
        if let Some(Value::Array(team_rows)) = row.get_mut("teamRows") {
            team_rows.retain(|team| {
                let Some(team) = team.as_object() else {
                    return true;
                };
                map_get_text(team, "teamId") != team_id
            });
        }
        if let Some(Value::Array(teams)) = row.get_mut("teams") {
            teams.retain(|team| team.as_str().map(|name| name != team_name).unwrap_or(true));
        }
        return true;
    }
    false
}
