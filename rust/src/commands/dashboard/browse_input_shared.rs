#![cfg(feature = "tui")]

use crate::common::{message, Result};

use crate::dashboard::browse_render::render_dashboard_browser_frame;
use crate::dashboard::browse_state::BrowserState;
use crate::dashboard::browse_support::DashboardBrowseNode;
use crate::dashboard::browse_terminal::TerminalSession;
use crate::dashboard::{build_http_client_for_org, BrowseArgs};

pub(super) fn redraw_browser(
    session: &mut TerminalSession,
    state: &mut BrowserState,
) -> Result<()> {
    session
        .terminal
        .draw(|frame| render_dashboard_browser_frame(frame, state))?;
    Ok(())
}

pub(super) fn live_view_cache_key(node: &DashboardBrowseNode) -> Option<String> {
    node.uid
        .as_ref()
        .map(|uid| format!("{}::{uid}", node.org_id))
}

pub(super) fn scoped_org_client(
    args: &BrowseArgs,
    node: &DashboardBrowseNode,
) -> Result<Option<crate::http::JsonHttpClient>> {
    if !args.all_orgs {
        return Ok(None);
    }
    let org_id = node.org_id.parse::<i64>().map_err(|_| {
        message(format!(
            "Dashboard browse could not parse org id '{}'.",
            node.org_id
        ))
    })?;
    Ok(Some(build_http_client_for_org(&args.common, org_id)?))
}
