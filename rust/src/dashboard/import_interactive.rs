#![cfg(feature = "tui")]
use std::io::{self, IsTerminal};
use std::path::PathBuf;

use reqwest::Method;
use serde_json::Value;

use crate::common::{message, Result};
use crate::grafana_api::DashboardResourceClient;

#[cfg(test)]
pub(crate) use super::import_interactive_loader::load_interactive_import_items;
pub(crate) use super::import_interactive_state::{
    InteractiveImportAction, InteractiveImportContextView, InteractiveImportDiffData,
    InteractiveImportDiffDepth, InteractiveImportGrouping, InteractiveImportItem,
    InteractiveImportReview, InteractiveImportReviewState, InteractiveImportState,
    InteractiveImportSummaryCounts, InteractiveImportSummaryScope,
};
use super::import_lookup::ImportLookupCache;

pub(crate) fn select_import_dashboard_files<F>(
    request_json: &mut F,
    lookup_cache: &mut ImportLookupCache,
    args: &super::ImportArgs,
) -> Result<Option<Vec<PathBuf>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if !args.interactive {
        return Ok(None);
    }
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(message("Dashboard import interactive mode requires a TTY."));
    }
    let (items, _folders_by_uid) =
        super::import_interactive_loader::load_interactive_import_context(args)?;
    if items.is_empty() {
        return Err(message(format!(
            "No dashboard JSON files were found under {}.",
            args.input_dir.display()
        )));
    }
    super::import_interactive_render::run_import_selector(
        request_json,
        lookup_cache,
        args,
        args.input_dir.display().to_string(),
        items,
    )
}

pub(crate) fn select_import_dashboard_files_with_client(
    client: &DashboardResourceClient<'_>,
    lookup_cache: &mut ImportLookupCache,
    args: &super::ImportArgs,
) -> Result<Option<Vec<PathBuf>>> {
    if !args.interactive {
        return Ok(None);
    }
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(message("Dashboard import interactive mode requires a TTY."));
    }
    let (items, _folders_by_uid) =
        super::import_interactive_loader::load_interactive_import_context(args)?;
    if items.is_empty() {
        return Err(message(format!(
            "No dashboard JSON files were found under {}.",
            args.input_dir.display()
        )));
    }
    super::import_interactive_render::run_import_selector_with_client(
        client,
        lookup_cache,
        args,
        args.input_dir.display().to_string(),
        items,
    )
}
