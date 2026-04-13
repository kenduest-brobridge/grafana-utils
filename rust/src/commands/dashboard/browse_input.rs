#![cfg(feature = "tui")]
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use reqwest::Method;
use serde_json::Value;

use crate::common::Result;

#[path = "browse_input_delete.rs"]
mod browse_input_delete;
#[path = "browse_input_edit.rs"]
mod browse_input_edit;
#[path = "browse_input_external_edit.rs"]
mod browse_input_external_edit;
#[path = "browse_input_history.rs"]
mod browse_input_history;
#[path = "browse_input_refresh.rs"]
mod browse_input_refresh;
#[path = "browse_input_search.rs"]
mod browse_input_search;
#[path = "browse_input_shared.rs"]
mod browse_input_shared;

use self::browse_input_delete::{confirm_delete, preview_selected_delete};
use self::browse_input_edit::{
    handle_edit_dialog_key, start_selected_dashboard_edit, start_selected_dashboard_move,
    start_selected_dashboard_rename,
};
use self::browse_input_external_edit::{
    handle_external_edit_dialog_key, handle_external_edit_error_key, run_selected_external_edit,
};
use self::browse_input_history::{handle_history_dialog_key, open_selected_dashboard_history};
use self::browse_input_refresh::refresh_selected_dashboard_view;
use self::browse_input_search::{handle_search_dialog_key, repeat_search};
use self::browse_input_shared::{live_view_cache_key, scoped_org_client};

use super::browse_actions::{load_live_detail_lines, refresh_browser_document};
use super::browse_state::{BrowserState, PaneFocus, SearchDirection};
use super::browse_support::DashboardBrowseNodeKind;
use super::browse_terminal::TerminalSession;
use super::BrowseArgs;

pub(crate) enum BrowserLoopAction {
    Continue,
    Exit,
}

pub(crate) fn handle_browser_key<F>(
    request_json: &mut F,
    args: &BrowseArgs,
    session: &mut TerminalSession,
    state: &mut BrowserState,
    key: &KeyEvent,
) -> Result<BrowserLoopAction>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if state.completion_notice.is_some() {
        state.completion_notice = None;
        return Ok(BrowserLoopAction::Continue);
    }
    if state.pending_external_edit_error.is_some() {
        handle_external_edit_error_key(request_json, args, session, state, key)?;
        return Ok(BrowserLoopAction::Continue);
    }
    if state.pending_history.is_some() {
        handle_history_dialog_key(request_json, args, session, state, key)?;
        return Ok(BrowserLoopAction::Continue);
    }
    if state.pending_search.is_some() {
        handle_search_dialog_key(state, key)?;
        return Ok(BrowserLoopAction::Continue);
    }
    if state.pending_external_edit.is_some() {
        handle_external_edit_dialog_key(request_json, args, session, state, key)?;
        return Ok(BrowserLoopAction::Continue);
    }
    if state.pending_edit.is_some() {
        handle_edit_dialog_key(request_json, args, state, key)?;
        return Ok(BrowserLoopAction::Continue);
    }

    match key.code {
        KeyCode::BackTab if state.pending_delete.is_none() => {
            state.focus_previous_pane();
            state.status = format!("Focused {} pane.", state.focus_label());
            Ok(BrowserLoopAction::Continue)
        }
        KeyCode::Tab if state.pending_delete.is_none() => {
            state.focus_next_pane();
            state.status = format!("Focused {} pane.", state.focus_label());
            Ok(BrowserLoopAction::Continue)
        }
        KeyCode::Esc => {
            if state.pending_delete.is_some() {
                state.pending_delete = None;
                state.detail_scroll = 0;
                state.status = "Cancelled delete preview.".to_string();
                Ok(BrowserLoopAction::Continue)
            } else {
                Ok(BrowserLoopAction::Exit)
            }
        }
        KeyCode::Char('q') => Ok(BrowserLoopAction::Exit),
        KeyCode::Up if state.pending_delete.is_none() => {
            if state.focus == PaneFocus::Tree {
                state.move_selection(-1);
                state.detail_scroll = 0;
                ensure_selected_dashboard_view(request_json, args, state, false)?;
            } else {
                state.detail_scroll = state.detail_scroll.saturating_sub(1);
            }
            Ok(BrowserLoopAction::Continue)
        }
        KeyCode::Down if state.pending_delete.is_none() => {
            if state.focus == PaneFocus::Tree {
                state.move_selection(1);
                state.detail_scroll = 0;
                ensure_selected_dashboard_view(request_json, args, state, false)?;
            } else {
                state.detail_scroll = state.detail_scroll.saturating_add(1);
            }
            Ok(BrowserLoopAction::Continue)
        }
        KeyCode::Home if state.pending_delete.is_none() => {
            if state.focus == PaneFocus::Tree {
                state.select_first();
                state.detail_scroll = 0;
                ensure_selected_dashboard_view(request_json, args, state, false)?;
            } else {
                state.detail_scroll = 0;
            }
            Ok(BrowserLoopAction::Continue)
        }
        KeyCode::End if state.pending_delete.is_none() => {
            if state.focus == PaneFocus::Tree {
                state.select_last();
                state.detail_scroll = 0;
                ensure_selected_dashboard_view(request_json, args, state, false)?;
            } else {
                state.detail_scroll = u16::MAX.saturating_sub(32);
            }
            Ok(BrowserLoopAction::Continue)
        }
        KeyCode::PageUp => {
            state.detail_scroll = state.detail_scroll.saturating_sub(8);
            Ok(BrowserLoopAction::Continue)
        }
        KeyCode::PageDown => {
            state.detail_scroll = state.detail_scroll.saturating_add(8);
            Ok(BrowserLoopAction::Continue)
        }
        KeyCode::Char('l') => {
            let document = refresh_browser_document(request_json, args)?;
            state.replace_document(document);
            state.status = "Refreshed dashboard tree.".to_string();
            ensure_selected_dashboard_view(request_json, args, state, false)?;
            Ok(BrowserLoopAction::Continue)
        }
        KeyCode::Char('/') if state.pending_delete.is_none() => {
            state.start_search(SearchDirection::Forward);
            Ok(BrowserLoopAction::Continue)
        }
        KeyCode::Char('?') if state.pending_delete.is_none() => {
            state.start_search(SearchDirection::Backward);
            Ok(BrowserLoopAction::Continue)
        }
        KeyCode::Char('n') if state.pending_delete.is_none() => {
            repeat_search(request_json, args, state)?;
            Ok(BrowserLoopAction::Continue)
        }
        KeyCode::Char('v') if state.pending_delete.is_none() => {
            if state.local_mode {
                state.status = "Local browse shows tree facts from export files. Live dashboard details are unavailable.".to_string();
                return Ok(BrowserLoopAction::Continue);
            }
            refresh_selected_dashboard_view(request_json, args, state)?;
            Ok(BrowserLoopAction::Continue)
        }
        KeyCode::Char('h') if state.pending_delete.is_none() => {
            if state.local_mode {
                state.status = "Local browse does not support live history browsing.".to_string();
                return Ok(BrowserLoopAction::Continue);
            }
            open_selected_dashboard_history(request_json, args, state)?;
            Ok(BrowserLoopAction::Continue)
        }
        KeyCode::Char('r')
            if state.pending_delete.is_none() && !key.modifiers.contains(KeyModifiers::SHIFT) =>
        {
            if state.local_mode {
                state.status = "Local browse is read-only. Rename is unavailable.".to_string();
                return Ok(BrowserLoopAction::Continue);
            }
            start_selected_dashboard_rename(request_json, args, state)?;
            Ok(BrowserLoopAction::Continue)
        }
        KeyCode::Char('m') if state.pending_delete.is_none() => {
            if state.local_mode {
                state.status = "Local browse is read-only. Move is unavailable.".to_string();
                return Ok(BrowserLoopAction::Continue);
            }
            start_selected_dashboard_move(request_json, args, state)?;
            Ok(BrowserLoopAction::Continue)
        }
        KeyCode::Char('e')
            if state.pending_delete.is_none() && !key.modifiers.contains(KeyModifiers::SHIFT) =>
        {
            if state.local_mode {
                state.status = "Local browse is read-only. Edit dialog is unavailable.".to_string();
                return Ok(BrowserLoopAction::Continue);
            }
            start_selected_dashboard_edit(request_json, args, state)?;
            Ok(BrowserLoopAction::Continue)
        }
        KeyCode::Char('E') if state.pending_delete.is_none() => {
            if state.local_mode {
                state.status =
                    "Local browse is read-only. Raw JSON edit is unavailable.".to_string();
                return Ok(BrowserLoopAction::Continue);
            }
            run_selected_external_edit(request_json, args, session, state)?;
            Ok(BrowserLoopAction::Continue)
        }
        KeyCode::Char('e')
            if state.pending_delete.is_none() && key.modifiers.contains(KeyModifiers::SHIFT) =>
        {
            if state.local_mode {
                state.status =
                    "Local browse is read-only. Raw JSON edit is unavailable.".to_string();
                return Ok(BrowserLoopAction::Continue);
            }
            run_selected_external_edit(request_json, args, session, state)?;
            Ok(BrowserLoopAction::Continue)
        }
        KeyCode::Char('d') if state.pending_delete.is_none() => {
            if state.local_mode {
                state.status = "Local browse is read-only. Delete is unavailable.".to_string();
                return Ok(BrowserLoopAction::Continue);
            }
            preview_selected_delete(request_json, args, state, false)?;
            Ok(BrowserLoopAction::Continue)
        }
        KeyCode::Char('D') if state.pending_delete.is_none() => {
            if state.local_mode {
                state.status = "Local browse is read-only. Delete is unavailable.".to_string();
                return Ok(BrowserLoopAction::Continue);
            }
            let include_folders = matches!(
                state.selected_node().map(|node| node.kind.clone()),
                Some(DashboardBrowseNodeKind::Folder)
            );
            preview_selected_delete(request_json, args, state, include_folders)?;
            Ok(BrowserLoopAction::Continue)
        }
        KeyCode::Char('n') if state.pending_delete.is_some() => {
            state.pending_delete = None;
            state.detail_scroll = 0;
            state.status = "Cancelled delete preview.".to_string();
            Ok(BrowserLoopAction::Continue)
        }
        KeyCode::Char('y') if state.pending_delete.is_some() => {
            confirm_delete(request_json, args, state)?;
            Ok(BrowserLoopAction::Continue)
        }
        _ => Ok(BrowserLoopAction::Continue),
    }
}

pub(crate) fn ensure_selected_dashboard_view<F>(
    request_json: &mut F,
    args: &BrowseArgs,
    state: &mut BrowserState,
    announce: bool,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let Some(node) = state.selected_node().cloned() else {
        return Ok(());
    };
    match node.kind {
        DashboardBrowseNodeKind::Org => {
            if announce {
                state.status = if state.local_mode {
                    "Org rows summarize the local export scope. Select a folder or dashboard row."
                        .to_string()
                } else {
                    "Org rows summarize browse scope. Select a folder or dashboard row.".to_string()
                };
            }
        }
        DashboardBrowseNodeKind::Dashboard => {
            if state.local_mode {
                if announce {
                    state.status =
                        "Local browse shows dashboard facts from export files. Live details and actions are unavailable."
                            .to_string();
                }
                state.detail_scroll = 0;
                return Ok(());
            }
            if let Some(cache_key) = live_view_cache_key(&node) {
                if state.live_view_cache.contains_key(&cache_key) {
                    return Ok(());
                }
            }
            let lines = if let Some(client) = scoped_org_client(args, &node)? {
                let mut scoped = |method: Method,
                                  path: &str,
                                  params: &[(String, String)],
                                  payload: Option<&Value>|
                 -> Result<Option<Value>> {
                    client.request_json(method, path, params, payload)
                };
                load_live_detail_lines(&mut scoped, &node)?
            } else {
                load_live_detail_lines(request_json, &node)?
            };
            if let Some(cache_key) = live_view_cache_key(&node) {
                state.live_view_cache.insert(cache_key, lines);
            }
            state.detail_scroll = 0;
            if announce {
                state.status = format!("Loaded live dashboard details for {}.", node.title);
            }
        }
        DashboardBrowseNodeKind::Folder => {
            if announce {
                state.status = if state.local_mode {
                    "Folder rows already show local tree metadata.".to_string()
                } else {
                    "Folder rows already show live tree metadata.".to_string()
                };
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::browse_support::DashboardBrowseDocument;
    use super::super::browse_support::DashboardBrowseSummary;
    use super::*;

    fn empty_document() -> DashboardBrowseDocument {
        DashboardBrowseDocument {
            summary: DashboardBrowseSummary {
                root_path: None,
                dashboard_count: 0,
                folder_count: 0,
                org_count: 1,
                scope_label: "current-org".to_string(),
            },
            nodes: Vec::new(),
        }
    }

    #[test]
    fn search_prompt_treats_q_as_query_text() {
        let mut state = BrowserState::new(empty_document());
        state.start_search(SearchDirection::Forward);

        handle_search_dialog_key(
            &mut state,
            &KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE),
        )
        .expect("search key should succeed");

        assert_eq!(
            state
                .pending_search
                .as_ref()
                .map(|search| search.query.as_str()),
            Some("q")
        );
    }

    #[test]
    fn external_edit_preview_lines_hide_staging_path_noise() {
        let lines = super::browse_input_external_edit::render_external_edit_preview_lines(
            "create-or-update",
            &[[
                "two-prom-query-smoke".to_string(),
                "exists".to_string(),
                "update".to_string(),
                "General".to_string(),
                "General".to_string(),
                "General".to_string(),
                "".to_string(),
                "/tmp/grafana-dashboard-browse-preview/dashboard.json".to_string(),
            ]],
        );

        let joined = lines.join("\n");
        assert!(joined.contains("Preview only. Nothing has been written to Grafana yet."));
        assert!(joined.contains("Dashboard: two-prom-query-smoke"));
        assert!(joined.contains("Action: update"));
        assert!(joined.contains("Folder: General"));
        assert!(!joined.contains("dashboard.json"));
    }
}
