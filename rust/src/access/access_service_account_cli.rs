//! CLI definitions for Grafana service-account access workflows.
//! This module owns the service-account list, add, modify, delete, import/export,
//! and token subcommand wiring. It converts clap arguments into shared access
//! workflow helpers and keeps the command help text aligned with operator usage.

use clap::{Args, Subcommand};
use std::path::PathBuf;

use super::super::pending_delete::{ServiceAccountDeleteArgs, ServiceAccountTokenDeleteArgs};
use super::access_cli_shared::{
    parse_bool_text, parse_positive_usize, CommonCliArgs, DryRunOutputFormat, ListOutputFormat,
    DEFAULT_ACCESS_SERVICE_ACCOUNT_EXPORT_DIR, DEFAULT_PAGE_SIZE,
};

fn parse_service_account_list_output_column(value: &str) -> std::result::Result<String, String> {
    match value {
        "all" => Ok("all".to_string()),
        "id" => Ok("id".to_string()),
        "name" => Ok("name".to_string()),
        "login" => Ok("login".to_string()),
        "role" => Ok("role".to_string()),
        "disabled" => Ok("disabled".to_string()),
        "tokens" => Ok("tokens".to_string()),
        "org_id" | "orgId" => Ok("org_id".to_string()),
        _ => Err(format!(
            "Unsupported --output-columns value '{value}'. Supported values: all, id, name, login, role, disabled, tokens, org_id."
        )),
    }
}

pub(crate) const ACCESS_SERVICE_ACCOUNT_HELP_TEXT: &str = "Examples:\n\n  grafana-util access service-account list --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --json\n  grafana-util access service-account token add --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --name deploy-bot --token-name nightly";
pub(crate) const ACCESS_SERVICE_ACCOUNT_LIST_HELP_TEXT: &str = "Examples:\n\n  grafana-util access service-account list --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format text\n  grafana-util access service-account list --url http://localhost:3000 --basic-user admin --basic-password admin --output-format yaml";
pub(crate) const ACCESS_SERVICE_ACCOUNT_ADD_HELP_TEXT: &str = "Examples:\n\n  grafana-util access service-account add --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --name deploy-bot --role Editor --json";
pub(crate) const ACCESS_SERVICE_ACCOUNT_EXPORT_HELP_TEXT: &str = "Examples:\n\n  grafana-util access service-account export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-dir ./access-service-accounts --overwrite";
pub(crate) const ACCESS_SERVICE_ACCOUNT_IMPORT_HELP_TEXT: &str = "Examples:\n\n  grafana-util access service-account import --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --input-dir ./access-service-accounts --dry-run --output-format table\n  grafana-util access service-account import --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --input-dir ./access-service-accounts --replace-existing --yes";
pub(crate) const ACCESS_SERVICE_ACCOUNT_DIFF_HELP_TEXT: &str = "Examples:\n\n  grafana-util access service-account diff --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --diff-dir ./access-service-accounts";
pub(crate) const ACCESS_SERVICE_ACCOUNT_DELETE_HELP_TEXT: &str = "Examples:\n\n  grafana-util access service-account delete --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --name deploy-bot --yes --json\n  grafana-util access service-account delete --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --prompt";
pub(crate) const ACCESS_SERVICE_ACCOUNT_TOKEN_HELP_TEXT: &str = "Examples:\n\n  grafana-util access service-account token add --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --name deploy-bot --token-name nightly\n  grafana-util access service-account token delete --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --name deploy-bot --token-name nightly --yes --json";
pub(crate) const ACCESS_SERVICE_ACCOUNT_TOKEN_ADD_HELP_TEXT: &str = "Examples:\n\n  grafana-util access service-account token add --token \"$GRAFANA_API_TOKEN\" --name deploy-bot --token-name nightly\n  grafana-util access service-account token add --token \"$GRAFANA_API_TOKEN\" --service-account-id 7 --token-name nightly --seconds-to-live 3600";
pub(crate) const ACCESS_SERVICE_ACCOUNT_TOKEN_DELETE_HELP_TEXT: &str = "Examples:\n\n  grafana-util access service-account token delete --token \"$GRAFANA_API_TOKEN\" --name deploy-bot --token-name nightly --yes --json\n  grafana-util access service-account token delete --token \"$GRAFANA_API_TOKEN\" --prompt\n  grafana-util access service-account token delete --token \"$GRAFANA_API_TOKEN\" --service-account-id 7 --token-name nightly --yes --json";

/// Struct definition for ServiceAccountListArgs.
#[derive(Debug, Clone, Args)]
pub struct ServiceAccountListArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help = "List service accounts from a local export bundle directory instead of live Grafana."
    )]
    pub input_dir: Option<PathBuf>,
    #[arg(long, help = "Filter service accounts by a free-text search.")]
    pub query: Option<String>,
    #[arg(
        long,
        default_value_t = 1,
        help = "Result page number for paginated Grafana list APIs."
    )]
    pub page: usize,
    #[arg(long, default_value_t = DEFAULT_PAGE_SIZE, help = "Number of service accounts to request per page.")]
    pub per_page: usize,
    #[arg(
        long,
        value_delimiter = ',',
        value_parser = parse_service_account_list_output_column,
        help = "For text, table, or csv output, render only these comma-separated columns. Use all to expand every supported column. Supported values: all, id, name, login, role, disabled, tokens, org_id. JSON-style aliases like orgId are also accepted."
    )]
    pub output_columns: Vec<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Print the supported --output-columns values and exit."
    )]
    pub list_columns: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["csv", "json", "yaml"], help = "Render service-account summaries as a table.")]
    pub table: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "json", "yaml"], help = "Render service-account summaries as CSV.")]
    pub csv: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "csv", "yaml"], help = "Render service-account summaries as JSON.")]
    pub json: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "csv", "json"], help = "Render service-account summaries as YAML.")]
    pub yaml: bool,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["table", "csv", "json", "yaml"],
        help = "Alternative single-flag output selector. Use text, table, csv, json, or yaml."
    )]
    pub output_format: Option<ListOutputFormat>,
}

/// Struct definition for ServiceAccountAddArgs.
#[derive(Debug, Clone, Args)]
pub struct ServiceAccountAddArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long, help = "Name for the new Grafana service account.")]
    pub name: String,
    #[arg(
        long,
        default_value = "Viewer",
        value_parser = parse_service_account_role,
        help = "Initial org role for the service account."
    )]
    pub role: String,
    #[arg(long, value_parser = parse_bool_text, default_value = "false", help = "Create the service account in a disabled state.")]
    pub disabled: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Render the create response as JSON."
    )]
    pub json: bool,
}

/// Struct definition for ServiceAccountExportArgs.
#[derive(Debug, Clone, Args)]
pub struct ServiceAccountExportArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long = "output-dir",
        default_value = DEFAULT_ACCESS_SERVICE_ACCOUNT_EXPORT_DIR,
        help = "Directory to write service-accounts.json and export-metadata.json."
    )]
    pub output_dir: PathBuf,
    #[arg(
        long,
        default_value_t = false,
        help = "Overwrite existing export files instead of failing."
    )]
    pub overwrite: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Preview export paths without writing files."
    )]
    pub dry_run: bool,
}

/// Struct definition for ServiceAccountImportArgs.
#[derive(Debug, Clone, Args)]
pub struct ServiceAccountImportArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long = "input-dir",
        help = "Import directory that contains service-accounts.json and export-metadata.json."
    )]
    pub input_dir: PathBuf,
    #[arg(
        long,
        default_value_t = false,
        help = "Update matching existing service accounts instead of failing on duplicates."
    )]
    pub replace_existing: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Preview import changes without writing to Grafana."
    )]
    pub dry_run: bool,
    #[arg(
        long,
        default_value_t = false,
        requires = "dry_run",
        help = "For --dry-run only, render a compact table instead of per-record log lines."
    )]
    pub table: bool,
    #[arg(
        long,
        default_value_t = false,
        requires = "dry_run",
        help = "For --dry-run only, render one JSON document with action rows and summary counts."
    )]
    pub json: bool,
    #[arg(
        long,
        value_enum,
        default_value_t = DryRunOutputFormat::Text,
        conflicts_with_all = ["table", "json"],
        help = "Alternative single-flag output selector for --dry-run output. Use text, table, or json."
    )]
    pub output_format: DryRunOutputFormat,
    #[arg(
        long,
        default_value_t = false,
        help = "Acknowledge destructive import operations when required."
    )]
    pub yes: bool,
}

/// Struct definition for ServiceAccountDiffArgs.
#[derive(Debug, Clone, Args)]
pub struct ServiceAccountDiffArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        default_value = DEFAULT_ACCESS_SERVICE_ACCOUNT_EXPORT_DIR,
        help = "Diff directory that contains service-accounts.json and export-metadata.json."
    )]
    pub diff_dir: PathBuf,
}

/// Struct definition for ServiceAccountTokenAddArgs.
#[derive(Debug, Clone, Args)]
pub struct ServiceAccountTokenAddArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        conflicts_with = "name",
        help = "Target one service account by numeric id."
    )]
    pub service_account_id: Option<String>,
    #[arg(
        long,
        conflicts_with = "service_account_id",
        help = "Target one service account by exact name."
    )]
    pub name: Option<String>,
    #[arg(long, help = "Name for the new service-account token.")]
    pub token_name: String,
    #[arg(
        long,
        value_parser = parse_positive_usize,
        help = "Optional token lifetime in seconds. Omit for a non-expiring token if Grafana allows it."
    )]
    pub seconds_to_live: Option<usize>,
    #[arg(
        long,
        default_value_t = false,
        help = "Render the token create response as JSON."
    )]
    pub json: bool,
}

/// Service-account token workflows.
#[derive(Debug, Clone, Subcommand)]
pub enum ServiceAccountTokenCommand {
    #[command(after_help = ACCESS_SERVICE_ACCOUNT_TOKEN_ADD_HELP_TEXT)]
    Add(ServiceAccountTokenAddArgs),
    #[command(after_help = ACCESS_SERVICE_ACCOUNT_TOKEN_DELETE_HELP_TEXT)]
    Delete(ServiceAccountTokenDeleteArgs),
}

/// Service-account inventory and lifecycle workflows.
#[derive(Debug, Clone, Subcommand)]
pub enum ServiceAccountCommand {
    #[command(after_help = ACCESS_SERVICE_ACCOUNT_LIST_HELP_TEXT)]
    List(ServiceAccountListArgs),
    #[command(after_help = ACCESS_SERVICE_ACCOUNT_ADD_HELP_TEXT)]
    Add(ServiceAccountAddArgs),
    #[command(after_help = ACCESS_SERVICE_ACCOUNT_EXPORT_HELP_TEXT)]
    Export(ServiceAccountExportArgs),
    #[command(after_help = ACCESS_SERVICE_ACCOUNT_IMPORT_HELP_TEXT)]
    Import(ServiceAccountImportArgs),
    #[command(after_help = ACCESS_SERVICE_ACCOUNT_DIFF_HELP_TEXT)]
    Diff(ServiceAccountDiffArgs),
    #[command(after_help = ACCESS_SERVICE_ACCOUNT_DELETE_HELP_TEXT)]
    Delete(ServiceAccountDeleteArgs),
    #[command(after_help = ACCESS_SERVICE_ACCOUNT_TOKEN_HELP_TEXT)]
    Token {
        #[command(subcommand)]
        command: ServiceAccountTokenCommand,
    },
}

fn parse_service_account_role(value: &str) -> std::result::Result<String, String> {
    // Call graph (hierarchy): this function is used in related modules.
    // Upstream callers: 無
    // Downstream callees: 無

    match value {
        "Viewer" | "Editor" | "Admin" | "None" => Ok(value.to_string()),
        _ => Err("valid values: Viewer, Editor, Admin, None".to_string()),
    }
}
