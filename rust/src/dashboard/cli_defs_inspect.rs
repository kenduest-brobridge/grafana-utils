//! Inspection path for Dashboard resources: analysis, extraction, and report shaping.

use clap::{Args, ValueEnum};
use std::path::PathBuf;

use super::super::DEFAULT_PAGE_SIZE;
use super::dashboard_runtime::parse_inspect_report_column;
use super::{
    CommonCliArgs, DashboardImportInputFormat, GovernanceGateOutputFormat, GovernancePolicySource,
    ImpactOutputFormat, InspectExportInputType, SimpleOutputFormat, TopologyOutputFormat,
    ValidationOutputFormat,
};

/// Enum definition for structured dashboard analysis outputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum InspectExportReportFormat {
    Table,
    Csv,
    #[value(name = "queries-json")]
    QueriesJson,
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
    Csv,
    Json,
    Yaml,
    Tree,
    TreeTable,
    Dependency,
    DependencyJson,
    Governance,
    GovernanceJson,
    #[value(name = "queries-json")]
    QueriesJson,
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
        help = "Grafana dashboard UID whose templating variables should be listed. Use this to pick one dashboard from a local export tree or to read one live dashboard."
    )]
    pub dashboard_uid: Option<String>,
    #[arg(
        long,
        conflicts_with = "input",
        help = "Full Grafana dashboard URL. When provided, the runtime can derive the dashboard UID from the URL path."
    )]
    pub dashboard_url: Option<String>,
    #[arg(
        long,
        value_name = "FILE",
        conflicts_with = "input_dir",
        help = "Read one local dashboard JSON file instead of calling Grafana. Use this for a raw dashboard file, a prompt file, or a file-provisioning dashboard object."
    )]
    pub input: Option<PathBuf>,
    #[arg(
        long = "input-dir",
        value_name = "DIR",
        conflicts_with = "input",
        help = "Read one local dashboard from an export tree instead of calling Grafana. Point this at a raw/ export root, an all-orgs export root, or a provisioning/ dashboards tree."
    )]
    pub input_dir: Option<PathBuf>,
    #[arg(
        long,
        value_enum,
        default_value_t = DashboardImportInputFormat::Raw,
        requires = "input_dir",
        help = "Interpret --input-dir as raw export files or Grafana file-provisioning artifacts. Use provisioning to accept either the provisioning/ root or its dashboards/ subdirectory."
    )]
    pub input_format: DashboardImportInputFormat,
    #[arg(
        long,
        value_name = "QUERY",
        help = "Grafana variable query-string fragment, for example 'var-env=prod&var-host=web01'. This overlays current values in list-vars output."
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
        help = "Render dashboard variables as table, csv, text, json, or yaml. Defaults to table."
    )]
    pub output_format: Option<SimpleOutputFormat>,
    #[arg(
        long,
        default_value_t = false,
        help = "Do not print table or CSV headers when rendering list-vars output."
    )]
    pub no_header: bool,
    #[arg(long, help = "Write list-vars output to this file.")]
    pub output_file: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        requires = "output_file",
        help = "When --output-file is set, also print list-vars output to stdout."
    )]
    pub also_stdout: bool,
}

/// Struct definition for AnalyzeArgs.
#[derive(Debug, Clone, Args)]
pub struct AnalyzeArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long = "input-dir",
        help = "Analyze dashboards from this directory instead of live Grafana. Use --input-format provisioning to point at a provisioning/ root or its dashboards/ subdirectory."
    )]
    pub input_dir: Option<PathBuf>,
    #[arg(
        long,
        value_enum,
        requires = "input_dir",
        help = "When --input-dir points at a dashboard export root that contains multiple variants, select which dashboard tree to analyze. Use raw for raw/ and source for prompt/."
    )]
    pub input_type: Option<InspectExportInputType>,
    #[arg(
        long,
        value_enum,
        default_value_t = DashboardImportInputFormat::Raw,
        requires = "input_dir",
        help = "Interpret --input-dir as raw export files or Grafana file-provisioning artifacts. Use provisioning to accept either the provisioning/ root or its dashboards/ subdirectory."
    )]
    pub input_format: DashboardImportInputFormat,
    #[arg(
        long,
        default_value_t = DEFAULT_PAGE_SIZE,
        help = "Dashboard search page size when analyze reads live Grafana."
    )]
    pub page_size: usize,
    #[arg(
        long,
        default_value_t = 8usize,
        help = "Maximum parallel dashboard fetch workers used when analyze reads live Grafana."
    )]
    pub concurrency: usize,
    #[arg(
        long,
        conflicts_with_all = ["all_orgs", "input_dir"],
        help = "Analyze dashboards from this Grafana org ID when reading live Grafana."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["org_id", "input_dir"],
        help = "Enumerate all visible Grafana orgs and analyze dashboards across them when reading live Grafana."
    )]
    pub all_orgs: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["table", "csv", "json", "yaml", "output_format"],
        help = "Render the dashboard analysis as plain text."
    )]
    pub text: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["text", "csv", "json", "yaml", "output_format"],
        help = "Render the dashboard analysis as a table-oriented summary."
    )]
    pub table: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["text", "table", "json", "yaml", "output_format"],
        help = "Render the dashboard analysis as CSV."
    )]
    pub csv: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["text", "table", "csv", "yaml", "output_format"],
        help = "Render the dashboard analysis as JSON."
    )]
    pub json: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["text", "table", "csv", "json", "output_format"],
        help = "Render the dashboard analysis as YAML."
    )]
    pub yaml: bool,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["text", "table", "csv", "json", "yaml"],
        help = "Single-flag output selector for dashboard analysis. Use text, table, csv, json, yaml, tree, tree-table, dependency, dependency-json, governance, governance-json, or queries-json."
    )]
    pub output_format: Option<InspectOutputFormat>,
    #[arg(
        long,
        value_delimiter = ',',
        value_parser = parse_inspect_report_column,
        help = "For table, csv, or tree-table query analysis output, limit the query report to the selected columns. Use all to expand every supported column."
    )]
    pub report_columns: Vec<String>,
    #[arg(
        long,
        help = "For table, csv, tree, tree-table, dependency, dependency-json, governance, governance-json, or queries-json output, include only rows whose datasource label, uid, type, or family exactly matches this value."
    )]
    pub report_filter_datasource: Option<String>,
    #[arg(
        long,
        help = "For table, csv, tree, tree-table, dependency, dependency-json, governance, governance-json, or queries-json output, include only rows whose panel id exactly matches this value."
    )]
    pub report_filter_panel_id: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Show a progress bar while live dashboards are fetched for analysis."
    )]
    pub progress: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Show extended help with advanced analysis examples."
    )]
    pub help_full: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Do not print headers when rendering table, csv, or tree-table analysis output."
    )]
    pub no_header: bool,
    #[arg(long, help = "Write analysis output to this file.")]
    pub output_file: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        requires = "output_file",
        help = "When --output-file is set, also print analysis output to stdout."
    )]
    pub also_stdout: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Open the shared interactive analysis workbench over the analyzed dashboard set."
    )]
    pub interactive: bool,
}

/// Struct definition for InspectExportArgs.
#[derive(Debug, Clone, Args)]
pub struct InspectExportArgs {
    #[arg(
        long = "input-dir",
        help = "Analyze dashboards from this directory. Use --input-format provisioning to point at a provisioning/ root or its dashboards/ subdirectory."
    )]
    pub input_dir: PathBuf,
    #[arg(
        long = "input-type",
        value_enum,
        help = "When --input-dir points at a dashboard export root that contains multiple variants, select which dashboard tree to inspect. Use raw for raw/ and source for prompt/."
    )]
    pub input_type: Option<InspectExportInputType>,
    #[arg(
        long = "input-format",
        value_enum,
        default_value_t = DashboardImportInputFormat::Raw,
        help = "Interpret --input-dir as raw export files or Grafana file-provisioning artifacts. Use provisioning to accept either the provisioning/ root or its dashboards/ subdirectory."
    )]
    pub input_format: DashboardImportInputFormat,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["table", "csv", "json", "yaml", "output_format"],
        help = "Render the export analysis as an operator-summary plain-text view."
    )]
    pub text: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["text", "csv", "json", "yaml", "output_format"],
        help = "Render the export analysis as an operator-summary table."
    )]
    pub table: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["text", "table", "json", "yaml", "output_format"],
        help = "Render the export analysis as operator-summary CSV."
    )]
    pub csv: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["text", "table", "csv", "yaml", "output_format"],
        help = "Render the export analysis as the full machine-readable summary contract in JSON."
    )]
    pub json: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["text", "table", "csv", "json", "output_format"],
        help = "Render the export analysis as the full machine-readable summary contract in YAML."
    )]
    pub yaml: bool,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["text", "table", "csv", "json", "yaml"],
        help = "Single-flag output selector for analyze-export output. Use text, table, csv, json, yaml, tree, tree-table, dependency, dependency-json, governance, governance-json, or queries-json."
    )]
    pub output_format: Option<InspectOutputFormat>,
    #[arg(
        long,
        value_delimiter = ',',
        value_parser = parse_inspect_report_column,
        help = "For table, csv, or tree-table query analysis output, limit the query report to the selected columns. Use all to expand every supported column. Supported values: org, org_id, dashboard_uid, dashboard_title, dashboard_tags, folder_path, folder_full_path, folder_level, folder_uid, parent_folder_uid, panel_id, panel_title, panel_type, panel_target_count, panel_query_count, panel_datasource_count, panel_variables, ref_id, datasource, datasource_name, datasource_uid, datasource_org, datasource_org_id, datasource_database, datasource_bucket, datasource_organization, datasource_index_pattern, datasource_type, datasource_family, query_field, target_hidden, target_disabled, query_variables, metrics, functions, measurements, buckets, query, file. JSON-style aliases like orgId, dashboardUid, dashboardTags, folderFullPath, folderLevel, folderUid, parentFolderUid, panelTargetCount, panelQueryCount, panelDatasourceCount, panelVariables, datasourceName, datasourceUid, datasourceOrg, datasourceOrgId, datasourceDatabase, datasourceBucket, datasourceOrganization, datasourceIndexPattern, datasourceType, datasourceFamily, targetHidden, targetDisabled, and queryVariables are also accepted."
    )]
    pub report_columns: Vec<String>,
    #[arg(
        long,
        help = "For table, csv, tree, tree-table, dependency, dependency-json, governance, governance-json, or queries-json output, include only rows whose datasource label, uid, type, or family exactly matches this value."
    )]
    pub report_filter_datasource: Option<String>,
    #[arg(
        long,
        help = "For table, csv, tree, tree-table, dependency, dependency-json, governance, governance-json, or queries-json output, include only rows whose panel id exactly matches this value."
    )]
    pub report_filter_panel_id: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Show extended help with analysis examples for analyze-export."
    )]
    pub help_full: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Do not print table headers when rendering the table summary, table, csv, or tree-table query analysis output."
    )]
    pub no_header: bool,
    #[arg(long, help = "Write inspect output to this file.")]
    pub output_file: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        requires = "output_file",
        help = "When --output-file is set, also print inspect output to stdout."
    )]
    pub also_stdout: bool,
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
        conflicts_with_all = ["table", "csv", "json", "yaml", "output_format"],
        help = "Render the live inspection analysis as plain text."
    )]
    pub text: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["text", "csv", "json", "yaml", "output_format"],
        help = "Render the live inspection analysis as a table-oriented summary."
    )]
    pub table: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["text", "table", "json", "yaml", "output_format"],
        help = "Render the live inspection analysis as CSV."
    )]
    pub csv: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["text", "table", "csv", "yaml", "output_format"],
        help = "Render the live inspection analysis as JSON."
    )]
    pub json: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["text", "table", "csv", "json", "output_format"],
        help = "Render the live inspection analysis as YAML."
    )]
    pub yaml: bool,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["text", "table", "csv", "json", "yaml"],
        help = "Single-flag output selector for analyze-live output. Use text, table, csv, json, yaml, tree, tree-table, dependency, dependency-json, governance, governance-json, or queries-json."
    )]
    pub output_format: Option<InspectOutputFormat>,
    #[arg(
        long,
        value_delimiter = ',',
        value_parser = parse_inspect_report_column,
        help = "For table, csv, or tree-table query analysis output, limit the query report to the selected columns. Use all to expand every supported column. Supported values: org, org_id, dashboard_uid, dashboard_title, dashboard_tags, folder_path, folder_full_path, folder_level, folder_uid, parent_folder_uid, panel_id, panel_title, panel_type, panel_target_count, panel_query_count, panel_datasource_count, panel_variables, ref_id, datasource, datasource_name, datasource_uid, datasource_org, datasource_org_id, datasource_database, datasource_bucket, datasource_organization, datasource_index_pattern, datasource_type, datasource_family, query_field, target_hidden, target_disabled, query_variables, metrics, functions, measurements, buckets, query, file. JSON-style aliases like orgId, dashboardUid, dashboardTags, folderFullPath, folderLevel, folderUid, parentFolderUid, panelTargetCount, panelQueryCount, panelDatasourceCount, panelVariables, datasourceName, datasourceUid, datasourceOrg, datasourceOrgId, datasourceDatabase, datasourceBucket, datasourceOrganization, datasourceIndexPattern, datasourceType, datasourceFamily, targetHidden, targetDisabled, and queryVariables are also accepted."
    )]
    pub report_columns: Vec<String>,
    #[arg(
        long,
        help = "For table, csv, tree, tree-table, dependency, dependency-json, governance, governance-json, or queries-json output, include only rows whose datasource label, uid, type, or family exactly matches this value."
    )]
    pub report_filter_datasource: Option<String>,
    #[arg(
        long,
        help = "For table, csv, tree, tree-table, dependency, dependency-json, governance, governance-json, or queries-json output, include only rows whose panel id exactly matches this value."
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
        help = "Show extended help with analysis examples for analyze-live."
    )]
    pub help_full: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Do not print headers when rendering table, csv, or tree-table inspection output."
    )]
    pub no_header: bool,
    #[arg(long, help = "Write inspect output to this file.")]
    pub output_file: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        requires = "output_file",
        help = "When --output-file is set, also print inspect output to stdout."
    )]
    pub also_stdout: bool,
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
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        default_value_t = DEFAULT_PAGE_SIZE,
        help = "Dashboard search page size when governance-gate stages live analysis artifacts."
    )]
    pub page_size: usize,
    #[arg(
        long,
        conflicts_with = "all_orgs",
        help = "Analyze dashboards from one explicit Grafana org ID instead of the current org when reading live Grafana."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "org_id",
        help = "Analyze dashboards across all visible Grafana orgs when reading live Grafana. Prefer Basic auth when you need cross-org analysis because API tokens are often scoped to one org."
    )]
    pub all_orgs: bool,
    #[arg(
        long = "input-dir",
        conflicts_with_all = ["governance", "queries"],
        help = "Analyze dashboards from this local export tree directly. Prefer --url for live Grafana or saved artifacts only for advanced reuse."
    )]
    pub input_dir: Option<PathBuf>,
    #[arg(
        long = "input-format",
        value_enum,
        default_value_t = DashboardImportInputFormat::Raw,
        help = "Interpret --input-dir as raw export files or Grafana file-provisioning artifacts from a local export tree."
    )]
    pub input_format: DashboardImportInputFormat,
    #[arg(
        long = "input-type",
        value_enum,
        help = "Disambiguate a mixed export root when --input-dir can resolve to both raw and source-style dashboard inputs."
    )]
    pub input_type: Option<InspectExportInputType>,
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
    #[arg(
        long,
        help = "Reuse a saved governance-json artifact. Prefer --url or --input-dir for the common path; keep this for advanced reuse."
    )]
    pub governance: Option<PathBuf>,
    #[arg(
        long,
        help = "Reuse a saved queries-json artifact. Prefer --url or --input-dir for the common path; keep this for advanced reuse."
    )]
    pub queries: Option<PathBuf>,
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
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        default_value_t = DEFAULT_PAGE_SIZE,
        help = "Dashboard search page size when topology stages live analysis artifacts."
    )]
    pub page_size: usize,
    #[arg(
        long,
        conflicts_with = "all_orgs",
        help = "Analyze dashboards from one explicit Grafana org ID instead of the current org when reading live Grafana."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "org_id",
        help = "Analyze dashboards across all visible Grafana orgs when reading live Grafana. Prefer Basic auth when you need cross-org analysis because API tokens are often scoped to one org."
    )]
    pub all_orgs: bool,
    #[arg(
        long = "input-dir",
        conflicts_with_all = ["governance", "queries"],
        help = "Analyze dashboards from this local export tree directly. Prefer --url for live Grafana or saved artifacts only for advanced reuse."
    )]
    pub input_dir: Option<PathBuf>,
    #[arg(
        long,
        value_enum,
        default_value_t = DashboardImportInputFormat::Raw,
        help = "Interpret --input-dir as raw export files or Grafana file-provisioning artifacts from a local export tree."
    )]
    pub input_format: DashboardImportInputFormat,
    #[arg(
        long,
        value_enum,
        help = "Disambiguate a mixed export root when --input-dir can resolve to both raw and source-style dashboard inputs."
    )]
    pub input_type: Option<InspectExportInputType>,
    #[arg(
        long,
        help = "Reuse a saved governance-json artifact. Prefer --url or --input-dir for the common path; keep this for advanced reuse."
    )]
    pub governance: Option<PathBuf>,
    #[arg(
        long,
        help = "Reuse a saved queries-json artifact when you already split analysis into artifact files. Prefer --url or --input-dir for the common path; keep this for advanced reuse."
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
        help = "Choose how to render the dependency view: text for terminal reading, json for CI/scripts, mermaid for Markdown/docs, or dot for Graphviz."
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
        requires = "output_file",
        help = "When --output-file is set, also print the rendered topology to stdout."
    )]
    pub also_stdout: bool,
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
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        default_value_t = DEFAULT_PAGE_SIZE,
        help = "Dashboard search page size when impact stages live analysis artifacts."
    )]
    pub page_size: usize,
    #[arg(
        long,
        conflicts_with = "all_orgs",
        help = "Analyze dashboards from one explicit Grafana org ID instead of the current org when reading live Grafana."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "org_id",
        help = "Analyze dashboards across all visible Grafana orgs when reading live Grafana. Prefer Basic auth when you need cross-org analysis because API tokens are often scoped to one org."
    )]
    pub all_orgs: bool,
    #[arg(
        long = "input-dir",
        conflicts_with_all = ["governance", "queries"],
        help = "Analyze dashboards from this export directory instead of live Grafana or prebuilt artifact files."
    )]
    pub input_dir: Option<PathBuf>,
    #[arg(
        long = "input-format",
        value_enum,
        default_value_t = DashboardImportInputFormat::Raw,
        help = "Interpret --input-dir as raw export files or Grafana file-provisioning artifacts."
    )]
    pub input_format: DashboardImportInputFormat,
    #[arg(
        long = "input-type",
        value_enum,
        help = "Disambiguate a mixed export root when --input-dir can resolve to both raw and source-style dashboard inputs."
    )]
    pub input_type: Option<InspectExportInputType>,
    #[arg(
        long,
        help = "Reuse this dashboard governance JSON artifact instead of analyzing live Grafana or an export tree first."
    )]
    pub governance: Option<PathBuf>,
    #[arg(
        long,
        help = "Optional query report artifact path when reusing prebuilt analysis files. Impact itself derives blast radius from governance data plus optional alert-contract data."
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
        long = "input-dir",
        help = "Validate dashboards from this export directory. Use raw/ by default, or use provisioning/ or its dashboards/ subdirectory with --input-format provisioning."
    )]
    pub input_dir: PathBuf,
    #[arg(
        long,
        value_enum,
        default_value_t = DashboardImportInputFormat::Raw,
        help = "Interpret --input-dir as raw export files or Grafana file-provisioning artifacts. Use provisioning to accept either the provisioning/ root or its dashboards/ subdirectory."
    )]
    pub input_format: DashboardImportInputFormat,
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
    #[arg(
        long,
        default_value_t = false,
        requires = "output_file",
        help = "When --output-file is set, also print the validation result to stdout."
    )]
    pub also_stdout: bool,
}
