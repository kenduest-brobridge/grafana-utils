use clap::{Args, ValueEnum};
use std::path::PathBuf;

use super::super::DEFAULT_PAGE_SIZE;
use super::dashboard_runtime::parse_inspect_report_column;
use super::{
    CommonCliArgs, GovernanceGateOutputFormat, GovernancePolicySource, ImpactOutputFormat,
    SimpleOutputFormat, TopologyOutputFormat, ValidationOutputFormat,
};

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
    #[arg(
        long,
        default_value_t = false,
        help = "Open the shared interactive inspect workbench over the export artifacts."
    )]
    pub interactive: bool,
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
        help = "Open the shared interactive inspect workbench over the live inspection artifacts."
    )]
    pub interactive: bool,
}

/// Struct definition for GovernanceGateArgs.
#[derive(Debug, Clone, Args)]
pub struct GovernanceGateArgs {
    #[arg(
        long,
        value_enum,
        default_value_t = GovernancePolicySource::File,
        help = "Select the governance policy source. Use file with --policy, or builtin without --policy."
    )]
    pub policy_source: GovernancePolicySource,
    #[arg(
        long,
        help = "Path to the dashboard governance policy file (JSON or YAML)."
    )]
    pub policy: Option<PathBuf>,
    #[arg(
        long = "builtin-policy",
        conflicts_with = "policy",
        help = "Built-in governance policy name. Use with --policy-source builtin."
    )]
    pub builtin_policy: Option<String>,
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
