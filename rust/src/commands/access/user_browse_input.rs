//! Interactive browse workflows and terminal-driven state flow for Access entities.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use reqwest::Method;
use serde_json::{Map, Value};

use crate::access::render::map_get_text;
use crate::access::user::{delete_user_with_request, modify_user_with_request};
use crate::access::{request_object, Scope, UserDeleteArgs, UserModifyArgs};
use crate::common::{message, Result};

use super::user_browse_dialog::{EditDialogAction, EditDialogState};
use super::user_browse_state::{
    row_kind, BrowserState, DisplayMode, PaneFocus, SearchDirection, SearchState,
};
#[path = "user_browse_load.rs"]
mod user_browse_load;
pub(crate) use self::user_browse_load::load_rows;
use super::UserBrowseArgs;

pub(super) enum BrowseAction {
    Continue,
    Exit,
    JumpToTeam,
}

pub(super) fn handle_key<F>(
    request_json: &mut F,
    args: &UserBrowseArgs,
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
                state.status = "Cancelled user edit.".to_string();
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
            KeyCode::Char('y') => {
                confirm_delete(request_json, args, state)?;
            }
            KeyCode::Char('n') | KeyCode::Esc | KeyCode::Char('q') => {
                state.pending_delete = false;
                state.status = "Cancelled user delete.".to_string();
            }
            _ => {}
        }
        return Ok(BrowseAction::Continue);
    }
    if state.pending_member_remove {
        match key.code {
            KeyCode::Char('y') => confirm_member_remove(request_json, state)?,
            KeyCode::Char('n') | KeyCode::Esc | KeyCode::Char('q') => {
                state.pending_member_remove = false;
                state.status = "Cancelled team membership removal.".to_string();
            }
            _ => {}
        }
        return Ok(BrowseAction::Continue);
    }

    match key.code {
        KeyCode::BackTab => {
            state.focus_previous();
            state.status = format!(
                "Focused {} pane.",
                if state.focus == PaneFocus::List {
                    "list"
                } else {
                    "facts"
                }
            );
        }
        KeyCode::Tab => {
            state.focus_next();
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
            if state.display_mode == DisplayMode::GlobalAccounts {
                state.status = "Expanded user teams.".to_string();
            }
        }
        KeyCode::Left if state.focus == PaneFocus::List => {
            state.collapse_selected();
            if state.display_mode == DisplayMode::GlobalAccounts {
                state.status = "Collapsed user teams.".to_string();
            }
        }
        KeyCode::Char('/') => state.start_search(SearchDirection::Forward),
        KeyCode::Char('?') => state.start_search(SearchDirection::Backward),
        KeyCode::Char('n') => repeat_search(state),
        KeyCode::Char('v') => {
            if args.input_dir.is_some() {
                state.status =
                    "Local user browse keeps the account view only. Reopen a live browse for org-grouped memberships."
                        .to_string();
            } else if args.scope != Scope::Global {
                state.status =
                    "Display mode toggle is available only in global/all-org browse.".to_string();
            } else {
                state.display_mode = match state.display_mode {
                    DisplayMode::GlobalAccounts => DisplayMode::OrgMemberships,
                    DisplayMode::OrgMemberships => DisplayMode::GlobalAccounts,
                };
                state.replace_rows(load_rows(request_json, args, state.display_mode)?);
                state.status = match state.display_mode {
                    DisplayMode::GlobalAccounts => "Switched to global account view.".to_string(),
                    DisplayMode::OrgMemberships => {
                        "Switched to org-grouped membership view.".to_string()
                    }
                };
            }
        }
        KeyCode::Char('c') => {
            if state.display_mode != DisplayMode::GlobalAccounts {
                state.status =
                    "Expand/collapse all is available only in global account view.".to_string();
            } else {
                state.toggle_all_expanded();
                state.status = if state.expanded_user_ids.is_empty() {
                    "Collapsed all user team rows.".to_string()
                } else {
                    "Expanded all user team rows.".to_string()
                };
            }
        }
        KeyCode::Char('g') => {
            if args.input_dir.is_some() {
                state.status =
                    "Jumping from local user browse to team browse is unavailable. Open the team bundle directly with access team browse --input-dir ..."
                        .to_string();
            } else {
                return Ok(BrowseAction::JumpToTeam);
            }
        }
        KeyCode::Char('i') => {
            state.show_numbers = !state.show_numbers;
            state.status = if state.show_numbers {
                "Enabled row numbers in user list.".to_string()
            } else {
                "Hid row numbers in user list.".to_string()
            };
        }
        KeyCode::Char('l') => {
            state.replace_rows(load_rows(request_json, args, state.display_mode)?);
            state.status = if args.input_dir.is_some() {
                "Reloaded user browser from local bundle.".to_string()
            } else {
                "Refreshed user browser from live Grafana.".to_string()
            };
        }
        KeyCode::Char('e') => {
            if args.input_dir.is_some() {
                state.status =
                    "Local user browse is read-only. Use access user import or live browse to apply changes."
                        .to_string();
                return Ok(BrowseAction::Continue);
            }
            if state.display_mode == DisplayMode::OrgMemberships {
                state.status =
                    "Org-grouped membership view is browse-only for now. Press v for global accounts."
                        .to_string();
                return Ok(BrowseAction::Continue);
            }
            if state.selected_team_membership_row().is_some() {
                state.status = "Select a user row to edit the user.".to_string();
                return Ok(BrowseAction::Continue);
            }
            let row = state
                .selected_row()
                .ok_or_else(|| message("User browse has no selected user to edit."))?
                .clone();
            let login = map_get_text(&row, "login");
            state.pending_edit = Some(EditDialogState::new(&row));
            state.status = format!("Editing user {}.", login);
        }
        KeyCode::Char('d') => {
            if state.selected_team_membership_row().is_some() {
                if args.input_dir.is_some() {
                    state.status =
                        "Local user browse is read-only. Use access user browse against live Grafana to remove team memberships."
                            .to_string();
                } else {
                    state.pending_member_remove = true;
                    state.status = "Previewing team membership removal.".to_string();
                }
            } else if args.input_dir.is_some() {
                state.status =
                    "Local user browse is read-only. Use access user delete against live Grafana instead."
                        .to_string();
            } else if state.display_mode == DisplayMode::OrgMemberships {
                state.status =
                    "Org-grouped membership view is browse-only for now. Press v for global accounts."
                        .to_string();
            } else if state.selected_row().is_some() {
                state.pending_delete = true;
                state.status = "Previewing user delete.".to_string();
            }
        }
        KeyCode::Char('r') => {
            if state.selected_team_membership_row().is_some() {
                if args.input_dir.is_some() {
                    state.status =
                        "Local user browse is read-only. Use access user browse against live Grafana to remove team memberships."
                            .to_string();
                } else {
                    state.pending_member_remove = true;
                    state.status = "Previewing team membership removal.".to_string();
                }
            } else {
                state.status = "Select a team membership row to remove the membership.".to_string();
            }
        }
        KeyCode::Esc | KeyCode::Char('q') => return Ok(BrowseAction::Exit),
        _ => {}
    }
    Ok(BrowseAction::Continue)
}

fn save_edit<F>(request_json: &mut F, args: &UserBrowseArgs, state: &mut BrowserState) -> Result<()>
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

fn confirm_delete<F>(
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

fn confirm_member_remove<F>(request_json: &mut F, state: &mut BrowserState) -> Result<()>
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

fn handle_search_key(state: &mut BrowserState, key: &KeyEvent) {
    let Some(mut search) = state.pending_search.take() else {
        return;
    };
    match key.code {
        KeyCode::Esc if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.status = "Cancelled user search.".to_string();
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
                state.status = format!("No user matched '{query}'.");
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
        state.status = "No previous user search. Use / or ? first.".to_string();
        return;
    };
    if let Some(index) = state.repeat_last_search() {
        state.select_index(index);
        state.status = format!("Next match for '{}' at row {}.", last.query, index + 1);
    } else {
        state.status = format!("No more matches for '{}'.", last.query);
    }
}

fn current_detail_line_count(state: &BrowserState) -> usize {
    if state.pending_delete || state.pending_member_remove {
        return 6;
    }
    let Some(row) = state.selected_row() else {
        return 1;
    };
    match row_kind(row) {
        "org" => 4,
        "team" => 4,
        _ => 13,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::access::CommonCliArgs;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn search_prompt_treats_q_as_query_text() {
        let mut state = BrowserState::new(Vec::new(), DisplayMode::GlobalAccounts);
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
    fn load_rows_reads_local_user_bundle_without_live_requests() {
        let temp = tempdir().unwrap();
        fs::write(
            temp.path().join("users.json"),
            r#"{
                "kind":"grafana-utils-access-user-export-index",
                "version":1,
                "records":[
                    {"login":"alice","email":"alice@example.com","name":"Alice","orgRole":"Editor","scope":"org","teams":["ops","sre"]},
                    {"login":"bob","email":"bob@example.com","name":"Bob","scope":"global","teams":["platform"]}
                ]
            }"#,
        )
        .unwrap();
        let args = UserBrowseArgs {
            common: CommonCliArgs {
                profile: None,
                url: "http://127.0.0.1:3000".to_string(),
                api_token: None,
                username: Some("admin".to_string()),
                password: Some("admin".to_string()),
                prompt_password: false,
                prompt_token: false,
                org_id: None,
                timeout: 30,
                verify_ssl: false,
                insecure: false,
                ca_cert: None,
            },
            input_dir: Some(temp.path().to_path_buf()),
            scope: Scope::Org,
            all_orgs: false,
            current_org: false,
            query: None,
            login: Some("alice".to_string()),
            email: None,
            org_role: None,
            grafana_admin: None,
            with_teams: false,
            page: 1,
            per_page: 100,
        };

        let rows = load_rows(
            |_method, _path, _params, _payload| {
                panic!("local user browse should not hit the request layer")
            },
            &args,
            DisplayMode::GlobalAccounts,
        )
        .unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(map_get_text(&rows[0], "login"), "alice");
        assert_eq!(map_get_text(&rows[0], "teams"), "ops,sre");
    }

    #[test]
    fn user_detail_navigation_reaches_all_fact_rows() {
        let mut state = BrowserState::new(
            vec![Map::from_iter(vec![
                ("id".to_string(), Value::String("1".to_string())),
                ("login".to_string(), Value::String("alice".to_string())),
            ])],
            DisplayMode::GlobalAccounts,
        );

        let line_count = current_detail_line_count(&state);
        state.set_detail_cursor(line_count.saturating_sub(1), line_count);

        assert_eq!(line_count, 13);
        assert_eq!(state.detail_cursor, 12);
    }

    #[test]
    fn team_row_d_opens_membership_remove_confirmation_without_api() {
        let mut state = BrowserState::new(
            vec![Map::from_iter(vec![
                ("id".to_string(), Value::String("7".to_string())),
                ("login".to_string(), Value::String("alice".to_string())),
                (
                    "teamRows".to_string(),
                    Value::Array(vec![Value::Object(Map::from_iter(vec![
                        ("teamId".to_string(), Value::String("55".to_string())),
                        (
                            "teamName".to_string(),
                            Value::String("platform-ops".to_string()),
                        ),
                    ]))]),
                ),
            ])],
            DisplayMode::GlobalAccounts,
        );
        state.expand_selected();
        state.select_index(1);
        let args = UserBrowseArgs {
            common: CommonCliArgs {
                profile: None,
                url: "http://127.0.0.1:3000".to_string(),
                api_token: None,
                username: Some("admin".to_string()),
                password: Some("admin".to_string()),
                prompt_password: false,
                prompt_token: false,
                org_id: None,
                timeout: 30,
                verify_ssl: false,
                insecure: false,
                ca_cert: None,
            },
            input_dir: None,
            scope: Scope::Org,
            all_orgs: false,
            current_org: false,
            query: None,
            login: None,
            email: None,
            org_role: None,
            grafana_admin: None,
            with_teams: false,
            page: 1,
            per_page: 100,
        };

        let mut request_json = |_method: Method,
                                _path: &str,
                                _params: &[(String, String)],
                                _payload: Option<&Value>| {
            panic!("membership removal preview should not call Grafana before confirmation")
        };

        handle_key(
            &mut request_json,
            &args,
            &mut state,
            &KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE),
        )
        .unwrap();

        assert!(state.pending_member_remove);
        assert_eq!(state.status, "Previewing team membership removal.");
    }

    #[test]
    fn team_membership_remove_confirms_with_delete_and_refreshes_user_selection() {
        let mut state = BrowserState::new(
            vec![Map::from_iter(vec![
                ("id".to_string(), Value::String("7".to_string())),
                ("login".to_string(), Value::String("alice".to_string())),
                (
                    "teamRows".to_string(),
                    Value::Array(vec![Value::Object(Map::from_iter(vec![
                        ("teamId".to_string(), Value::String("55".to_string())),
                        (
                            "teamName".to_string(),
                            Value::String("platform-ops".to_string()),
                        ),
                    ]))]),
                ),
            ])],
            DisplayMode::GlobalAccounts,
        );
        state.expand_selected();
        state.select_index(1);
        state.pending_member_remove = true;

        let args = UserBrowseArgs {
            common: CommonCliArgs {
                profile: None,
                url: "http://127.0.0.1:3000".to_string(),
                api_token: None,
                username: Some("admin".to_string()),
                password: Some("admin".to_string()),
                prompt_password: false,
                prompt_token: false,
                org_id: None,
                timeout: 30,
                verify_ssl: false,
                insecure: false,
                ca_cert: None,
            },
            input_dir: None,
            scope: Scope::Org,
            all_orgs: false,
            current_org: false,
            query: None,
            login: None,
            email: None,
            org_role: None,
            grafana_admin: None,
            with_teams: false,
            page: 1,
            per_page: 100,
        };

        let mut delete_seen = false;
        let mut request_json =
            |method: Method, path: &str, _params: &[(String, String)], payload: Option<&Value>| {
                match (method.clone(), path) {
                    (Method::DELETE, "/api/teams/55/members/7") => {
                        delete_seen = true;
                        assert!(payload.is_none());
                        Ok(Some(Value::Object(Map::new())))
                    }
                    (Method::GET, "/api/org/users") => {
                        let user = Value::Object(Map::from_iter(vec![
                            ("id".to_string(), Value::String("7".to_string())),
                            ("login".to_string(), Value::String("alice".to_string())),
                            (
                                "email".to_string(),
                                Value::String("alice@example.com".to_string()),
                            ),
                            ("name".to_string(), Value::String("Alice".to_string())),
                            ("orgRole".to_string(), Value::String("Editor".to_string())),
                            ("scope".to_string(), Value::String("org".to_string())),
                        ]));
                        Ok(Some(Value::Array(vec![user])))
                    }
                    (Method::GET, "/api/users/7/teams") => {
                        if delete_seen {
                            Ok(Some(Value::Array(vec![])))
                        } else {
                            let team = Value::Object(Map::from_iter(vec![
                                ("id".to_string(), Value::String("55".to_string())),
                                (
                                    "name".to_string(),
                                    Value::String("platform-ops".to_string()),
                                ),
                            ]));
                            Ok(Some(Value::Array(vec![team])))
                        }
                    }
                    _ => panic!("unexpected request: {method:?} {path}"),
                }
            };

        handle_key(
            &mut request_json,
            &args,
            &mut state,
            &KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE),
        )
        .unwrap();

        assert!(delete_seen);
        assert!(!state.pending_member_remove);
        assert_eq!(state.status, "Removed membership from alice.");
        assert_eq!(state.selected_row().map(row_kind), Some("user"));
        assert_eq!(state.selected_user_id().as_deref(), Some("7"));
    }
}
