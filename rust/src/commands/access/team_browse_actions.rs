//! Team browser mutation actions.

use reqwest::Method;
use serde_json::Value;

use crate::access::render::map_get_text;
use crate::access::team::modify_team_with_request;
use crate::access::{TeamBrowseArgs, TeamModifyArgs};
use crate::common::{message, Result};

use super::team_browse_input::load_rows;
use super::team_browse_state::BrowserState;

pub(super) fn save_edit<F>(
    request_json: &mut F,
    args: &TeamBrowseArgs,
    state: &mut BrowserState,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let edit = state
        .pending_edit
        .take()
        .ok_or_else(|| message("Team browse edit state is missing."))?;
    let modify = TeamModifyArgs {
        common: args.common.clone(),
        team_id: Some(edit.id.clone()),
        name: None,
        add_member: split_csv(&edit.add_member),
        remove_member: split_csv(&edit.remove_member),
        add_admin: split_csv(&edit.add_admin),
        remove_admin: split_csv(&edit.remove_admin),
        json: false,
    };
    if modify.add_member.is_empty()
        && modify.remove_member.is_empty()
        && modify.add_admin.is_empty()
        && modify.remove_admin.is_empty()
    {
        state.status = format!("No team changes detected for {}.", edit.name);
        return Ok(());
    }
    let _ = modify_team_with_request(&mut *request_json, &modify)?;
    state.replace_rows(load_rows(&mut *request_json, args)?);
    state.status = format!("Updated team {}.", edit.name);
    Ok(())
}

pub(super) fn toggle_member_admin<F>(
    request_json: &mut F,
    args: &TeamBrowseArgs,
    state: &mut BrowserState,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let row = state
        .selected_member_row()
        .ok_or_else(|| message("Team browse has no selected member to update."))?
        .clone();
    let team_id = map_get_text(&row, "parentTeamId");
    let team_name = map_get_text(&row, "parentTeamName");
    let identity = state
        .selected_member_identity()
        .ok_or_else(|| message("Team member row is missing the member identity."))?;
    if team_id.is_empty() || identity.is_empty() {
        return Err(message(
            "Team member row is missing the team id or member identity.",
        ));
    }
    let is_admin = state
        .selected_member_role()
        .is_some_and(|role| role.eq_ignore_ascii_case("admin"));
    let modify = TeamModifyArgs {
        common: args.common.clone(),
        team_id: Some(team_id.clone()),
        name: None,
        add_member: Vec::new(),
        remove_member: Vec::new(),
        add_admin: if is_admin {
            Vec::new()
        } else {
            vec![identity.clone()]
        },
        remove_admin: if is_admin {
            vec![identity.clone()]
        } else {
            Vec::new()
        },
        json: false,
    };
    let _ = modify_team_with_request(&mut *request_json, &modify)?;
    state.replace_rows(load_rows(&mut *request_json, args)?);
    state.status = if is_admin {
        format!("Removed team admin from {} on {}.", identity, team_name)
    } else {
        format!("Granted team admin to {} on {}.", identity, team_name)
    };
    Ok(())
}

fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}
