//! CLI definitions for dashboard list workflows.

use clap::Args;

use super::super::super::DEFAULT_PAGE_SIZE;
use super::super::cli_defs_shared::{CommonCliArgs, SimpleOutputFormat};
use super::super::dashboard_runtime::parse_dashboard_list_output_column;

/// Arguments for listing dashboards from live Grafana.
#[derive(Debug, Clone, Args)]
pub struct ListArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long, default_value_t = DEFAULT_PAGE_SIZE, help = "Dashboard search page size.")]
    pub page_size: usize,
    #[arg(
        long,
        conflicts_with = "all_orgs",
        help = "List dashboards from one explicit Grafana org ID instead of the current org. Use this when the same Basic auth credentials can reach multiple orgs."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "org_id",
        help = "Enumerate all visible Grafana orgs and aggregate dashboard list output across them. Prefer Basic auth when you need cross-org listing because API tokens are often scoped to one org."
    )]
    pub all_orgs: bool,
    #[arg(
        long = "show-sources",
        alias = "with-sources",
        default_value_t = false,
        help = "Fetch each dashboard payload and include resolved datasource names in the list output. This is slower because it makes extra API calls per dashboard."
    )]
    pub show_sources: bool,
    #[arg(
        long,
        value_delimiter = ',',
        value_parser = parse_dashboard_list_output_column,
        help = "Render only these comma-separated list columns. Supported values: uid, name, folder, folder_uid, path, org, org_id, sources, source_uids. JSON-style aliases like folderUid, orgId, and sourceUids are also accepted. Selecting sources or source_uids also enables datasource resolution."
    )]
    pub output_columns: Vec<String>,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "csv", "json", "yaml", "output_format"], help = "Render dashboard summaries as plain text.")]
    pub text: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["text", "csv", "json", "yaml", "output_format"], help = "Render dashboard summaries as a table.")]
    pub table: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["text", "table", "json", "yaml", "output_format"], help = "Render dashboard summaries as CSV.")]
    pub csv: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["text", "table", "csv", "yaml", "output_format"], help = "Render dashboard summaries as JSON.")]
    pub json: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["text", "table", "csv", "json", "output_format"], help = "Render dashboard summaries as YAML.")]
    pub yaml: bool,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["text", "table", "csv", "json", "yaml"],
        help = "Alternative single-flag output selector. Use text, table, csv, json, or yaml."
    )]
    pub output_format: Option<SimpleOutputFormat>,
    #[arg(
        long,
        default_value_t = false,
        help = "Do not print table headers when rendering the default table output."
    )]
    pub no_header: bool,
}
