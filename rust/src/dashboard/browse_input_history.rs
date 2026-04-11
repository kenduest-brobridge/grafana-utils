#![cfg(feature = "tui")]
use crossterm::event::KeyEvent;
use reqwest::Method;
use serde_json::Value;

use crate::common::Result;

use super::browse_input_shared::{redraw_browser, scoped_org_client};
use crate::dashboard::browse_actions::{
    begin_dashboard_history, refresh_browser_document, restore_dashboard_history_version,
};
use crate::dashboard::browse_history_dialog::HistoryDialogAction;
use crate::dashboard::browse_state::BrowserState;
use crate::dashboard::browse_support::DashboardBrowseNodeKind;
use crate::dashboard::browse_terminal::TerminalSession;
use crate::dashboard::BrowseArgs;

pub(super) fn handle_history_dialog_key<F>(
    request_json: &mut F,
    args: &BrowseArgs,
    session: &mut TerminalSession,
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
        HistoryDialogAction::Restore {
            uid,
            version,
            message,
        } => {
            let Some(node) = state.selected_node().cloned() else {
                return Ok(());
            };
            if let Some(dialog) = state.pending_history.as_mut() {
                dialog.set_busy_message(format!(
                    "Working... restoring {} to version {}.",
                    uid, version
                ));
            }
            state.status = format!("Restoring {} to version {}...", uid, version);
            redraw_browser(session, state)?;
            if let Some(client) = scoped_org_client(args, &node)? {
                let mut scoped = |method: Method,
                                  path: &str,
                                  params: &[(String, String)],
                                  payload: Option<&Value>|
                 -> Result<Option<Value>> {
                    client.request_json(method, path, params, payload)
                };
                restore_dashboard_history_version(&mut scoped, &uid, version, &message)?;
            } else {
                restore_dashboard_history_version(request_json, &uid, version, &message)?;
            }
            state.pending_history = None;
            let document = refresh_browser_document(request_json, args)?;
            state.replace_document(document);
            state.status = format!("Restored dashboard {} to version {}.", uid, version);
            super::ensure_selected_dashboard_view(request_json, args, state, false)?;
        }
    }
    Ok(())
}

pub(super) fn open_selected_dashboard_history<F>(
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
            state.status = "History is only available for dashboard rows.".to_string();
        }
        DashboardBrowseNodeKind::Dashboard => {
            state.pending_history = Some(if let Some(client) = scoped_org_client(args, &node)? {
                let mut scoped = |method: Method,
                                  path: &str,
                                  params: &[(String, String)],
                                  payload: Option<&Value>|
                 -> Result<Option<Value>> {
                    client.request_json(method, path, params, payload)
                };
                begin_dashboard_history(&mut scoped, &node)?
            } else {
                begin_dashboard_history(request_json, &node)?
            });
            state.status = format!("Viewing dashboard history for {}.", node.title);
        }
    }
    Ok(())
}
