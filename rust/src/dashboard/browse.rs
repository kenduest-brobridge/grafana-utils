use std::io::{stdin, stdout, IsTerminal};

use crate::common::{message, Result};
use crate::http::JsonHttpClient;

use super::browse_tui::run_dashboard_browser_tui;
use super::{build_http_client, build_http_client_for_org, BrowseArgs};

pub(crate) fn browse_dashboards_with_client(
    client: &JsonHttpClient,
    args: &BrowseArgs,
) -> Result<usize> {
    ensure_interactive_terminal()?;
    run_dashboard_browser_tui(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        args,
    )
}

pub(crate) fn browse_dashboards_with_org_client(args: &BrowseArgs) -> Result<usize> {
    let client = match args.org_id {
        Some(org_id) => build_http_client_for_org(&args.common, org_id)?,
        None => build_http_client(&args.common)?,
    };
    browse_dashboards_with_client(&client, args)
}

fn ensure_interactive_terminal() -> Result<()> {
    if stdin().is_terminal() && stdout().is_terminal() {
        Ok(())
    } else {
        Err(message(
            "Dashboard browse requires an interactive terminal (TTY).",
        ))
    }
}
