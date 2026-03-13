use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use crate::common::{resolve_auth_headers, Result};
use crate::http::{JsonHttpClient, JsonHttpClientConfig};

use super::{
    DEFAULT_EXPORT_DIR, DEFAULT_IMPORT_MESSAGE, DEFAULT_PAGE_SIZE, DEFAULT_TIMEOUT, DEFAULT_URL,
};

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
    #[arg(
        long,
        conflicts_with = "all_orgs",
        help = "Export dashboards from one explicit Grafana org ID instead of the current org. Use this when the same credentials can see multiple orgs."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "org_id",
        help = "Enumerate all visible Grafana orgs and export dashboards from each org into per-org subdirectories under the export root."
    )]
    pub all_orgs: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Write dashboard files directly into each export variant directory instead of recreating Grafana folder-based subdirectories on disk."
    )]
    pub flat: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Replace existing local export files in the target directory instead of failing when a file already exists."
    )]
    pub overwrite: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Skip the API-safe raw/ export variant. Use this only when you do not need later API import or diff workflows."
    )]
    pub without_dashboard_raw: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Skip the web-import prompt/ export variant. Use this only when you do not need Grafana UI import with datasource prompts."
    )]
    pub without_dashboard_prompt: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Preview the dashboard files and indexes that would be written without changing disk."
    )]
    pub dry_run: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Show concise per-dashboard export progress in <current>/<total> form while processing files."
    )]
    pub progress: bool,
    #[arg(
        short = 'v',
        long,
        default_value_t = false,
        help = "Show detailed per-item export output, including variants and output paths. Overrides --progress output."
    )]
    pub verbose: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ListArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long, default_value_t = DEFAULT_PAGE_SIZE, help = "Dashboard search page size.")]
    pub page_size: usize,
    #[arg(
        long,
        conflicts_with = "all_orgs",
        help = "List dashboards from this Grafana org ID."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "org_id",
        help = "Enumerate all visible Grafana orgs and aggregate dashboard list output across them."
    )]
    pub all_orgs: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "For table or CSV output, fetch each dashboard payload and include resolved datasource names in the list output. JSON already includes datasource names and UIDs by default. This is slower because it makes extra API calls per dashboard."
    )]
    pub with_sources: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["csv", "json"], help = "Render dashboard summaries as a table.")]
    pub table: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "json"], help = "Render dashboard summaries as CSV.")]
    pub csv: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "csv"], help = "Render dashboard summaries as JSON.")]
    pub json: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Do not print table headers when rendering the default table output."
    )]
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
    #[arg(
        long,
        default_value_t = false,
        help = "Do not print table headers when rendering the default table output."
    )]
    pub no_header: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ImportArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help = "Import dashboards from this directory. Point this to the raw/ export directory explicitly, not the combined export root."
    )]
    pub import_dir: PathBuf,
    #[arg(
        long,
        help = "Force every imported dashboard into one destination Grafana folder UID. This overrides any folder UID carried by the exported dashboard files."
    )]
    pub import_folder_uid: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Use the exported raw folder inventory to create any missing destination folders before import. In dry-run mode, also report folder missing/match/mismatch state first."
    )]
    pub ensure_folders: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Update an existing destination dashboard when the imported dashboard UID already exists. Without this flag, existing UIDs are blocked."
    )]
    pub replace_existing: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Reconcile only dashboards whose UID already exists in Grafana. Missing destination UIDs are skipped instead of created."
    )]
    pub update_existing_only: bool,
    #[arg(long, default_value = DEFAULT_IMPORT_MESSAGE, help = "Version-history message to attach to each imported dashboard revision in Grafana.")]
    pub import_message: String,
    #[arg(
        long,
        default_value_t = false,
        help = "Preview what import would do without changing Grafana. This reports whether each dashboard would create, update, or be skipped/blocked."
    )]
    pub dry_run: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "For --dry-run only, render a compact table instead of per-dashboard log lines. With --ensure-folders, the folder check is also shown in table form."
    )]
    pub table: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "For --dry-run only, render one JSON document with mode, folder checks, dashboard actions, and summary counts."
    )]
    pub json: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "For --dry-run --table only, omit the table header row."
    )]
    pub no_header: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Show concise per-dashboard import progress in <current>/<total> form while processing files. Use this for long-running batch imports."
    )]
    pub progress: bool,
    #[arg(
        short = 'v',
        long,
        default_value_t = false,
        help = "Show detailed per-item import output, including target paths, dry-run actions, and folder status details. Overrides --progress output."
    )]
    pub verbose: bool,
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
    #[arg(
        long,
        help = "Override the destination Grafana folder UID when comparing imported dashboards."
    )]
    pub import_folder_uid: Option<String>,
    #[arg(
        long,
        default_value_t = 3,
        help = "Number of unified diff context lines."
    )]
    pub context_lines: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum InspectExportReportFormat {
    Table,
    Csv,
    Json,
}

#[derive(Debug, Clone, Args)]
pub struct InspectExportArgs {
    #[arg(
        long,
        help = "Analyze dashboards from this raw export directory. Point this to the raw/ export directory explicitly."
    )]
    pub import_dir: PathBuf,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "report",
        conflicts_with = "table",
        help = "Render the export analysis as JSON."
    )]
    pub json: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "report",
        conflicts_with = "json",
        help = "Render the export analysis as a table-oriented summary."
    )]
    pub table: bool,
    #[arg(
        long,
        value_enum,
        num_args = 0..=1,
        default_missing_value = "table",
        conflicts_with_all = ["json", "table"],
        help = "Render a full per-query inspection report. Defaults to table; use --report csv or --report json for machine-readable output."
    )]
    pub report: Option<InspectExportReportFormat>,
    #[arg(
        long,
        value_delimiter = ',',
        help = "For --report table or csv output, limit the query report to the selected columns. Supported values: dashboard_uid, dashboard_title, folder_path, panel_id, panel_title, panel_type, ref_id, datasource, datasource_uid, query_field, metrics, measurements, buckets, query."
    )]
    pub report_columns: Vec<String>,
    #[arg(
        long,
        help = "For --report output, include only rows whose datasource label exactly matches this value."
    )]
    pub report_filter_datasource: Option<String>,
    #[arg(
        long,
        help = "For --report output, include only rows whose panel id exactly matches this value."
    )]
    pub report_filter_panel_id: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Do not print table headers when rendering the table summary or table report."
    )]
    pub no_header: bool,
}

#[derive(Debug, Clone, Args)]
pub struct InspectLiveArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long, default_value_t = DEFAULT_PAGE_SIZE, help = "Dashboard search page size.")]
    pub page_size: usize,
    #[arg(
        long,
        conflicts_with = "all_orgs",
        help = "Inspect dashboards from this Grafana org ID."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "org_id",
        help = "Enumerate all visible Grafana orgs and inspect dashboards across them."
    )]
    pub all_orgs: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "report",
        conflicts_with = "table",
        help = "Render the live inspection analysis as JSON."
    )]
    pub json: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "report",
        conflicts_with = "json",
        help = "Render the live inspection analysis as a table-oriented summary."
    )]
    pub table: bool,
    #[arg(
        long,
        value_enum,
        num_args = 0..=1,
        default_missing_value = "table",
        conflicts_with_all = ["json", "table"],
        help = "Render a full per-query inspection report. Defaults to table; use --report csv or --report json for alternate output."
    )]
    pub report: Option<InspectExportReportFormat>,
    #[arg(
        long,
        value_delimiter = ',',
        help = "For --report table or csv output, limit the query report to the selected columns. Supported values: dashboard_uid, dashboard_title, folder_path, panel_id, panel_title, panel_type, ref_id, datasource, datasource_uid, query_field, metrics, measurements, buckets, query."
    )]
    pub report_columns: Vec<String>,
    #[arg(
        long,
        help = "For --report output, include only rows whose datasource label exactly matches this value."
    )]
    pub report_filter_datasource: Option<String>,
    #[arg(
        long,
        help = "For --report output, include only rows whose panel id exactly matches this value."
    )]
    pub report_filter_panel_id: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Do not print headers when rendering table or csv inspection output."
    )]
    pub no_header: bool,
}

#[derive(Debug, Clone, Subcommand)]
pub enum DashboardCommand {
    #[command(
        name = "list",
        visible_alias = "list-dashboard",
        about = "List dashboard summaries without writing export files."
    )]
    List(ListArgs),
    #[command(name = "list-data-sources", about = "List Grafana data sources.")]
    ListDataSources(ListDataSourcesArgs),
    #[command(
        name = "export",
        visible_alias = "export-dashboard",
        about = "Export dashboards to raw/ and prompt/ JSON files."
    )]
    Export(ExportArgs),
    #[command(
        name = "import",
        visible_alias = "import-dashboard",
        about = "Import dashboard JSON files through the Grafana API."
    )]
    Import(ImportArgs),
    #[command(about = "Compare local raw dashboard files against live Grafana dashboards.")]
    Diff(DiffArgs),
    #[command(
        name = "inspect-export",
        about = "Analyze a raw dashboard export directory and summarize its structure."
    )]
    InspectExport(InspectExportArgs),
    #[command(
        name = "inspect-live",
        about = "Analyze live Grafana dashboards via a temporary raw-export snapshot."
    )]
    InspectLive(InspectLiveArgs),
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
