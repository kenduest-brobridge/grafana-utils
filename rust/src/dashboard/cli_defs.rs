//! Clap schema for dashboard CLI commands.
//! Hosts dashboard command enums/args and parser helpers consumed by the dashboard runtime module.
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use crate::common::{resolve_auth_headers, Result};
use crate::http::{JsonHttpClient, JsonHttpClientConfig};

use super::{
    DEFAULT_EXPORT_DIR, DEFAULT_IMPORT_MESSAGE, DEFAULT_PAGE_SIZE, DEFAULT_TIMEOUT, DEFAULT_URL,
};

/// Enum definition for SimpleOutputFormat.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum SimpleOutputFormat {
    Table,
    Csv,
    Json,
}

/// Enum definition for DryRunOutputFormat.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum DryRunOutputFormat {
    Text,
    Table,
    Json,
}

/// Enum definition for GovernanceGateOutputFormat.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum GovernanceGateOutputFormat {
    Text,
    Json,
}

/// Enum definition for TopologyOutputFormat.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum TopologyOutputFormat {
    Text,
    Json,
    Mermaid,
    Dot,
}

/// Enum definition for ImpactOutputFormat.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum ImpactOutputFormat {
    Text,
    Json,
}

/// Enum definition for ValidationOutputFormat.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum ValidationOutputFormat {
    Text,
    Json,
}

/// Struct definition for CommonCliArgs.
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

/// Struct definition for ExportArgs.
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
        help = "Enumerate all visible Grafana orgs and export dashboards from each org into per-org subdirectories under the export root. Prefer Basic auth when you need cross-org export because API tokens are often scoped to one org."
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

/// Struct definition for ListArgs.
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
        long,
        default_value_t = false,
        help = "For table or CSV output, fetch each dashboard payload and include resolved datasource names in the list output. JSON already includes datasource names and UIDs by default. This is slower because it makes extra API calls per dashboard."
    )]
    pub with_sources: bool,
    #[arg(
        long,
        value_delimiter = ',',
        value_parser = parse_dashboard_list_output_column,
        help = "Render only these comma-separated list columns. Supported values: uid, name, folder, folder_uid, path, org, org_id, sources, source_uids. JSON-style aliases like folderUid, orgId, and sourceUids are also accepted. Selecting sources or source_uids also enables datasource resolution."
    )]
    pub output_columns: Vec<String>,
    #[arg(long, default_value_t = false, conflicts_with_all = ["csv", "json"], help = "Render dashboard summaries as a table.")]
    pub table: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "json"], help = "Render dashboard summaries as CSV.")]
    pub csv: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "csv"], help = "Render dashboard summaries as JSON.")]
    pub json: bool,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["table", "csv", "json"],
        help = "Alternative single-flag output selector. Use table, csv, or json."
    )]
    pub output_format: Option<SimpleOutputFormat>,
    #[arg(
        long,
        default_value_t = false,
        help = "Do not print table headers when rendering the default table output."
    )]
    pub no_header: bool,
}

/// Struct definition for ImportArgs.
#[derive(Debug, Clone, Args)]
pub struct ImportArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        conflicts_with = "use_export_org",
        help = "Import dashboards into this Grafana org ID instead of the current org. This switches the whole import run to one explicit destination org and requires Basic auth."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "require_matching_export_org",
        help = "Import a combined multi-org export root by routing each org-specific raw export back into the matching Grafana org. This requires Basic auth."
    )]
    pub use_export_org: bool,
    #[arg(
        long = "only-org-id",
        requires = "use_export_org",
        conflicts_with = "org_id",
        help = "With --use-export-org, import only these exported source org IDs. Repeat the flag to select multiple orgs."
    )]
    pub only_org_id: Vec<i64>,
    #[arg(
        long,
        default_value_t = false,
        requires = "use_export_org",
        help = "With --use-export-org, create a missing destination org when an exported source org ID does not exist in Grafana. The new org is created from the exported org name and then used as the import target."
    )]
    pub create_missing_orgs: bool,
    #[arg(
        long,
        help = "Import dashboards from this directory. Use the raw/ export directory for single-org import, or the combined export root when --use-export-org is enabled."
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
    #[arg(
        long,
        default_value_t = false,
        help = "Only update an existing dashboard when the source raw folder path matches the destination Grafana folder path exactly. Missing dashboards still follow the active create/skip mode."
    )]
    pub require_matching_folder_path: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Fail the import when the raw export orgId metadata does not match the target Grafana org for this run. This is a safety check for accidental cross-org imports."
    )]
    pub require_matching_export_org: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Enable strict dashboard schema validation before import. This rejects unsupported custom plugins, legacy layout shapes, and other preflight issues before any live write."
    )]
    pub strict_schema: bool,
    #[arg(
        long,
        requires = "strict_schema",
        help = "Optional target dashboard schemaVersion required by strict validation. Dashboards below this version are blocked as migration-required."
    )]
    pub target_schema_version: Option<i64>,
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
        value_enum,
        conflicts_with_all = ["table", "json"],
        help = "Alternative single-flag output selector for --dry-run output. Use text, table, or json."
    )]
    pub output_format: Option<DryRunOutputFormat>,
    #[arg(
        long,
        default_value_t = false,
        help = "For --dry-run --table only, omit the table header row."
    )]
    pub no_header: bool,
    #[arg(
        long,
        value_delimiter = ',',
        requires = "dry_run",
        value_parser = parse_dashboard_import_output_column,
        help = "For --dry-run --table only, render only these comma-separated columns. Supported values: uid, destination, action, folder_path, source_folder_path, destination_folder_path, reason, file."
    )]
    pub output_columns: Vec<String>,
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

/// Struct definition for DiffArgs.
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

/// Enum definition for InspectExportReportFormat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum InspectExportReportFormat {
    Table,
    Csv,
    Json,
    Tree,
    TreeTable,
    Dependency,
    DependencyJson,
    Governance,
    GovernanceJson,
}

/// Enum definition for InspectOutputFormat.
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
    ReportDependency,
    ReportDependencyJson,
    Governance,
    GovernanceJson,
}

/// Enum definition for ScreenshotOutputFormat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ScreenshotOutputFormat {
    Png,
    Jpeg,
    Pdf,
}

/// Enum definition for ScreenshotTheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ScreenshotTheme {
    Light,
    Dark,
}

/// Enum definition for ScreenshotFullPageOutput.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ScreenshotFullPageOutput {
    Single,
    Tiles,
    Manifest,
}

/// Struct definition for ScreenshotArgs.
#[derive(Debug, Clone, Args)]
pub struct ScreenshotArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        required_unless_present = "dashboard_url",
        help_heading = "Target Options",
        help = "Grafana dashboard UID to capture from the browser-rendered UI. Required unless --dashboard-url is provided."
    )]
    pub dashboard_uid: Option<String>,
    #[arg(
        long,
        required_unless_present = "dashboard_uid",
        help_heading = "Target Options",
        help = "Full Grafana dashboard URL. When provided, the runtime can reuse URL state such as var-*, from, to, orgId, and panelId."
    )]
    pub dashboard_url: Option<String>,
    #[arg(
        long,
        help_heading = "Target Options",
        help = "Optional dashboard slug. When omitted, the runtime can reuse the UID as a fallback route segment."
    )]
    pub slug: Option<String>,
    #[arg(
        long,
        help_heading = "Output Options",
        help = "Write the captured browser output to this file path."
    )]
    pub output: PathBuf,
    #[arg(
        long,
        help_heading = "Target Options",
        help = "Capture only this Grafana panel ID through the solo dashboard route."
    )]
    pub panel_id: Option<i64>,
    #[arg(
        long,
        help_heading = "State Options",
        help = "Scope the browser session to this Grafana org ID by sending X-Grafana-Org-Id."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        help_heading = "State Options",
        help = "Grafana time range start, for example now-6h or 2026-03-16T00:00:00Z."
    )]
    pub from: Option<String>,
    #[arg(
        long,
        help_heading = "State Options",
        help = "Grafana time range end, for example now or 2026-03-16T12:00:00Z."
    )]
    pub to: Option<String>,
    #[arg(
        long,
        help_heading = "State Options",
        value_name = "QUERY",
        help = "Grafana variable query-string fragment, for example 'var-env=prod&var-host=web01'. Useful for pasting ${__all_variables} expansion output."
    )]
    pub vars_query: Option<String>,
    #[arg(
        long,
        help_heading = "Output Options",
        default_value_t = false,
        help = "Print the final resolved Grafana capture URL before launching Chromium."
    )]
    pub print_capture_url: bool,
    #[arg(
        long,
        help_heading = "Header Options",
        value_name = "TITLE",
        num_args = 0..=1,
        default_missing_value = "__auto__",
        help = "Add a header title block above PNG/JPEG output. Pass no value to auto-detect the dashboard or panel title."
    )]
    pub header_title: Option<String>,
    #[arg(
        long,
        help_heading = "Header Options",
        value_name = "URL",
        num_args = 0..=1,
        default_missing_value = "__auto__",
        help = "Add a header URL line above PNG/JPEG output. Pass no value to reuse the resolved capture URL."
    )]
    pub header_url: Option<String>,
    #[arg(
        long,
        help_heading = "Header Options",
        default_value_t = false,
        help = "Add a header capture timestamp above PNG/JPEG output using local time formatted as YYYY-MM-DD HH:MM:SS."
    )]
    pub header_captured_at: bool,
    #[arg(
        long,
        help_heading = "Header Options",
        help = "Add a free-form header text line above PNG/JPEG output."
    )]
    pub header_text: Option<String>,
    #[arg(
        long = "var",
        help_heading = "State Options",
        value_name = "NAME=VALUE",
        help = "Repeatable Grafana template variable assignment. Example: --var env=prod --var region=us-east-1."
    )]
    pub vars: Vec<String>,
    #[arg(
        long,
        help_heading = "Rendering Options",
        value_enum,
        default_value_t = ScreenshotTheme::Dark,
        help = "Override the Grafana UI theme used for the browser capture."
    )]
    pub theme: ScreenshotTheme,
    #[arg(
        long,
        help_heading = "Output Options",
        value_enum,
        help = "Force the output format instead of inferring it from the output filename."
    )]
    pub output_format: Option<ScreenshotOutputFormat>,
    #[arg(
        long,
        help_heading = "Rendering Options",
        default_value_t = 1440,
        help = "Browser viewport width in pixels."
    )]
    pub width: u32,
    #[arg(
        long,
        help_heading = "Rendering Options",
        default_value_t = 1024,
        help = "Browser viewport height in pixels."
    )]
    pub height: u32,
    #[arg(
        long,
        help_heading = "Rendering Options",
        default_value_t = 1.0,
        help = "Browser device scale factor for higher-density raster capture."
    )]
    pub device_scale_factor: f64,
    #[arg(
        long,
        help_heading = "Rendering Options",
        default_value_t = false,
        help = "Capture the full scrollable page instead of only the initial viewport. Ignored for PDF output."
    )]
    pub full_page: bool,
    #[arg(
        long,
        help_heading = "Output Options",
        value_enum,
        default_value_t = ScreenshotFullPageOutput::Single,
        help = "When --full-page is enabled, write one stitched file, a tiles directory, or a tiles directory plus manifest metadata."
    )]
    pub full_page_output: ScreenshotFullPageOutput,
    #[arg(
        long,
        help_heading = "Rendering Options",
        default_value_t = 5000,
        help = "Extra wait time in milliseconds after navigation so Grafana panels can finish rendering."
    )]
    pub wait_ms: u64,
    #[arg(
        long,
        help_heading = "Rendering Options",
        help = "Optional Chromium or Chrome executable path for the headless browser session."
    )]
    pub browser_path: Option<PathBuf>,
}

/// Struct definition for InspectVarsArgs.
#[derive(Debug, Clone, Args)]
pub struct InspectVarsArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        required_unless_present = "dashboard_url",
        help = "Grafana dashboard UID whose templating variables should be listed. Required unless --dashboard-url is provided."
    )]
    pub dashboard_uid: Option<String>,
    #[arg(
        long,
        required_unless_present = "dashboard_uid",
        help = "Full Grafana dashboard URL. When provided, the runtime can derive the dashboard UID from the URL path."
    )]
    pub dashboard_url: Option<String>,
    #[arg(
        long,
        value_name = "QUERY",
        help = "Grafana variable query-string fragment, for example 'var-env=prod&var-host=web01'. This overlays current values in inspect-vars output."
    )]
    pub vars_query: Option<String>,
    #[arg(
        long,
        help = "Scope the variable inspection to this Grafana org ID by sending X-Grafana-Org-Id."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        value_enum,
        help = "Render dashboard variables as table, csv, or json. Defaults to table."
    )]
    pub output_format: Option<SimpleOutputFormat>,
    #[arg(
        long,
        default_value_t = false,
        help = "Do not print table or CSV headers when rendering inspect-vars output."
    )]
    pub no_header: bool,
    #[arg(
        long,
        help = "Write inspect-vars output to this file while still printing to stdout."
    )]
    pub output_file: Option<PathBuf>,
}

/// Struct definition for InspectExportArgs.
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
        help = "Render a full inspection report. Defaults to flat per-query table output; use --report csv or --report json for machine-readable output, --report tree for dashboard-first grouped text, --report tree-table for dashboard-first grouped tables, --report dependency for dependency contracts, --report dependency-json for dependency contract JSON, --report governance for datasource governance tables, or --report governance-json for governance JSON."
    )]
    pub report: Option<InspectExportReportFormat>,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["json", "table", "report"],
        help = "Alternative single-flag output selector for inspect output. Use text, table, json, report-table, report-csv, report-json, report-tree, report-tree-table, report-dependency, report-dependency-json, governance, or governance-json."
    )]
    pub output_format: Option<InspectOutputFormat>,
    #[arg(
        long,
        value_delimiter = ',',
        value_parser = parse_inspect_report_column,
        help = "For --report table, csv, or tree-table output, or the equivalent report-like --output-format values, limit the query report to the selected columns. Use all to expand every supported column. Supported values: org, org_id, dashboard_uid, dashboard_title, dashboard_tags, folder_path, folder_full_path, folder_level, folder_uid, parent_folder_uid, panel_id, panel_title, panel_type, panel_target_count, panel_query_count, panel_datasource_count, panel_variables, ref_id, datasource, datasource_name, datasource_uid, datasource_org, datasource_org_id, datasource_database, datasource_bucket, datasource_organization, datasource_index_pattern, datasource_type, datasource_family, query_field, target_hidden, target_disabled, query_variables, metrics, functions, measurements, buckets, query, file. JSON-style aliases like orgId, dashboardUid, dashboardTags, folderFullPath, folderLevel, folderUid, parentFolderUid, panelTargetCount, panelQueryCount, panelDatasourceCount, panelVariables, datasourceName, datasourceUid, datasourceOrg, datasourceOrgId, datasourceDatabase, datasourceBucket, datasourceOrganization, datasourceIndexPattern, datasourceType, datasourceFamily, targetHidden, targetDisabled, and queryVariables are also accepted."
    )]
    pub report_columns: Vec<String>,
    #[arg(
        long,
        help = "For --report output or report-like --output-format values, include only rows whose datasource label, uid, type, or family exactly matches this value."
    )]
    pub report_filter_datasource: Option<String>,
    #[arg(
        long,
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
        help = "Do not print table headers when rendering the table summary, table-like --report output, or compatible --output-format values."
    )]
    pub no_header: bool,
    #[arg(
        long,
        help = "Write inspect output to this file while still printing to stdout."
    )]
    pub output_file: Option<PathBuf>,
}

/// Struct definition for InspectLiveArgs.
#[derive(Debug, Clone, Args)]
pub struct InspectLiveArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long, default_value_t = DEFAULT_PAGE_SIZE, help = "Dashboard search page size.")]
    pub page_size: usize,
    #[arg(
        long,
        default_value_t = 8usize,
        help = "Maximum parallel dashboard fetch workers used for live inspect when supported."
    )]
    pub concurrency: usize,
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
        help = "Render a full inspection report. Defaults to flat per-query table output; use --report csv or --report json for alternate output, --report tree for dashboard-first grouped text, --report tree-table for dashboard-first grouped tables, --report dependency for dependency contracts, --report dependency-json for dependency contract JSON, --report governance for datasource governance tables, or --report governance-json for governance JSON."
    )]
    pub report: Option<InspectExportReportFormat>,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["json", "table", "report"],
        help = "Alternative single-flag output selector for inspect output. Use text, table, json, report-table, report-csv, report-json, report-tree, report-tree-table, report-dependency, report-dependency-json, governance, or governance-json."
    )]
    pub output_format: Option<InspectOutputFormat>,
    #[arg(
        long,
        value_delimiter = ',',
        value_parser = parse_inspect_report_column,
        help = "For --report table, csv, or tree-table output, or the equivalent report-like --output-format values, limit the query report to the selected columns. Use all to expand every supported column. Supported values: org, org_id, dashboard_uid, dashboard_title, dashboard_tags, folder_path, folder_full_path, folder_level, folder_uid, parent_folder_uid, panel_id, panel_title, panel_type, panel_target_count, panel_query_count, panel_datasource_count, panel_variables, ref_id, datasource, datasource_name, datasource_uid, datasource_org, datasource_org_id, datasource_database, datasource_bucket, datasource_organization, datasource_index_pattern, datasource_type, datasource_family, query_field, target_hidden, target_disabled, query_variables, metrics, functions, measurements, buckets, query, file. JSON-style aliases like orgId, dashboardUid, dashboardTags, folderFullPath, folderLevel, folderUid, parentFolderUid, panelTargetCount, panelQueryCount, panelDatasourceCount, panelVariables, datasourceName, datasourceUid, datasourceOrg, datasourceOrgId, datasourceDatabase, datasourceBucket, datasourceOrganization, datasourceIndexPattern, datasourceType, datasourceFamily, targetHidden, targetDisabled, and queryVariables are also accepted."
    )]
    pub report_columns: Vec<String>,
    #[arg(
        long,
        help = "For --report output or report-like --output-format values, include only rows whose datasource label, uid, type, or family exactly matches this value."
    )]
    pub report_filter_datasource: Option<String>,
    #[arg(
        long,
        help = "For --report output or report-like --output-format values, include only rows whose panel id exactly matches this value."
    )]
    pub report_filter_panel_id: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Show a progress bar while live dashboards are fetched for inspection."
    )]
    pub progress: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Show extended help with report examples for inspect-live."
    )]
    pub help_full: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Do not print headers when rendering table, csv, or tree-table inspection output, including compatible --output-format values."
    )]
    pub no_header: bool,
    #[arg(
        long,
        help = "Write inspect output to this file while still printing to stdout."
    )]
    pub output_file: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        help = "Open an interactive terminal browser over the live inspection artifacts."
    )]
    pub interactive: bool,
}

/// Struct definition for GovernanceGateArgs.
#[derive(Debug, Clone, Args)]
pub struct GovernanceGateArgs {
    #[arg(long, help = "Path to the dashboard governance policy JSON.")]
    pub policy: PathBuf,
    #[arg(long, help = "Path to dashboard inspect governance-json output.")]
    pub governance: PathBuf,
    #[arg(long, help = "Path to dashboard inspect report json output.")]
    pub queries: PathBuf,
    #[arg(
        long,
        value_enum,
        default_value_t = GovernanceGateOutputFormat::Text,
        help = "Render the governance gate result as text or JSON."
    )]
    pub output_format: GovernanceGateOutputFormat,
    #[arg(
        long,
        help = "Optional path to also write the normalized governance gate result JSON."
    )]
    pub json_output: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        help = "Open an interactive terminal browser over governance findings."
    )]
    pub interactive: bool,
}

/// Struct definition for TopologyArgs.
#[derive(Debug, Clone, Args)]
pub struct TopologyArgs {
    #[arg(long, help = "Path to dashboard governance JSON.")]
    pub governance: PathBuf,
    #[arg(
        long,
        help = "Optional path to dashboard query-report JSON so the graph can include variables and panels."
    )]
    pub queries: Option<PathBuf>,
    #[arg(
        long = "alert-contract",
        help = "Optional path to a sync alert contract JSON document."
    )]
    pub alert_contract: Option<PathBuf>,
    #[arg(
        long,
        value_enum,
        default_value_t = TopologyOutputFormat::Text,
        help = "Render the topology as text, json, mermaid, or dot."
    )]
    pub output_format: TopologyOutputFormat,
    #[arg(
        long,
        help = "Optional path to also write the rendered topology output."
    )]
    pub output_file: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        help = "Open an interactive terminal browser over topology nodes and edges."
    )]
    pub interactive: bool,
}

/// Struct definition for ImpactArgs.
#[derive(Debug, Clone, Args)]
pub struct ImpactArgs {
    #[arg(long, help = "Path to dashboard governance JSON.")]
    pub governance: PathBuf,
    #[arg(
        long,
        help = "Optional path to dashboard query-report JSON so blast radius can include variables and panels."
    )]
    pub queries: Option<PathBuf>,
    #[arg(
        long = "datasource-uid",
        help = "Datasource UID to analyze for downstream dashboard and alert impact."
    )]
    pub datasource_uid: String,
    #[arg(
        long = "alert-contract",
        help = "Optional path to a sync alert contract JSON document."
    )]
    pub alert_contract: Option<PathBuf>,
    #[arg(
        long,
        value_enum,
        default_value_t = ImpactOutputFormat::Text,
        help = "Render the blast radius summary as text or json."
    )]
    pub output_format: ImpactOutputFormat,
    #[arg(
        long,
        default_value_t = false,
        help = "Open an interactive terminal browser over the blast radius document."
    )]
    pub interactive: bool,
}

/// Struct definition for ValidateExportArgs.
#[derive(Debug, Clone, Args)]
pub struct ValidateExportArgs {
    #[arg(
        long,
        help = "Validate dashboards from this raw export directory. Point this to the raw/ export directory explicitly."
    )]
    pub import_dir: PathBuf,
    #[arg(
        long,
        default_value_t = false,
        help = "Reject unsupported custom panel and datasource plugin types."
    )]
    pub reject_custom_plugins: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Reject legacy dashboard properties such as row layouts or web-import scaffolding."
    )]
    pub reject_legacy_properties: bool,
    #[arg(
        long,
        help = "Optional target dashboard schemaVersion required for this export set."
    )]
    pub target_schema_version: Option<i64>,
    #[arg(
        long,
        value_enum,
        default_value_t = ValidationOutputFormat::Text,
        help = "Render the validation result as text or JSON."
    )]
    pub output_format: ValidationOutputFormat,
    #[arg(long, help = "Optional path to also write the validation JSON result.")]
    pub output_file: Option<PathBuf>,
}

/// Enum definition for DashboardCommand.
#[derive(Debug, Clone, Subcommand)]
pub enum DashboardCommand {
    #[command(
        name = "list",
        about = "List dashboard summaries without writing export files.",
        after_help = "Examples:\n\n  List dashboards from the current org with Basic auth:\n    grafana-util list --url http://localhost:3000 --basic-user admin --basic-password admin\n\n  List dashboards across all visible orgs with Basic auth:\n    grafana-util list --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --json\n\n  List dashboards from one explicit org ID:\n    grafana-util list --url http://localhost:3000 --basic-user admin --basic-password admin --org-id 2 --csv\n\n  List dashboards from the current org with an API token:\n    grafana-util list --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --json"
    )]
    List(ListArgs),
    #[command(
        name = "export",
        about = "Export dashboards to raw/ and prompt/ JSON files.",
        after_help = "Examples:\n\n  Export dashboards from the current org with Basic auth:\n    grafana-util export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./dashboards --overwrite\n\n  Export dashboards across all visible orgs with Basic auth:\n    grafana-util export --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --export-dir ./dashboards --overwrite\n\n  Export dashboards from one explicit org ID:\n    grafana-util export --url http://localhost:3000 --basic-user admin --basic-password admin --org-id 2 --export-dir ./dashboards --overwrite\n\n  Export dashboards from the current org with an API token:\n    export GRAFANA_API_TOKEN='your-token'\n    grafana-util export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --export-dir ./dashboards --overwrite"
    )]
    Export(ExportArgs),
    #[command(
        name = "import",
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
    #[command(
        name = "inspect-vars",
        about = "List dashboard templating variables and datasource-like choices from live Grafana."
    )]
    InspectVars(InspectVarsArgs),
    #[command(
        name = "governance-gate",
        about = "Evaluate a governance policy against dashboard governance-json and query-report JSON artifacts.",
        after_help = "Examples:\n\n  Evaluate governance policy with text output:\n    grafana-util dashboard governance-gate --policy ./policy.json --governance ./governance.json --queries ./queries.json\n\n  Write the normalized result JSON while also printing machine-readable output:\n    grafana-util dashboard governance-gate --policy ./policy.json --governance ./governance.json --queries ./queries.json --output-format json --json-output ./governance-check.json"
    )]
    GovernanceGate(GovernanceGateArgs),
    #[command(
        name = "topology",
        visible_alias = "graph",
        about = "Build a deterministic dashboard, datasource, variable, and alert topology from JSON artifacts.",
        after_help = "Examples:\n\n  Render a dashboard topology graph in Mermaid:\n    grafana-util dashboard topology --governance ./governance.json --queries ./queries.json --alert-contract ./alert-contract.json --output-format mermaid\n\n  Render the same graph through the graph alias as DOT while also writing it to disk:\n    grafana-util dashboard graph --governance ./governance.json --queries ./queries.json --alert-contract ./alert-contract.json --output-format dot --output-file ./dashboard-topology.dot"
    )]
    Topology(TopologyArgs),
    #[command(
        name = "impact",
        about = "Summarize dashboard, variable, panel, and alert blast radius for one datasource from JSON artifacts.",
        after_help = "Examples:\n\n  Summarize blast radius as text:\n    grafana-util dashboard impact --governance ./governance.json --queries ./queries.json --datasource-uid prom-main --alert-contract ./alert-contract.json --output-format text\n\n  Render the same blast radius as JSON:\n    grafana-util dashboard impact --governance ./governance.json --queries ./queries.json --datasource-uid prom-main --output-format json"
    )]
    Impact(ImpactArgs),
    #[command(
        name = "validate-export",
        about = "Run strict schema validation against dashboard raw export files before GitOps sync.",
        after_help = "Examples:\n\n  Validate a raw export and fail on migration or plugin issues:\n    grafana-util dashboard validate-export --import-dir ./dashboards/raw --reject-custom-plugins --reject-legacy-properties --target-schema-version 39\n\n  Write the validation report as JSON:\n    grafana-util dashboard validate-export --import-dir ./dashboards/raw --output-format json --output-file ./dashboard-validation.json"
    )]
    ValidateExport(ValidateExportArgs),
    #[command(
        name = "screenshot",
        about = "Open one Grafana dashboard in a headless browser and capture PNG, JPEG, or PDF output.",
        after_help = "Examples:\n\n  Capture a full dashboard from a browser URL and add an auto title/header block:\n    grafana-util dashboard screenshot --dashboard-url 'https://grafana.example.com/d/cpu-main/cpu-overview?var-cluster=prod-a' --token \"$GRAFANA_API_TOKEN\" --output ./cpu-main.png --full-page --header-title --header-url --header-captured-at\n\n  Capture a solo panel with a vars-query fragment and custom header note:\n    grafana-util dashboard screenshot --url https://grafana.example.com --dashboard-uid rYdddlPWk --panel-id 20 --vars-query 'var-datasource=prom-main&var-job=node-exporter&var-node=host01:9100' --token \"$GRAFANA_API_TOKEN\" --output ./panel.png --header-title 'CPU Busy' --header-text 'Solo panel debug capture'"
    )]
    Screenshot(ScreenshotArgs),
}

#[derive(Debug, Clone, Parser)]
#[command(
    about = "Export or import Grafana dashboards.",
    after_help = "Examples:\n\n  Export dashboards from local Grafana with Basic auth:\n    grafana-util export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./dashboards --overwrite\n\n  Export dashboards across all visible orgs with Basic auth:\n    grafana-util export --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --export-dir ./dashboards --overwrite\n\n  List dashboards across all visible orgs with Basic auth:\n    grafana-util list --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --json\n\n  Export dashboards with an API token from the current org:\n    export GRAFANA_API_TOKEN='your-token'\n    grafana-util export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --export-dir ./dashboards --overwrite\n\n  Compare raw dashboard exports against local Grafana:\n    grafana-util diff --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards/raw\n\n  Capture a browser-rendered dashboard screenshot:\n    grafana-util screenshot --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --dashboard-uid cpu-main --output ./cpu-main.png --from now-6h --to now",
    styles = crate::help_styles::CLI_HELP_STYLES
)]
/// Struct definition for DashboardCliArgs.
pub struct DashboardCliArgs {
    #[command(subcommand)]
    pub command: DashboardCommand,
}

/// Struct definition for DashboardAuthContext.
#[derive(Debug, Clone)]
pub struct DashboardAuthContext {
    pub url: String,
    pub timeout: u64,
    pub verify_ssl: bool,
    pub auth_mode: String,
    pub headers: Vec<(String, String)>,
}

/// Parse dashboard CLI argv and normalize output-format aliases to keep
/// downstream handlers deterministic.
pub fn parse_cli_from<I, T>(iter: I) -> DashboardCliArgs
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    // Call graph (hierarchy): this function is used in related modules.
    // Upstream callers: 無
    // Downstream callees: dashboard_cli_defs.rs:normalize_dashboard_cli_args

    normalize_dashboard_cli_args(DashboardCliArgs::parse_from(iter))
}

// Accept both user-facing legacy aliases and canonical snake_case column names for
// import dry-run table formatting.
fn parse_dashboard_import_output_column(value: &str) -> std::result::Result<String, String> {
    // Call graph (hierarchy): this function is used in related modules.
    // Upstream callers: 無
    // Downstream callees: 無

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

fn parse_dashboard_list_output_column(value: &str) -> std::result::Result<String, String> {
    // Call graph (hierarchy): this function is used in related modules.
    // Upstream callers: 無
    // Downstream callees: 無

    match value {
        "uid" => Ok("uid".to_string()),
        "name" => Ok("name".to_string()),
        "folder" => Ok("folder".to_string()),
        "folder_uid" | "folderUid" => Ok("folder_uid".to_string()),
        "path" => Ok("path".to_string()),
        "org" => Ok("org".to_string()),
        "org_id" | "orgId" => Ok("org_id".to_string()),
        "sources" => Ok("sources".to_string()),
        "source_uids" | "sourceUids" => Ok("source_uids".to_string()),
        _ => Err(format!(
            "Unsupported --output-columns value '{value}'. Supported values: uid, name, folder, folder_uid, path, org, org_id, sources, source_uids."
        )),
    }
}

fn parse_inspect_report_column(value: &str) -> std::result::Result<String, String> {
    // Call graph (hierarchy): this function is used in related modules.
    // Upstream callers: 無
    // Downstream callees: 無

    match value {
        "all" => Ok("all".to_string()),
        "org" => Ok("org".to_string()),
        "org_id" | "orgId" => Ok("org_id".to_string()),
        "dashboard_uid" | "dashboardUid" => Ok("dashboard_uid".to_string()),
        "dashboard_title" | "dashboardTitle" => Ok("dashboard_title".to_string()),
        "dashboard_tags" | "dashboardTags" => Ok("dashboard_tags".to_string()),
        "folder_path" | "folderPath" => Ok("folder_path".to_string()),
        "folder_full_path" | "folderFullPath" => Ok("folder_full_path".to_string()),
        "folder_level" | "folderLevel" => Ok("folder_level".to_string()),
        "folder_uid" | "folderUid" => Ok("folder_uid".to_string()),
        "parent_folder_uid" | "parentFolderUid" => Ok("parent_folder_uid".to_string()),
        "panel_id" | "panelId" => Ok("panel_id".to_string()),
        "panel_title" | "panelTitle" => Ok("panel_title".to_string()),
        "panel_type" | "panelType" => Ok("panel_type".to_string()),
        "panel_target_count" | "panelTargetCount" => Ok("panel_target_count".to_string()),
        "panel_query_count" | "panelQueryCount" => Ok("panel_query_count".to_string()),
        "panel_datasource_count" | "panelDatasourceCount" => {
            Ok("panel_datasource_count".to_string())
        }
        "panel_variables" | "panelVariables" => Ok("panel_variables".to_string()),
        "ref_id" | "refId" => Ok("ref_id".to_string()),
        "datasource" => Ok("datasource".to_string()),
        "datasource_name" | "datasourceName" => Ok("datasource_name".to_string()),
        "datasource_uid" | "datasourceUid" => Ok("datasource_uid".to_string()),
        "datasource_org" | "datasourceOrg" => Ok("datasource_org".to_string()),
        "datasource_org_id" | "datasourceOrgId" => Ok("datasource_org_id".to_string()),
        "datasource_database" | "datasourceDatabase" => Ok("datasource_database".to_string()),
        "datasource_bucket" | "datasourceBucket" => Ok("datasource_bucket".to_string()),
        "datasource_organization" | "datasourceOrganization" => {
            Ok("datasource_organization".to_string())
        }
        "datasource_index_pattern" | "datasourceIndexPattern" => {
            Ok("datasource_index_pattern".to_string())
        }
        "datasource_type" | "datasourceType" => Ok("datasource_type".to_string()),
        "datasource_family" | "datasourceFamily" => Ok("datasource_family".to_string()),
        "query_field" | "queryField" => Ok("query_field".to_string()),
        "target_hidden" | "targetHidden" => Ok("target_hidden".to_string()),
        "target_disabled" | "targetDisabled" => Ok("target_disabled".to_string()),
        "query_variables" | "queryVariables" => Ok("query_variables".to_string()),
        "metrics" => Ok("metrics".to_string()),
        "functions" => Ok("functions".to_string()),
        "measurements" => Ok("measurements".to_string()),
        "buckets" => Ok("buckets".to_string()),
        "query" => Ok("query".to_string()),
        "file" => Ok("file".to_string()),
        _ => Err(format!(
            "Unsupported --report-columns value '{value}'. Supported values: all, org, org_id, dashboard_uid, dashboard_title, dashboard_tags, folder_path, folder_full_path, folder_level, folder_uid, parent_folder_uid, panel_id, panel_title, panel_type, panel_target_count, panel_query_count, panel_datasource_count, panel_variables, ref_id, datasource, datasource_name, datasource_uid, datasource_org, datasource_org_id, datasource_database, datasource_bucket, datasource_organization, datasource_index_pattern, datasource_type, datasource_family, query_field, target_hidden, target_disabled, query_variables, metrics, functions, measurements, buckets, query, file."
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

/// Normalize dashboard subcommand variants so legacy and explicit flags end up with
/// the same boolean state contract for command handlers.
pub fn normalize_dashboard_cli_args(mut args: DashboardCliArgs) -> DashboardCliArgs {
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
        _ => {}
    }
    args
}

/// Purpose: implementation note.
///
/// Args: see function signature.
/// Returns: see implementation.
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

/// Purpose: implementation note.
///
/// Args: see function signature.
/// Returns: see implementation.
pub fn build_http_client(common: &CommonCliArgs) -> Result<JsonHttpClient> {
    // Call graph (hierarchy): this function is used in related modules.
    // Upstream callers: 無
    // Downstream callees: dashboard_cli_defs.rs:build_auth_context

    let context = build_auth_context(common)?;
    JsonHttpClient::new(JsonHttpClientConfig {
        base_url: context.url,
        headers: context.headers,
        timeout_secs: context.timeout,
        verify_ssl: context.verify_ssl,
    })
}

/// Purpose: implementation note.
///
/// Args: see function signature.
/// Returns: see implementation.
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
