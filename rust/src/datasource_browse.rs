//! Interactive browse workflows and terminal-driven state flow for Core entities.

#[cfg(not(feature = "tui"))]
use crate::common::message;
use crate::common::Result;

use super::{resolve_target_client, DatasourceBrowseArgs};

#[cfg(feature = "tui")]
use super::datasource_browse_tui::run_datasource_browser_tui;

pub(crate) fn browse_datasources(args: &DatasourceBrowseArgs) -> Result<usize> {
    let client = resolve_target_client(&args.common, args.org_id)?;
    browse_datasources_with_client(&client, args)
}

#[cfg(feature = "tui")]
pub(crate) fn browse_datasources_with_client(
    client: &crate::http::JsonHttpClient,
    args: &DatasourceBrowseArgs,
) -> Result<usize> {
    run_datasource_browser_tui(client, args)
}

#[cfg(not(feature = "tui"))]
pub(crate) fn browse_datasources_with_client(
    _client: &crate::http::JsonHttpClient,
    _args: &DatasourceBrowseArgs,
) -> Result<usize> {
    Err(message(
        "Datasource browse requires TUI support, but it was not built in.",
    ))
}
