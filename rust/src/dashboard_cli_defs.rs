use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

use crate::common::{resolve_auth_headers, Result};
use crate::http::{JsonHttpClient, JsonHttpClientConfig};

use super::{DEFAULT_EXPORT_DIR, DEFAULT_IMPORT_MESSAGE, DEFAULT_PAGE_SIZE, DEFAULT_TIMEOUT, DEFAULT_URL};

#[derive(Debug, Clone, Args)]
pub struct CommonCliArgs {
    #[arg(long, default_value = DEFAULT_URL, help = "Grafana base URL.")]
    pub url: String,
    #[arg(
        long = "token",
        visible_alias = "api-token",
        help = "Grafana API token. Preferred flag: --token. Falls back to GRAFANA_API_TOKEN."
    )]
    pub api_token: Option<String>,
    #[arg(
        long = "basic-user",
        visible_alias = "username",
        help = "Grafana Basic auth username. Preferred flag: --basic-user. Falls back to GRAFANA_USERNAME."
    )]
    pub username: Option<String>,
    #[arg(
        long = "basic-password",
        visible_alias = "password",
        help = "Grafana Basic auth password. Preferred flag: --basic-password. Falls back to GRAFANA_PASSWORD."
    )]
    pub password: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Prompt for the Grafana Basic auth password without echo instead of passing --basic-password on the command line."
    )]
    pub prompt_password: bool,
    #[arg(long, default_value_t = DEFAULT_TIMEOUT, help = "HTTP timeout in seconds.")]
    pub timeout: u64,
    #[arg(
        long,
        default_value_t = false,
        help = "Enable TLS certificate verification. Verification is disabled by default."
    )]
    pub verify_ssl: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ExportArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        default_value = DEFAULT_EXPORT_DIR,
        help = "Directory to write exported dashboards into. Export writes raw/ and prompt/ subdirectories by default."
    )]
    pub export_dir: PathBuf,
    #[arg(long, default_value_t = DEFAULT_PAGE_SIZE, help = "Dashboard search page size.")]
    pub page_size: usize,
    #[arg(long, conflicts_with = "all_orgs", help = "Export dashboards from this Grafana org ID.")]
    pub org_id: Option<i64>,
    #[arg(long, default_value_t = false, conflicts_with = "org_id", help = "Enumerate all visible Grafana orgs and export dashboards from each org.")]
    pub all_orgs: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Write dashboard files directly into the export variant directory instead of per-folder subdirectories."
    )]
    pub flat: bool,
    #[arg(long, default_value_t = false, help = "Overwrite existing dashboard files.")]
    pub overwrite: bool,
    #[arg(long, default_value_t = false, help = "Skip exporting the raw/ variant.")]
    pub without_dashboard_raw: bool,
    #[arg(long, default_value_t = false, help = "Skip exporting the prompt/ variant.")]
    pub without_dashboard_prompt: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Preview the dashboard files and indexes that would be written without changing disk."
    )]
    pub dry_run: bool,
    #[arg(long, default_value_t = false, help = "Show per-dashboard export progress while processing files.")]
    pub progress: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ListArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long, default_value_t = DEFAULT_PAGE_SIZE, help = "Dashboard search page size.")]
    pub page_size: usize,
    #[arg(long, conflicts_with = "all_orgs", help = "List dashboards from this Grafana org ID.")]
    pub org_id: Option<i64>,
    #[arg(long, default_value_t = false, conflicts_with = "org_id", help = "Enumerate all visible Grafana orgs and aggregate dashboard list output across them.")]
    pub all_orgs: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Fetch each dashboard payload and include resolved datasource names in the list output."
    )]
    pub with_sources: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["csv", "json"], help = "Render dashboard summaries as a table.")]
    pub table: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "json"], help = "Render dashboard summaries as CSV.")]
    pub csv: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "csv"], help = "Render dashboard summaries as JSON.")]
    pub json: bool,
    #[arg(long, default_value_t = false, help = "Do not print table headers when rendering the default table output.")]
    pub no_header: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ListDataSourcesArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long, default_value_t = false, conflicts_with_all = ["csv", "json"], help = "Render datasource summaries as a table.")]
    pub table: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "json"], help = "Render datasource summaries as CSV.")]
    pub csv: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "csv"], help = "Render datasource summaries as JSON.")]
    pub json: bool,
    #[arg(long, default_value_t = false, help = "Do not print table headers when rendering the default table output.")]
    pub no_header: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ImportArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help = "Import dashboards from this directory. Point this to the raw/ export directory explicitly."
    )]
    pub import_dir: PathBuf,
    #[arg(long, help = "Override the destination Grafana folder UID for all imported dashboards.")]
    pub import_folder_uid: Option<String>,
    #[arg(long, default_value_t = false, help = "Allow imports to replace existing dashboards with the same UID.")]
    pub replace_existing: bool,
    #[arg(long, default_value = DEFAULT_IMPORT_MESSAGE, help = "Version history message to attach to imported dashboards.")]
    pub import_message: String,
    #[arg(long, default_value_t = false, help = "Show whether each dashboard would be created or updated without importing it.")]
    pub dry_run: bool,
    #[arg(long, default_value_t = false, help = "Show per-dashboard import progress while processing files.")]
    pub progress: bool,
}

#[derive(Debug, Clone, Args)]
pub struct DiffArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help = "Compare dashboards from this directory against Grafana. Point this to the raw/ export directory explicitly."
    )]
    pub import_dir: PathBuf,
    #[arg(long, help = "Override the destination Grafana folder UID when comparing imported dashboards.")]
    pub import_folder_uid: Option<String>,
    #[arg(long, default_value_t = 3, help = "Number of unified diff context lines.")]
    pub context_lines: usize,
}

#[derive(Debug, Clone, Subcommand)]
pub enum DashboardCommand {
    #[command(name = "list", visible_alias = "list-dashboard", about = "List dashboard summaries without writing export files.")]
    List(ListArgs),
    #[command(name = "list-data-sources", about = "List Grafana data sources.")]
    ListDataSources(ListDataSourcesArgs),
    #[command(name = "export", visible_alias = "export-dashboard", about = "Export dashboards to raw/ and prompt/ JSON files.")]
    Export(ExportArgs),
    #[command(name = "import", visible_alias = "import-dashboard", about = "Import dashboard JSON files through the Grafana API.")]
    Import(ImportArgs),
    #[command(about = "Compare local raw dashboard files against live Grafana dashboards.")]
    Diff(DiffArgs),
}

#[derive(Debug, Clone, Parser)]
#[command(
    about = "Export or import Grafana dashboards.",
    after_help = "Examples:\n\n  Export dashboards from local Grafana with Basic auth:\n    grafana-utils export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./dashboards --overwrite\n\n  Export dashboards with an API token:\n    export GRAFANA_API_TOKEN='your-token'\n    grafana-utils export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --export-dir ./dashboards --overwrite\n\n  Export into a flat directory layout instead of per-folder subdirectories:\n    grafana-utils export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./dashboards --flat\n\n  Compare raw dashboard exports against local Grafana:\n    grafana-utils diff --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards/raw"
)]
pub struct DashboardCliArgs {
    #[command(subcommand)]
    pub command: DashboardCommand,
}

#[derive(Debug, Clone)]
pub struct DashboardAuthContext {
    pub url: String,
    pub timeout: u64,
    pub verify_ssl: bool,
    pub headers: Vec<(String, String)>,
}

pub fn parse_cli_from<I, T>(iter: I) -> DashboardCliArgs
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    DashboardCliArgs::parse_from(iter)
}

pub fn build_auth_context(common: &CommonCliArgs) -> Result<DashboardAuthContext> {
    Ok(DashboardAuthContext {
        url: common.url.clone(),
        timeout: common.timeout,
        verify_ssl: common.verify_ssl,
        headers: resolve_auth_headers(
            common.api_token.as_deref(),
            common.username.as_deref(),
            common.password.as_deref(),
            common.prompt_password,
        )?,
    })
}

pub fn build_http_client(common: &CommonCliArgs) -> Result<JsonHttpClient> {
    let context = build_auth_context(common)?;
    JsonHttpClient::new(JsonHttpClientConfig {
        base_url: context.url,
        headers: context.headers,
        timeout_secs: context.timeout,
        verify_ssl: context.verify_ssl,
    })
}

pub fn build_http_client_for_org(common: &CommonCliArgs, org_id: i64) -> Result<JsonHttpClient> {
    let mut context = build_auth_context(common)?;
    context
        .headers
        .push(("X-Grafana-Org-Id".to_string(), org_id.to_string()));
    JsonHttpClient::new(JsonHttpClientConfig {
        base_url: context.url,
        headers: context.headers,
        timeout_secs: context.timeout,
        verify_ssl: context.verify_ssl,
    })
}
