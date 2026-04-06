//! CLI definitions for Dashboard command surface and option compatibility behavior.

use crate::common::CliColorChoice;
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use super::super::{DEFAULT_EXPORT_DIR, DEFAULT_IMPORT_MESSAGE, DEFAULT_PAGE_SIZE};
use super::cli_defs_inspect::{
    AnalyzeArgs, GovernanceGateArgs, ImpactArgs, InspectExportArgs, InspectLiveArgs, InspectVarsArgs,
    ScreenshotArgs, TopologyArgs, ValidateExportArgs,
};
use super::cli_defs_shared::{
    CommonCliArgs, DryRunOutputFormat, HistoryOutputFormat, RawToPromptLogFormat,
    RawToPromptOutputFormat, RawToPromptResolution, SimpleOutputFormat,
};
use super::dashboard_runtime::{
    parse_dashboard_import_output_column, parse_dashboard_list_output_column,
};

/// Arguments for exporting dashboards into raw, prompt, and provisioning variants.
#[derive(Debug, Clone, Args)]
pub struct ExportArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        default_value = DEFAULT_EXPORT_DIR,
        help = "Directory to write exported dashboards into. Export writes raw/, prompt/, and provisioning/ subdirectories by default."
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

/// Arguments for importing dashboards from a local export directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DashboardImportInputFormat {
    Raw,
    Provisioning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum InspectExportInputType {
    Raw,
    Source,
}

/// Arguments for importing dashboards from a local export directory.
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
        value_enum,
        default_value_t = DashboardImportInputFormat::Raw,
        help = "Interpret --import-dir as raw export files or Grafana file-provisioning artifacts. Use provisioning to accept either the provisioning/ root or its dashboards/ subdirectory."
    )]
    pub input_format: DashboardImportInputFormat,
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
        help = "Fail the import when the export orgId metadata does not match the target Grafana org for this run. This is a safety check for accidental cross-org imports."
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
        help = "Open an interactive review picker to choose which exported dashboards to import from --import-dir and preview each file's create/update/skip action. With --dry-run, Enter runs the dry-run only for the selected dashboards."
    )]
    pub interactive: bool,
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

/// Arguments for patching one local dashboard JSON file in place or to a new path.
#[derive(Debug, Clone, Args)]
pub struct PatchFileArgs {
    #[arg(
        long,
        help = "Input dashboard JSON file to patch. Use - to read one wrapped or bare dashboard JSON document from standard input."
    )]
    pub input: PathBuf,
    #[arg(
        long,
        help = "Write the patched JSON to this path instead of overwriting --input in place."
    )]
    pub output: Option<PathBuf>,
    #[arg(long, help = "Replace dashboard.title with this value.")]
    pub name: Option<String>,
    #[arg(long, help = "Replace dashboard.uid with this value.")]
    pub uid: Option<String>,
    #[arg(
        long = "folder-uid",
        help = "Set meta.folderUid to this value so later publish/import runs target the right Grafana folder."
    )]
    pub folder_uid: Option<String>,
    #[arg(
        long,
        help = "Store a human-readable note in meta.message alongside the patched file."
    )]
    pub message: Option<String>,
    #[arg(
        long = "tag",
        help = "Replace dashboard.tags with these values. Repeat --tag to set multiple tags."
    )]
    pub tags: Vec<String>,
}

/// Arguments for reviewing one local dashboard JSON file without touching Grafana.
#[derive(Debug, Clone, Args)]
pub struct ReviewArgs {
    #[arg(
        long,
        help = "Input dashboard JSON file to review locally. Use - to read one wrapped or bare dashboard JSON document from standard input. Review never contacts Grafana."
    )]
    pub input: PathBuf,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["table", "csv", "yaml", "output_format"],
        help = "Render the review as JSON instead of text."
    )]
    pub json: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["json", "csv", "yaml", "output_format"], help = "Render the review as a table.")]
    pub table: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["json", "table", "yaml", "output_format"], help = "Render the review as CSV.")]
    pub csv: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["json", "table", "csv", "output_format"], help = "Render the review as YAML.")]
    pub yaml: bool,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["json", "table", "csv", "yaml"],
        help = "Alternative single-flag output selector. Use text, table, csv, json, or yaml."
    )]
    pub output_format: Option<SimpleOutputFormat>,
}

/// Arguments for publishing one local dashboard JSON file through the live import pipeline.
#[derive(Debug, Clone, Args)]
pub struct PublishArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help = "Dashboard JSON file to stage and publish. Use - to read one wrapped or bare dashboard JSON document from standard input."
    )]
    pub input: PathBuf,
    #[arg(
        long,
        default_value_t = false,
        help = "Update an existing dashboard when the UID already exists instead of failing on duplicates."
    )]
    pub replace_existing: bool,
    #[arg(
        long = "folder-uid",
        help = "Override the destination Grafana folder UID for this publish."
    )]
    pub folder_uid: Option<String>,
    #[arg(
        long,
        default_value = DEFAULT_IMPORT_MESSAGE,
        help = "Version-history message to attach to the published dashboard revision."
    )]
    pub message: String,
    #[arg(
        long,
        default_value_t = false,
        help = "Preview the publish through the existing import dry-run flow without changing Grafana."
    )]
    pub dry_run: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Watch the local input file and rerun publish or dry-run each time it changes. This only supports file input, not --input -."
    )]
    pub watch: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "For --dry-run only, render a compact table instead of plain text."
    )]
    pub table: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "For --dry-run only, render one JSON document."
    )]
    pub json: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DashboardServeScriptFormat {
    Json,
    Yaml,
}

/// Arguments for serving dashboard drafts through a local preview server.
#[derive(Debug, Clone, Args)]
pub struct ServeArgs {
    #[arg(
        long,
        conflicts_with = "script",
        help = "Load one dashboard draft file or a directory of dashboard draft files into the preview server."
    )]
    pub input: Option<PathBuf>,
    #[arg(
        long,
        conflicts_with = "input",
        help = "Run this local script and treat stdout as one dashboard document or an array of dashboard documents."
    )]
    pub script: Option<String>,
    #[arg(
        long = "script-format",
        value_enum,
        default_value_t = DashboardServeScriptFormat::Json,
        help = "Interpret --script stdout as json or yaml."
    )]
    pub script_format: DashboardServeScriptFormat,
    #[arg(
        long,
        default_value = "127.0.0.1",
        help = "Address for the local preview server to bind."
    )]
    pub address: String,
    #[arg(
        long,
        default_value_t = 8080,
        help = "Port for the local preview server to bind."
    )]
    pub port: u16,
    #[arg(
        long,
        default_value_t = false,
        help = "Do not watch input paths for changes after the initial preview is loaded."
    )]
    pub no_watch: bool,
    #[arg(
        long = "watch",
        help = "Extra local paths to watch for preview reloads. Repeat --watch for multiple paths."
    )]
    pub watch: Vec<PathBuf>,
    #[arg(
        long = "open-browser",
        default_value_t = false,
        help = "Open the preview URL in your default browser after the server starts."
    )]
    pub open_browser: bool,
}

/// Arguments for editing one live dashboard through an external editor.
#[derive(Debug, Clone, Args)]
pub struct EditLiveArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long = "dashboard-uid", help = "Live Grafana dashboard UID to edit.")]
    pub dashboard_uid: String,
    #[arg(
        long,
        help = "Write the edited dashboard draft to this file path instead of using ./<uid>.edited.json."
    )]
    pub output: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        help = "Apply the edited dashboard back to Grafana immediately instead of writing a local draft file."
    )]
    pub apply_live: bool,
    #[arg(
        long,
        default_value = DEFAULT_IMPORT_MESSAGE,
        help = "Revision message to use when --apply-live writes the edited dashboard back to Grafana."
    )]
    pub message: String,
    #[arg(
        long,
        default_value_t = false,
        help = "Acknowledge the live writeback when --apply-live is set."
    )]
    pub yes: bool,
}

/// Arguments for fetching one live dashboard into a local draft file.
#[derive(Debug, Clone, Args)]
pub struct GetArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long = "dashboard-uid", help = "Live Grafana dashboard UID to fetch.")]
    pub dashboard_uid: String,
    #[arg(long, help = "Write the fetched dashboard draft to this file path.")]
    pub output: PathBuf,
}

/// Arguments for cloning one live dashboard into a local draft file.
#[derive(Debug, Clone, Args)]
pub struct CloneLiveArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long = "source-uid", help = "Live Grafana dashboard UID to clone.")]
    pub source_uid: String,
    #[arg(long, help = "Write the cloned dashboard draft to this file path.")]
    pub output: PathBuf,
    #[arg(
        long,
        help = "Override the cloned dashboard title. Defaults to the source title."
    )]
    pub name: Option<String>,
    #[arg(
        long,
        help = "Override the cloned dashboard UID. Defaults to the source UID."
    )]
    pub uid: Option<String>,
    #[arg(
        long = "folder-uid",
        help = "Override the cloned dashboard folder UID in the preserved Grafana metadata."
    )]
    pub folder_uid: Option<String>,
}

/// Arguments for deleting live dashboards by UID or folder path.
#[derive(Debug, Clone, Args)]
pub struct DeleteArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        default_value_t = DEFAULT_PAGE_SIZE,
        help = "Dashboard search page size used to resolve delete selectors."
    )]
    pub page_size: usize,
    #[arg(
        long,
        help = "Delete dashboards from one explicit Grafana org ID instead of the current org. Use this when the same Basic auth credentials can reach multiple orgs."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        help = "Dashboard UID to delete.",
        help_heading = "Target Options"
    )]
    pub uid: Option<String>,
    #[arg(
        long,
        help = "Grafana folder path root to delete recursively, for example 'Platform / Infra'.",
        help_heading = "Target Options"
    )]
    pub path: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "With --path, also delete matched Grafana folders after deleting dashboards in the subtree.",
        help_heading = "Target Options"
    )]
    pub delete_folders: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Acknowledge the live dashboard delete. Required unless --dry-run or --interactive is set.",
        help_heading = "Safety Options"
    )]
    pub yes: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Prompt for the delete selector, preview the delete plan, and confirm interactively.",
        help_heading = "Safety Options"
    )]
    pub interactive: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Preview what dashboard delete would do without changing Grafana.",
        help_heading = "Output Options"
    )]
    pub dry_run: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "For --dry-run only, render a compact table instead of plain text.",
        help_heading = "Output Options"
    )]
    pub table: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "For --dry-run only, render one JSON document.",
        help_heading = "Output Options"
    )]
    pub json: bool,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["table", "json"],
        help = "Alternative single-flag output selector for dashboard delete dry-run output. Use text, table, or json.",
        help_heading = "Output Options"
    )]
    pub output_format: Option<DryRunOutputFormat>,
    #[arg(
        long,
        default_value_t = false,
        help = "For --dry-run --table only, omit the table header row.",
        help_heading = "Output Options"
    )]
    pub no_header: bool,
}

/// Arguments for browsing the live dashboard tree in a TUI.
#[derive(Debug, Clone, Args)]
pub struct BrowseArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        default_value_t = DEFAULT_PAGE_SIZE,
        help = "Dashboard search page size used to build the live browser tree."
    )]
    pub page_size: usize,
    #[arg(
        long,
        conflicts_with = "all_orgs",
        help = "Browse dashboards from one explicit Grafana org ID instead of the current org."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "org_id",
        help = "Enumerate all visible Grafana orgs and browse the dashboard tree across them. Prefer Basic auth when you need cross-org browse because API tokens are often scoped to one org."
    )]
    pub all_orgs: bool,
    #[arg(
        long,
        help = "Optional folder path root to open instead of the full dashboard tree, for example 'Platform / Infra'."
    )]
    pub path: Option<String>,
}

/// Struct definition for DiffArgs.
#[derive(Debug, Clone, Args)]
pub struct DiffArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help = "Compare dashboards from this directory against Grafana. Point this to the raw/ export directory explicitly, or use with --input-format provisioning for a provisioning root or its dashboards/ subdirectory."
    )]
    pub import_dir: PathBuf,
    #[arg(
        long,
        value_enum,
        default_value_t = DashboardImportInputFormat::Raw,
        help = "Interpret --import-dir as raw export files or Grafana file-provisioning artifacts. Use provisioning to accept either the provisioning/ root or its dashboards/ subdirectory."
    )]
    pub input_format: DashboardImportInputFormat,
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

/// Arguments for dashboard history list.
#[derive(Debug, Clone, Args)]
pub struct HistoryListArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help = "Dashboard UID to inspect. Required for live Grafana history, optional when filtering a local export tree, and optional validation when reading one local history artifact."
    )]
    pub dashboard_uid: Option<String>,
    #[arg(
        long,
        value_name = "FILE",
        conflicts_with = "import_dir",
        help = "Read one local history artifact JSON produced by `dashboard history export` instead of calling Grafana."
    )]
    pub input: Option<PathBuf>,
    #[arg(
        long,
        value_name = "DIR",
        conflicts_with = "input",
        help = "Read history artifacts from a dashboard export root produced by `dashboard export --include-history` instead of calling Grafana."
    )]
    pub import_dir: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = 20,
        help = "Maximum number of recent versions to request from Grafana in live mode."
    )]
    pub limit: usize,
    #[arg(
        long,
        value_enum,
        default_value_t = HistoryOutputFormat::Table,
        help = "Render history as text, table, json, or yaml."
    )]
    pub output_format: HistoryOutputFormat,
}

/// Arguments for dashboard history restore.
#[derive(Debug, Clone, Args)]
pub struct HistoryRestoreArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long, help = "Dashboard UID to restore from Grafana history.")]
    pub dashboard_uid: String,
    #[arg(long, help = "Dashboard history version number to restore.")]
    pub version: i64,
    #[arg(
        long,
        default_value_t = false,
        help = "Preview the restore without writing a new Grafana revision."
    )]
    pub dry_run: bool,
    #[arg(
        long,
        value_enum,
        default_value_t = HistoryOutputFormat::Text,
        help = "Render restore preview or result as text, table, json, or yaml."
    )]
    pub output_format: HistoryOutputFormat,
    #[arg(
        long,
        help = "Revision message to attach to the new Grafana revision. Default: 'Restored by grafana-util dashboard history to version <n>'."
    )]
    pub message: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Confirm the live restore. Required unless --dry-run is set."
    )]
    pub yes: bool,
}

/// Arguments for exporting dashboard history into a reusable JSON artifact.
#[derive(Debug, Clone, Args)]
pub struct HistoryExportArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long, help = "Dashboard UID to export from Grafana history.")]
    pub dashboard_uid: String,
    #[arg(
        long,
        value_name = "FILE",
        help = "Write the exported dashboard history artifact to this JSON file."
    )]
    pub output: PathBuf,
    #[arg(
        long,
        default_value_t = 20,
        help = "Maximum number of recent versions to include in the exported history artifact."
    )]
    pub limit: usize,
    #[arg(
        long,
        default_value_t = false,
        help = "Overwrite an existing history artifact file."
    )]
    pub overwrite: bool,
}

/// Dashboard history subcommands.
#[derive(Debug, Clone, Subcommand)]
pub enum DashboardHistorySubcommand {
    #[command(
        name = "list",
        about = "List live dashboard revision history or review local history artifacts.",
        after_help = "Examples:\n\n  List the last 20 live versions as a table:\n    grafana-util dashboard history list --url http://localhost:3000 --basic-user admin --basic-password admin --dashboard-uid cpu-main --limit 20 --output-format table\n\n  Review one saved history artifact without calling Grafana:\n    grafana-util dashboard history list --input ./cpu-main.history.json --output-format yaml\n\n  Scan one export tree created with --include-history:\n    grafana-util dashboard history list --import-dir ./dashboards --dashboard-uid cpu-main --output-format json"
    )]
    List(HistoryListArgs),
    #[command(
        name = "restore",
        about = "Restore one historical dashboard version as a new latest revision entry on the same dashboard.",
        after_help = "Examples:\n\n  Preview a restore without changing Grafana:\n    grafana-util dashboard history restore --url http://localhost:3000 --basic-user admin --basic-password admin --dashboard-uid cpu-main --version 17 --dry-run --output-format table\n\n  Restore a historical version and record a new revision message:\n    grafana-util dashboard history restore --url http://localhost:3000 --basic-user admin --basic-password admin --dashboard-uid cpu-main --version 17 --message 'Restore known good CPU dashboard after regression' --yes"
    )]
    Restore(HistoryRestoreArgs),
    #[command(
        name = "export",
        about = "Export dashboard revision history into a reusable JSON artifact.",
        after_help = "Examples:\n\n  Export the last 20 revisions to a JSON artifact:\n    grafana-util dashboard history export --url http://localhost:3000 --basic-user admin --basic-password admin --dashboard-uid cpu-main --output ./cpu-main.history.json\n\n  Overwrite an existing history artifact and raise the export limit:\n    grafana-util dashboard history export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --dashboard-uid cpu-main --limit 50 --output ./cpu-main.history.json --overwrite"
    )]
    Export(HistoryExportArgs),
}

/// Dashboard history namespace arguments.
#[derive(Debug, Clone, Args)]
pub struct DashboardHistoryArgs {
    #[command(subcommand)]
    pub command: DashboardHistorySubcommand,
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
        name = "fetch-live",
        about = "Fetch one live dashboard into an API-safe local JSON draft.",
        after_help = "What it does:\n  Fetch one live dashboard and write an API-safe local draft file without mutating Grafana.\n\nWhen to use:\n  - Start a local edit or review flow from the current live dashboard.\n  - Capture one dashboard before patching, diffing, or publishing locally.\n\nRelated commands:\n  - dashboard clone-live  Fetch then override title, UID, or folder metadata.\n  - dashboard review      Inspect one local draft before publish.\n  - dashboard publish     Send one reviewed local draft back to Grafana.\n\nExamples:\n\n  Fetch one live dashboard and write a local draft file:\n    grafana-util dashboard fetch-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --dashboard-uid cpu-main --output ./cpu-main.json\n\n  Fetch one dashboard with Basic auth and a saved profile:\n    grafana-util dashboard fetch-live --profile prod --url http://localhost:3000 --basic-user admin --basic-password admin --dashboard-uid cpu-main --output ./cpu-main.json"
    )]
    Get(GetArgs),
    #[command(
        name = "clone-live",
        about = "Clone one live dashboard into a local draft with optional overrides.",
        after_help = "What it does:\n  Fetch one live dashboard into a local draft and optionally override title, UID, or folder metadata before saving it.\n\nWhen to use:\n  - Fork a live dashboard into a new draft for another folder, environment, or owner.\n  - Prepare a publishable variant without mutating the source dashboard first.\n\nRelated commands:\n  - dashboard fetch-live  Fetch the live dashboard without changing any metadata.\n  - dashboard patch-file  Adjust the local draft after the initial clone step.\n  - dashboard publish     Push the reviewed clone into Grafana.\n\nExamples:\n\n  Clone one live dashboard, keep the source UID and title, and write a local draft:\n    grafana-util dashboard clone-live --url http://localhost:3000 --basic-user admin --basic-password admin --source-uid cpu-main --output ./cpu-main-clone.json\n\n  Clone a live dashboard with a new title, UID, and folder UID:\n    grafana-util dashboard clone-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --source-uid cpu-main --name 'CPU Clone' --uid cpu-main-clone --folder-uid infra --output ./cpu-main-clone.json"
    )]
    CloneLive(CloneLiveArgs),
    #[command(
        name = "serve",
        about = "Serve dashboard drafts through a local preview server.",
        after_help = "Examples:\n\n  Serve one local draft file and open the browser:\n    grafana-util dashboard serve --input ./drafts/cpu-main.json --open-browser\n\n  Serve a directory of dashboard drafts:\n    grafana-util dashboard serve --input ./dashboards/raw\n\n  Serve one generated dashboard and watch the generator inputs:\n    grafana-util dashboard serve --script 'jsonnet dashboards/cpu.jsonnet' --watch ./dashboards --watch ./lib"
    )]
    Serve(ServeArgs),
    #[command(
        name = "edit-live",
        about = "Fetch one live dashboard into an external editor, review the edited draft, and save the result as a local draft or explicit live writeback.",
        after_help = "Examples:\n\n  Edit one live dashboard and write the result to the default local draft path:\n    grafana-util dashboard edit-live --url http://localhost:3000 --basic-user admin --basic-password admin --dashboard-uid cpu-main\n\n  Edit one live dashboard into an explicit output file:\n    grafana-util dashboard edit-live --profile prod --dashboard-uid cpu-main --output ./drafts/cpu-main.edited.json\n\n  Edit one live dashboard and write it back to Grafana after explicit acknowledgement:\n    grafana-util dashboard edit-live --profile prod --dashboard-uid cpu-main --apply-live --yes --message 'Hotfix CPU dashboard'\n\n  Edit one live dashboard and inspect the review output before deciding whether to publish:\n    grafana-util dashboard edit-live --profile prod --dashboard-uid cpu-main"
    )]
    EditLive(EditLiveArgs),
    #[command(
        name = "export",
        about = "Export dashboards to raw/, prompt/, provisioning/, and optional history/ files.",
        after_help = "The provisioning export writes a Grafana file-provisioning provider file at provisioning/provisioning/dashboards.yaml. Override the provider name, org ID, path, or update behavior when you need a different on-disk deployment target. Add --include-history when you also want per-dashboard revision history under history/ for each exported scope.\n\nExamples:\n\n  Export dashboards from the current org with Basic auth and history artifacts:\n    grafana-util export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./dashboards --overwrite --include-history\n\n  Export dashboards across all visible orgs with Basic auth and history artifacts:\n    grafana-util export --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --export-dir ./dashboards --overwrite --include-history\n\n  Export dashboards with a custom provisioning provider path:\n    grafana-util export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./dashboards --overwrite --provisioning-provider-name grafana-utils-prod --provisioning-provider-org-id 2 --provisioning-provider-path /srv/grafana/dashboards --provisioning-provider-disable-deletion --provisioning-provider-update-interval-seconds 60\n\n  Export dashboards from one explicit org ID:\n    grafana-util export --url http://localhost:3000 --basic-user admin --basic-password admin --org-id 2 --export-dir ./dashboards --overwrite\n\n  Export dashboards from the current org with an API token:\n    export GRAFANA_API_TOKEN='your-token'\n    grafana-util export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --export-dir ./dashboards --overwrite"
    )]
    Export(ExportArgs),
    #[command(
        name = "raw-to-prompt",
        about = "Convert raw dashboard exports into prompt lane artifacts.",
        after_help = "Examples:\n\n  Convert one raw dashboard file and rely on the sibling .prompt.json target:\n    grafana-util dashboard raw-to-prompt --input-file ./dashboards/raw/cpu-main.json\n\n  Convert one raw export root into a sibling prompt/ lane:\n    grafana-util dashboard raw-to-prompt --input-dir ./dashboards/raw --output-dir ./dashboards/prompt --overwrite\n\n  Convert a raw file with explicit datasource resolution settings:\n    grafana-util dashboard raw-to-prompt --input-file ./dashboards/raw/cpu-main.json --datasource-map ./datasource-map.json --resolution exact --output-format json\n\n  Augment datasource resolution with live lookup from a profile:\n    grafana-util dashboard raw-to-prompt --input-file ./dashboards/raw/cpu-main.json --profile prod --org-id 2"
    )]
    RawToPrompt(RawToPromptArgs),
    #[command(
        name = "import",
        about = "Import dashboard JSON files through the Grafana API.",
        after_help = "Examples:\n\n  Import one raw export directory into the current org:\n    grafana-util dashboard import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards/raw --replace-existing\n\n  Preview import actions without changing Grafana:\n    grafana-util dashboard import --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --import-dir ./dashboards/raw --dry-run --table\n\n  Interactively choose exported dashboards to restore/import:\n    grafana-util dashboard import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards/raw --interactive --replace-existing"
    )]
    Import(ImportArgs),
    #[command(
        name = "browse",
        about = "Browse the live dashboard tree in an interactive terminal UI.",
        after_help = "Examples:\n\n  Browse the full dashboard tree from the current org:\n    grafana-util dashboard browse --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\"\n\n  Open the browser at one folder subtree:\n    grafana-util dashboard browse --url http://localhost:3000 --basic-user admin --basic-password admin --path 'Platform / Infra'\n\n  Browse one explicit org:\n    grafana-util dashboard browse --url http://localhost:3000 --basic-user admin --basic-password admin --org-id 2\n\n  Browse all visible orgs with Basic auth:\n    grafana-util dashboard browse --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs"
    )]
    Browse(BrowseArgs),
    #[command(
        name = "history",
        about = "List or restore dashboard revision history from Grafana."
    )]
    History(DashboardHistoryArgs),
    #[command(
        name = "delete",
        about = "Delete live dashboards by UID or folder path.",
        after_help = "Examples:\n\n  Dry-run one dashboard delete by UID:\n    grafana-util dashboard delete --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --uid cpu-main --dry-run --json\n\n  Delete all dashboards under one folder subtree:\n    grafana-util dashboard delete --url http://localhost:3000 --basic-user admin --basic-password admin --path 'Platform / Infra' --yes\n\n  Interactively preview and confirm a folder delete:\n    grafana-util dashboard delete --url http://localhost:3000 --interactive"
    )]
    Delete(DeleteArgs),
    #[command(
        about = "Compare local dashboard files against live Grafana dashboards.",
        after_help = "Examples:\n\n  Compare one raw export directory against the current org:\n    grafana-util dashboard diff --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards/raw\n\n  Compare a provisioning export root against the current org:\n    grafana-util dashboard diff --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards/provisioning --input-format provisioning\n\n  Compare against one explicit org as structured JSON:\n    grafana-util dashboard diff --url http://localhost:3000 --basic-user admin --basic-password admin --org-id 2 --import-dir ./dashboards/raw --json"
    )]
    Diff(DiffArgs),
    #[command(
        name = "patch-file",
        about = "Patch one local dashboard JSON file in place or to a new path.",
        after_help = "Examples:\n\n  Patch a raw export file in place:\n    grafana-util dashboard patch-file --input ./dashboards/raw/cpu-main.json --name 'CPU Overview' --folder-uid infra --tag prod --tag sre\n\n  Patch one draft file into a new output path:\n    grafana-util dashboard patch-file --input ./drafts/cpu-main.json --output ./drafts/cpu-main-patched.json --uid cpu-main --message 'Add folder metadata before publish'\n\n  Patch one dashboard from standard input into an explicit output file:\n    jsonnet dashboards/cpu.jsonnet | grafana-util dashboard patch-file --input - --output ./drafts/cpu-main.json --folder-uid infra"
    )]
    PatchFile(PatchFileArgs),
    #[command(
        name = "review",
        about = "Review one local dashboard JSON file without touching Grafana.",
        after_help = "What it does:\n  Review one local dashboard draft without touching Grafana and render the draft in text, YAML, or JSON form.\n\nWhen to use:\n  - Check a generated or edited draft before publish.\n  - Confirm folder, tags, UID, panels, and datasource references in CI or local review.\n\nRelated commands:\n  - dashboard fetch-live  Fetch a live dashboard into a local draft first.\n  - dashboard patch-file  Adjust the local draft before review.\n  - dashboard publish     Send the reviewed draft to Grafana.\n\nExamples:\n\n  Review one local dashboard file in text mode:\n    grafana-util dashboard review --input ./drafts/cpu-main.json\n\n  Review one local dashboard file as YAML:\n    grafana-util dashboard review --input ./drafts/cpu-main.json --output-format yaml\n\n  Review one generated dashboard from standard input:\n    jsonnet dashboards/cpu.jsonnet | grafana-util dashboard review --input - --output-format json"
    )]
    Review(ReviewArgs),
    #[command(
        name = "publish",
        about = "Publish one local dashboard JSON file through the existing dashboard import pipeline.",
        after_help = "What it does:\n  Publish one local dashboard draft through the import pipeline, with dry-run support before any live write.\n\nWhen to use:\n  - Promote a reviewed draft back into Grafana.\n  - Reuse the same import semantics for one-off dashboard edits or generated drafts.\n\nRelated commands:\n  - dashboard review      Inspect the local draft before publish.\n  - dashboard fetch-live  Start from the current live dashboard state.\n  - dashboard clone-live  Prepare a new variant before publish.\n\nExamples:\n\n  Publish one draft file to the current Grafana org:\n    grafana-util dashboard publish --url http://localhost:3000 --basic-user admin --basic-password admin --input ./drafts/cpu-main.json --folder-uid infra --message 'Promote CPU dashboard'\n\n  Preview the same publish without writing to Grafana:\n    grafana-util dashboard publish --url http://localhost:3000 --basic-user admin --basic-password admin --input ./drafts/cpu-main.json --dry-run --table\n\n  Publish one generated dashboard from standard input:\n    jsonnet dashboards/cpu.jsonnet | grafana-util dashboard publish --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --input - --replace-existing\n\n  Watch one local draft file and rerun dry-run after each save:\n    grafana-util dashboard publish --url http://localhost:3000 --basic-user admin --basic-password admin --input ./drafts/cpu-main.json --dry-run --watch"
    )]
    Publish(PublishArgs),
    #[command(
        name = "analyze",
        about = "Analyze dashboards from live Grafana or a local export tree and build summary or governance artifacts.",
        after_help = "What it does:\n  Analyze dashboards from live Grafana or a local export tree and render summary, governance, dependency, or queries-json outputs.\n\nWhen to use:\n  - Inspect a live environment before topology, governance-gate, or impact checks.\n  - Reuse a local export tree in CI without calling Grafana again.\n\nRelated commands:\n  - dashboard topology         Show which dashboards, variables, data sources, and alerts depend on each other.\n  - dashboard governance-gate  Check dashboard findings against a policy.\n  - dashboard list-vars        List one dashboard's current variables only.\n\nExamples:\n\n  Analyze live Grafana and render governance JSON:\n    grafana-util dashboard analyze --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format governance-json\n\n  Analyze a raw export tree without calling Grafana:\n    grafana-util dashboard analyze --import-dir ./dashboards/raw --input-format raw --output-format tree-table\n\n  Analyze a provisioning export tree:\n    grafana-util dashboard analyze --import-dir ./dashboards/provisioning --input-format provisioning --output-format governance\n"
    )]
    Analyze(AnalyzeArgs),
    #[command(
        name = "analyze-export",
        hide = true,
        alias = "inspect-export",
        about = "Analyze dashboard export directories with operator-summary, governance, dependency, and queries-json views.",
        after_help = "Examples:\n\n  Render an operator-summary table from raw exports:\n    grafana-util dashboard analyze-export --import-dir ./dashboards/raw --input-format raw --output-format table\n\n  Open the interactive inspect workbench over raw exports:\n    grafana-util dashboard analyze-export --import-dir ./dashboards/raw --input-format raw --interactive\n\n  Render the machine-readable governance artifact from raw exports:\n    grafana-util dashboard analyze-export --import-dir ./dashboards/raw --input-format raw --output-format governance-json\n\n  Render the queries-json artifact from raw exports:\n    grafana-util dashboard analyze-export --import-dir ./dashboards/raw --input-format raw --output-format queries-json\n\n  Inspect a file-provisioning tree from the provisioning root:\n    grafana-util dashboard analyze-export --import-dir ./dashboards/provisioning --input-format provisioning --output-format tree-table"
    )]
    InspectExport(InspectExportArgs),
    #[command(
        name = "analyze-live",
        hide = true,
        alias = "inspect-live",
        about = "Analyze live Grafana dashboards via a temporary raw-export snapshot.",
        after_help = "Examples:\n\n  Render governance JSON from live Grafana:\n    grafana-util dashboard analyze-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format governance-json\n\n  Render the queries-json artifact from live Grafana:\n    grafana-util dashboard analyze-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format queries-json\n\n  Open the interactive inspect workbench over live Grafana:\n    grafana-util dashboard analyze-live --url http://localhost:3000 --basic-user admin --basic-password admin --interactive"
    )]
    InspectLive(InspectLiveArgs),
    #[command(
        name = "list-vars",
        alias = "inspect-vars",
        about = "List dashboard templating variables and datasource-like choices from live Grafana.",
        after_help = "Examples:\n\n  List variables from a browser URL directly:\n    grafana-util dashboard list-vars --dashboard-url 'https://grafana.example.com/d/cpu-main/cpu-overview?var-cluster=prod-a' --token \"$GRAFANA_API_TOKEN\" --output-format table\n\n  List one dashboard UID with a vars-query fragment:\n    grafana-util dashboard list-vars --url https://grafana.example.com --dashboard-uid cpu-main --vars-query 'var-cluster=prod-a&var-instance=node01' --token \"$GRAFANA_API_TOKEN\" --output-format json\n\n  Render the same variable inventory as YAML:\n    grafana-util dashboard list-vars --url https://grafana.example.com --dashboard-uid cpu-main --token \"$GRAFANA_API_TOKEN\" --output-format yaml"
    )]
    InspectVars(InspectVarsArgs),
    #[command(
        name = "governance-gate",
        about = "Check dashboard query and governance findings against a policy from live Grafana, an export tree, or saved artifacts.",
        after_help = "Examples:\n\n  Check live Grafana directly with a JSON/YAML policy file:\n    grafana-util dashboard governance-gate --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --policy-source file --policy ./policy.yaml\n\n  Check an export tree without calling Grafana:\n    grafana-util dashboard governance-gate --policy-source builtin --builtin-policy default --import-dir ./dashboards/raw --input-format raw\n\n  Reuse saved analysis artifacts and write normalized JSON:\n    grafana-util dashboard governance-gate --policy-source builtin --builtin-policy default --governance ./governance.json --queries ./queries.json --output-format json --json-output ./governance-check.json"
    )]
    GovernanceGate(GovernanceGateArgs),
    #[command(
        name = "topology",
        visible_alias = "graph",
        about = "Show which dashboards, variables, data sources, and alerts depend on each other.",
        after_help = "Examples:\n\n  Analyze live Grafana directly and render Mermaid:\n    grafana-util dashboard topology --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format mermaid\n\n  Analyze an export tree without calling Grafana:\n    grafana-util dashboard topology --import-dir ./dashboards/raw --input-format raw --output-format text\n\n  Reuse saved artifacts and write Graphviz DOT through the graph alias:\n    grafana-util dashboard graph --governance ./governance.json --queries ./queries.json --alert-contract ./alert-contract.json --output-format dot --output-file ./dashboard-topology.dot"
    )]
    Topology(TopologyArgs),
    #[command(
        name = "impact",
        about = "Show which dashboards and alert resources would be affected by one data source from live Grafana, an export tree, or saved artifacts.",
        after_help = "Examples:\n\n  Check blast radius directly from live Grafana:\n    grafana-util dashboard impact --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --datasource-uid prom-main --output-format text\n\n  Check blast radius from an export tree:\n    grafana-util dashboard impact --import-dir ./dashboards/raw --input-format raw --datasource-uid prom-main --output-format json\n\n  Reuse saved artifacts and add alert contract context:\n    grafana-util dashboard impact --governance ./governance.json --queries ./queries.json --datasource-uid prom-main --alert-contract ./alert-contract.json --output-format json"
    )]
    Impact(ImpactArgs),
    #[command(
        name = "validate-export",
        about = "Run strict schema validation against dashboard raw export files before GitOps sync.",
        after_help = "Examples:\n\n  Validate a raw export and fail on migration or plugin issues:\n    grafana-util dashboard validate-export --import-dir ./dashboards/raw --reject-custom-plugins --reject-legacy-properties --target-schema-version 39\n\n  Validate a provisioning export root explicitly:\n    grafana-util dashboard validate-export --import-dir ./dashboards/provisioning --input-format provisioning --reject-custom-plugins\n\n  Write the validation report as JSON:\n    grafana-util dashboard validate-export --import-dir ./dashboards/raw --output-format json --output-file ./dashboard-validation.json"
    )]
    ValidateExport(ValidateExportArgs),
    #[command(
        name = "screenshot",
        about = "Open one Grafana dashboard in a headless browser and capture PNG, JPEG, or PDF output.",
        after_help = "Examples:\n\n  Capture a full dashboard from a browser URL and add an auto title/header block:\n    grafana-util dashboard screenshot --dashboard-url 'https://grafana.example.com/d/cpu-main/cpu-overview?var-cluster=prod-a' --token \"$GRAFANA_API_TOKEN\" --output ./cpu-main.png --full-page --header-title --header-url --header-captured-at\n\n  Capture a solo panel with a vars-query fragment and custom header note:\n    grafana-util dashboard screenshot --url https://grafana.example.com --dashboard-uid rYdddlPWk --panel-id 20 --vars-query 'var-datasource=prom-main&var-job=node-exporter&var-node=host01:9100' --token \"$GRAFANA_API_TOKEN\" --output ./panel.png --header-title 'CPU Busy' --header-text 'Solo panel debug capture'"
    )]
    Screenshot(ScreenshotArgs),
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn dashboard_history_restore_help_mentions_same_dashboard() {
        let command = DashboardCliArgs::command();
        let history = command.find_subcommand("history").unwrap();
        let restore = history.find_subcommand("restore").unwrap();
        let about = restore.get_about().unwrap().to_string();
        assert!(about.contains("same dashboard"));
    }
}

#[derive(Debug, Clone, Parser)]
#[command(
    about = "Export or import Grafana dashboards.",
    after_help = "Examples:\n\n  Fetch one live dashboard into a local draft:\n    grafana-util dashboard fetch-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --dashboard-uid cpu-main --output ./cpu-main.json\n\n  Clone one live dashboard with a new UID and folder:\n    grafana-util dashboard clone-live --url http://localhost:3000 --basic-user admin --basic-password admin --source-uid cpu-main --uid cpu-main-clone --folder-uid infra --output ./cpu-main-clone.json\n\n  Export dashboards from local Grafana with Basic auth:\n    grafana-util export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./dashboards --overwrite\n\n  Export dashboards across all visible orgs with Basic auth:\n    grafana-util export --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --export-dir ./dashboards --overwrite\n\n  List dashboards across all visible orgs with Basic auth:\n    grafana-util list --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --json\n\n  Export dashboards with an API token from the current org:\n    export GRAFANA_API_TOKEN='your-token'\n    grafana-util export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --export-dir ./dashboards --overwrite\n\n  Compare raw dashboard exports against local Grafana:\n    grafana-util diff --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards/raw\n\n  Patch a local dashboard file before publishing:\n    grafana-util patch-file --input ./dashboards/raw/cpu-main.json --name 'CPU Overview' --folder-uid infra --tag prod --tag sre\n\n  Publish one local draft to Grafana:\n    grafana-util publish --url http://localhost:3000 --basic-user admin --basic-password admin --input ./drafts/cpu-main.json --dry-run --table\n\n  Capture a browser-rendered dashboard screenshot:\n    grafana-util screenshot --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --dashboard-uid cpu-main --output ./cpu-main.png --from now-6h --to now",
    styles = crate::help_styles::CLI_HELP_STYLES
)]
/// Struct definition for DashboardCliArgs.
pub struct DashboardCliArgs {
    #[arg(
        long,
        value_enum,
        default_value_t = CliColorChoice::Auto,
        help = "Colorize JSON output. Use auto, always, or never."
    )]
    pub color: CliColorChoice,
    #[command(subcommand)]
    pub command: DashboardCommand,
}
