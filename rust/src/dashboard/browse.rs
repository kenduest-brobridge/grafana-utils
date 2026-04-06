//! Interactive browse workflows and terminal-driven state flow for Dashboard entities.

#[cfg(feature = "tui")]
use std::io::{stdin, stdout, IsTerminal};

#[cfg(feature = "tui")]
use crate::common::message;
use crate::common::Result;
use crate::http::JsonHttpClient;

#[cfg(feature = "tui")]
use super::browse_tui::run_dashboard_browser_tui;
use super::BrowseArgs;
#[cfg(feature = "tui")]
use super::{build_http_client, build_http_client_for_org};

#[cfg(feature = "tui")]
pub(crate) fn browse_dashboards_with_client(
    client: &JsonHttpClient,
    args: &BrowseArgs,
) -> Result<usize> {
    if args.input_dir.is_some() {
        return browse_dashboards_with_local_args(args);
    }
    ensure_interactive_terminal()?;
    run_dashboard_browser_tui(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        args,
    )
}

#[cfg(feature = "tui")]
pub(crate) fn browse_dashboards_with_org_client(args: &BrowseArgs) -> Result<usize> {
    if args.input_dir.is_some() {
        return browse_dashboards_with_local_args(args);
    }
    let client = if args.all_orgs {
        build_http_client(&args.common)?
    } else {
        match args.org_id {
            Some(org_id) => build_http_client_for_org(&args.common, org_id)?,
            None => build_http_client(&args.common)?,
        }
    };
    browse_dashboards_with_client(&client, args)
}

#[cfg(feature = "tui")]
fn browse_dashboards_with_local_args(args: &BrowseArgs) -> Result<usize> {
    ensure_interactive_terminal()?;
    run_dashboard_browser_tui(
        |_method, _path, _params, _payload| {
            Err(message(
                "Local dashboard browse does not use live Grafana requests.",
            ))
        },
        args,
    )
}

#[cfg(feature = "tui")]
fn ensure_interactive_terminal() -> Result<()> {
    if stdin().is_terminal() && stdout().is_terminal() {
        Ok(())
    } else {
        Err(message(
            "Dashboard browse requires an interactive terminal (TTY).",
        ))
    }
}

#[cfg(not(feature = "tui"))]
pub(crate) fn browse_dashboards_with_client(
    _client: &JsonHttpClient,
    _args: &BrowseArgs,
) -> Result<usize> {
    super::tui_not_built("browse")
}

#[cfg(not(feature = "tui"))]
pub(crate) fn browse_dashboards_with_org_client(_args: &BrowseArgs) -> Result<usize> {
    super::tui_not_built("browse")
}
