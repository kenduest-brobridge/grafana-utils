//! Interactive browse workflows and terminal-driven state flow for Access entities.

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
use crate::access::team_import_export_diff::load_team_import_records;
use crate::access::{TeamModifyArgs, ACCESS_EXPORT_KIND_TEAMS};

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
    if state.pending_member_remove {
        match key.code {
            KeyCode::Char('y') => remove_member(request_json, args, state)?,
            KeyCode::Char('n') | KeyCode::Esc | KeyCode::Char('q') => {
                state.pending_member_remove = false;
                state.status = "Cancelled team membership removal.".to_string();
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
        KeyCode::Char('g') => {
            if args.input_dir.is_some() {
                state.status =
                    "Jumping from local team browse to user browse is unavailable. Open the user bundle directly with access user browse --input-dir ..."
                        .to_string();
            } else {
                return Ok(BrowseAction::JumpToUser);
            }
        }
        KeyCode::Char('l') => {
            state.replace_rows(load_rows(request_json, args)?);
            state.status = if args.input_dir.is_some() {
                "Reloaded team browser from local bundle.".to_string()
            } else {
                "Refreshed team browser from live Grafana.".to_string()
            };
        }
        KeyCode::Char('e') => {
            if args.input_dir.is_some() {
                state.status =
                    "Local team browse is read-only. Use access team import or live browse to apply changes."
                        .to_string();
                return Ok(BrowseAction::Continue);
            }
            let row = state
                .selected_row()
                .ok_or_else(|| message("Team browse has no selected team to edit."))?
                .clone();
            if row_kind(&row) == "member" {
                state.status =
                    "Member rows do not edit user fields. Use access user browse to edit the user."
                        .to_string();
                return Ok(BrowseAction::Continue);
            }
            let name = map_get_text(&row, "name");
            state.pending_edit = Some(EditDialogState::new(&row));
            state.status = format!("Editing team {}.", name);
        }
        KeyCode::Char('a') => {
            if state.selected_member_row().is_none() {
                state.status = "Select a member row to toggle team admin state.".to_string();
                return Ok(BrowseAction::Continue);
            }
            if args.input_dir.is_some() {
                state.status =
                    "Local team browse is read-only. Use access team import or live browse to apply member changes."
                        .to_string();
                return Ok(BrowseAction::Continue);
            }
            toggle_member_admin(request_json, args, state)?;
        }
        KeyCode::Char('r') => {
            if state.selected_member_row().is_some() {
                if args.input_dir.is_some() {
                    state.status =
                        "Local team browse is read-only. Use access team import or live browse to apply member changes."
                            .to_string();
                    return Ok(BrowseAction::Continue);
                }
                state.pending_member_remove = true;
                state.status = "Previewing team membership removal.".to_string();
                return Ok(BrowseAction::Continue);
            }
            state.status = "Select a member row to remove a team membership.".to_string();
        }
        KeyCode::Char('d') => {
            if state.selected_member_row().is_some() {
                if args.input_dir.is_some() {
                    state.status =
                        "Local team browse is read-only. Use access team import or live browse to apply member changes."
                            .to_string();
                    return Ok(BrowseAction::Continue);
                }
                state.pending_member_remove = true;
                state.status = "Previewing team membership removal.".to_string();
                return Ok(BrowseAction::Continue);
            }
            if args.input_dir.is_some() {
                state.status =
                    "Local team browse is read-only. Use access team delete against live Grafana instead."
                        .to_string();
            } else if state.selected_row().is_some() {
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

fn toggle_member_admin<F>(
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
    if state.pending_delete || state.pending_member_remove {
        6
    } else if state.selected_member_row().is_some() {
        7
    } else {
        5
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::access::CommonCliArgs;
    use crossterm::event::KeyEvent;
    use reqwest::Method;
    use serde_json::json;
    use std::fs;
    use tempfile::tempdir;

    fn common_args(api_token: Option<&str>) -> CommonCliArgs {
        CommonCliArgs {
            profile: None,
            url: "http://127.0.0.1:3000".to_string(),
            api_token: api_token.map(ToOwned::to_owned),
            username: None,
            password: None,
            prompt_password: false,
            prompt_token: false,
            org_id: None,
            timeout: 30,
            verify_ssl: false,
            insecure: false,
            ca_cert: None,
        }
    }

    fn live_browse_args() -> TeamBrowseArgs {
        TeamBrowseArgs {
            common: common_args(Some("token")),
            input_dir: None,
            query: None,
            name: None,
            with_members: true,
            page: 1,
            per_page: 100,
        }
    }

    fn member_row(identity: &str, role: &str) -> Value {
        let email = format!("{identity}@example.com");
        Value::Object(Map::from_iter(vec![
            (
                "memberIdentity".to_string(),
                Value::String(identity.to_string()),
            ),
            ("memberRole".to_string(), Value::String(role.to_string())),
            (
                "memberLogin".to_string(),
                Value::String(identity.to_string()),
            ),
            ("memberEmail".to_string(), Value::String(email)),
            ("parentTeamId".to_string(), Value::String("7".to_string())),
            (
                "parentTeamName".to_string(),
                Value::String("platform-ops".to_string()),
            ),
        ]))
    }

    fn selected_member_state(members: Vec<Value>) -> BrowserState {
        let mut state = BrowserState::new(vec![Map::from_iter(vec![
            ("id".to_string(), Value::String("7".to_string())),
            (
                "name".to_string(),
                Value::String("platform-ops".to_string()),
            ),
            (
                "email".to_string(),
                Value::String("platform@example.com".to_string()),
            ),
            ("memberRows".to_string(), Value::Array(members)),
        ])]);
        state.expand_selected();
        state.select_index(1);
        state
    }

    #[test]
    fn search_prompt_treats_q_as_query_text() {
        let mut state = BrowserState::new(Vec::new());
        state.start_search(SearchDirection::Forward);

        handle_search_key(
            &mut state,
            &KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE),
        );

        assert_eq!(
            state
                .pending_search
                .as_ref()
                .map(|search| search.query.as_str()),
            Some("q")
        );
    }

    #[test]
    fn load_rows_reads_local_team_bundle_without_live_requests() {
        let temp = tempdir().unwrap();
        fs::write(
            temp.path().join("teams.json"),
            r#"{
                "kind":"grafana-utils-access-team-export-index",
                "version":1,
                "records":[
                    {"name":"platform-team","email":"platform@example.com","members":["alice"],"admins":["bob"]}
                ]
            }"#,
        )
        .unwrap();
        let args = TeamBrowseArgs {
            common: common_args(None),
            input_dir: Some(temp.path().to_path_buf()),
            query: None,
            name: Some("platform-team".to_string()),
            with_members: true,
            page: 1,
            per_page: 100,
        };

        let rows = load_rows(
            |_method, _path, _params, _payload| {
                panic!("local team browse should not hit the request layer")
            },
            &args,
        )
        .unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(map_get_text(&rows[0], "name"), "platform-team");
        assert_eq!(map_get_text(&rows[0], "members"), "alice,bob");
        assert!(
            matches!(rows[0].get("memberRows"), Some(Value::Array(values)) if values.len() == 2)
        );
    }

    #[test]
    fn member_row_edit_prompts_user_browse_instead_of_team_editor() {
        let mut state = selected_member_state(vec![member_row("alice", "Member")]);
        let args = live_browse_args();

        let mut request_json = |_method: Method,
                                _path: &str,
                                _params: &[(String, String)],
                                _payload: Option<&Value>|
         -> Result<Option<Value>> {
            panic!("member row edit should not call the request layer");
        };

        let action = handle_key(
            &mut request_json,
            &args,
            &mut state,
            &KeyEvent::new(
                crossterm::event::KeyCode::Char('e'),
                crossterm::event::KeyModifiers::NONE,
            ),
        )
        .unwrap();

        assert!(matches!(action, BrowseAction::Continue));
        assert!(state.pending_edit.is_none());
        assert!(state.status.contains("access user browse"));
    }

    #[test]
    fn member_row_remove_updates_membership_and_keeps_parent_selected() {
        let mut state = selected_member_state(vec![
            member_row("alice", "Member"),
            member_row("bob", "Admin"),
        ]);
        let args = live_browse_args();
        let mut removed = false;
        let mut request_json = |method: Method,
                                path: &str,
                                _params: &[(String, String)],
                                payload: Option<&Value>|
         -> Result<Option<Value>> {
            match (method, path) {
                (Method::GET, "/api/teams/search") => Ok(Some(json!({
                    "teams": [
                        {"id": "7", "name": "platform-ops", "email": "platform@example.com", "memberCount": 2}
                    ]
                }))),
                (Method::GET, "/api/teams/7") => Ok(Some(json!({
                    "id": "7",
                    "name": "platform-ops",
                    "email": "platform@example.com"
                }))),
                (Method::GET, "/api/teams/7/members") => {
                    if removed {
                        Ok(Some(json!([
                            {"email": "bob@example.com", "login": "bob", "name": "Bob", "isAdmin": true, "userId": "43"}
                        ])))
                    } else {
                        Ok(Some(json!([
                            {"email": "alice@example.com", "login": "alice", "name": "Alice", "isAdmin": false, "userId": "42"},
                            {"email": "bob@example.com", "login": "bob", "name": "Bob", "isAdmin": true, "userId": "43"}
                        ])))
                    }
                }
                (Method::GET, "/api/org/users") => Ok(Some(json!([
                    {"userId": "42", "login": "alice", "email": "alice@example.com", "name": "Alice"},
                    {"userId": "43", "login": "bob", "email": "bob@example.com", "name": "Bob"}
                ]))),
                (Method::DELETE, "/api/teams/7/members/42") => {
                    assert!(payload.is_none());
                    removed = true;
                    Ok(Some(json!({})))
                }
                other => panic!("unexpected request: {:?}", other),
            }
        };

        let action = handle_key(
            &mut request_json,
            &args,
            &mut state,
            &KeyEvent::new(
                crossterm::event::KeyCode::Char('r'),
                crossterm::event::KeyModifiers::NONE,
            ),
        )
        .unwrap();

        assert!(matches!(action, BrowseAction::Continue));
        assert!(state.pending_member_remove);
        assert_eq!(state.status, "Previewing team membership removal.");

        let action = handle_key(
            &mut request_json,
            &args,
            &mut state,
            &KeyEvent::new(
                crossterm::event::KeyCode::Char('y'),
                crossterm::event::KeyModifiers::NONE,
            ),
        )
        .unwrap();

        assert!(matches!(action, BrowseAction::Continue));
        assert!(state
            .status
            .contains("Removed alice from team platform-ops."));
        assert_eq!(state.selected_team_id().as_deref(), Some("7"));
        assert_eq!(state.rows.len(), 2);
        assert_eq!(map_get_text(&state.rows[1], "memberIdentity"), "bob");
    }

    #[test]
    fn member_row_d_opens_membership_remove_confirmation() {
        let mut state = selected_member_state(vec![member_row("alice", "Member")]);
        let args = live_browse_args();

        let action = handle_key(
            &mut |_method, _path, _params, _payload| {
                panic!("member-row delete preview should not call Grafana before confirmation")
            },
            &args,
            &mut state,
            &KeyEvent::new(
                crossterm::event::KeyCode::Char('d'),
                crossterm::event::KeyModifiers::NONE,
            ),
        )
        .unwrap();

        assert!(matches!(action, BrowseAction::Continue));
        assert!(state.pending_member_remove);
        assert_eq!(state.status, "Previewing team membership removal.");
    }

    #[test]
    fn member_row_toggle_admin_posts_the_team_admin_update_payload() {
        let mut state = selected_member_state(vec![member_row("alice", "Member")]);
        let args = live_browse_args();
        let mut admin_updated = false;
        let mut saw_payload = None::<Value>;
        let mut request_json = |method: Method,
                                path: &str,
                                _params: &[(String, String)],
                                payload: Option<&Value>|
         -> Result<Option<Value>> {
            match (method, path) {
                (Method::GET, "/api/teams/search") => Ok(Some(json!({
                    "teams": [
                        {"id": "7", "name": "platform-ops", "email": "platform@example.com", "memberCount": 1}
                    ]
                }))),
                (Method::GET, "/api/teams/7") => Ok(Some(json!({
                    "id": "7",
                    "name": "platform-ops",
                    "email": "platform@example.com"
                }))),
                (Method::GET, "/api/teams/7/members") => {
                    if admin_updated {
                        Ok(Some(json!([
                            {"email": "alice@example.com", "login": "alice", "name": "Alice", "isAdmin": true, "userId": "42"}
                        ])))
                    } else {
                        Ok(Some(json!([
                            {"email": "alice@example.com", "login": "alice", "name": "Alice", "isAdmin": false, "userId": "42"}
                        ])))
                    }
                }
                (Method::GET, "/api/org/users") => Ok(Some(json!([
                    {"userId": "42", "login": "alice", "email": "alice@example.com", "name": "Alice"}
                ]))),
                (Method::PUT, "/api/teams/7/members") => {
                    saw_payload = payload.cloned();
                    admin_updated = true;
                    Ok(Some(json!({})))
                }
                other => panic!("unexpected request: {:?}", other),
            }
        };

        let action = handle_key(
            &mut request_json,
            &args,
            &mut state,
            &KeyEvent::new(
                crossterm::event::KeyCode::Char('a'),
                crossterm::event::KeyModifiers::NONE,
            ),
        )
        .unwrap();

        assert!(matches!(action, BrowseAction::Continue));
        assert!(state
            .status
            .contains("Granted team admin to alice on platform-ops."));
        assert_eq!(
            saw_payload,
            Some(json!({
                "members": ["alice@example.com"],
                "admins": ["alice@example.com"]
            }))
        );
    }
}
