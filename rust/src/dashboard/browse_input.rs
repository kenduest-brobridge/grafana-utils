use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use reqwest::Method;
use serde_json::Value;

use crate::common::Result;

use super::browse_actions::{
    apply_dashboard_edit_save, begin_dashboard_edit, begin_dashboard_history, build_delete_preview,
    delete_status_message, execute_delete_plan_with_request, load_live_detail_lines,
    refresh_browser_document, restore_dashboard_history_version, run_external_dashboard_edit,
};
use super::browse_edit_dialog::EditDialogAction;
use super::browse_history_dialog::HistoryDialogAction;
use super::browse_state::BrowserState;
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
    if state.pending_history.is_some() {
        handle_history_dialog_key(request_json, args, state, key)?;
        return Ok(BrowserLoopAction::Continue);
    }
    if state.pending_edit.is_some() {
        handle_edit_dialog_key(request_json, args, state, key)?;
        return Ok(BrowserLoopAction::Continue);
    }

    match key.code {
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
            state.move_selection(-1);
            state.detail_scroll = 0;
            ensure_selected_dashboard_view(request_json, state, false)?;
            Ok(BrowserLoopAction::Continue)
        }
        KeyCode::Down if state.pending_delete.is_none() => {
            state.move_selection(1);
            state.detail_scroll = 0;
            ensure_selected_dashboard_view(request_json, state, false)?;
            Ok(BrowserLoopAction::Continue)
        }
        KeyCode::Home if state.pending_delete.is_none() => {
            state.select_first();
            state.detail_scroll = 0;
            ensure_selected_dashboard_view(request_json, state, false)?;
            Ok(BrowserLoopAction::Continue)
        }
        KeyCode::End if state.pending_delete.is_none() => {
            state.select_last();
            state.detail_scroll = 0;
            ensure_selected_dashboard_view(request_json, state, false)?;
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
            ensure_selected_dashboard_view(request_json, state, false)?;
            Ok(BrowserLoopAction::Continue)
        }
        KeyCode::Char('v') if state.pending_delete.is_none() => {
            refresh_selected_dashboard_view(request_json, state)?;
            Ok(BrowserLoopAction::Continue)
        }
        KeyCode::Char('h') if state.pending_delete.is_none() => {
            open_selected_dashboard_history(request_json, state)?;
            Ok(BrowserLoopAction::Continue)
        }
        KeyCode::Char('r')
            if state.pending_delete.is_none() && !key.modifiers.contains(KeyModifiers::SHIFT) =>
        {
            start_selected_dashboard_rename(request_json, state)?;
            Ok(BrowserLoopAction::Continue)
        }
        KeyCode::Char('m') if state.pending_delete.is_none() => {
            start_selected_dashboard_move(request_json, state)?;
            Ok(BrowserLoopAction::Continue)
        }
        KeyCode::Char('e')
            if state.pending_delete.is_none() && !key.modifiers.contains(KeyModifiers::SHIFT) =>
        {
            start_selected_dashboard_edit(request_json, state)?;
            Ok(BrowserLoopAction::Continue)
        }
        KeyCode::Char('E') if state.pending_delete.is_none() => {
            run_selected_external_edit(request_json, args, session, state)?;
            Ok(BrowserLoopAction::Continue)
        }
        KeyCode::Char('e')
            if state.pending_delete.is_none() && key.modifiers.contains(KeyModifiers::SHIFT) =>
        {
            run_selected_external_edit(request_json, args, session, state)?;
            Ok(BrowserLoopAction::Continue)
        }
        KeyCode::Char('d') if state.pending_delete.is_none() => {
            preview_selected_delete(request_json, args, state, false)?;
            Ok(BrowserLoopAction::Continue)
        }
        KeyCode::Char('D') if state.pending_delete.is_none() => {
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

fn handle_history_dialog_key<F>(
    request_json: &mut F,
    args: &BrowseArgs,
    state: &mut BrowserState,
    key: &KeyEvent,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let Some(history_state) = state.pending_history.as_mut() else {
        return Ok(());
    };
    match history_state.handle_key(key) {
        HistoryDialogAction::Continue => {}
        HistoryDialogAction::Close => {
            state.pending_history = None;
            state.status = "Closed dashboard history.".to_string();
        }
        HistoryDialogAction::Restore { uid, version } => {
            restore_dashboard_history_version(request_json, &uid, version)?;
            state.pending_history = None;
            let document = refresh_browser_document(request_json, args)?;
            state.replace_document(document);
            state.status = format!("Restored dashboard {} to version {}.", uid, version);
            ensure_selected_dashboard_view(request_json, state, false)?;
        }
    }
    Ok(())
}

fn handle_edit_dialog_key<F>(
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
            state.pending_edit = None;
            if !apply_dashboard_edit_save(request_json, &state.document, &draft, &update)? {
                state.status = format!("No dashboard changes to apply for {}.", draft.uid);
                return Ok(());
            }
            let document = refresh_browser_document(request_json, args)?;
            state.replace_document(document);
            state.status = format!("Updated dashboard {}.", draft.uid);
            ensure_selected_dashboard_view(request_json, state, false)?;
        }
    }
    Ok(())
}

pub(crate) fn ensure_selected_dashboard_view<F>(
    request_json: &mut F,
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
        DashboardBrowseNodeKind::Dashboard => {
            if let Some(uid) = node.uid.as_ref() {
                if state.live_view_cache.contains_key(uid) {
                    return Ok(());
                }
            }
            let lines = load_live_detail_lines(request_json, &node)?;
            if let Some(uid) = node.uid.as_ref() {
                state.live_view_cache.insert(uid.clone(), lines);
            }
            state.detail_scroll = 0;
            if announce {
                state.status = format!("Loaded live dashboard details for {}.", node.title);
            }
        }
        DashboardBrowseNodeKind::Folder => {
            if announce {
                state.status = "Folder rows already show live tree metadata.".to_string();
            }
        }
    }
    Ok(())
}

fn refresh_selected_dashboard_view<F>(request_json: &mut F, state: &mut BrowserState) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if let Some(uid) = state
        .selected_node()
        .and_then(|node| node.uid.as_ref().cloned())
    {
        state.live_view_cache.remove(&uid);
    }
    ensure_selected_dashboard_view(request_json, state, true)
}

fn open_selected_dashboard_history<F>(request_json: &mut F, state: &mut BrowserState) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let Some(node) = state.selected_node().cloned() else {
        return Ok(());
    };
    match node.kind {
        DashboardBrowseNodeKind::Dashboard => {
            state.pending_history = Some(begin_dashboard_history(request_json, &node)?);
            state.status = format!("Viewing dashboard history for {}.", node.title);
        }
        DashboardBrowseNodeKind::Folder => {
            state.status = "History is only available for dashboard rows.".to_string();
        }
    }
    Ok(())
}

fn start_selected_dashboard_edit<F>(request_json: &mut F, state: &mut BrowserState) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let Some(node) = state.selected_node().cloned() else {
        return Ok(());
    };
    match node.kind {
        DashboardBrowseNodeKind::Dashboard => {
            state.pending_edit = Some(begin_dashboard_edit(request_json, &state.document, &node)?);
            state.status =
                "Editing dashboard in TUI dialog. Ctrl+S saves, Esc cancels.".to_string();
        }
        DashboardBrowseNodeKind::Folder => {
            state.status =
                "Folder edit is not available in v2 yet. Select a dashboard row.".to_string();
        }
    }
    Ok(())
}

fn start_selected_dashboard_rename<F>(request_json: &mut F, state: &mut BrowserState) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let Some(node) = state.selected_node().cloned() else {
        return Ok(());
    };
    match node.kind {
        DashboardBrowseNodeKind::Dashboard => {
            let mut dialog = begin_dashboard_edit(request_json, &state.document, &node)?;
            dialog.focus_title_rename();
            state.pending_edit = Some(dialog);
            state.status = "Rename dashboard in TUI dialog. Ctrl+S saves, Esc cancels.".to_string();
        }
        DashboardBrowseNodeKind::Folder => {
            state.status = "Rename is only available for dashboard rows right now.".to_string();
        }
    }
    Ok(())
}

fn start_selected_dashboard_move<F>(request_json: &mut F, state: &mut BrowserState) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let Some(node) = state.selected_node().cloned() else {
        return Ok(());
    };
    match node.kind {
        DashboardBrowseNodeKind::Dashboard => {
            let mut dialog = begin_dashboard_edit(request_json, &state.document, &node)?;
            dialog.focus_folder_move();
            state.pending_edit = Some(dialog);
            state.status =
                "Move dashboard to another folder. Choose a folder, then Ctrl+S saves.".to_string();
        }
        DashboardBrowseNodeKind::Folder => {
            state.status = "Move is only available for dashboard rows right now.".to_string();
        }
    }
    Ok(())
}

fn run_selected_external_edit<F>(
    request_json: &mut F,
    args: &BrowseArgs,
    session: &mut TerminalSession,
    state: &mut BrowserState,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let Some(node) = state.selected_node().cloned() else {
        return Ok(());
    };
    match node.kind {
        DashboardBrowseNodeKind::Dashboard => {
            session.suspend()?;
            let raw_result = run_external_dashboard_edit(request_json, &node);
            session.resume()?;
            let (uid, applied) = raw_result?;
            if applied {
                let document = refresh_browser_document(request_json, args)?;
                state.replace_document(document);
                state.status = format!("Applied raw JSON edit for dashboard {}.", uid);
                ensure_selected_dashboard_view(request_json, state, false)?;
            } else {
                state.status = format!("Raw JSON edit cancelled or unchanged for {}.", uid);
            }
        }
        DashboardBrowseNodeKind::Folder => {
            state.status = "Raw JSON edit is only available for dashboard rows.".to_string();
        }
    }
    Ok(())
}

fn preview_selected_delete<F>(
    request_json: &mut F,
    args: &BrowseArgs,
    state: &mut BrowserState,
    include_folders: bool,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let Some(node) = state.selected_node().cloned() else {
        return Ok(());
    };
    state.pending_delete = Some(build_delete_preview(
        request_json,
        args,
        &node,
        include_folders,
    )?);
    state.detail_scroll = 0;
    state.status = delete_status_message(&node, include_folders);
    Ok(())
}

fn confirm_delete<F>(
    request_json: &mut F,
    args: &BrowseArgs,
    state: &mut BrowserState,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let Some(plan) = state.pending_delete.take() else {
        return Ok(());
    };
    let deleted = execute_delete_plan_with_request(request_json, &plan)?;
    let document = refresh_browser_document(request_json, args)?;
    state.replace_document(document);
    state.status = format!("Deleted {} item(s) from the live dashboard tree.", deleted);
    ensure_selected_dashboard_view(request_json, state, false)?;
    Ok(())
}
