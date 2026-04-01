//! Clap schema for access-management CLI commands.
//! Centralizes CLI argument enums and parser-normalization helpers for access handlers.
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[path = "access_cli_runtime.rs"]
mod access_cli_runtime;

#[path = "access_cli_shared.rs"]
mod access_cli_shared;

#[path = "access_service_account_cli.rs"]
mod access_service_account_cli;

use super::pending_delete::TeamDeleteArgs;

pub use access_cli_runtime::{
    build_auth_context, build_auth_context_no_org_id, build_http_client,
    build_http_client_no_org_id, normalize_access_cli_args, parse_cli_from, root_command,
    AccessAuthContext,
};
pub use access_cli_shared::{
    CommonCliArgs, CommonCliArgsNoOrgId, DryRunOutputFormat, ListOutputFormat, Scope,
    ACCESS_EXPORT_KIND_ORGS, ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS, ACCESS_EXPORT_KIND_TEAMS,
    ACCESS_EXPORT_KIND_USERS, ACCESS_EXPORT_METADATA_FILENAME, ACCESS_EXPORT_VERSION,
    ACCESS_ORG_EXPORT_FILENAME, ACCESS_SERVICE_ACCOUNT_EXPORT_FILENAME,
    ACCESS_TEAM_EXPORT_FILENAME, ACCESS_USER_EXPORT_FILENAME, DEFAULT_ACCESS_ORG_EXPORT_DIR,
    DEFAULT_ACCESS_TEAM_EXPORT_DIR, DEFAULT_PAGE_SIZE, DEFAULT_TIMEOUT, DEFAULT_URL,
};
use access_cli_shared::{
    ACCESS_ORG_ADD_HELP_TEXT, ACCESS_ORG_DELETE_HELP_TEXT, ACCESS_ORG_DIFF_HELP_TEXT,
    ACCESS_ORG_EXPORT_HELP_TEXT, ACCESS_ORG_HELP_TEXT, ACCESS_ORG_IMPORT_HELP_TEXT,
    ACCESS_ORG_LIST_HELP_TEXT, ACCESS_ORG_MODIFY_HELP_TEXT, ACCESS_ROOT_HELP_TEXT,
    ACCESS_TEAM_ADD_HELP_TEXT, ACCESS_TEAM_BROWSE_HELP_TEXT, ACCESS_TEAM_DELETE_HELP_TEXT,
    ACCESS_TEAM_DIFF_HELP_TEXT, ACCESS_TEAM_EXPORT_HELP_TEXT, ACCESS_TEAM_HELP_TEXT,
    ACCESS_TEAM_IMPORT_HELP_TEXT, ACCESS_TEAM_LIST_HELP_TEXT, ACCESS_TEAM_MODIFY_HELP_TEXT,
    ACCESS_USER_HELP_TEXT,
};
use access_service_account_cli::ACCESS_SERVICE_ACCOUNT_HELP_TEXT;
pub use access_service_account_cli::{
    ServiceAccountAddArgs, ServiceAccountCommand, ServiceAccountDiffArgs, ServiceAccountExportArgs,
    ServiceAccountImportArgs, ServiceAccountListArgs, ServiceAccountTokenAddArgs,
    ServiceAccountTokenCommand,
};

#[path = "access_user_cli.rs"]
mod access_user_cli;

pub use access_user_cli::{
    UserAddArgs, UserBrowseArgs, UserCommand, UserDeleteArgs, UserDiffArgs, UserExportArgs,
    UserImportArgs, UserListArgs, UserModifyArgs,
};

/// Struct definition for TeamListArgs.
#[derive(Debug, Clone, Args)]
pub struct TeamListArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long, help = "Filter teams by a free-text search.")]
    pub query: Option<String>,
    #[arg(long, help = "Filter teams by exact team name.")]
    pub name: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Include team members and admins in the rendered output."
    )]
    pub with_members: bool,
    #[arg(
        long,
        default_value_t = 1,
        help = "Result page number for paginated Grafana list APIs."
    )]
    pub page: usize,
    #[arg(long, default_value_t = DEFAULT_PAGE_SIZE, help = "Number of teams to request per page.")]
    pub per_page: usize,
    #[arg(long, default_value_t = false, conflicts_with_all = ["csv", "json", "yaml"], help = "Render team summaries as a table.")]
    pub table: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "json", "yaml"], help = "Render team summaries as CSV.")]
    pub csv: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "csv", "yaml"], help = "Render team summaries as JSON.")]
    pub json: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "csv", "json"], help = "Render team summaries as YAML.")]
    pub yaml: bool,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["table", "csv", "json", "yaml"],
        help = "Alternative single-flag output selector. Use text, table, csv, json, or yaml."
    )]
    pub output_format: Option<ListOutputFormat>,
}

/// Arguments for interactive team browsing.
#[derive(Debug, Clone, Args)]
pub struct TeamBrowseArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long, help = "Filter teams by a free-text search.")]
    pub query: Option<String>,
    #[arg(long, help = "Filter teams by exact team name.")]
    pub name: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Include team members and admins in the browse detail view."
    )]
    pub with_members: bool,
    #[arg(
        long,
        default_value_t = 1,
        help = "Result page number for paginated Grafana list APIs."
    )]
    pub page: usize,
    #[arg(long, default_value_t = DEFAULT_PAGE_SIZE, help = "Number of teams to request per page.")]
    pub per_page: usize,
}

/// Struct definition for TeamAddArgs.
#[derive(Debug, Clone, Args)]
pub struct TeamAddArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long, help = "Name for the new Grafana team.")]
    pub name: String,
    #[arg(long, help = "Optional contact email for the new Grafana team.")]
    pub email: Option<String>,
    #[arg(
        long = "member",
        help = "Add one or more members by user id, exact login, or exact email as part of team creation."
    )]
    pub members: Vec<String>,
    #[arg(
        long = "admin",
        help = "Add one or more team admins by user id, exact login, or exact email as part of team creation."
    )]
    pub admins: Vec<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Render the create response as JSON."
    )]
    pub json: bool,
}

/// Struct definition for TeamExportArgs.
#[derive(Debug, Clone, Args)]
pub struct TeamExportArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        default_value = DEFAULT_ACCESS_TEAM_EXPORT_DIR,
        help = "Directory to write teams.json and export-metadata.json."
    )]
    pub export_dir: PathBuf,
    #[arg(
        long,
        default_value_t = false,
        help = "Replace existing export files in the target directory instead of failing."
    )]
    pub overwrite: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Preview export paths without writing files."
    )]
    pub dry_run: bool,
    #[arg(
        long,
        default_value_t = true,
        help = "Include team members and admins in exported team records."
    )]
    pub with_members: bool,
}

/// Struct definition for TeamImportArgs.
#[derive(Debug, Clone, Args)]
pub struct TeamImportArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help = "Import directory that contains teams.json and export-metadata.json."
    )]
    pub import_dir: PathBuf,
    #[arg(
        long,
        default_value_t = false,
        help = "Update matching existing teams instead of failing on duplicates."
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
        help = "Acknowledge destructive team-member synchronization operations."
    )]
    pub yes: bool,
}

/// Struct definition for TeamDiffArgs.
#[derive(Debug, Clone, Args)]
pub struct TeamDiffArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        default_value = "access-teams",
        help = "Diff directory that contains teams.json and export-metadata.json."
    )]
    pub diff_dir: PathBuf,
}

/// Struct definition for TeamModifyArgs.
#[derive(Debug, Clone, Args)]
pub struct TeamModifyArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        conflicts_with = "name",
        help = "Target one team by numeric Grafana team id."
    )]
    pub team_id: Option<String>,
    #[arg(
        long,
        conflicts_with = "team_id",
        help = "Target one team by exact team name."
    )]
    pub name: Option<String>,
    #[arg(
        long = "add-member",
        help = "Add one or more members by user id, exact login, or exact email."
    )]
    pub add_member: Vec<String>,
    #[arg(
        long = "remove-member",
        help = "Remove one or more members by user id, exact login, or exact email."
    )]
    pub remove_member: Vec<String>,
    #[arg(
        long = "add-admin",
        help = "Promote one or more members to team admin by user id, exact login, or exact email."
    )]
    pub add_admin: Vec<String>,
    #[arg(
        long = "remove-admin",
        help = "Remove team-admin status from one or more members by user id, exact login, or exact email."
    )]
    pub remove_admin: Vec<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Render the modify response as JSON."
    )]
    pub json: bool,
}

/// Struct definition for OrgListArgs.
#[derive(Debug, Clone, Args)]
pub struct OrgListArgs {
    #[command(flatten)]
    pub common: CommonCliArgsNoOrgId,
    #[arg(long = "org-id", help = "Filter to one exact organization id.")]
    pub org_id: Option<i64>,
    #[arg(long, help = "Filter organizations by exact name.")]
    pub name: Option<String>,
    #[arg(long, help = "Filter organizations by a free-text search.")]
    pub query: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Include org users and org roles in the rendered output."
    )]
    pub with_users: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["csv", "json", "yaml"], help = "Render org summaries as a table.")]
    pub table: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "json", "yaml"], help = "Render org summaries as CSV.")]
    pub csv: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "csv", "yaml"], help = "Render org summaries as JSON.")]
    pub json: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "csv", "json"], help = "Render org summaries as YAML.")]
    pub yaml: bool,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["table", "csv", "json", "yaml"],
        help = "Alternative single-flag output selector. Use text, table, csv, json, or yaml."
    )]
    pub output_format: Option<ListOutputFormat>,
}

/// Struct definition for OrgAddArgs.
#[derive(Debug, Clone, Args)]
pub struct OrgAddArgs {
    #[command(flatten)]
    pub common: CommonCliArgsNoOrgId,
    #[arg(long, help = "Name for the new Grafana organization.")]
    pub name: String,
    #[arg(
        long,
        default_value_t = false,
        help = "Render the create response as JSON."
    )]
    pub json: bool,
}

/// Struct definition for OrgModifyArgs.
#[derive(Debug, Clone, Args)]
pub struct OrgModifyArgs {
    #[command(flatten)]
    pub common: CommonCliArgsNoOrgId,
    #[arg(
        long = "org-id",
        conflicts_with = "name",
        help = "Target one organization by numeric Grafana org id."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        conflicts_with = "org_id",
        help = "Target one organization by exact name."
    )]
    pub name: Option<String>,
    #[arg(long, help = "Replace the organization name with this new value.")]
    pub set_name: String,
    #[arg(
        long,
        default_value_t = false,
        help = "Render the modify response as JSON."
    )]
    pub json: bool,
}

/// Struct definition for OrgDeleteArgs.
#[derive(Debug, Clone, Args)]
pub struct OrgDeleteArgs {
    #[command(flatten)]
    pub common: CommonCliArgsNoOrgId,
    #[arg(
        long = "org-id",
        conflicts_with = "name",
        help = "Delete one organization by numeric Grafana org id."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        conflicts_with = "org_id",
        help = "Delete one organization by exact name."
    )]
    pub name: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Skip the interactive confirmation prompt."
    )]
    pub yes: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Render the delete response as JSON."
    )]
    pub json: bool,
}

/// Struct definition for OrgExportArgs.
#[derive(Debug, Clone, Args)]
pub struct OrgExportArgs {
    #[command(flatten)]
    pub common: CommonCliArgsNoOrgId,
    #[arg(long = "org-id", help = "Filter export to one exact organization id.")]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value = DEFAULT_ACCESS_ORG_EXPORT_DIR,
        help = "Directory to write orgs.json and export-metadata.json."
    )]
    pub export_dir: PathBuf,
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
    #[arg(long, help = "Filter export to one exact organization name.")]
    pub name: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Include org users and org roles in the export bundle."
    )]
    pub with_users: bool,
}

/// Struct definition for OrgImportArgs.
#[derive(Debug, Clone, Args)]
pub struct OrgImportArgs {
    #[command(flatten)]
    pub common: CommonCliArgsNoOrgId,
    #[arg(
        long,
        help = "Import directory that contains orgs.json and export-metadata.json."
    )]
    pub import_dir: PathBuf,
    #[arg(
        long,
        default_value_t = false,
        help = "Update matching existing orgs or create missing orgs instead of skipping them."
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
        help = "Acknowledge destructive import operations when required."
    )]
    pub yes: bool,
}

/// Struct definition for OrgDiffArgs.
#[derive(Debug, Clone, Args)]
pub struct OrgDiffArgs {
    #[command(flatten)]
    pub common: CommonCliArgsNoOrgId,
    #[arg(
        long,
        default_value = DEFAULT_ACCESS_ORG_EXPORT_DIR,
        help = "Diff directory that contains orgs.json and export-metadata.json."
    )]
    pub diff_dir: PathBuf,
}

/// Enum definition for OrgCommand.
#[derive(Debug, Clone, Subcommand)]
pub enum OrgCommand {
    #[command(after_help = ACCESS_ORG_LIST_HELP_TEXT)]
    List(OrgListArgs),
    #[command(after_help = ACCESS_ORG_ADD_HELP_TEXT)]
    Add(OrgAddArgs),
    #[command(after_help = ACCESS_ORG_MODIFY_HELP_TEXT)]
    Modify(OrgModifyArgs),
    #[command(after_help = ACCESS_ORG_EXPORT_HELP_TEXT)]
    Export(OrgExportArgs),
    #[command(after_help = ACCESS_ORG_IMPORT_HELP_TEXT)]
    Import(OrgImportArgs),
    #[command(after_help = ACCESS_ORG_DIFF_HELP_TEXT)]
    Diff(OrgDiffArgs),
    #[command(after_help = ACCESS_ORG_DELETE_HELP_TEXT)]
    Delete(OrgDeleteArgs),
}

/// Enum definition for TeamCommand.
#[derive(Debug, Clone, Subcommand)]
pub enum TeamCommand {
    #[command(after_help = ACCESS_TEAM_LIST_HELP_TEXT)]
    List(TeamListArgs),
    #[command(after_help = ACCESS_TEAM_BROWSE_HELP_TEXT)]
    Browse(TeamBrowseArgs),
    #[command(after_help = ACCESS_TEAM_ADD_HELP_TEXT)]
    Add(TeamAddArgs),
    #[command(after_help = ACCESS_TEAM_MODIFY_HELP_TEXT)]
    Modify(TeamModifyArgs),
    #[command(after_help = ACCESS_TEAM_EXPORT_HELP_TEXT)]
    Export(TeamExportArgs),
    #[command(after_help = ACCESS_TEAM_IMPORT_HELP_TEXT)]
    Import(TeamImportArgs),
    #[command(after_help = ACCESS_TEAM_DIFF_HELP_TEXT)]
    Diff(TeamDiffArgs),
    #[command(after_help = ACCESS_TEAM_DELETE_HELP_TEXT)]
    Delete(TeamDeleteArgs),
}

/// Enum definition for AccessCommand.
#[derive(Debug, Clone, Subcommand)]
pub enum AccessCommand {
    #[command(after_help = ACCESS_USER_HELP_TEXT)]
    User {
        #[command(subcommand)]
        command: UserCommand,
    },
    #[command(after_help = ACCESS_ORG_HELP_TEXT)]
    Org {
        #[command(subcommand)]
        command: OrgCommand,
    },
    #[command(visible_alias = "group", after_help = ACCESS_TEAM_HELP_TEXT)]
    Team {
        #[command(subcommand)]
        command: TeamCommand,
    },
    #[command(name = "service-account", after_help = ACCESS_SERVICE_ACCOUNT_HELP_TEXT)]
    ServiceAccount {
        #[command(subcommand)]
        command: ServiceAccountCommand,
    },
}

#[derive(Debug, Clone, Parser)]
#[command(
    name = "grafana-util access",
    about = "List and manage Grafana users, orgs, teams, and service accounts.",
    after_help = ACCESS_ROOT_HELP_TEXT,
    styles = crate::help_styles::CLI_HELP_STYLES
)]
/// Struct definition for AccessCliRoot.
pub(crate) struct AccessCliRoot {
    #[command(flatten)]
    args: AccessCliArgs,
}

/// Struct definition for AccessCliArgs.
#[derive(Debug, Clone, Args)]
pub struct AccessCliArgs {
    #[command(subcommand)]
    pub command: AccessCommand,
}
