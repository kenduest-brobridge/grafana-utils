use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use reqwest::Method;
use serde_json::{Map, Value};

use crate::common::{message, Result};

use super::team_browse_dialog::{EditDialogAction, EditDialogState};
use super::team_browse_state::{row_kind, BrowserState, PaneFocus, SearchDirection, SearchState};
use super::TeamBrowseArgs;
use crate::access::pending_delete::{delete_team_with_request, TeamDeleteArgs};
use crate::access::render::{map_get_text, normalize_team_row, value_bool};
use crate::access::team::{
    iter_teams_with_request, list_team_members_with_request, modify_team_with_request,
    team_member_identity,
};
use crate::access::TeamModifyArgs;

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
                save_edit(request_json, args, state)?;
                return Ok(BrowseAction::Continue);
            }
        }
    }
    if state.pending_search.is_some() {
        handle_search_key(state, key);
        return Ok(BrowseAction::Continue);
    }
    if state.pending_delete {
        match key.code {
            KeyCode::Char('y') => confirm_delete(request_json, args, state)?,
            KeyCode::Char('n') | KeyCode::Esc | KeyCode::Char('q') => {
                state.pending_delete = false;
                state.status = "Cancelled team delete.".to_string();
            }
            _ => {}
        }
        return Ok(BrowseAction::Continue);
    }

    match key.code {
        KeyCode::BackTab | KeyCode::Tab => {
            state.toggle_focus();
            state.status = format!(
                "Focused {} pane.",
                if state.focus == PaneFocus::List {
                    "list"
                } else {
                    "facts"
                }
            );
        }
        KeyCode::Up => {
            if state.focus == PaneFocus::List {
                state.move_selection(-1);
            } else {
                let line_count = current_detail_line_count(state);
                state.move_detail_cursor(-1, line_count);
            }
        }
        KeyCode::Down => {
            if state.focus == PaneFocus::List {
                state.move_selection(1);
            } else {
                let line_count = current_detail_line_count(state);
                state.move_detail_cursor(1, line_count);
            }
        }
        KeyCode::Home => {
            if state.focus == PaneFocus::List {
                state.select_first();
            } else {
                let line_count = current_detail_line_count(state);
                state.set_detail_cursor(0, line_count);
            }
        }
        KeyCode::End => {
            if state.focus == PaneFocus::List {
                state.select_last();
            } else {
                let line_count = current_detail_line_count(state);
                state.set_detail_cursor(line_count.saturating_sub(1), line_count);
            }
        }
        KeyCode::PageUp => {
            let line_count = current_detail_line_count(state);
            state.move_detail_cursor(-10, line_count);
        }
        KeyCode::PageDown => {
            let line_count = current_detail_line_count(state);
            state.move_detail_cursor(10, line_count);
        }
        KeyCode::Right | KeyCode::Enter if state.focus == PaneFocus::List => {
            state.expand_selected();
            state.status = "Expanded team members.".to_string();
        }
        KeyCode::Left if state.focus == PaneFocus::List => {
            state.collapse_selected();
            state.status = "Collapsed team members.".to_string();
        }
        KeyCode::Char('/') => state.start_search(SearchDirection::Forward),
        KeyCode::Char('?') => state.start_search(SearchDirection::Backward),
        KeyCode::Char('n') => repeat_search(state),
        KeyCode::Char('i') => {
            state.show_numbers = !state.show_numbers;
            state.status = if state.show_numbers {
                "Enabled row numbers in team list.".to_string()
            } else {
                "Hid row numbers in team list.".to_string()
            };
        }
        KeyCode::Char('c') => {
            state.toggle_all_expanded();
            state.status = if state.expanded_team_ids.is_empty() {
                "Collapsed all team member rows.".to_string()
            } else {
                "Expanded all team member rows.".to_string()
            };
        }
        KeyCode::Char('g') => return Ok(BrowseAction::JumpToUser),
        KeyCode::Char('l') => {
            state.replace_rows(load_rows(request_json, args)?);
            state.status = "Refreshed team browser from live Grafana.".to_string();
        }
        KeyCode::Char('e') => {
            let row = state
                .selected_row()
                .ok_or_else(|| message("Team browse has no selected team to edit."))?
                .clone();
            if row_kind(&row) == "member" {
                state.status = "Select a team row to edit team membership.".to_string();
                return Ok(BrowseAction::Continue);
            }
            let name = map_get_text(&row, "name");
            state.pending_edit = Some(EditDialogState::new(&row));
            state.status = format!("Editing team {}.", name);
        }
        KeyCode::Char('d') => {
            if let Some(row) = state.selected_row() {
                if row_kind(row) == "member" {
                    state.status = "Select a team row to delete a team.".to_string();
                    return Ok(BrowseAction::Continue);
                }
                state.pending_delete = true;
                state.status = "Previewing team delete.".to_string();
            }
        }
        KeyCode::Esc | KeyCode::Char('q') => return Ok(BrowseAction::Exit),
        _ => {}
    }
    Ok(BrowseAction::Continue)
}

pub(super) fn load_rows<F>(
    mut request_json: F,
    args: &TeamBrowseArgs,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
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

fn save_edit<F>(request_json: &mut F, args: &TeamBrowseArgs, state: &mut BrowserState) -> Result<()>
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
        yes: true,
        json: false,
    };
    let _ = delete_team_with_request(&mut *request_json, &delete)?;
    state.replace_rows(load_rows(&mut *request_json, args)?);
    state.status = format!("Deleted team {}.", name);
    Ok(())
}

fn handle_search_key(state: &mut BrowserState, key: &KeyEvent) {
    let Some(mut search) = state.pending_search.take() else {
        return;
    };
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
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

fn repeat_search(state: &mut BrowserState) {
    let Some(last) = state.last_search.clone() else {
        state.status = "No previous team search. Use / or ? first.".to_string();
        return;
    };
    if let Some(index) = state.repeat_last_search() {
        state.select_index(index);
        state.status = format!("Next match for '{}' at row {}.", last.query, index + 1);
    } else {
        state.status = format!("No more matches for '{}'.", last.query);
    }
}

fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn current_detail_line_count(state: &BrowserState) -> usize {
    if state.pending_delete {
        6
    } else {
        5
    }
}
