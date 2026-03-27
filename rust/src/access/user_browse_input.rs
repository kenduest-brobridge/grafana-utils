use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use reqwest::Method;
use serde_json::{Map, Value};

use crate::access::render::{
    map_get_text, normalize_org_role, normalize_user_row, paginate_rows, scalar_text,
};
use crate::access::request_array;
use crate::access::user::{
    annotate_user_account_scope, delete_user_with_request, iter_global_users_with_request,
    list_org_users_with_request, list_user_teams_with_request, modify_user_with_request,
    validate_user_scope_auth,
};
use crate::access::{build_auth_context, Scope, UserDeleteArgs, UserModifyArgs};
use crate::common::{message, Result};

use super::user_browse_dialog::{EditDialogAction, EditDialogState};
use super::user_browse_state::{
    row_kind, row_matches_args, BrowserState, DisplayMode, PaneFocus, SearchDirection, SearchState,
};
use super::UserBrowseArgs;
use std::collections::{BTreeMap, BTreeSet};

type RawOrgUsers = (String, String, Vec<Map<String, Value>>);

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
            if args.scope != Scope::Global {
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
        KeyCode::Char('g') => return Ok(BrowseAction::JumpToTeam),
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
            state.status = "Refreshed user browser from live Grafana.".to_string();
        }
        KeyCode::Char('e') => {
            if state.display_mode == DisplayMode::OrgMemberships {
                state.status =
                    "Org-grouped membership view is browse-only for now. Press v for global accounts."
                        .to_string();
                return Ok(BrowseAction::Continue);
            }
            if state
                .selected_row()
                .map(row_kind)
                .is_some_and(|kind| kind == "team")
            {
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
            if state.display_mode == DisplayMode::OrgMemberships {
                state.status =
                    "Org-grouped membership view is browse-only for now. Press v for global accounts."
                        .to_string();
            } else if state
                .selected_row()
                .map(row_kind)
                .is_some_and(|kind| kind == "team")
            {
                state.status = "Select a user row to delete the user.".to_string();
            } else if state.selected_row().is_some() {
                state.pending_delete = true;
                state.status = "Previewing user delete.".to_string();
            }
        }
        KeyCode::Esc | KeyCode::Char('q') => return Ok(BrowseAction::Exit),
        _ => {}
    }
    Ok(BrowseAction::Continue)
}

pub(super) fn load_rows<F>(
    mut request_json: F,
    args: &UserBrowseArgs,
    display_mode: DisplayMode,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let auth_mode = build_auth_context(&args.common)?.auth_mode;
    validate_user_scope_auth(&args.scope, true, &auth_mode)?;
    let mut rows = match (args.scope.clone(), display_mode) {
        (Scope::Global, DisplayMode::OrgMemberships) => {
            load_grouped_org_membership_rows(&mut request_json, args)?
        }
        (Scope::Org, _) => list_org_users_with_request(&mut request_json)?
            .into_iter()
            .map(|item| normalize_user_row(&item, &Scope::Org))
            .collect::<Vec<_>>(),
        (Scope::Global, _) => {
            iter_global_users_with_request(&mut request_json, args.per_page.max(1))?
                .into_iter()
                .map(|item| normalize_user_row(&item, &Scope::Global))
                .collect::<Vec<_>>()
        }
    };
    if display_mode != DisplayMode::OrgMemberships {
        for row in &mut rows {
            let user_id = map_get_text(row, "id");
            let teams = list_user_teams_with_request(&mut request_json, &user_id)?
                .into_iter()
                .map(|team| crate::common::string_field(&team, "name", ""))
                .filter(|name| !name.is_empty())
                .map(Value::String)
                .collect::<Vec<_>>();
            row.insert("teams".to_string(), Value::Array(teams));
            row.insert("rowKind".to_string(), Value::String("user".to_string()));
        }
        if args.scope == Scope::Global {
            annotate_global_membership_summaries(&mut request_json, &mut rows)?;
        }
    }
    for row in &mut rows {
        if row_kind(row) != "org" {
            annotate_user_account_scope(std::slice::from_mut(row));
        }
    }
    rows.retain(|row| row_matches_args(row, args));
    Ok(paginate_rows(&rows, args.page, args.per_page))
}

fn annotate_global_membership_summaries<F>(
    request_json: &mut F,
    rows: &mut [Map<String, Value>],
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let orgs = request_array(
        &mut *request_json,
        Method::GET,
        "/api/orgs",
        &[],
        None,
        "Unexpected organization list response from Grafana.",
    )?;
    let mut summaries = BTreeMap::<String, Vec<String>>::new();
    let mut roles = BTreeMap::<String, BTreeSet<String>>::new();
    let mut org_counts = BTreeMap::<String, usize>::new();
    for org in orgs {
        let org_id = scalar_text(org.get("id"));
        let org_name = crate::common::string_field(&org, "name", "");
        let users = request_array(
            &mut *request_json,
            Method::GET,
            &format!("/api/orgs/{org_id}/users"),
            &[],
            None,
            &format!("Unexpected organization user list response for Grafana org {org_id}."),
        )?;
        for user in users {
            let user_id = {
                let value = scalar_text(user.get("userId"));
                if value.is_empty() {
                    scalar_text(user.get("id"))
                } else {
                    value
                }
            };
            let org_role = normalize_org_role(user.get("role").or_else(|| user.get("orgRole")));
            let summary_role = if org_role.is_empty() {
                "Unknown".to_string()
            } else {
                org_role.clone()
            };
            summaries
                .entry(user_id.clone())
                .or_default()
                .push(format!("{org_name}: {summary_role}"));
            if !org_role.is_empty() {
                roles.entry(user_id.clone()).or_default().insert(org_role);
            }
            *org_counts.entry(user_id).or_default() += 1;
        }
    }
    for row in rows {
        let user_id = map_get_text(row, "id");
        row.insert(
            "crossOrgMemberships".to_string(),
            Value::String(
                summaries
                    .get(&user_id)
                    .cloned()
                    .unwrap_or_default()
                    .join(" | "),
            ),
        );
        row.insert(
            "roleSummary".to_string(),
            Value::String(
                roles
                    .get(&user_id)
                    .map(|set| set.iter().cloned().collect::<Vec<_>>().join("/"))
                    .unwrap_or_default(),
            ),
        );
        row.insert(
            "orgMembershipCount".to_string(),
            Value::String(org_counts.get(&user_id).copied().unwrap_or(0).to_string()),
        );
    }
    Ok(())
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
        scope: args.scope.clone(),
        yes: true,
        json: false,
    };
    let _ = delete_user_with_request(&mut *request_json, &delete)?;
    state.replace_rows(load_rows(&mut *request_json, args, state.display_mode)?);
    state.status = format!("Deleted user {}.", login);
    Ok(())
}

fn handle_search_key(state: &mut BrowserState, key: &KeyEvent) {
    let Some(mut search) = state.pending_search.take() else {
        return;
    };
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
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
    if state.pending_delete {
        return 6;
    }
    let Some(row) = state.selected_row() else {
        return 1;
    };
    match row_kind(row) {
        "org" => 4,
        "team" => 4,
        _ => 10,
    }
}

fn load_grouped_org_membership_rows<F>(
    request_json: &mut F,
    args: &UserBrowseArgs,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let global_users = iter_global_users_with_request(&mut *request_json, args.per_page.max(1))?;
    let global_by_id = global_users
        .into_iter()
        .map(|user| {
            let normalized = normalize_user_row(&user, &Scope::Global);
            (map_get_text(&normalized, "id"), normalized)
        })
        .collect::<BTreeMap<_, _>>();

    let orgs = request_array(
        &mut *request_json,
        Method::GET,
        "/api/orgs",
        &[],
        None,
        "Unexpected organization list response from Grafana.",
    )?;

    let mut raw_orgs: Vec<RawOrgUsers> = Vec::new();
    let mut summaries: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for org in orgs {
        let org_id = scalar_text(org.get("id"));
        let org_name = crate::common::string_field(&org, "name", "");
        let users = request_array(
            &mut *request_json,
            Method::GET,
            &format!("/api/orgs/{org_id}/users"),
            &[],
            None,
            &format!("Unexpected organization user list response for Grafana org {org_id}."),
        )?;
        let mut membership_rows = Vec::new();
        for user in users {
            let user_id = {
                let value = scalar_text(user.get("userId"));
                if value.is_empty() {
                    scalar_text(user.get("id"))
                } else {
                    value
                }
            };
            let global = global_by_id.get(&user_id);
            let login = global
                .map(|row| map_get_text(row, "login"))
                .unwrap_or_else(|| crate::common::string_field(&user, "login", ""));
            let email = global
                .map(|row| map_get_text(row, "email"))
                .unwrap_or_else(|| crate::common::string_field(&user, "email", ""));
            let name = global
                .map(|row| map_get_text(row, "name"))
                .unwrap_or_else(|| crate::common::string_field(&user, "name", ""));
            let grafana_admin = global
                .map(|row| map_get_text(row, "grafanaAdmin"))
                .unwrap_or_default();
            let org_role = normalize_org_role(user.get("role").or_else(|| user.get("orgRole")));
            summaries
                .entry(user_id.clone())
                .or_default()
                .push(format!("{org_name}: {org_role}"));
            membership_rows.push(Map::from_iter(vec![
                ("rowKind".to_string(), Value::String("member".to_string())),
                (
                    "id".to_string(),
                    Value::String(format!("{org_id}:{user_id}")),
                ),
                ("userId".to_string(), Value::String(user_id)),
                ("orgId".to_string(), Value::String(org_id.clone())),
                ("orgName".to_string(), Value::String(org_name.clone())),
                (
                    "scope".to_string(),
                    Value::String("org-membership".to_string()),
                ),
                ("login".to_string(), Value::String(login)),
                ("email".to_string(), Value::String(email)),
                ("name".to_string(), Value::String(name)),
                ("orgRole".to_string(), Value::String(org_role)),
                ("grafanaAdmin".to_string(), Value::String(grafana_admin)),
                ("teams".to_string(), Value::String(String::new())),
            ]));
        }
        raw_orgs.push((org_id, org_name, membership_rows));
    }

    let mut rows = Vec::new();
    for (org_id, org_name, mut members) in raw_orgs {
        let member_count = members.len().to_string();
        rows.push(Map::from_iter(vec![
            ("rowKind".to_string(), Value::String("org".to_string())),
            ("id".to_string(), Value::String(org_id.clone())),
            ("orgId".to_string(), Value::String(org_id)),
            ("orgName".to_string(), Value::String(org_name.clone())),
            ("name".to_string(), Value::String(org_name)),
            ("memberCount".to_string(), Value::String(member_count)),
        ]));
        for member in &mut members {
            let user_id = map_get_text(member, "userId");
            let cross = summaries
                .get(&user_id)
                .cloned()
                .unwrap_or_default()
                .join(" | ");
            member.insert("crossOrgMemberships".to_string(), Value::String(cross));
        }
        rows.extend(members);
    }
    Ok(rows)
}
