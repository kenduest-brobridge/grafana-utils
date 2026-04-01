//! CLI definitions for Core command surface and option compatibility behavior.

use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use crate::dashboard::{CommonCliArgs, SimpleOutputFormat};
use crate::datasource_catalog::DatasourcePresetProfile;

const DEFAULT_EXPORT_DIR: &str = "datasources";
const DATASOURCE_ROOT_HELP_TEXT: &str = "Examples:\n\n  Browse live datasources in a TUI:\n    grafana-util datasource browse --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\"\n\n  Inspect a local datasource export root:\n    grafana-util datasource inspect-export --input-dir ./datasources --json\n\n  Show the built-in datasource type catalog:\n    grafana-util datasource types --output-format yaml\n\n  List datasources from the current org as JSON:\n    grafana-util datasource list --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --json\n\n  List datasources across all visible orgs with Basic auth:\n    grafana-util datasource list --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --json\n\n  Dry-run a live datasource create with the richer preset scaffold:\n    grafana-util datasource add --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --name prometheus-main --type prometheus --datasource-url http://prometheus:9090 --preset-profile full --dry-run --table\n\n  Dry-run a datasource import:\n    grafana-util datasource import --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --import-dir ./datasources --dry-run --json";
const DATASOURCE_TYPES_HELP_TEXT: &str =
    "Examples:\n\n  grafana-util datasource types\n  grafana-util datasource types --output-format yaml";
const DATASOURCE_LIST_HELP_TEXT: &str = "Examples:\n\n  grafana-util datasource list --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format text\n  grafana-util datasource list --url http://localhost:3000 --basic-user admin --basic-password admin --org-id 2 --output-format csv\n  grafana-util datasource list --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --output-format yaml";
const DATASOURCE_BROWSE_HELP_TEXT: &str = "Live-only browse against Grafana. Use `grafana-util datasource inspect-export` for local export inspection.\n\nKeys:\n\n  e edit selected datasource\n  d delete selected datasource\n  l refresh live inventory\n  q or Esc exit\n\nExamples:\n\n  grafana-util datasource browse --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\"\n  grafana-util datasource browse --url http://localhost:3000 --basic-user admin --basic-password admin --org-id 2\n  grafana-util datasource browse --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs";
const DATASOURCE_INSPECT_EXPORT_HELP_TEXT: &str = "Local export inspection only. This command reads datasource export files from disk and does not talk to Grafana.\n\nExamples:\n\n  grafana-util datasource inspect-export --input-dir ./datasources --table\n  grafana-util datasource inspect-export --input-dir ./datasources --csv\n  grafana-util datasource inspect-export --input-dir ./datasources --json\n  grafana-util datasource inspect-export --input-dir ./datasources --yaml\n  grafana-util datasource inspect-export --input-dir ./datasources --interactive";
const DATASOURCE_EXPORT_HELP_TEXT: &str = "Examples:\n\n  grafana-util datasource export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./datasources --overwrite\n  grafana-util datasource export --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --export-dir ./datasources";
const DATASOURCE_IMPORT_HELP_TEXT: &str = "Examples:\n\n  grafana-util datasource import --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --import-dir ./datasources --dry-run --table\n  grafana-util datasource import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./datasources --secret-values '{\"loki-basic-auth\":\"secret-value\",\"loki-tenant-token\":\"tenant-token\"}'\n  grafana-util datasource import --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --input-format provisioning --import-dir ./datasources/provisioning --dry-run --json\n  grafana-util datasource import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./datasources --use-export-org --only-org-id 2 --create-missing-orgs --dry-run --json";
const DATASOURCE_DIFF_HELP_TEXT: &str = "Examples:\n\n  grafana-util datasource diff --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --diff-dir ./datasources --input-format inventory\n  grafana-util datasource diff --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --diff-dir ./datasources/provisioning --input-format provisioning";
const DATASOURCE_ADD_HELP_TEXT: &str = "Examples:\n\n  grafana-util datasource add --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --name prometheus-main --type prometheus --datasource-url http://prometheus:9090 --dry-run --table\n  grafana-util datasource add --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --name logs-main --type grafana-loki-datasource --datasource-url http://loki:3100 --apply-supported-defaults --dry-run --json\n  grafana-util datasource add --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --name tempo-main --type tempo --datasource-url http://tempo:3200 --preset-profile full --dry-run --json\n  grafana-util datasource add --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --uid loki-main --name loki-main --type loki --datasource-url http://loki:3100 --json-data '{\"timeout\":60}' --dry-run --json";
const DATASOURCE_MODIFY_HELP_TEXT: &str = "Examples:\n\n  grafana-util datasource modify --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --uid prom-main --set-url http://prometheus-v2:9090 --dry-run --json\n  grafana-util datasource modify --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --uid prom-main --set-default true --dry-run --table";
const DATASOURCE_DELETE_HELP_TEXT: &str = "Examples:\n\n  grafana-util datasource delete --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --uid prom-main --dry-run --json\n  grafana-util datasource delete --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --uid prom-main --yes";

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum ListOutputFormat {
    Text,
    Table,
    Csv,
    Json,
    Yaml,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum DryRunOutputFormat {
    Text,
    Table,
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum DatasourceInspectExportOutputFormat {
    Text,
    Table,
    Csv,
    Json,
    Yaml,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum DatasourceImportInputFormat {
    Inventory,
    Provisioning,
}

#[derive(Debug, Clone, Args)]
pub struct DatasourceTypesArgs {
    #[arg(
        long,
        value_enum,
        default_value_t = SimpleOutputFormat::Text,
        help = "Render the supported datasource catalog as text, table, csv, json, or yaml."
    )]
    pub output_format: SimpleOutputFormat,
}

#[derive(Debug, Clone, Args)]
pub struct DatasourceListArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        conflicts_with = "all_orgs",
        help = "List datasources from one explicit Grafana org ID instead of the current org. Requires Basic auth."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "org_id",
        help = "Enumerate all visible Grafana orgs and aggregate datasource inventory across them. Requires Basic auth."
    )]
    pub all_orgs: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "csv", "json", "yaml", "output_format"], help = "Render datasource summaries as plain text.", help_heading = "Output Options")]
    pub text: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["text", "csv", "json", "yaml", "output_format"], help = "Render datasource summaries as a table.", help_heading = "Output Options")]
    pub table: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["text", "table", "json", "yaml", "output_format"], help = "Render datasource summaries as CSV.", help_heading = "Output Options")]
    pub csv: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["text", "table", "csv", "yaml", "output_format"], help = "Render datasource summaries as JSON.", help_heading = "Output Options")]
    pub json: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["text", "table", "csv", "json", "output_format"], help = "Render datasource summaries as YAML.", help_heading = "Output Options")]
    pub yaml: bool,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["text", "table", "csv", "json", "yaml"],
        help = "Alternative single-flag output selector. Use text, table, csv, json, or yaml.",
        help_heading = "Output Options"
    )]
    pub output_format: Option<ListOutputFormat>,
    #[arg(
        long,
        default_value_t = false,
        help = "Do not print table headers when rendering the default table output.",
        help_heading = "Output Options"
    )]
    pub no_header: bool,
}

#[derive(Debug, Clone, Args)]
pub struct DatasourceBrowseArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        conflicts_with = "all_orgs",
        help = "Browse datasources from one explicit Grafana org ID instead of the current org. Requires Basic auth."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "org_id",
        help = "Enumerate all visible Grafana orgs and browse datasource inventory across them. Requires Basic auth."
    )]
    pub all_orgs: bool,
}

#[derive(Debug, Clone, Args)]
pub struct DatasourceInspectExportArgs {
    #[arg(
        long,
        help = "Inspect datasource export files from this local directory. This command reads disk-only export artifacts and never contacts Grafana.",
        help_heading = "Input Options"
    )]
    pub input_dir: PathBuf,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["table", "csv", "text", "json", "yaml", "output_format"],
        help = "Open the local export inspection workbench.",
        help_heading = "Output Options"
    )]
    pub interactive: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["csv", "text", "json", "yaml", "interactive", "output_format"],
        help = "Render the local export inspection as a table.",
        help_heading = "Output Options"
    )]
    pub table: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["table", "text", "json", "yaml", "interactive", "output_format"],
        help = "Render the local export inspection as CSV.",
        help_heading = "Output Options"
    )]
    pub csv: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["table", "csv", "json", "yaml", "interactive", "output_format"],
        help = "Render the local export inspection as plain text.",
        help_heading = "Output Options"
    )]
    pub text: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["table", "csv", "text", "yaml", "interactive", "output_format"],
        help = "Render the local export inspection as JSON.",
        help_heading = "Output Options"
    )]
    pub json: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["table", "csv", "text", "json", "interactive", "output_format"],
        help = "Render the local export inspection as YAML.",
        help_heading = "Output Options"
    )]
    pub yaml: bool,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["table", "csv", "text", "json", "yaml", "interactive"],
        help = "Alternative single-flag output selector. Use text, table, csv, json, or yaml.",
        help_heading = "Output Options"
    )]
    pub output_format: Option<DatasourceInspectExportOutputFormat>,
}

#[derive(Debug, Clone, Args)]
pub struct DatasourceExportArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        default_value = DEFAULT_EXPORT_DIR,
        help = "Directory to write exported datasource inventory into. Export writes datasources.json plus index/manifest files at that root, and writes provisioning/datasources.yaml by default."
    )]
    pub export_dir: PathBuf,
    #[arg(
        long,
        conflicts_with = "all_orgs",
        help = "Export datasource inventory from this explicit Grafana org ID instead of the current org. Requires Basic auth."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "org_id",
        help = "Enumerate all visible Grafana orgs and export one datasource inventory bundle per org under the export root. Requires Basic auth."
    )]
    pub all_orgs: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Replace existing export files in the target directory instead of failing."
    )]
    pub overwrite: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Skip the file-provisioning provisioning/ export variant. Use this only when Grafana datasource provisioning files are not needed."
    )]
    pub without_datasource_provisioning: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Preview the datasource export files that would be written without changing disk."
    )]
    pub dry_run: bool,
}

#[derive(Debug, Clone, Args)]
pub struct DatasourceImportArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help = "Import datasource inventory from this path. For inventory input, point this at the datasource export root that contains datasources.json and export-metadata.json. For provisioning input, point this at the export root, provisioning directory, or concrete datasources.yaml file.",
        help_heading = "Input Options"
    )]
    pub import_dir: PathBuf,
    #[arg(
        long,
        value_enum,
        default_value_t = DatasourceImportInputFormat::Inventory,
        help = "Select the datasource import input format. Use inventory for the existing datasources.json export contract, or provisioning to load Grafana datasource provisioning YAML from an export root, provisioning directory, or concrete datasources.yaml file.",
        help_heading = "Input Options"
    )]
    pub input_format: DatasourceImportInputFormat,
    #[arg(
        long,
        conflicts_with = "use_export_org",
        help = "Import datasources into this Grafana org ID instead of the current org context. Requires Basic auth."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "require_matching_export_org",
        help = "Import a combined multi-org datasource export root by routing each org-scoped datasource bundle back into the matching Grafana org. Requires Basic auth."
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
        default_value_t = false,
        help = "Require the datasource export orgId to match the target Grafana org before dry-run or live import."
    )]
    pub require_matching_export_org: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Update an existing destination datasource when the imported datasource identity already exists. Without this flag, existing matches are blocked."
    )]
    pub replace_existing: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Reconcile only datasources that already exist in Grafana. Missing destination identities are skipped instead of created."
    )]
    pub update_existing_only: bool,
    #[arg(
        long,
        help = "Inline JSON object string mapping datasource secret placeholder names to resolved secret values for live import when the export records include secureJsonDataPlaceholders.",
        help_heading = "Secrets"
    )]
    pub secret_values: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Preview what datasource import would do without changing Grafana."
    )]
    pub dry_run: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "For --dry-run only, render a compact table instead of per-datasource log lines."
    )]
    pub table: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "For --dry-run only, render one JSON document with mode, datasource actions, and summary counts."
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
        value_parser = parse_datasource_import_output_column,
        help = "For --dry-run --table only, render only these comma-separated columns. Supported values: uid, name, type, destination, action, org_id, file."
    )]
    pub output_columns: Vec<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Show concise per-datasource progress in <current>/<total> form while processing files."
    )]
    pub progress: bool,
    #[arg(
        short = 'v',
        long,
        default_value_t = false,
        help = "Show detailed per-item import output. Overrides --progress output."
    )]
    pub verbose: bool,
}

#[derive(Debug, Clone, Args)]
pub struct DatasourceDiffArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help = "Compare datasource inventory from this directory against live Grafana. For provisioning input, point this at the export root, provisioning directory, or concrete datasources.yaml file.",
        help_heading = "Input Options"
    )]
    pub diff_dir: PathBuf,
    #[arg(
        long,
        value_enum,
        default_value_t = DatasourceImportInputFormat::Inventory,
        help = "Select the datasource diff input format. Use inventory for the existing datasources.json export contract, or provisioning to load Grafana datasource provisioning YAML from an export root, provisioning directory, or concrete datasources.yaml file.",
        help_heading = "Input Options"
    )]
    pub input_format: DatasourceImportInputFormat,
}

#[derive(Debug, Clone, Args)]
pub struct DatasourceAddArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help = "Datasource UID to create. Optional but recommended for stable identity."
    )]
    pub uid: Option<String>,
    #[arg(long, help = "Datasource name to create.")]
    pub name: String,
    #[arg(
        long = "type",
        help = "Grafana datasource plugin type id to create. Supported aliases from `datasource types` are normalized to canonical type ids."
    )]
    pub datasource_type: String,
    #[arg(
        long,
        default_value_t = false,
        help = "Legacy shortcut for starter preset defaults on supported datasource types."
    )]
    pub apply_supported_defaults: bool,
    #[arg(
        long,
        value_enum,
        help = "Apply a preset profile for supported datasource types. Use starter to match --apply-supported-defaults or full for a richer scaffold."
    )]
    pub preset_profile: Option<DatasourcePresetProfile>,
    #[arg(long, help = "Datasource access mode such as proxy or direct.")]
    pub access: Option<String>,
    #[arg(long, help = "Datasource target URL to store in Grafana.")]
    pub datasource_url: Option<String>,
    #[arg(
        long = "default",
        default_value_t = false,
        help = "Mark the new datasource as the default datasource."
    )]
    pub is_default: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Enable basic auth for the datasource."
    )]
    pub basic_auth: bool,
    #[arg(long, help = "Username for datasource basic auth.")]
    pub basic_auth_user: Option<String>,
    #[arg(
        long,
        help = "Password for datasource basic auth. Stored in secureJsonData."
    )]
    pub basic_auth_password: Option<String>,
    #[arg(
        long,
        help = "Datasource user/login field where the plugin supports it."
    )]
    pub user: Option<String>,
    #[arg(
        long = "password",
        help = "Datasource password field where the plugin supports it. Stored in secureJsonData."
    )]
    pub datasource_password: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Send browser credentials such as cookies for supported datasource types."
    )]
    pub with_credentials: bool,
    #[arg(long, action = ArgAction::Append, help = "Add one custom HTTP header for supported datasource types. May be specified multiple times.", value_name = "NAME=VALUE")]
    pub http_header: Vec<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Set jsonData.tlsSkipVerify=true for supported datasource types."
    )]
    pub tls_skip_verify: bool,
    #[arg(
        long,
        help = "Set jsonData.serverName for supported datasource TLS validation."
    )]
    pub server_name: Option<String>,
    #[arg(long, help = "Inline JSON object string for datasource jsonData.")]
    pub json_data: Option<String>,
    #[arg(
        long,
        help = "Inline JSON object string for datasource secureJsonData."
    )]
    pub secure_json_data: Option<String>,
    #[arg(
        long,
        help = "Inline JSON object string mapping secureJsonData field names to ${secret:...} placeholders."
    )]
    pub secure_json_data_placeholders: Option<String>,
    #[arg(
        long,
        help = "Inline JSON object string mapping secret placeholder names to resolved secret values for add."
    )]
    pub secret_values: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Preview what datasource add would do without changing Grafana."
    )]
    pub dry_run: bool,
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
    #[arg(long, value_enum, conflicts_with_all = ["table", "json"], help = "Alternative single-flag output selector for datasource add dry-run output. Use text, table, or json.")]
    pub output_format: Option<DryRunOutputFormat>,
    #[arg(
        long,
        default_value_t = false,
        help = "For --dry-run --table only, omit the table header row."
    )]
    pub no_header: bool,
}

#[derive(Debug, Clone, Args)]
pub struct DatasourceDeleteArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        required_unless_present = "name",
        conflicts_with = "name",
        help = "Datasource UID to delete.",
        help_heading = "Target Options"
    )]
    pub uid: Option<String>,
    #[arg(
        long,
        required_unless_present = "uid",
        conflicts_with = "uid",
        help = "Datasource name to delete when UID is not available.",
        help_heading = "Target Options"
    )]
    pub name: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Acknowledge the live datasource delete. Required unless --dry-run is set.",
        help_heading = "Safety Options"
    )]
    pub yes: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Preview what datasource delete would do without changing Grafana.",
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
    #[arg(long, value_enum, conflicts_with_all = ["table", "json"], help = "Alternative single-flag output selector for datasource delete dry-run output. Use text, table, or json.", help_heading = "Output Options")]
    pub output_format: Option<DryRunOutputFormat>,
    #[arg(
        long,
        default_value_t = false,
        help = "For --dry-run --table only, omit the table header row.",
        help_heading = "Output Options"
    )]
    pub no_header: bool,
}

#[derive(Debug, Clone, Args)]
pub struct DatasourceModifyArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long, help = "Datasource UID to modify.")]
    pub uid: String,
    #[arg(long, help = "Replace the datasource URL stored in Grafana.")]
    pub set_url: Option<String>,
    #[arg(
        long,
        help = "Replace the datasource access mode such as proxy or direct."
    )]
    pub set_access: Option<String>,
    #[arg(
        long,
        value_parser = parse_bool_choice,
        help = "Set whether Grafana treats this datasource as default. Use true or false."
    )]
    pub set_default: Option<bool>,
    #[arg(
        long,
        default_value_t = false,
        help = "Enable basic auth for the datasource."
    )]
    pub basic_auth: bool,
    #[arg(long, help = "Replace datasource basic auth username.")]
    pub basic_auth_user: Option<String>,
    #[arg(
        long,
        help = "Replace datasource basic auth password. Stored in secureJsonData."
    )]
    pub basic_auth_password: Option<String>,
    #[arg(
        long,
        help = "Replace datasource user/login field where the plugin supports it."
    )]
    pub user: Option<String>,
    #[arg(
        long = "password",
        help = "Replace datasource password field where the plugin supports it. Stored in secureJsonData."
    )]
    pub datasource_password: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Set withCredentials=true for supported datasource types."
    )]
    pub with_credentials: bool,
    #[arg(long, action = ArgAction::Append, help = "Replace or add one custom HTTP header for supported datasource types. May be specified multiple times.", value_name = "NAME=VALUE")]
    pub http_header: Vec<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Set jsonData.tlsSkipVerify=true for supported datasource types."
    )]
    pub tls_skip_verify: bool,
    #[arg(
        long,
        help = "Set jsonData.serverName for supported datasource TLS validation."
    )]
    pub server_name: Option<String>,
    #[arg(
        long,
        help = "Inline JSON object string to merge into datasource jsonData."
    )]
    pub json_data: Option<String>,
    #[arg(
        long,
        help = "Inline JSON object string to send in datasource secureJsonData."
    )]
    pub secure_json_data: Option<String>,
    #[arg(
        long,
        help = "Inline JSON object string mapping secureJsonData field names to ${secret:...} placeholders."
    )]
    pub secure_json_data_placeholders: Option<String>,
    #[arg(
        long,
        help = "Inline JSON object string mapping secret placeholder names to resolved secret values for modify."
    )]
    pub secret_values: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Preview what datasource modify would do without changing Grafana."
    )]
    pub dry_run: bool,
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
    #[arg(long, value_enum, conflicts_with_all = ["table", "json"], help = "Alternative single-flag output selector for datasource modify dry-run output. Use text, table, or json.")]
    pub output_format: Option<DryRunOutputFormat>,
    #[arg(
        long,
        default_value_t = false,
        help = "For --dry-run --table only, omit the table header row."
    )]
    pub no_header: bool,
}

#[derive(Debug, Clone, Subcommand)]
pub enum DatasourceGroupCommand {
    #[command(about = "Show the built-in supported datasource type catalog.", after_help = DATASOURCE_TYPES_HELP_TEXT)]
    Types(DatasourceTypesArgs),
    #[command(about = "List live Grafana datasource inventory.", after_help = DATASOURCE_LIST_HELP_TEXT)]
    List(DatasourceListArgs),
    #[command(
        about = "Open a live datasource browser against Grafana with in-place modify and delete actions.",
        after_help = DATASOURCE_BROWSE_HELP_TEXT
    )]
    Browse(DatasourceBrowseArgs),
    #[command(
        name = "inspect-export",
        about = "Inspect a local datasource export root without connecting to Grafana.",
        after_help = DATASOURCE_INSPECT_EXPORT_HELP_TEXT
    )]
    InspectExport(DatasourceInspectExportArgs),
    #[command(about = "Create one live Grafana datasource through the Grafana API.", after_help = DATASOURCE_ADD_HELP_TEXT)]
    Add(DatasourceAddArgs),
    #[command(about = "Modify one live Grafana datasource through the Grafana API.", after_help = DATASOURCE_MODIFY_HELP_TEXT)]
    Modify(DatasourceModifyArgs),
    #[command(about = "Delete one live Grafana datasource through the Grafana API.", after_help = DATASOURCE_DELETE_HELP_TEXT)]
    Delete(DatasourceDeleteArgs),
    #[command(about = "Export live Grafana datasource inventory as normalized JSON plus provisioning files.", after_help = DATASOURCE_EXPORT_HELP_TEXT)]
    Export(DatasourceExportArgs),
    #[command(about = "Import datasource inventory through the Grafana API.", after_help = DATASOURCE_IMPORT_HELP_TEXT)]
    Import(DatasourceImportArgs),
    #[command(about = "Compare local datasource export files against live Grafana datasources.", after_help = DATASOURCE_DIFF_HELP_TEXT)]
    Diff(DatasourceDiffArgs),
}

#[derive(Debug, Clone, Parser)]
#[command(
    name = "grafana-util datasource",
    about = "List, browse live, inspect local exports, add, modify, delete, export, import, and diff Grafana datasources.",
    after_help = DATASOURCE_ROOT_HELP_TEXT,
    styles = crate::help_styles::CLI_HELP_STYLES
)]
pub struct DatasourceCliArgs {
    #[command(subcommand)]
    pub command: DatasourceGroupCommand,
}

#[cfg(test)]
pub(crate) fn normalize_output_formats(args: &mut DatasourceCliArgs) {
    match &mut args.command {
        DatasourceGroupCommand::List(inner) => match inner.output_format {
            Some(ListOutputFormat::Text) => inner.text = true,
            Some(ListOutputFormat::Table) => inner.table = true,
            Some(ListOutputFormat::Csv) => inner.csv = true,
            Some(ListOutputFormat::Json) => inner.json = true,
            Some(ListOutputFormat::Yaml) => inner.yaml = true,
            None => {}
        },
        DatasourceGroupCommand::Browse(_) => {}
        DatasourceGroupCommand::InspectExport(inner) => match inner.output_format {
            Some(DatasourceInspectExportOutputFormat::Table) => inner.table = true,
            Some(DatasourceInspectExportOutputFormat::Csv) => inner.csv = true,
            Some(DatasourceInspectExportOutputFormat::Text) => inner.text = true,
            Some(DatasourceInspectExportOutputFormat::Json) => inner.json = true,
            Some(DatasourceInspectExportOutputFormat::Yaml) => inner.yaml = true,
            None if !inner.table
                && !inner.csv
                && !inner.json
                && !inner.yaml
                && !inner.interactive =>
            {
                inner.text = true
            }
            None => {}
        },
        DatasourceGroupCommand::Import(inner) => match inner.output_format {
            Some(DryRunOutputFormat::Table) => inner.table = true,
            Some(DryRunOutputFormat::Json) => inner.json = true,
            Some(DryRunOutputFormat::Text) | None => {}
        },
        DatasourceGroupCommand::Add(inner) => match inner.output_format {
            Some(DryRunOutputFormat::Table) => inner.table = true,
            Some(DryRunOutputFormat::Json) => inner.json = true,
            Some(DryRunOutputFormat::Text) | None => {}
        },
        DatasourceGroupCommand::Modify(inner) => match inner.output_format {
            Some(DryRunOutputFormat::Table) => inner.table = true,
            Some(DryRunOutputFormat::Json) => inner.json = true,
            Some(DryRunOutputFormat::Text) | None => {}
        },
        DatasourceGroupCommand::Delete(inner) => match inner.output_format {
            Some(DryRunOutputFormat::Table) => inner.table = true,
            Some(DryRunOutputFormat::Json) => inner.json = true,
            Some(DryRunOutputFormat::Text) | None => {}
        },
        _ => {}
    }
}

pub(crate) fn normalize_datasource_group_command(
    mut command: DatasourceGroupCommand,
) -> DatasourceGroupCommand {
    match &mut command {
        DatasourceGroupCommand::List(inner) => match inner.output_format {
            Some(ListOutputFormat::Text) => inner.text = true,
            Some(ListOutputFormat::Table) => inner.table = true,
            Some(ListOutputFormat::Csv) => inner.csv = true,
            Some(ListOutputFormat::Json) => inner.json = true,
            Some(ListOutputFormat::Yaml) => inner.yaml = true,
            None => {}
        },
        DatasourceGroupCommand::Browse(_) => {}
        DatasourceGroupCommand::InspectExport(inner) => match inner.output_format {
            Some(DatasourceInspectExportOutputFormat::Table) => inner.table = true,
            Some(DatasourceInspectExportOutputFormat::Csv) => inner.csv = true,
            Some(DatasourceInspectExportOutputFormat::Text) => inner.text = true,
            Some(DatasourceInspectExportOutputFormat::Json) => inner.json = true,
            Some(DatasourceInspectExportOutputFormat::Yaml) => inner.yaml = true,
            None if !inner.table
                && !inner.csv
                && !inner.json
                && !inner.yaml
                && !inner.interactive =>
            {
                inner.text = true
            }
            None => {}
        },
        DatasourceGroupCommand::Import(inner) => match inner.output_format {
            Some(DryRunOutputFormat::Table) => inner.table = true,
            Some(DryRunOutputFormat::Json) => inner.json = true,
            Some(DryRunOutputFormat::Text) | None => {}
        },
        DatasourceGroupCommand::Add(inner) => match inner.output_format {
            Some(DryRunOutputFormat::Table) => inner.table = true,
            Some(DryRunOutputFormat::Json) => inner.json = true,
            Some(DryRunOutputFormat::Text) | None => {}
        },
        DatasourceGroupCommand::Modify(inner) => match inner.output_format {
            Some(DryRunOutputFormat::Table) => inner.table = true,
            Some(DryRunOutputFormat::Json) => inner.json = true,
            Some(DryRunOutputFormat::Text) | None => {}
        },
        DatasourceGroupCommand::Delete(inner) => match inner.output_format {
            Some(DryRunOutputFormat::Table) => inner.table = true,
            Some(DryRunOutputFormat::Json) => inner.json = true,
            Some(DryRunOutputFormat::Text) | None => {}
        },
        _ => {}
    }
    command
}

fn parse_datasource_import_output_column(value: &str) -> std::result::Result<String, String> {
    match value {
        "uid" => Ok("uid".to_string()),
        "name" => Ok("name".to_string()),
        "type" => Ok("type".to_string()),
        "destination" => Ok("destination".to_string()),
        "action" => Ok("action".to_string()),
        "org_id" | "orgId" => Ok("org_id".to_string()),
        "file" => Ok("file".to_string()),
        _ => Err(format!(
            "Unsupported --output-columns value '{value}'. Supported values: uid, name, type, destination, action, org_id, file."
        )),
    }
}

fn parse_bool_choice(value: &str) -> std::result::Result<bool, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err("value must be true or false".to_string()),
    }
}

#[cfg(test)]
impl DatasourceCliArgs {
    pub(crate) fn parse_normalized_from<I, T>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        let mut args = Self::parse_from(iter);
        normalize_output_formats(&mut args);
        args
    }
}
