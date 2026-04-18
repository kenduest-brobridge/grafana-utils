use clap::{Args, ValueEnum};
use std::path::PathBuf;

use super::super::super::DEFAULT_PAGE_SIZE;
use super::super::dashboard_runtime::parse_inspect_report_column;
use super::super::{
    CommonCliArgs, DashboardImportInputFormat, InspectExportInputType, SimpleOutputFormat,
};
use super::parse_dashboard_summary_input_format;

/// Enum definition for structured dashboard summary outputs.
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
        help = "Grafana variable query-string fragment, for example 'var-env=prod&var-host=web01'. This overlays current values in variables output."
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
        help = "Do not print table or CSV headers when rendering variables output."
    )]
    pub no_header: bool,
    #[arg(long, help = "Write variables output to this file.")]
    pub output_file: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        requires = "output_file",
        help = "When --output-file is set, also print variables output to stdout."
    )]
    pub also_stdout: bool,
}

/// Struct definition for SummaryArgs.
#[derive(Debug, Clone, Args)]
pub struct SummaryArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long = "input-dir",
        help = "Summarize dashboards from this directory instead of live Grafana. Use --input-format provisioning for a provisioning/ root or its dashboards/ subdirectory, or --input-format git-sync for a repo-backed dashboard tree."
    )]
    pub input_dir: Option<PathBuf>,
    #[arg(
        long,
        value_enum,
        requires = "input_dir",
        help = "When --input-dir points at a dashboard export root that contains multiple variants, select which dashboard tree to summarize. Use raw for raw/ and source for prompt/."
    )]
    pub input_type: Option<InspectExportInputType>,
    #[arg(
        long,
        default_value = "raw",
        value_parser = parse_dashboard_summary_input_format,
        requires = "input_dir",
        value_name = "raw|provisioning|git-sync",
        help = "Interpret --input-dir as raw export files, Grafana file-provisioning artifacts, or a repo-backed Git Sync dashboard tree. Use git-sync for a Grafana OaC repo root; use provisioning for a provisioning/ root or its dashboards/ subdirectory."
    )]
    pub input_format: DashboardImportInputFormat,
    #[arg(
        long,
        default_value_t = DEFAULT_PAGE_SIZE,
        help = "Dashboard search page size when summary reads live Grafana."
    )]
    pub page_size: usize,
    #[arg(
        long,
        default_value_t = 8usize,
        help = "Maximum parallel dashboard fetch workers used when summary reads live Grafana."
    )]
    pub concurrency: usize,
    #[arg(
        long,
        conflicts_with_all = ["all_orgs", "input_dir"],
        help = "Summarize dashboards from this Grafana org ID when reading live Grafana."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["org_id", "input_dir"],
        help = "Enumerate all visible Grafana orgs and summarize dashboards across them when reading live Grafana."
    )]
    pub all_orgs: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["table", "csv", "json", "yaml", "output_format"],
        help = "Render the dashboard summary as plain text."
    )]
    pub text: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["text", "csv", "json", "yaml", "output_format"],
        help = "Render the dashboard summary as a table-oriented view."
    )]
    pub table: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["text", "table", "json", "yaml", "output_format"],
        help = "Render the dashboard summary as CSV."
    )]
    pub csv: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["text", "table", "csv", "yaml", "output_format"],
        help = "Render the dashboard summary as JSON."
    )]
    pub json: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["text", "table", "csv", "json", "output_format"],
        help = "Render the dashboard summary as YAML."
    )]
    pub yaml: bool,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["text", "table", "csv", "json", "yaml"],
        help = "Single-flag output selector for dashboard summary. Use text, table, csv, json, yaml, tree, tree-table, dependency, dependency-json, governance, governance-json, or queries-json."
    )]
    pub output_format: Option<InspectOutputFormat>,
    #[arg(
        long,
        value_delimiter = ',',
        value_parser = parse_inspect_report_column,
        help = "For table, csv, or tree-table query report output, limit the query report to the selected columns. Use all to expand every supported column."
    )]
    pub report_columns: Vec<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Print the supported --report-columns values and exit."
    )]
    pub list_columns: bool,
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
        help = "Show a progress bar while live dashboards are fetched for the summary."
    )]
    pub progress: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Show extended help with advanced summary examples."
    )]
    pub help_full: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Do not print headers when rendering table, csv, or tree-table summary output."
    )]
    pub no_header: bool,
    #[arg(long, help = "Write summary output to this file.")]
    pub output_file: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        requires = "output_file",
        help = "When --output-file is set, also print summary output to stdout."
    )]
    pub also_stdout: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Open the shared interactive summary workbench over the summarized dashboard set."
    )]
    pub interactive: bool,
}

/// Struct definition for InspectExportArgs.
#[derive(Debug, Clone, Args)]
pub struct InspectExportArgs {
    #[arg(
        long = "input-dir",
        help = "Summarize dashboards from this directory. Use --input-format provisioning for a provisioning/ root or its dashboards/ subdirectory, or --input-format git-sync for a repo-backed dashboard tree."
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
        default_value = "raw",
        value_parser = parse_dashboard_summary_input_format,
        value_name = "raw|provisioning|git-sync",
        help = "Interpret --input-dir as raw export files, Grafana file-provisioning artifacts, or a repo-backed Git Sync dashboard tree. Use git-sync for a Grafana OaC repo root; use provisioning for a provisioning/ root or its dashboards/ subdirectory."
    )]
    pub input_format: DashboardImportInputFormat,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["table", "csv", "json", "yaml", "output_format"],
        help = "Render the export summary as an operator-summary plain-text view."
    )]
    pub text: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["text", "csv", "json", "yaml", "output_format"],
        help = "Render the export summary as an operator-summary table."
    )]
    pub table: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["text", "table", "json", "yaml", "output_format"],
        help = "Render the export summary as operator-summary CSV."
    )]
    pub csv: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["text", "table", "csv", "yaml", "output_format"],
        help = "Render the export summary as the full machine-readable summary contract in JSON."
    )]
    pub json: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["text", "table", "csv", "json", "output_format"],
        help = "Render the export summary as the full machine-readable summary contract in YAML."
    )]
    pub yaml: bool,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["text", "table", "csv", "json", "yaml"],
        help = "Single-flag output selector for dashboard summary output. Use text, table, csv, json, yaml, tree, tree-table, dependency, dependency-json, governance, governance-json, or queries-json."
    )]
    pub output_format: Option<InspectOutputFormat>,
    #[arg(
        long,
        value_delimiter = ',',
        value_parser = parse_inspect_report_column,
        help = "For table, csv, or tree-table query report output, limit the query report to the selected columns. Use all to expand every supported column. Supported values: org, org_id, dashboard_uid, dashboard_title, dashboard_tags, folder_path, folder_full_path, folder_level, folder_uid, parent_folder_uid, panel_id, panel_title, panel_type, panel_target_count, panel_query_count, panel_datasource_count, panel_variables, ref_id, datasource, datasource_name, datasource_uid, datasource_org, datasource_org_id, datasource_database, datasource_bucket, datasource_organization, datasource_index_pattern, datasource_type, datasource_family, query_field, target_hidden, target_disabled, query_variables, metrics, functions, measurements, buckets, query, file. JSON-style aliases like orgId, dashboardUid, dashboardTags, folderFullPath, folderLevel, folderUid, parentFolderUid, panelTargetCount, panelQueryCount, panelDatasourceCount, panelVariables, datasourceName, datasourceUid, datasourceOrg, datasourceOrgId, datasourceDatabase, datasourceBucket, datasourceOrganization, datasourceIndexPattern, datasourceType, datasourceFamily, targetHidden, targetDisabled, and queryVariables are also accepted."
    )]
    pub report_columns: Vec<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Print the supported --report-columns values and exit."
    )]
    pub list_columns: bool,
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
        help = "Show extended help with summary examples for dashboard summary."
    )]
    pub help_full: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Do not print table headers when rendering the table summary, table, csv, or tree-table query report output."
    )]
    pub no_header: bool,
    #[arg(long, help = "Write dashboard summary output to this file.")]
    pub output_file: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        requires = "output_file",
        help = "When --output-file is set, also print dashboard summary output to stdout."
    )]
    pub also_stdout: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Open the shared interactive summary workbench over the export artifacts."
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
        help = "Maximum parallel dashboard fetch workers used for live summary when supported."
    )]
    pub concurrency: usize,
    #[arg(
        long,
        conflicts_with = "all_orgs",
        help = "Summarize dashboards from this Grafana org ID."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "org_id",
        help = "Enumerate all visible Grafana orgs and summarize dashboards across them."
    )]
    pub all_orgs: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["table", "csv", "json", "yaml", "output_format"],
        help = "Render the live dashboard summary as plain text."
    )]
    pub text: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["text", "csv", "json", "yaml", "output_format"],
        help = "Render the live dashboard summary as a table-oriented view."
    )]
    pub table: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["text", "table", "json", "yaml", "output_format"],
        help = "Render the live dashboard summary as CSV."
    )]
    pub csv: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["text", "table", "csv", "yaml", "output_format"],
        help = "Render the live dashboard summary as JSON."
    )]
    pub json: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["text", "table", "csv", "json", "output_format"],
        help = "Render the live dashboard summary as YAML."
    )]
    pub yaml: bool,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["text", "table", "csv", "json", "yaml"],
        help = "Single-flag output selector for dashboard summary output. Use text, table, csv, json, yaml, tree, tree-table, dependency, dependency-json, governance, governance-json, or queries-json."
    )]
    pub output_format: Option<InspectOutputFormat>,
    #[arg(
        long,
        value_delimiter = ',',
        value_parser = parse_inspect_report_column,
        help = "For table, csv, or tree-table query report output, limit the query report to the selected columns. Use all to expand every supported column. Supported values: org, org_id, dashboard_uid, dashboard_title, dashboard_tags, folder_path, folder_full_path, folder_level, folder_uid, parent_folder_uid, panel_id, panel_title, panel_type, panel_target_count, panel_query_count, panel_datasource_count, panel_variables, ref_id, datasource, datasource_name, datasource_uid, datasource_org, datasource_org_id, datasource_database, datasource_bucket, datasource_organization, datasource_index_pattern, datasource_type, datasource_family, query_field, target_hidden, target_disabled, query_variables, metrics, functions, measurements, buckets, query, file. JSON-style aliases like orgId, dashboardUid, dashboardTags, folderFullPath, folderLevel, folderUid, parentFolderUid, panelTargetCount, panelQueryCount, panelDatasourceCount, panelVariables, datasourceName, datasourceUid, datasourceOrg, datasourceOrgId, datasourceDatabase, datasourceBucket, datasourceOrganization, datasourceIndexPattern, datasourceType, datasourceFamily, targetHidden, targetDisabled, and queryVariables are also accepted."
    )]
    pub report_columns: Vec<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Print the supported --report-columns values and exit."
    )]
    pub list_columns: bool,
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
        help = "Show a progress bar while live dashboards are fetched for summary."
    )]
    pub progress: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Show extended help with summary examples for dashboard summary."
    )]
    pub help_full: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Do not print headers when rendering table, csv, or tree-table summary output."
    )]
    pub no_header: bool,
    #[arg(long, help = "Write summary output to this file.")]
    pub output_file: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        requires = "output_file",
        help = "When --output-file is set, also print summary output to stdout."
    )]
    pub also_stdout: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Open the shared interactive summary workbench over the live summary artifacts."
    )]
    pub interactive: bool,
}
