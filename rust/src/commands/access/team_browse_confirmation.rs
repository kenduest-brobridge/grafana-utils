//! Team browser confirmation and decision flow.

use crossterm::event::{KeyCode, KeyEvent};
use reqwest::Method;
use serde_json::Value;

use crate::access::pending_delete::{delete_team_with_request, TeamDeleteArgs};
use crate::access::render::map_get_text;
use crate::access::team::modify_team_with_request;
use crate::access::{TeamBrowseArgs, TeamModifyArgs};
use crate::common::{message, Result};

use super::team_browse_input::load_rows;
use super::team_browse_state::{row_kind, BrowserState};

pub(super) fn handle_pending_confirmation_key<F>(
    request_json: &mut F,
    args: &TeamBrowseArgs,
    state: &mut BrowserState,
    key: &KeyEvent,
) -> Result<bool>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if state.pending_delete {
        match key.code {
            KeyCode::Char('y') => confirm_delete(request_json, args, state)?,
            KeyCode::Char('n') | KeyCode::Esc | KeyCode::Char('q') => {
                state.pending_delete = false;
                state.status = "Cancelled team delete.".to_string();
            }
            _ => {}
        }
        return Ok(true);
    }
    if state.pending_member_remove {
        match key.code {
            KeyCode::Char('y') => remove_member(request_json, args, state)?,
            KeyCode::Char('n') | KeyCode::Esc | KeyCode::Char('q') => {
                state.pending_member_remove = false;
                state.status = "Cancelled team membership removal.".to_string();
            }
            _ => {}
        }
        return Ok(true);
    }
    Ok(false)
}

fn confirm_delete<F>(
    request_json: &mut F,
    args: &TeamBrowseArgs,
    state: &mut BrowserState,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let row = state
        .selected_row()
        .ok_or_else(|| message("Team browse has no selected row to delete."))?
        .clone();
    if row_kind(&row) == "member" {
        return Err(message("Select a team row before deleting a team."));
    }
    let name = map_get_text(&row, "name");
    let delete = TeamDeleteArgs {
        common: args.common.clone(),
        team_id: Some(map_get_text(&row, "id")),
        name: None,
        prompt: false,
        yes: true,
        json: false,
    };
    let _ = delete_team_with_request(&mut *request_json, &delete)?;
    state.replace_rows(load_rows(&mut *request_json, args)?);
    state.status = format!("Deleted team {}.", name);
    Ok(())
}

fn remove_member<F>(
    request_json: &mut F,
    args: &TeamBrowseArgs,
    state: &mut BrowserState,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let row = state
        .selected_member_row()
        .ok_or_else(|| message("Team browse has no selected member to remove."))?
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
    let modify = TeamModifyArgs {
        common: args.common.clone(),
        team_id: Some(team_id.clone()),
        name: None,
        add_member: Vec::new(),
        remove_member: vec![identity.clone()],
        add_admin: Vec::new(),
        remove_admin: Vec::new(),
        json: false,
    };
    let _ = modify_team_with_request(&mut *request_json, &modify)?;
    state.replace_rows(load_rows(&mut *request_json, args)?);
    state.status = format!("Removed {} from team {}.", identity, team_name);
    Ok(())
}
