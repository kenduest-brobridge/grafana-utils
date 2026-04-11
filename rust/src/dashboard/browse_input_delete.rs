#![cfg(feature = "tui")]
use reqwest::Method;
use serde_json::Value;

use crate::common::Result;

use super::browse_input_shared::scoped_org_client;
use crate::dashboard::browse_actions::{
    build_delete_preview, delete_status_message, execute_delete_plan_with_request,
    refresh_browser_document,
};
use crate::dashboard::browse_state::BrowserState;
use crate::dashboard::browse_support::DashboardBrowseNodeKind;
use crate::dashboard::BrowseArgs;

pub(super) fn preview_selected_delete<F>(
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
    if node.kind == DashboardBrowseNodeKind::Org {
        state.status =
            "Org rows do not support delete. Select a folder or dashboard row.".to_string();
        return Ok(());
    }
    state.pending_delete = Some(if let Some(client) = scoped_org_client(args, &node)? {
        let mut scoped = |method: Method,
                          path: &str,
                          params: &[(String, String)],
                          payload: Option<&Value>|
         -> Result<Option<Value>> {
            client.request_json(method, path, params, payload)
        };
        build_delete_preview(&mut scoped, args, &node, include_folders)?
    } else {
        build_delete_preview(request_json, args, &node, include_folders)?
    });
    state.detail_scroll = 0;
    state.status = delete_status_message(&node, include_folders);
    Ok(())
}

pub(super) fn confirm_delete<F>(
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
    let Some(node) = state.selected_node().cloned() else {
        return Ok(());
    };
    let deleted = if let Some(client) = scoped_org_client(args, &node)? {
        let mut scoped = |method: Method,
                          path: &str,
                          params: &[(String, String)],
                          payload: Option<&Value>|
         -> Result<Option<Value>> {
            client.request_json(method, path, params, payload)
        };
        execute_delete_plan_with_request(&mut scoped, &plan)?
    } else {
        execute_delete_plan_with_request(request_json, &plan)?
    };
    let document = refresh_browser_document(request_json, args)?;
    state.replace_document(document);
    state.status = format!("Deleted {} item(s) from the live dashboard tree.", deleted);
    super::ensure_selected_dashboard_view(request_json, args, state, false)?;
    Ok(())
}
