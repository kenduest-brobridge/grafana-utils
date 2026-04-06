//! CLI definitions for dashboard export and raw-to-prompt workflows.

use crate::common::CliColorChoice;
use clap::Args;
use std::path::PathBuf;

use super::super::super::{DEFAULT_EXPORT_DIR, DEFAULT_PAGE_SIZE};
use super::super::cli_defs_shared::{
    CommonCliArgs, RawToPromptLogFormat, RawToPromptOutputFormat, RawToPromptResolution,
};

/// Arguments for exporting dashboards into raw, prompt, and provisioning variants.
#[derive(Debug, Clone, Args)]
pub struct ExportArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long = "output-dir",
        default_value = DEFAULT_EXPORT_DIR,
        help = "Directory to write exported dashboards into. Export writes raw/, prompt/, and provisioning/ subdirectories by default."
    )]
    pub output_dir: PathBuf,
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
        help = "Skip the file-provisioning provisioning/ export variant. Use this only when you do not need Grafana file provisioning artifacts."
    )]
    pub without_dashboard_provisioning: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Write dashboard revision history artifacts under a history/ subdirectory for each exported org scope."
    )]
    pub include_history: bool,
    #[arg(
        long,
        default_value = "grafana-utils-dashboards",
        help = "Set the Grafana provisioning provider name written into provisioning/provisioning/dashboards.yaml."
    )]
    pub provisioning_provider_name: String,
    #[arg(
        long,
        help = "Override the Grafana org ID written into the provisioning provider config. By default the export uses the current org ID."
    )]
    pub provisioning_provider_org_id: Option<i64>,
    #[arg(
        long,
        help = "Override the dashboard directory path written into the provisioning provider config. By default the export points at the current export tree path under provisioning/dashboards."
    )]
    pub provisioning_provider_path: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        help = "Set disableDeletion in the generated provisioning provider config."
    )]
    pub provisioning_provider_disable_deletion: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Set allowUiUpdates in the generated provisioning provider config."
    )]
    pub provisioning_provider_allow_ui_updates: bool,
    #[arg(
        long,
        default_value_t = 30,
        help = "Set updateIntervalSeconds in the generated provisioning provider config."
    )]
    pub provisioning_provider_update_interval_seconds: i64,
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
