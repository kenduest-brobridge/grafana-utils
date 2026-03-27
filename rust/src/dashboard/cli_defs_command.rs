use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

use super::super::{DEFAULT_EXPORT_DIR, DEFAULT_IMPORT_MESSAGE, DEFAULT_PAGE_SIZE};
use super::cli_defs_inspect::{
    GovernanceGateArgs, ImpactArgs, InspectExportArgs, InspectLiveArgs, InspectVarsArgs,
    ScreenshotArgs, TopologyArgs, ValidateExportArgs,
};
use super::cli_defs_shared::{CommonCliArgs, DryRunOutputFormat, SimpleOutputFormat};
use super::dashboard_runtime::{
    parse_dashboard_import_output_column, parse_dashboard_list_output_column,
};

/// Arguments for exporting dashboards into raw and prompt variants.
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
        help = "Open an interactive picker to choose which exported dashboards to import from --import-dir."
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
        name = "delete",
        about = "Delete live dashboards by UID or folder path.",
        after_help = "Examples:\n\n  Dry-run one dashboard delete by UID:\n    grafana-util dashboard delete --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --uid cpu-main --dry-run --json\n\n  Delete all dashboards under one folder subtree:\n    grafana-util dashboard delete --url http://localhost:3000 --basic-user admin --basic-password admin --path 'Platform / Infra' --yes\n\n  Interactively preview and confirm a folder delete:\n    grafana-util dashboard delete --url http://localhost:3000 --interactive"
    )]
    Delete(DeleteArgs),
    #[command(about = "Compare local raw dashboard files against live Grafana dashboards.")]
    Diff(DiffArgs),
    #[command(
        name = "inspect-export",
        about = "Analyze a raw dashboard export directory and summarize its structure.",
        after_help = "Examples:\n\n  Render a dashboard summary table from raw exports:\n    grafana-util dashboard inspect-export --import-dir ./dashboards/raw --table\n\n  Open the interactive inspect workbench over raw exports:\n    grafana-util dashboard inspect-export --import-dir ./dashboards/raw --interactive\n\n  Render governance JSON from raw exports:\n    grafana-util dashboard inspect-export --import-dir ./dashboards/raw --report governance-json"
    )]
    InspectExport(InspectExportArgs),
    #[command(
        name = "inspect-live",
        about = "Analyze live Grafana dashboards via a temporary raw-export snapshot.",
        after_help = "Examples:\n\n  Render governance JSON from live Grafana:\n    grafana-util dashboard inspect-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format governance-json\n\n  Open the interactive inspect workbench over live Grafana:\n    grafana-util dashboard inspect-live --url http://localhost:3000 --basic-user admin --basic-password admin --interactive"
    )]
    InspectLive(InspectLiveArgs),
    #[command(
        name = "inspect-vars",
        about = "List dashboard templating variables and datasource-like choices from live Grafana."
    )]
    InspectVars(InspectVarsArgs),
    #[command(
        name = "governance-gate",
        about = "Evaluate a governance policy file or built-in policy against dashboard governance-json and query-report JSON artifacts.",
        after_help = "Examples:\n\n  Evaluate a JSON/YAML policy file with text output:\n    grafana-util dashboard governance-gate --policy-source file --policy ./policy.yaml --governance ./governance.json --queries ./queries.json\n\n  Evaluate the built-in policy by name and write the normalized result JSON:\n    grafana-util dashboard governance-gate --policy-source builtin --builtin-policy default --governance ./governance.json --queries ./queries.json --output-format json --json-output ./governance-check.json"
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
