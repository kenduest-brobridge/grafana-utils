//! Team browser general key dispatch and state transitions.

use crossterm::event::{KeyCode, KeyEvent};
use reqwest::Method;
use serde_json::Value;

use crate::access::render::map_get_text;
use crate::access::TeamBrowseArgs;
use crate::common::{message, Result};

use super::team_browse_dialog::EditDialogState;
use super::team_browse_input::{load_rows, BrowseAction};
use super::team_browse_state::{row_kind, BrowserState, PaneFocus, SearchDirection};

pub(super) fn handle_normal_key<F>(
    request_json: &mut F,
    args: &TeamBrowseArgs,
    state: &mut BrowserState,
    key: &KeyEvent,
) -> Result<BrowseAction>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
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
            super::team_browse_actions::toggle_member_admin(request_json, args, state)?;
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

fn current_detail_line_count(state: &BrowserState) -> usize {
    if state.pending_delete || state.pending_member_remove {
        6
    } else if state.selected_member_row().is_some() {
        7
    } else {
        5
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
