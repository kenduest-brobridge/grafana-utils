//! Interactive browse workflows and terminal-driven state flow for Access entities.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use reqwest::Method;
use serde_json::{Map, Value};

use crate::common::{message, Result};

use super::team_browse_dialog::EditDialogAction;
use super::team_browse_state::{BrowserState, SearchState};
use super::TeamBrowseArgs;
use crate::access::render::{map_get_text, normalize_team_row, value_bool};
use crate::access::team::{
    iter_teams_with_request, list_team_members_with_request, team_member_identity,
};
use crate::access::team_import_export_diff::load_team_import_records;
use crate::access::ACCESS_EXPORT_KIND_TEAMS;

pub(super) enum BrowseAction {
    Continue,
    Exit,
    JumpToUser,
}

pub(super) fn handle_key<F>(
    request_json: &mut F,
    args: &TeamBrowseArgs,
    state: &mut BrowserState,
    key: &KeyEvent,
) -> Result<BrowseAction>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if let Some(edit) = state.pending_edit.as_mut() {
        match edit.handle_key(key) {
            EditDialogAction::None => return Ok(BrowseAction::Continue),
            EditDialogAction::Cancel => {
                state.pending_edit = None;
                state.status = "Cancelled team edit.".to_string();
                return Ok(BrowseAction::Continue);
            }
            EditDialogAction::Save => {
                super::team_browse_actions::save_edit(request_json, args, state)?;
                return Ok(BrowseAction::Continue);
            }
        }
    }
    if state.pending_search.is_some() {
        handle_search_key(state, key);
        return Ok(BrowseAction::Continue);
    }
    if super::team_browse_actions::handle_pending_confirmation_key(request_json, args, state, key)?
    {
        return Ok(BrowseAction::Continue);
    }
    super::team_browse_dispatch::handle_normal_key(request_json, args, state, key)
}

pub(super) fn load_rows<F>(
    mut request_json: F,
    args: &TeamBrowseArgs,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if args.input_dir.is_some() {
        return load_rows_from_input_dir(args);
    }
    let mut rows = iter_teams_with_request(&mut request_json, args.query.as_deref())?
        .into_iter()
        .map(|team| normalize_team_row(&team))
        .collect::<Vec<_>>();
    if let Some(name) = &args.name {
        rows.retain(|row| map_get_text(row, "name") == *name);
    }
    for row in &mut rows {
        let team_id = map_get_text(row, "id");
        let member_records = list_team_members_with_request(&mut request_json, &team_id)?;
        let members = member_records
            .iter()
            .map(team_member_identity)
            .filter(|identity| !identity.is_empty())
            .map(Value::String)
            .collect::<Vec<_>>();
        let member_rows = member_records
            .iter()
            .map(|member| {
                let login = crate::common::string_field(member, "login", "");
                let email = crate::common::string_field(member, "email", "");
                let name = crate::common::string_field(member, "name", "");
                let identity = if !login.is_empty() {
                    login.clone()
                } else {
                    team_member_identity(member)
                };
                let role = if value_bool(member.get("isAdmin"))
                    .unwrap_or_else(|| value_bool(member.get("admin")).unwrap_or(false))
                {
                    "Admin"
                } else {
                    "Member"
                };
                Value::Object(Map::from_iter(vec![
                    ("memberIdentity".to_string(), Value::String(identity)),
                    ("memberLogin".to_string(), Value::String(login)),
                    ("memberEmail".to_string(), Value::String(email)),
                    ("memberName".to_string(), Value::String(name)),
                    ("memberRole".to_string(), Value::String(role.to_string())),
                ]))
            })
            .collect::<Vec<_>>();
        row.insert("members".to_string(), Value::Array(members));
        row.insert("memberRows".to_string(), Value::Array(member_rows));
    }
    let start = args.per_page.saturating_mul(args.page.saturating_sub(1));
    Ok(rows.into_iter().skip(start).take(args.per_page).collect())
}

fn build_local_member_rows(team: &Map<String, Value>) -> Vec<Value> {
    let mut member_rows = Vec::new();
    for (field, role) in [("members", "Member"), ("admins", "Admin")] {
        if let Some(Value::Array(values)) = team.get(field) {
            for value in values {
                if let Some(identity) = value.as_str() {
                    let identity = identity.trim();
                    if identity.is_empty() {
                        continue;
                    }
                    member_rows.push(Value::Object(Map::from_iter(vec![
                        (
                            "memberIdentity".to_string(),
                            Value::String(identity.to_string()),
                        ),
                        ("memberLogin".to_string(), Value::String(String::new())),
                        ("memberEmail".to_string(), Value::String(String::new())),
                        ("memberName".to_string(), Value::String(String::new())),
                        ("memberRole".to_string(), Value::String(role.to_string())),
                    ])));
                }
            }
        }
    }
    member_rows
}

fn load_rows_from_input_dir(args: &TeamBrowseArgs) -> Result<Vec<Map<String, Value>>> {
    let input_dir = args
        .input_dir
        .as_ref()
        .ok_or_else(|| message("Team browse local mode requires --input-dir."))?;
    let mut rows = load_team_import_records(input_dir, ACCESS_EXPORT_KIND_TEAMS)?
        .into_iter()
        .map(|team| {
            let member_rows = build_local_member_rows(&team);
            let mut row = normalize_team_row(&team);
            row.insert("memberRows".to_string(), Value::Array(member_rows));
            row
        })
        .collect::<Vec<_>>();
    if let Some(query) = &args.query {
        let query = query.to_ascii_lowercase();
        rows.retain(|row| {
            map_get_text(row, "name")
                .to_ascii_lowercase()
                .contains(&query)
                || map_get_text(row, "email")
                    .to_ascii_lowercase()
                    .contains(&query)
                || map_get_text(row, "members")
                    .to_ascii_lowercase()
                    .contains(&query)
        });
    }
    if let Some(name) = &args.name {
        rows.retain(|row| map_get_text(row, "name") == *name);
    }
    let start = args.per_page.saturating_mul(args.page.saturating_sub(1));
    Ok(rows.into_iter().skip(start).take(args.per_page).collect())
}

fn handle_search_key(state: &mut BrowserState, key: &KeyEvent) {
    let Some(mut search) = state.pending_search.take() else {
        return;
    };
    match key.code {
        KeyCode::Esc if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.status = "Cancelled team search.".to_string();
        }
        KeyCode::Enter => {
            let query = search.query.trim().to_string();
            if query.is_empty() {
                state.status = "Search query is empty.".to_string();
            } else if let Some(index) = state.find_match(&query, search.direction) {
                state.select_index(index);
                state.last_search = Some(SearchState {
                    direction: search.direction,
                    query: query.clone(),
                });
                state.status = format!("Matched '{query}' at row {}.", index + 1);
            } else {
                state.status = format!("No team matched '{query}'.");
                state.last_search = Some(SearchState {
                    direction: search.direction,
                    query,
                });
            }
        }
        KeyCode::Backspace => {
            search.query.pop();
            state.pending_search = Some(search);
        }
        KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            search.query.push(ch);
            state.pending_search = Some(search);
        }
        _ => state.pending_search = Some(search),
    }
}

#[cfg(test)]
#[path = "team_browse_input_tests.rs"]
mod tests;
