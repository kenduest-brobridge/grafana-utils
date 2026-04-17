//! CLI definitions for dashboard export and raw-to-prompt workflows.

use crate::common::CliColorChoice;
use clap::{Args, ValueEnum};
use std::path::PathBuf;

use super::super::super::{DEFAULT_EXPORT_DIR, DEFAULT_PAGE_SIZE};
use super::super::cli_defs_shared::{
    CommonCliArgs, RawToPromptLogFormat, RawToPromptOutputFormat, RawToPromptResolution,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ExportLayoutVariant {
    Raw,
    Prompt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ExportLayoutOutputFormat {
    Text,
    Table,
    Csv,
    Json,
    Yaml,
}

/// Arguments for exporting dashboards into raw, prompt, and provisioning variants.
#[derive(Debug, Clone, Args)]
pub struct ExportArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long = "output-dir",
        default_value = DEFAULT_EXPORT_DIR,
        help = "Directory to write the export tree into.",
        help_heading = "Export Output Options"
    )]
    pub output_dir: PathBuf,
    #[arg(
        long,
        default_value_t = DEFAULT_PAGE_SIZE,
        help = "Dashboard search page size.",
        help_heading = "Selection Options"
    )]
    pub page_size: usize,
    #[arg(
        long,
        conflicts_with = "all_orgs",
        help = "Export dashboards from one explicit Grafana org ID.",
        help_heading = "Selection Options"
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "org_id",
        help = "Export dashboards from every visible Grafana org.",
        help_heading = "Selection Options"
    )]
    pub all_orgs: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Write files directly into each export variant directory.",
        help_heading = "Export Layout Options"
    )]
    pub flat: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Replace existing export files in the target directory.",
        help_heading = "Safety Options"
    )]
    pub overwrite: bool,
    #[arg(
        long = "without-raw",
        alias = "without-dashboard-raw",
        default_value_t = false,
        help = "Skip the raw/ export variant.",
        help_heading = "Export Variant Options"
    )]
    pub without_dashboard_raw: bool,
    #[arg(
        long = "without-prompt",
        alias = "without-dashboard-prompt",
        default_value_t = false,
        help = "Skip the prompt/ export variant.",
        help_heading = "Export Variant Options"
    )]
    pub without_dashboard_prompt: bool,
    #[arg(
        long = "without-provisioning",
        alias = "without-dashboard-provisioning",
        default_value_t = false,
        help = "Skip the provisioning/ export variant.",
        help_heading = "Export Variant Options"
    )]
    pub without_dashboard_provisioning: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Also write history/ artifacts for each exported org scope.",
        help_heading = "Export Variant Options"
    )]
    pub include_history: bool,
    #[arg(
        long = "provider-name",
        value_name = "NAME",
        alias = "provisioning-provider-name",
        default_value = "grafana-utils-dashboards",
        help = "Set the generated provisioning provider name.",
        help_heading = "Provisioning Options"
    )]
    pub provisioning_provider_name: String,
    #[arg(
        long = "provider-org-id",
        value_name = "ORG_ID",
        alias = "provisioning-provider-org-id",
        help = "Override the org ID written into the provisioning config.",
        help_heading = "Provisioning Options"
    )]
    pub provisioning_provider_org_id: Option<i64>,
    #[arg(
        long = "provider-path",
        value_name = "PATH",
        alias = "provisioning-provider-path",
        help = "Override the dashboard path written into the provisioning config.",
        help_heading = "Provisioning Options"
    )]
    pub provisioning_provider_path: Option<PathBuf>,
    #[arg(
        long = "provider-disable-deletion",
        alias = "provisioning-provider-disable-deletion",
        default_value_t = false,
        help = "Set disableDeletion in the provisioning provider config.",
        help_heading = "Provisioning Options"
    )]
    pub provisioning_provider_disable_deletion: bool,
    #[arg(
        long = "provider-allow-ui-updates",
        alias = "provisioning-provider-allow-ui-updates",
        default_value_t = false,
        help = "Set allowUiUpdates in the provisioning provider config.",
        help_heading = "Provisioning Options"
    )]
    pub provisioning_provider_allow_ui_updates: bool,
    #[arg(
        long = "provider-update-interval-seconds",
        value_name = "SECONDS",
        alias = "provisioning-provider-update-interval-seconds",
        default_value_t = 30,
        help = "Set updateIntervalSeconds in the provisioning provider config.",
        help_heading = "Provisioning Options"
    )]
    pub provisioning_provider_update_interval_seconds: i64,
    #[arg(
        long,
        default_value_t = false,
        help = "Preview the files and indexes without writing to disk.",
        help_heading = "Output Options"
    )]
    pub dry_run: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Show concise <current>/<total> export progress.",
        help_heading = "Output Options"
    )]
    pub progress: bool,
    #[arg(
        short = 'v',
        long,
        default_value_t = false,
        help = "Show detailed per-item export output. Overrides --progress.",
        help_heading = "Output Options"
    )]
    pub verbose: bool,
}

/// Arguments for converting raw dashboard exports into prompt-lane artifacts.
#[derive(Debug, Clone, Args)]
pub struct RawToPromptArgs {
    #[arg(
        long = "input-file",
        value_name = "FILE",
        conflicts_with = "input_dir",
        required_unless_present = "input_dir",
        help = "Repeat this flag for each raw dashboard file to convert. When output-file is omitted, the default target is the sibling .prompt.json path."
    )]
    pub input_file: Vec<PathBuf>,
    #[arg(
        long = "input-dir",
        value_name = "DIR",
        conflicts_with = "input_file",
        required_unless_present = "input_file",
        help = "Convert every raw dashboard file in this directory. Point this at a raw export root or its raw/ lane when generating a prompt/ lane."
    )]
    pub input_dir: Option<PathBuf>,
    #[arg(
        long = "output-file",
        value_name = "FILE",
        conflicts_with = "output_dir",
        help = "Write the converted prompt document to this file. For single-file mode, the default is the sibling .prompt.json path."
    )]
    pub output_file: Option<PathBuf>,
    #[arg(
        long = "output-dir",
        value_name = "DIR",
        conflicts_with = "output_file",
        help = "Write converted prompt artifacts into this directory. For raw export roots, the default is the sibling prompt/ lane."
    )]
    pub output_dir: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        help = "Overwrite existing output files instead of failing when the target already exists."
    )]
    pub overwrite: bool,
    #[arg(
        long,
        value_enum,
        default_value_t = RawToPromptOutputFormat::Text,
        help = "Render the command summary as text, table, json, or yaml."
    )]
    pub output_format: RawToPromptOutputFormat,
    #[arg(
        long,
        default_value_t = false,
        help = "Do not print table headers when rendering table output."
    )]
    pub no_header: bool,
    #[arg(
        long,
        value_enum,
        default_value_t = CliColorChoice::Auto,
        help = "Colorize output. Use auto, always, or never."
    )]
    pub color: CliColorChoice,
    #[arg(
        long,
        default_value_t = false,
        help = "Show concise per-item conversion progress while processing files."
    )]
    pub progress: bool,
    #[arg(
        short = 'v',
        long,
        default_value_t = false,
        help = "Show detailed per-item conversion output. Overrides --progress output."
    )]
    pub verbose: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Preview the conversion without writing files."
    )]
    pub dry_run: bool,
    #[arg(
        long = "log-file",
        value_name = "FILE",
        help = "Write structured conversion logs to this file."
    )]
    pub log_file: Option<PathBuf>,
    #[arg(
        long = "log-format",
        value_enum,
        default_value_t = RawToPromptLogFormat::Text,
        help = "Render logs as text or json."
    )]
    pub log_format: RawToPromptLogFormat,
    #[arg(
        long,
        value_enum,
        default_value_t = RawToPromptResolution::InferFamily,
        help = "Choose how datasource references are resolved. Use infer-family, exact, or strict."
    )]
    pub resolution: RawToPromptResolution,
    #[arg(
        long = "datasource-map",
        value_name = "FILE",
        help = "Optional datasource mapping file used while resolving prompt output."
    )]
    pub datasource_map: Option<PathBuf>,
    #[arg(
        long,
        help = "Load live lookup defaults from the selected repo-local profile in grafana-util.yaml. When set, raw-to-prompt can query Grafana datasources to resolve prompt output."
    )]
    pub profile: Option<String>,
    #[arg(
        long,
        help = "Grafana base URL used for optional live datasource lookup."
    )]
    pub url: Option<String>,
    #[arg(
        long = "token",
        visible_alias = "api-token",
        help = "Grafana API token used for optional live datasource lookup."
    )]
    pub api_token: Option<String>,
    #[arg(
        long = "basic-user",
        help = "Grafana Basic auth username used for optional live datasource lookup."
    )]
    pub username: Option<String>,
    #[arg(
        long = "basic-password",
        help = "Grafana Basic auth password used for optional live datasource lookup."
    )]
    pub password: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Prompt for the Grafana Basic auth password for optional live datasource lookup."
    )]
    pub prompt_password: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Prompt for the Grafana API token for optional live datasource lookup."
    )]
    pub prompt_token: bool,
    #[arg(
        long,
        help = "Scope optional live datasource lookup to one explicit Grafana org ID."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        help = "HTTP timeout in seconds for optional live datasource lookup."
    )]
    pub timeout: Option<u64>,
    #[arg(
        long,
        default_value_t = false,
        help = "Enable TLS certificate verification for optional live datasource lookup."
    )]
    pub verify_ssl: bool,
}

/// Arguments for repairing old dashboard export folder layouts.
#[derive(Debug, Clone, Args)]
pub struct ExportLayoutArgs {
    #[arg(
        long = "input-dir",
        value_name = "DIR",
        help = "Existing dashboard export root or variant directory to repair."
    )]
    pub input_dir: PathBuf,
    #[arg(
        long = "output-dir",
        value_name = "DIR",
        required_unless_present = "in_place",
        conflicts_with = "in_place",
        help = "Directory to write the repaired copy into."
    )]
    pub output_dir: Option<PathBuf>,
    #[arg(
        long = "in-place",
        default_value_t = false,
        requires = "backup_dir",
        help = "Repair the input tree in place after backing up changed files."
    )]
    pub in_place: bool,
    #[arg(
        long = "backup-dir",
        value_name = "DIR",
        requires = "in_place",
        help = "Directory used to back up changed files before in-place repair."
    )]
    pub backup_dir: Option<PathBuf>,
    #[arg(
        long = "variant",
        value_enum,
        help = "Repair one export variant. Repeat for raw and prompt; defaults to both."
    )]
    pub variant: Vec<ExportLayoutVariant>,
    #[arg(
        long = "raw-dir",
        value_name = "DIR",
        help = "Raw export lane used as metadata source when repairing prompt-only input."
    )]
    pub raw_dir: Option<PathBuf>,
    #[arg(
        long = "folders-file",
        value_name = "FILE",
        help = "Override the folder inventory file used for repair."
    )]
    pub folders_file: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        help = "Preview the layout repair plan without writing files."
    )]
    pub dry_run: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Replace existing repaired files in the target tree."
    )]
    pub overwrite: bool,
    #[arg(
        long = "show-operations",
        alias = "verbose",
        default_value_t = false,
        help = "Show per-dashboard MOVE, SAME, BLOCKED, and EXTRA operations when rendering text output."
    )]
    pub show_operations: bool,
    #[arg(
        long,
        value_enum,
        default_value_t = ExportLayoutOutputFormat::Text,
        help = "Render the repair plan as text, table, csv, json, or yaml."
    )]
    pub output_format: ExportLayoutOutputFormat,
    #[arg(
        long,
        default_value_t = false,
        help = "Do not print table headers when rendering table output."
    )]
    pub no_header: bool,
    #[arg(
        long,
        value_enum,
        default_value_t = CliColorChoice::Auto,
        help = "Colorize output. Use auto, always, or never."
    )]
    pub color: CliColorChoice,
}
