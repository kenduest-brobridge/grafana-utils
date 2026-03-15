//! Clap schema for dashboard CLI commands.
//! Hosts dashboard command enums/args and parser helpers consumed by the dashboard runtime module.
use clap::{error::ErrorKind, Args, CommandFactory, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use crate::common::{resolve_auth_headers, Result};
use crate::http::{JsonHttpClient, JsonHttpClientConfig};

use super::{
    DEFAULT_EXPORT_DIR, DEFAULT_IMPORT_MESSAGE, DEFAULT_PAGE_SIZE, DEFAULT_TIMEOUT, DEFAULT_URL,
};

const DASHBOARD_LIST_HELP_EXAMPLES: &str =
    "Examples:\n\n  Table output with folder paths:\n    grafana-util dashboard list --url http://localhost:3000 --table --show-folder-path\n\n  JSON output for scripting:\n    grafana-util dashboard list --url http://localhost:3000 --output-format json\n\n  CSV output without a header row:\n    grafana-util dashboard list --url http://localhost:3000 --csv --no-header";
const DASHBOARD_EXPORT_HELP_EXAMPLES: &str =
    "Examples:\n\n  Export dashboards into raw/ and prompt/ variants:\n    grafana-util dashboard export --url http://localhost:3000 --export-dir ./dashboards --overwrite\n\n  Export every visible org into per-org directories:\n    grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --export-dir ./dashboards --overwrite";
const DASHBOARD_IMPORT_HELP_EXAMPLES: &str =
    "Examples:\n\n  Preview import actions as a table:\n    grafana-util dashboard import --url http://localhost:3000 --import-dir ./dashboards/raw --replace-existing --dry-run --output-format table\n\n  Replay routed exports and create missing orgs:\n    grafana-util dashboard import --url http://localhost:3000 --import-dir ./dashboards --use-export-org --create-missing-orgs --replace-existing";
const DASHBOARD_DIFF_HELP_EXAMPLES: &str =
    "Examples:\n\n  Compare local raw exports against live Grafana:\n    grafana-util dashboard diff --url http://localhost:3000 --import-dir ./dashboards/raw\n\n  Compare only one explicit org:\n    grafana-util dashboard diff --url http://localhost:3000 --import-dir ./dashboards --org-id 2";
const DASHBOARD_INSPECT_EXPORT_HELP_EXAMPLES: &str =
    "Examples:\n\n  Render a query inventory table:\n    grafana-util dashboard inspect-export --import-dir ./dashboards/raw --view query --format table\n\n  Render governance JSON:\n    grafana-util dashboard inspect-export --import-dir ./dashboards/raw --view governance --format json";
const DASHBOARD_INSPECT_LIVE_HELP_EXAMPLES: &str =
    "Examples:\n\n  Analyze live dashboards as a summary table:\n    grafana-util dashboard inspect-live --url http://localhost:3000 --view summary --format table\n\n  Render live query inventory as tree-table output:\n    grafana-util dashboard inspect-live --url http://localhost:3000 --view query --format table --layout tree";

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum SimpleOutputFormat {
    Table,
    Csv,
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum DryRunOutputFormat {
    Text,
    Table,
    Json,
}

#[derive(Debug, Clone, Args, Default)]
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
        help = "Grafana Basic auth username. Preferred flag: --basic-user. Falls back to GRAFANA_USERNAME."
    )]
    pub username: Option<String>,
    #[arg(
        long = "basic-password",
        help = "Grafana Basic auth password. Preferred flag: --basic-password. Falls back to GRAFANA_PASSWORD."
    )]
    pub password: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Prompt for the Grafana Basic auth password without echo instead of passing --basic-password on the command line."
    )]
    pub prompt_password: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Prompt for the Grafana API token without echo instead of passing --token on the command line."
    )]
    pub prompt_token: bool,
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
    #[command(flatten, next_help_heading = "Connection And Auth")]
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
    #[command(flatten, next_help_heading = "Connection And Auth")]
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
    #[arg(long, default_value_t = false, conflicts_with_all = ["csv", "json"], help_heading = "Output Options", help = "Render dashboard summaries as a table.")]
    pub table: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "json"], help_heading = "Output Options", help = "Render dashboard summaries as CSV.")]
    pub csv: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "csv"], help_heading = "Output Options", help = "Render dashboard summaries as JSON.")]
    pub json: bool,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["table", "csv", "json"],
        help_heading = "Output Options",
        help = "Alternative single-flag output selector. Use table, csv, or json."
    )]
    pub output_format: Option<SimpleOutputFormat>,
    #[arg(
        long,
        default_value_t = false,
        help_heading = "Output Options",
        help = "Do not print table headers when rendering the default table output."
    )]
    pub no_header: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ListDataSourcesArgs {
    #[command(flatten, next_help_heading = "Connection And Auth")]
    pub common: CommonCliArgs,
    #[arg(long, default_value_t = false, conflicts_with_all = ["csv", "json"], help_heading = "Output Options", help = "Render datasource summaries as a table.")]
    pub table: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "json"], help_heading = "Output Options", help = "Render datasource summaries as CSV.")]
    pub csv: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "csv"], help_heading = "Output Options", help = "Render datasource summaries as JSON.")]
    pub json: bool,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["table", "csv", "json"],
        help_heading = "Output Options",
        help = "Alternative single-flag output selector. Use table, csv, or json."
    )]
    pub output_format: Option<SimpleOutputFormat>,
    #[arg(
        long,
        default_value_t = false,
        help_heading = "Output Options",
        help = "Do not print table headers when rendering the default table output."
    )]
    pub no_header: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ImportArgs {
    #[command(flatten, next_help_heading = "Connection And Auth")]
    pub common: CommonCliArgs,
    #[arg(
        long,
        conflicts_with = "use_export_org",
        help_heading = "Import Input And Org Routing",
        help = "Import dashboards into this Grafana org ID instead of the current org. This switches the whole import run to one explicit destination org and requires Basic auth."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "require_matching_export_org",
        help_heading = "Import Input And Org Routing",
        help = "Import a combined multi-org export root by routing each org-specific raw export back into the matching Grafana org. This requires Basic auth."
    )]
    pub use_export_org: bool,
    #[arg(
        long = "only-org-id",
        requires = "use_export_org",
        conflicts_with = "org_id",
        help_heading = "Import Input And Org Routing",
        help = "With --use-export-org, import only these exported source org IDs. Repeat the flag to select multiple orgs."
    )]
    pub only_org_id: Vec<i64>,
    #[arg(
        long,
        default_value_t = false,
        requires = "use_export_org",
        help_heading = "Import Input And Org Routing",
        help = "With --use-export-org, create a missing destination org when an exported source org ID does not exist in Grafana. The new org is created from the exported org name and then used as the import target."
    )]
    pub create_missing_orgs: bool,
    #[arg(
        long,
        help_heading = "Import Input And Org Routing",
        help = "Import dashboards from this directory. Use the raw/ export directory for single-org import, or the combined export root when --use-export-org is enabled."
    )]
    pub import_dir: PathBuf,
    #[arg(
        long,
        help_heading = "Import Behavior",
        help = "Force every imported dashboard into one destination Grafana folder UID. This overrides any folder UID carried by the exported dashboard files."
    )]
    pub import_folder_uid: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help_heading = "Import Behavior",
        help = "Use the exported raw folder inventory to create any missing destination folders before import. In dry-run mode, also report folder missing/match/mismatch state first."
    )]
    pub ensure_folders: bool,
    #[arg(
        long,
        default_value_t = false,
        help_heading = "Import Behavior",
        help = "Update an existing destination dashboard when the imported dashboard UID already exists. Without this flag, existing UIDs are blocked."
    )]
    pub replace_existing: bool,
    #[arg(
        long,
        default_value_t = false,
        help_heading = "Import Behavior",
        help = "Reconcile only dashboards whose UID already exists in Grafana. Missing destination UIDs are skipped instead of created."
    )]
    pub update_existing_only: bool,
    #[arg(
        long,
        default_value_t = false,
        help_heading = "Import Behavior",
        help = "Only update an existing dashboard when the source raw folder path matches the destination Grafana folder path exactly. Missing dashboards still follow the active create/skip mode."
    )]
    pub require_matching_folder_path: bool,
    #[arg(
        long,
        default_value_t = false,
        help_heading = "Import Safety",
        help = "Fail the import when the raw export orgId metadata does not match the target Grafana org for this run. This is a safety check for accidental cross-org imports."
    )]
    pub require_matching_export_org: bool,
    #[arg(
        long,
        default_value = DEFAULT_IMPORT_MESSAGE,
        help_heading = "Import Behavior",
        help = "Version-history message to attach to each imported dashboard revision in Grafana."
    )]
    pub import_message: String,
    #[arg(
        long,
        default_value_t = false,
        help_heading = "Dry-Run Output",
        help = "Preview what import would do without changing Grafana. This reports whether each dashboard would create, update, or be skipped/blocked."
    )]
    pub dry_run: bool,
    #[arg(
        long,
        default_value_t = false,
        help_heading = "Dry-Run Output",
        help = "For --dry-run only, render a compact table instead of per-dashboard log lines. With --ensure-folders, the folder check is also shown in table form."
    )]
    pub table: bool,
    #[arg(
        long,
        default_value_t = false,
        help_heading = "Dry-Run Output",
        help = "For --dry-run only, render one JSON document with mode, folder checks, dashboard actions, and summary counts."
    )]
    pub json: bool,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["table", "json"],
        help_heading = "Dry-Run Output",
        help = "Alternative single-flag output selector for --dry-run output. Use text, table, or json."
    )]
    pub output_format: Option<DryRunOutputFormat>,
    #[arg(
        long,
        default_value_t = false,
        help_heading = "Dry-Run Output",
        help = "For --dry-run --table only, omit the table header row."
    )]
    pub no_header: bool,
    #[arg(
        long,
        default_value_t = false,
        help_heading = "Import Safety",
        help = "Keep processing remaining dashboards if one item fails; return a non-zero exit status when any item fails."
    )]
    pub continue_on_error: bool,
    #[arg(
        long,
        value_delimiter = ',',
        requires = "dry_run",
        value_parser = parse_dashboard_import_output_column,
        help_heading = "Dry-Run Output",
        help = "For --dry-run --table only, render only these comma-separated columns. Supported values: uid, destination, action, folder_path, source_folder_path, destination_folder_path, reason, file."
    )]
    pub output_columns: Vec<String>,
    #[arg(
        long,
        default_value_t = false,
        help_heading = "Progress And Logging",
        help = "Show concise per-dashboard import progress in <current>/<total> form while processing files. Use this for long-running batch imports."
    )]
    pub progress: bool,
    #[arg(
        short = 'v',
        long,
        default_value_t = false,
        help_heading = "Progress And Logging",
        help = "Show detailed per-item import output, including target paths, dry-run actions, and folder status details. Overrides --progress output."
    )]
    pub verbose: bool,
}

#[derive(Debug, Clone, Args)]
pub struct DiffArgs {
    #[command(flatten, next_help_heading = "Connection And Auth")]
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
    Tree,
    TreeTable,
    DatasourceSummary,
    DatasourceSummaryJson,
    Governance,
    GovernanceJson,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum InspectOutputFormat {
    Text,
    Table,
    Json,
    ReportTable,
    ReportCsv,
    ReportJson,
    ReportTree,
    ReportTreeTable,
    DatasourceSummary,
    DatasourceSummaryJson,
    Governance,
    GovernanceJson,
}
/// Preferred selector for which inspection view to render.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum InspectView {
    Summary,
    Query,
    Datasource,
    Governance,
}

/// Preferred selector for output encoding within inspect views.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum InspectRenderFormat {
    Text,
    Table,
    Csv,
    Json,
}

/// Preferred selector for query-oriented inspect layouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum InspectLayout {
    Flat,
    Tree,
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
        help_heading = "Output Options",
        help = "Render the export analysis as JSON."
    )]
    pub json: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "report",
        conflicts_with = "json",
        help_heading = "Output Options",
        help = "Render the export analysis as a table-oriented summary."
    )]
    pub table: bool,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["json", "table", "report", "output_format"],
        help_heading = "Output Options",
        help = "Preferred inspect selector for what to render. Use summary, query, datasource, or governance. Combine with --format and optional --layout instead of legacy --output-format."
    )]
    pub view: Option<InspectView>,
    #[arg(
        long,
        value_enum,
        requires = "view",
        conflicts_with_all = ["json", "table", "report", "output_format"],
        help_heading = "Output Options",
        help = "Preferred inspect selector for output encoding. Use text, table, csv, or json with --view."
    )]
    pub format: Option<InspectRenderFormat>,
    #[arg(
        long,
        value_enum,
        requires = "view",
        conflicts_with_all = ["json", "table", "report", "output_format"],
        help_heading = "Output Options",
        help = "Preferred inspect selector for query layout. Use flat or tree with --view query."
    )]
    pub layout: Option<InspectLayout>,
    #[arg(
        long,
        value_enum,
        num_args = 0..=1,
        default_missing_value = "table",
        conflicts_with_all = ["json", "table"],
        help_heading = "Output Options",
        help = "Render a full inspection report. Defaults to flat per-query table output; use --report csv or --report json for machine-readable output, --report tree for dashboard-first grouped text, --report tree-table for dashboard-first grouped tables, --report datasource-summary or --report datasource-summary-json for datasource dependency aggregates, --report governance for datasource governance tables, or --report governance-json for governance JSON."
    )]
    pub report: Option<InspectExportReportFormat>,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["view", "format", "layout"],
        help_heading = "Output Options",
        help = "Legacy single-flag output selector for inspect output. Prefer --view plus --format (and --layout for query views)."
    )]
    pub output_format: Option<InspectOutputFormat>,
    #[arg(
        long,
        value_delimiter = ',',
        help_heading = "Output Options",
        help = "For query-table output, limit the query report to the selected columns. Supported values: dashboard_uid, dashboard_title, folder_path, panel_id, panel_title, panel_type, ref_id, datasource, datasource_uid, query_field, metrics, measurements, buckets, query."
    )]
    pub report_columns: Vec<String>,
    #[arg(
        long,
        help_heading = "Output Options",
        help = "For --report output or report-like --output-format values, include only rows whose datasource label exactly matches this value."
    )]
    pub report_filter_datasource: Option<String>,
    #[arg(
        long,
        help_heading = "Output Options",
        help = "For --report output or report-like --output-format values, include only rows whose panel id exactly matches this value."
    )]
    pub report_filter_panel_id: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Show extended help with report examples for inspect-export."
    )]
    pub help_full: bool,
    #[arg(
        long,
        default_value_t = false,
        help_heading = "Output Options",
        help = "Do not print table headers when rendering the table summary, table-like --report output, or compatible --output-format values."
    )]
    pub no_header: bool,
}

#[derive(Debug, Clone, Args)]
pub struct InspectLiveArgs {
    #[command(flatten, next_help_heading = "Connection And Auth")]
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
        help_heading = "Output Options",
        help = "Render the live inspection analysis as JSON."
    )]
    pub json: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "report",
        conflicts_with = "json",
        help_heading = "Output Options",
        help = "Render the live inspection analysis as a table-oriented summary."
    )]
    pub table: bool,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["json", "table", "report", "output_format"],
        help_heading = "Output Options",
        help = "Preferred inspect selector for what to render. Use summary, query, datasource, or governance. Combine with --format and optional --layout instead of legacy --output-format."
    )]
    pub view: Option<InspectView>,
    #[arg(
        long,
        value_enum,
        requires = "view",
        conflicts_with_all = ["json", "table", "report", "output_format"],
        help_heading = "Output Options",
        help = "Preferred inspect selector for output encoding. Use text, table, csv, or json with --view."
    )]
    pub format: Option<InspectRenderFormat>,
    #[arg(
        long,
        value_enum,
        requires = "view",
        conflicts_with_all = ["json", "table", "report", "output_format"],
        help_heading = "Output Options",
        help = "Preferred inspect selector for query layout. Use flat or tree with --view query."
    )]
    pub layout: Option<InspectLayout>,
    #[arg(
        long,
        value_enum,
        num_args = 0..=1,
        default_missing_value = "table",
        conflicts_with_all = ["json", "table"],
        help_heading = "Output Options",
        help = "Render a full inspection report. Defaults to flat per-query table output; use --report csv or --report json for alternate output, --report tree for dashboard-first grouped text, --report tree-table for dashboard-first grouped tables, --report datasource-summary or --report datasource-summary-json for datasource dependency aggregates, --report governance for datasource governance tables, or --report governance-json for governance JSON."
    )]
    pub report: Option<InspectExportReportFormat>,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["view", "format", "layout"],
        help_heading = "Output Options",
        help = "Legacy single-flag output selector for inspect output. Prefer --view plus --format (and --layout for query views)."
    )]
    pub output_format: Option<InspectOutputFormat>,
    #[arg(
        long,
        value_delimiter = ',',
        help_heading = "Output Options",
        help = "For query-table output, limit the query report to the selected columns. Supported values: dashboard_uid, dashboard_title, folder_path, panel_id, panel_title, panel_type, ref_id, datasource, datasource_uid, query_field, metrics, measurements, buckets, query."
    )]
    pub report_columns: Vec<String>,
    #[arg(
        long,
        help_heading = "Output Options",
        help = "For --report output or report-like --output-format values, include only rows whose datasource label exactly matches this value."
    )]
    pub report_filter_datasource: Option<String>,
    #[arg(
        long,
        help_heading = "Output Options",
        help = "For --report output or report-like --output-format values, include only rows whose panel id exactly matches this value."
    )]
    pub report_filter_panel_id: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Show extended help with report examples for inspect-live."
    )]
    pub help_full: bool,
    #[arg(
        long,
        default_value_t = false,
        help_heading = "Output Options",
        help = "Do not print headers when rendering table, csv, or tree-table inspection output, including compatible --output-format values."
    )]
    pub no_header: bool,
}

#[derive(Debug, Clone, Subcommand)]
pub enum DashboardCommand {
    #[command(
        name = "list",
        about = "List dashboard summaries without writing export files.",
        after_help = DASHBOARD_LIST_HELP_EXAMPLES
    )]
    List(ListArgs),
    #[command(
        name = "export",
        about = "Export dashboards to raw/ and prompt/ JSON files.",
        after_help = DASHBOARD_EXPORT_HELP_EXAMPLES
    )]
    Export(ExportArgs),
    #[command(
        name = "import",
        about = "Import dashboard JSON files through the Grafana API.",
        after_help = DASHBOARD_IMPORT_HELP_EXAMPLES
    )]
    Import(ImportArgs),
    #[command(
        about = "Compare local raw dashboard files against live Grafana dashboards.",
        after_help = DASHBOARD_DIFF_HELP_EXAMPLES
    )]
    Diff(DiffArgs),
    #[command(
        name = "inspect-export",
        about = "Analyze a raw dashboard export directory and summarize its structure.",
        after_help = DASHBOARD_INSPECT_EXPORT_HELP_EXAMPLES
    )]
    InspectExport(InspectExportArgs),
    #[command(
        name = "inspect-live",
        about = "Analyze live Grafana dashboards via a temporary raw-export snapshot.",
        after_help = DASHBOARD_INSPECT_LIVE_HELP_EXAMPLES
    )]
    InspectLive(InspectLiveArgs),
}

#[derive(Debug, Clone, Parser)]
#[command(
    about = "Export or import Grafana dashboards.",
    after_help = "Examples:\n\n  Export dashboards from local Grafana with Basic auth:\n    grafana-util export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./dashboards --overwrite\n\n  Export dashboards with an API token:\n    export GRAFANA_API_TOKEN='your-token'\n    grafana-util export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --export-dir ./dashboards --overwrite\n\n  Export into a flat directory layout instead of per-folder subdirectories:\n    grafana-util export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./dashboards --flat\n\n  Compare raw dashboard exports against local Grafana:\n    grafana-util diff --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards/raw"
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
    pub auth_mode: String,
    pub headers: Vec<(String, String)>,
}

// Parse dashboard CLI argv and normalize output-format aliases to keep
// downstream handlers deterministic.
pub fn parse_cli_from<I, T>(iter: I) -> DashboardCliArgs
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    normalize_dashboard_cli_args(DashboardCliArgs::parse_from(iter))
}

// Accept both user-facing legacy aliases and canonical snake_case column names for
// import dry-run table formatting.
fn parse_dashboard_import_output_column(value: &str) -> std::result::Result<String, String> {
    match value {
        "uid" => Ok("uid".to_string()),
        "destination" => Ok("destination".to_string()),
        "action" => Ok("action".to_string()),
        "folder_path" | "folderPath" => Ok("folder_path".to_string()),
        "source_folder_path" | "sourceFolderPath" => Ok("source_folder_path".to_string()),
        "destination_folder_path" | "destinationFolderPath" => {
            Ok("destination_folder_path".to_string())
        }
        "reason" => Ok("reason".to_string()),
        "file" => Ok("file".to_string()),
        _ => Err(format!(
            "Unsupported --output-columns value '{value}'. Supported values: uid, destination, action, folder_path, source_folder_path, destination_folder_path, reason, file."
        )),
    }
}

// Map legacy output_format enum selections into boolean render flags for list
// commands.
fn normalize_simple_output_format(
    table: &mut bool,
    csv: &mut bool,
    json: &mut bool,
    output_format: Option<SimpleOutputFormat>,
) {
    match output_format {
        Some(SimpleOutputFormat::Table) => *table = true,
        Some(SimpleOutputFormat::Csv) => *csv = true,
        Some(SimpleOutputFormat::Json) => *json = true,
        None => {}
    }
}

// Map dry-run output_format enum selections into render flags, treating text mode
// as implicit default.
fn normalize_dry_run_output_format(
    table: &mut bool,
    json: &mut bool,
    output_format: Option<DryRunOutputFormat>,
) {
    match output_format {
        Some(DryRunOutputFormat::Table) => *table = true,
        Some(DryRunOutputFormat::Json) => *json = true,
        Some(DryRunOutputFormat::Text) | None => {}
    }
}

fn inspect_report_from_view_format_layout(
    view: InspectView,
    format: Option<InspectRenderFormat>,
    layout: Option<InspectLayout>,
) -> std::result::Result<(bool, bool, Option<InspectExportReportFormat>), String> {
    let chosen_format = format.unwrap_or(match view {
        InspectView::Summary => InspectRenderFormat::Text,
        InspectView::Query => InspectRenderFormat::Table,
        InspectView::Datasource | InspectView::Governance => InspectRenderFormat::Table,
    });
    let chosen_layout = layout.unwrap_or(match view {
        InspectView::Query => InspectLayout::Flat,
        _ => InspectLayout::Flat,
    });
    match view {
        InspectView::Summary => {
            if layout.is_some() {
                return Err("--layout is only supported with --view query.".to_string());
            }
            match chosen_format {
                InspectRenderFormat::Text => Ok((false, false, None)),
                InspectRenderFormat::Table => Ok((false, true, None)),
                InspectRenderFormat::Json => Ok((true, false, None)),
                InspectRenderFormat::Csv => {
                    Err("--view summary supports only --format text, table, or json.".to_string())
                }
            }
        }
        InspectView::Query => match (chosen_format, chosen_layout) {
            (InspectRenderFormat::Table, InspectLayout::Flat) => {
                Ok((false, false, Some(InspectExportReportFormat::Table)))
            }
            (InspectRenderFormat::Table, InspectLayout::Tree) => {
                Ok((false, false, Some(InspectExportReportFormat::TreeTable)))
            }
            (InspectRenderFormat::Csv, InspectLayout::Flat) => {
                Ok((false, false, Some(InspectExportReportFormat::Csv)))
            }
            (InspectRenderFormat::Json, InspectLayout::Flat) => {
                Ok((false, false, Some(InspectExportReportFormat::Json)))
            }
            (InspectRenderFormat::Text, InspectLayout::Tree) => {
                Ok((false, false, Some(InspectExportReportFormat::Tree)))
            }
            (InspectRenderFormat::Text, InspectLayout::Flat) => {
                Err("--view query with --format text requires --layout tree.".to_string())
            }
            (InspectRenderFormat::Csv | InspectRenderFormat::Json, InspectLayout::Tree) => Err(
                "--layout tree is only supported with --format table or text for --view query."
                    .to_string(),
            ),
        },
        InspectView::Datasource => {
            if layout.is_some() {
                return Err("--layout is only supported with --view query.".to_string());
            }
            match chosen_format {
                InspectRenderFormat::Table => Ok((
                    false,
                    false,
                    Some(InspectExportReportFormat::DatasourceSummary),
                )),
                InspectRenderFormat::Json => Ok((
                    false,
                    false,
                    Some(InspectExportReportFormat::DatasourceSummaryJson),
                )),
                InspectRenderFormat::Text | InspectRenderFormat::Csv => {
                    Err("--view datasource supports only --format table or json.".to_string())
                }
            }
        }
        InspectView::Governance => {
            if layout.is_some() {
                return Err("--layout is only supported with --view query.".to_string());
            }
            match chosen_format {
                InspectRenderFormat::Table => {
                    Ok((false, false, Some(InspectExportReportFormat::Governance)))
                }
                InspectRenderFormat::Json => Ok((
                    false,
                    false,
                    Some(InspectExportReportFormat::GovernanceJson),
                )),
                InspectRenderFormat::Text | InspectRenderFormat::Csv => {
                    Err("--view governance supports only --format table or json.".to_string())
                }
            }
        }
    }
}

fn normalize_inspect_args(
    json: &mut bool,
    table: &mut bool,
    report: &mut Option<InspectExportReportFormat>,
    output_format: Option<InspectOutputFormat>,
    view: Option<InspectView>,
    format: Option<InspectRenderFormat>,
    layout: Option<InspectLayout>,
) -> std::result::Result<(), String> {
    if view.is_none() && format.is_none() && layout.is_none() {
        return Ok(());
    }
    if view.is_none() {
        return Err("--format and --layout require --view.".to_string());
    }
    let (normalized_json, normalized_table, normalized_report) =
        inspect_report_from_view_format_layout(view.expect("validated"), format, layout)?;
    *json = normalized_json;
    *table = normalized_table;
    *report = normalized_report;
    let _ = output_format;
    Ok(())
}

/// Normalize dashboard CLI variants into a common output-mode flag contract.
///
/// Legacy boolean output switches and enum-style aliases are collapsed into the
/// shared handler shape before dispatch.
pub(crate) fn try_normalize_dashboard_cli_args(
    mut args: DashboardCliArgs,
) -> std::result::Result<DashboardCliArgs, String> {
    match &mut args.command {
        DashboardCommand::List(list_args) => normalize_simple_output_format(
            &mut list_args.table,
            &mut list_args.csv,
            &mut list_args.json,
            list_args.output_format,
        ),
        DashboardCommand::Import(import_args) => normalize_dry_run_output_format(
            &mut import_args.table,
            &mut import_args.json,
            import_args.output_format,
        ),
        DashboardCommand::InspectExport(inspect_args) => normalize_inspect_args(
            &mut inspect_args.json,
            &mut inspect_args.table,
            &mut inspect_args.report,
            inspect_args.output_format,
            inspect_args.view,
            inspect_args.format,
            inspect_args.layout,
        )?,
        DashboardCommand::InspectLive(inspect_args) => normalize_inspect_args(
            &mut inspect_args.json,
            &mut inspect_args.table,
            &mut inspect_args.report,
            inspect_args.output_format,
            inspect_args.view,
            inspect_args.format,
            inspect_args.layout,
        )?,
        _ => {}
    }
    Ok(args)
}

pub fn normalize_dashboard_cli_args(args: DashboardCliArgs) -> DashboardCliArgs {
    try_normalize_dashboard_cli_args(args).unwrap_or_else(|message| {
        DashboardCliArgs::command()
            .error(ErrorKind::ArgumentConflict, message)
            .exit()
    })
}

pub fn build_auth_context(common: &CommonCliArgs) -> Result<DashboardAuthContext> {
    let headers = resolve_auth_headers(
        common.api_token.as_deref(),
        common.username.as_deref(),
        common.password.as_deref(),
        common.prompt_password,
        common.prompt_token,
    )?;
    let auth_mode = headers
        .iter()
        .find(|(name, _)| name == "Authorization")
        .map(|(_, value)| {
            if value.starts_with("Basic ") {
                "basic".to_string()
            } else {
                "token".to_string()
            }
        })
        .unwrap_or_else(|| "unknown".to_string());
    Ok(DashboardAuthContext {
        url: common.url.clone(),
        timeout: common.timeout,
        verify_ssl: common.verify_ssl,
        auth_mode,
        headers,
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
