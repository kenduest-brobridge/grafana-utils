#![cfg(feature = "tui")]
use crossterm::event::KeyEvent;
use reqwest::Method;
use serde_json::Value;

use crate::common::Result;

use super::browse_input_shared::scoped_org_client;
use crate::dashboard::browse_actions::{
    apply_dashboard_edit_save, begin_dashboard_edit, refresh_browser_document,
};
use crate::dashboard::browse_edit_dialog::EditDialogAction;
use crate::dashboard::browse_state::BrowserState;
use crate::dashboard::browse_support::DashboardBrowseNodeKind;
use crate::dashboard::BrowseArgs;

pub(super) fn handle_edit_dialog_key<F>(
    request_json: &mut F,
    args: &BrowseArgs,
    state: &mut BrowserState,
    key: &KeyEvent,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let Some(edit_state) = state.pending_edit.as_mut() else {
        return Ok(());
    };
    match edit_state.handle_key(key) {
        EditDialogAction::Continue => {}
        EditDialogAction::Cancelled => {
            state.pending_edit = None;
            state.status = "Cancelled dashboard edit.".to_string();
        }
        EditDialogAction::Save { draft, update } => {
            let Some(node) = state.selected_node().cloned() else {
                return Ok(());
            };
            state.pending_edit = None;
            let applied = if let Some(client) = scoped_org_client(args, &node)? {
                let mut scoped = |method: Method,
                                  path: &str,
                                  params: &[(String, String)],
                                  payload: Option<&Value>|
                 -> Result<Option<Value>> {
                    client.request_json(method, path, params, payload)
                };
                apply_dashboard_edit_save(&mut scoped, &state.document, &draft, &update)?
            } else {
                apply_dashboard_edit_save(request_json, &state.document, &draft, &update)?
            };
            if !applied {
                state.status = format!("No dashboard changes to apply for {}.", draft.uid);
                return Ok(());
            }
            let document = refresh_browser_document(request_json, args)?;
            state.replace_document(document);
            state.status = format!("Updated dashboard {}.", draft.uid);
            super::ensure_selected_dashboard_view(request_json, args, state, false)?;
        }
    }
    Ok(())
}

pub(super) fn start_selected_dashboard_edit<F>(
    request_json: &mut F,
    args: &BrowseArgs,
    state: &mut BrowserState,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let Some(node) = state.selected_node().cloned() else {
        return Ok(());
    };
    match node.kind {
        DashboardBrowseNodeKind::Org | DashboardBrowseNodeKind::Folder => {
            state.status =
                "Folder/org edit is not available in v2 yet. Select a dashboard row.".to_string();
        }
        DashboardBrowseNodeKind::Dashboard => {
            state.pending_edit = Some(if let Some(client) = scoped_org_client(args, &node)? {
                let mut scoped = |method: Method,
                                  path: &str,
                                  params: &[(String, String)],
                                  payload: Option<&Value>|
                 -> Result<Option<Value>> {
                    client.request_json(method, path, params, payload)
                };
                begin_dashboard_edit(&mut scoped, &state.document, &node)?
            } else {
                begin_dashboard_edit(request_json, &state.document, &node)?
            });
            state.status =
                "Editing dashboard in TUI dialog. Ctrl+S saves, Esc cancels.".to_string();
        }
    }
    Ok(())
}

pub(super) fn start_selected_dashboard_rename<F>(
    request_json: &mut F,
    args: &BrowseArgs,
    state: &mut BrowserState,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let Some(node) = state.selected_node().cloned() else {
        return Ok(());
    };
    match node.kind {
        DashboardBrowseNodeKind::Org | DashboardBrowseNodeKind::Folder => {
            state.status = "Rename is only available for dashboard rows right now.".to_string();
        }
        DashboardBrowseNodeKind::Dashboard => {
            let mut dialog = if let Some(client) = scoped_org_client(args, &node)? {
                let mut scoped = |method: Method,
                                  path: &str,
                                  params: &[(String, String)],
                                  payload: Option<&Value>|
                 -> Result<Option<Value>> {
                    client.request_json(method, path, params, payload)
                };
                begin_dashboard_edit(&mut scoped, &state.document, &node)?
            } else {
                begin_dashboard_edit(request_json, &state.document, &node)?
            };
            dialog.focus_title_rename();
            state.pending_edit = Some(dialog);
            state.status = "Rename dashboard in TUI dialog. Ctrl+S saves, Esc cancels.".to_string();
        }
    }
    Ok(())
}

pub(super) fn start_selected_dashboard_move<F>(
    request_json: &mut F,
    args: &BrowseArgs,
    state: &mut BrowserState,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let Some(node) = state.selected_node().cloned() else {
        return Ok(());
    };
    match node.kind {
        DashboardBrowseNodeKind::Org | DashboardBrowseNodeKind::Folder => {
            state.status = "Move is only available for dashboard rows right now.".to_string();
        }
        DashboardBrowseNodeKind::Dashboard => {
            let mut dialog = if let Some(client) = scoped_org_client(args, &node)? {
                let mut scoped = |method: Method,
                                  path: &str,
                                  params: &[(String, String)],
                                  payload: Option<&Value>|
                 -> Result<Option<Value>> {
                    client.request_json(method, path, params, payload)
                };
                begin_dashboard_edit(&mut scoped, &state.document, &node)?
            } else {
                begin_dashboard_edit(request_json, &state.document, &node)?
            };
            dialog.focus_folder_move();
            state.pending_edit = Some(dialog);
            state.status =
                "Move dashboard to another folder. Choose a folder, then Ctrl+S saves.".to_string();
        }
    }
    Ok(())
}
