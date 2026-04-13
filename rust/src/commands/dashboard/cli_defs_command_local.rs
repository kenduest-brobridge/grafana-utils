//! CLI definitions for dashboard local-edit and publish workflows.

use clap::Args;
use std::path::PathBuf;

use super::super::super::DEFAULT_IMPORT_MESSAGE;
use super::super::cli_defs_shared::{CommonCliArgs, DryRunOutputFormat, SimpleOutputFormat};

/// Arguments for importing dashboards from a local export directory.
#[derive(Debug, Clone, Args)]
pub struct ImportArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        conflicts_with = "use_export_org",
        help = "Import dashboards into this Grafana org ID instead of the current org. This switches the whole import run to one explicit destination org and requires Basic auth.",
        help_heading = "Routing Options"
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "require_matching_export_org",
        help = "Import a combined multi-org export root by routing each org-specific raw export back into the matching Grafana org. This requires Basic auth.",
        help_heading = "Routing Options"
    )]
    pub use_export_org: bool,
    #[arg(
        long = "only-org-id",
        requires = "use_export_org",
        conflicts_with = "org_id",
        help = "With --use-export-org, import only these exported source org IDs. Repeat the flag to select multiple orgs.",
        help_heading = "Routing Options"
    )]
    pub only_org_id: Vec<i64>,
    #[arg(
        long,
        default_value_t = false,
        requires = "use_export_org",
        help = "With --use-export-org, create a missing destination org when an exported source org ID does not exist in Grafana. The new org is created from the exported org name and then used as the import target.",
        help_heading = "Routing Options"
    )]
    pub create_missing_orgs: bool,
    #[arg(
        long = "input-dir",
        help = "Import dashboards from this directory. Use the raw/ export directory for single-org import, or the combined export root when --use-export-org is enabled.",
        help_heading = "Input Options"
    )]
    pub input_dir: PathBuf,
    #[arg(
        long,
        value_enum,
        default_value_t = super::DashboardImportInputFormat::Raw,
        help = "Interpret --input-dir as raw export files or Grafana file-provisioning artifacts. Use provisioning to accept either the provisioning/ root or its dashboards/ subdirectory.",
        help_heading = "Input Options"
    )]
    pub input_format: super::DashboardImportInputFormat,
    #[arg(
        long,
        help = "Force every imported dashboard into one destination Grafana folder UID. This overrides any folder UID carried by the exported dashboard files.",
        help_heading = "Folder Options"
    )]
    pub import_folder_uid: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Use the exported raw folder inventory to create any missing destination folders before import. In dry-run mode, also report folder missing/match/mismatch state first.",
        help_heading = "Folder Options"
    )]
    pub ensure_folders: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Update an existing destination dashboard when the imported dashboard UID already exists. Without this flag, existing UIDs are blocked.",
        help_heading = "Safety Options"
    )]
    pub replace_existing: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Reconcile only dashboards whose UID already exists in Grafana. Missing destination UIDs are skipped instead of created.",
        help_heading = "Safety Options"
    )]
    pub update_existing_only: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Only update an existing dashboard when the source raw folder path matches the destination Grafana folder path exactly. Missing dashboards still follow the active create/skip mode.",
        help_heading = "Safety Options"
    )]
    pub require_matching_folder_path: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Fail the import when the export orgId metadata does not match the target Grafana org for this run. This is a safety check for accidental cross-org imports.",
        help_heading = "Safety Options"
    )]
    pub require_matching_export_org: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Enable strict dashboard schema validation before import. This rejects unsupported custom plugins, legacy layout shapes, and other preflight issues before any live write.",
        help_heading = "Validation Options"
    )]
    pub strict_schema: bool,
    #[arg(
        long,
        requires = "strict_schema",
        help = "Optional target dashboard schemaVersion required by strict validation. Dashboards below this version are blocked as migration-required.",
        help_heading = "Validation Options"
    )]
    pub target_schema_version: Option<i64>,
    #[arg(
        long,
        default_value = DEFAULT_IMPORT_MESSAGE,
        help = "Version-history message to attach to each imported dashboard revision in Grafana.",
        help_heading = "Validation Options"
    )]
    pub import_message: String,
    #[arg(
        long,
        default_value_t = false,
        help = "Open an interactive review picker to choose which exported dashboards to import from --input-dir and preview each file's create/update/skip action. With --dry-run, Enter runs the dry-run only for the selected dashboards.",
        help_heading = "Review Output Options"
    )]
    pub interactive: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Preview what import would do without changing Grafana. This reports whether each dashboard would create, update, or be skipped/blocked.",
        help_heading = "Review Output Options"
    )]
    pub dry_run: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "For --dry-run only, render a compact table instead of per-dashboard log lines. With --ensure-folders, the folder check is also shown in table form.",
        help_heading = "Review Output Options"
    )]
    pub table: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "For --dry-run only, render one JSON document with mode, folder checks, dashboard actions, and summary counts.",
        help_heading = "Review Output Options"
    )]
    pub json: bool,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["table", "json"],
        help = "Alternative single-flag output selector for --dry-run output. Use text, table, or json.",
        help_heading = "Review Output Options"
    )]
    pub output_format: Option<DryRunOutputFormat>,
    #[arg(
        long,
        default_value_t = false,
        help = "For --dry-run --table only, omit the table header row.",
        help_heading = "Review Output Options"
    )]
    pub no_header: bool,
    #[arg(
        long,
        value_delimiter = ',',
        requires = "dry_run",
        value_parser = super::super::dashboard_runtime::parse_dashboard_import_output_column,
        help = "For --dry-run --table only, render only these comma-separated columns. Supported values: uid, destination, action, folder_path, source_folder_path, destination_folder_path, reason, file.",
        help_heading = "Review Output Options"
    )]
    pub output_columns: Vec<String>,
    #[arg(
        long,
        default_value_t = false,
        requires = "dry_run",
        help = "Print the supported --output-columns values and exit.",
        help_heading = "Review Output Options"
    )]
    pub list_columns: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Show concise per-dashboard import progress in <current>/<total> form while processing files. Use this for long-running batch imports.",
        help_heading = "Progress Options"
    )]
    pub progress: bool,
    #[arg(
        short = 'v',
        long,
        default_value_t = false,
        help = "Show detailed per-item import output, including target paths, dry-run actions, and folder status details. Overrides --progress output.",
        help_heading = "Progress Options"
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
        default_value_t = super::DashboardServeScriptFormat::Json,
        help = "Interpret --script stdout as json or yaml."
    )]
    pub script_format: super::DashboardServeScriptFormat,
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
