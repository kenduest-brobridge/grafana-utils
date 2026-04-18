//! Clap schema for access-management CLI commands.
//! Centralizes CLI argument enums and parser-normalization helpers for access handlers.
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[path = "access_cli_runtime.rs"]
mod access_cli_runtime;

#[path = "access_cli_shared.rs"]
mod access_cli_shared;

#[path = "access_service_account_cli.rs"]
mod access_service_account_cli;

use super::pending_delete::TeamDeleteArgs;
use crate::common::CliColorChoice;

pub use access_cli_runtime::{
    build_auth_context, build_auth_context_no_org_id, build_http_client,
    build_http_client_no_org_id, normalize_access_cli_args, parse_cli_from, root_command,
    AccessAuthContext,
};
pub(crate) use access_cli_runtime::{
    materialize_access_common_auth, materialize_access_common_auth_no_org_id,
};
pub use access_cli_shared::{
    CommonCliArgs, CommonCliArgsNoOrgId, DryRunOutputFormat, ListOutputFormat, PlanOutputFormat,
    Scope, ACCESS_EXPORT_KIND_ORGS, ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS, ACCESS_EXPORT_KIND_TEAMS,
    ACCESS_EXPORT_KIND_USERS, ACCESS_EXPORT_METADATA_FILENAME, ACCESS_EXPORT_VERSION,
    ACCESS_ORG_EXPORT_FILENAME, ACCESS_SERVICE_ACCOUNT_EXPORT_FILENAME,
    ACCESS_TEAM_EXPORT_FILENAME, ACCESS_USER_EXPORT_FILENAME, DEFAULT_ACCESS_ORG_EXPORT_DIR,
    DEFAULT_ACCESS_TEAM_EXPORT_DIR, DEFAULT_PAGE_SIZE, DEFAULT_TIMEOUT, DEFAULT_URL,
};
use access_cli_shared::{
    ACCESS_ORG_ADD_HELP_TEXT, ACCESS_ORG_DELETE_HELP_TEXT, ACCESS_ORG_DIFF_HELP_TEXT,
    ACCESS_ORG_EXPORT_HELP_TEXT, ACCESS_ORG_HELP_TEXT, ACCESS_ORG_IMPORT_HELP_TEXT,
    ACCESS_ORG_LIST_HELP_TEXT, ACCESS_ORG_MODIFY_HELP_TEXT, ACCESS_PLAN_HELP_TEXT,
    ACCESS_ROOT_HELP_TEXT, ACCESS_TEAM_ADD_HELP_TEXT, ACCESS_TEAM_BROWSE_HELP_TEXT,
    ACCESS_TEAM_DELETE_HELP_TEXT, ACCESS_TEAM_DIFF_HELP_TEXT, ACCESS_TEAM_EXPORT_HELP_TEXT,
    ACCESS_TEAM_HELP_TEXT, ACCESS_TEAM_IMPORT_HELP_TEXT, ACCESS_TEAM_LIST_HELP_TEXT,
    ACCESS_TEAM_MODIFY_HELP_TEXT, ACCESS_USER_HELP_TEXT,
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

fn parse_team_list_output_column(value: &str) -> std::result::Result<String, String> {
    match value {
        "all" => Ok("all".to_string()),
        "id" => Ok("id".to_string()),
        "name" => Ok("name".to_string()),
        "email" => Ok("email".to_string()),
        "member_count" | "memberCount" => Ok("member_count".to_string()),
        "members" => Ok("members".to_string()),
        _ => Err(format!(
            "Unsupported --output-columns value '{value}'. Supported values: all, id, name, email, member_count, members."
        )),
    }
}

fn parse_access_plan_output_column(value: &str) -> std::result::Result<String, String> {
    match value {
        "all" => Ok("all".to_string()),
        "action_id" | "actionId" => Ok("action_id".to_string()),
        "resource_kind" | "resourceKind" => Ok("resource_kind".to_string()),
        "identity" => Ok("identity".to_string()),
        "action" => Ok("action".to_string()),
        "status" => Ok("status".to_string()),
        "changed_fields" | "changedFields" => Ok("changed_fields".to_string()),
        "changes" => Ok("changes".to_string()),
        "target" => Ok("target".to_string()),
        "blocked_reason" | "blockedReason" => Ok("blocked_reason".to_string()),
        "review_hints" | "reviewHints" => Ok("review_hints".to_string()),
        "source_path" | "sourcePath" => Ok("source_path".to_string()),
        _ => Err(format!(
            "Unsupported --output-columns value '{value}'. Supported values: all, action_id, resource_kind, identity, action, status, changed_fields, changes, target, blocked_reason, review_hints, source_path."
        )),
    }
}

/// Struct definition for TeamListArgs.
#[derive(Debug, Clone, Args)]
pub struct TeamListArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help = "List teams from a local export bundle directory instead of live Grafana."
    )]
    pub input_dir: Option<PathBuf>,
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
        value_delimiter = ',',
        value_parser = parse_team_list_output_column,
        help = "For text, table, or csv output, render only these comma-separated columns. Use all to expand every supported column. Supported values: all, id, name, email, member_count, members. JSON-style aliases like memberCount are also accepted."
    )]
    pub output_columns: Vec<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Print the supported --output-columns values and exit."
    )]
    pub list_columns: bool,
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
    #[arg(
        long,
        help = "Browse teams from a local export bundle directory instead of live Grafana."
    )]
    pub input_dir: Option<PathBuf>,
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

/// Create one Grafana team with optional contact and membership data.
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
        long = "output-dir",
        default_value = DEFAULT_ACCESS_TEAM_EXPORT_DIR,
        help = "Directory to write teams.json and export-metadata.json."
    )]
    pub output_dir: PathBuf,
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
        long = "input-dir",
        help = "Import directory that contains teams.json and export-metadata.json."
    )]
    pub input_dir: PathBuf,
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
    #[arg(
        long,
        help = "List organizations from a local export bundle directory instead of live Grafana."
    )]
    pub input_dir: Option<PathBuf>,
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
        help = "Prompt for the target organization, show a terminal confirmation, and then delete."
    )]
    pub prompt: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Skip the terminal confirmation prompt in non-prompt mode."
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
        long = "output-dir",
        default_value = DEFAULT_ACCESS_ORG_EXPORT_DIR,
        help = "Directory to write orgs.json and export-metadata.json."
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
        long = "input-dir",
        help = "Import directory that contains orgs.json and export-metadata.json."
    )]
    pub input_dir: PathBuf,
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

/// Resource selector for access plan review flows.
#[derive(Debug, Clone, ValueEnum, PartialEq, Eq)]
pub enum AccessPlanResource {
    User,
    Team,
    Org,
    #[value(name = "service-account")]
    ServiceAccount,
    All,
}

/// Access plan review arguments.
#[derive(Debug, Clone, Args)]
pub struct AccessPlanArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long = "input-dir",
        help = "Directory that contains one or more access export bundles to review."
    )]
    pub input_dir: PathBuf,
    #[arg(
        long,
        value_enum,
        default_value_t = AccessPlanResource::User,
        help = "Select which access resource bundle to plan. The current vertical slice supports user bundles."
    )]
    pub resource: AccessPlanResource,
    #[arg(
        long,
        default_value_t = false,
        help = "Include remote-only resources as delete candidates instead of just reporting them."
    )]
    pub prune: bool,
    #[arg(
        long,
        value_delimiter = ',',
        value_parser = parse_access_plan_output_column,
        help = "For text or table output, render only these comma-separated columns. Use all to expand every supported column. Supported values: all, action_id, resource_kind, identity, action, status, changed_fields, changes, target, blocked_reason, review_hints, source_path. JSON-style aliases like actionId and resourceKind are also accepted."
    )]
    pub output_columns: Vec<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Print the supported --output-columns values and exit."
    )]
    pub list_columns: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Hide the header row from table output."
    )]
    pub no_header: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Keep same-state rows visible in table output."
    )]
    pub show_same: bool,
    #[arg(
        long,
        value_enum,
        default_value_t = PlanOutputFormat::Text,
        help = "Output format for access plan review. Use text, table, or json."
    )]
    pub output_format: PlanOutputFormat,
}

/// Organization inventory and lifecycle workflows.
#[derive(Debug, Clone, Subcommand)]
pub enum OrgCommand {
    #[command(
        about = "List live or local Grafana organizations.",
        after_help = ACCESS_ORG_LIST_HELP_TEXT
    )]
    List(OrgListArgs),
    #[command(
        about = "Create one Grafana organization.",
        after_help = ACCESS_ORG_ADD_HELP_TEXT
    )]
    Add(OrgAddArgs),
    #[command(
        about = "Rename one Grafana organization.",
        after_help = ACCESS_ORG_MODIFY_HELP_TEXT
    )]
    Modify(OrgModifyArgs),
    #[command(
        about = "Export Grafana organizations into a local reviewable bundle.",
        after_help = ACCESS_ORG_EXPORT_HELP_TEXT
    )]
    Export(OrgExportArgs),
    #[command(
        about = "Import Grafana organizations from a local bundle.",
        after_help = ACCESS_ORG_IMPORT_HELP_TEXT
    )]
    Import(OrgImportArgs),
    #[command(
        about = "Compare a local organization bundle against live Grafana organizations.",
        after_help = ACCESS_ORG_DIFF_HELP_TEXT
    )]
    Diff(OrgDiffArgs),
    #[command(
        about = "Delete one Grafana organization.",
        after_help = ACCESS_ORG_DELETE_HELP_TEXT
    )]
    Delete(OrgDeleteArgs),
}

/// Team inventory and membership workflows.
#[derive(Debug, Clone, Subcommand)]
pub enum TeamCommand {
    #[command(
        about = "List live or local Grafana teams.",
        after_help = ACCESS_TEAM_LIST_HELP_TEXT
    )]
    List(TeamListArgs),
    #[command(
        about = "Browse live or local Grafana teams interactively.",
        after_help = ACCESS_TEAM_BROWSE_HELP_TEXT
    )]
    Browse(TeamBrowseArgs),
    #[command(
        about = "Create one Grafana team with optional contact and membership data.",
        after_help = ACCESS_TEAM_ADD_HELP_TEXT
    )]
    Add(TeamAddArgs),
    #[command(
        about = "Modify one Grafana team and its membership.",
        after_help = ACCESS_TEAM_MODIFY_HELP_TEXT
    )]
    Modify(TeamModifyArgs),
    #[command(
        about = "Export Grafana teams into a local reviewable bundle.",
        after_help = ACCESS_TEAM_EXPORT_HELP_TEXT
    )]
    Export(TeamExportArgs),
    #[command(
        about = "Import Grafana teams from a local bundle.",
        after_help = ACCESS_TEAM_IMPORT_HELP_TEXT
    )]
    Import(TeamImportArgs),
    #[command(
        about = "Compare a local team bundle against live Grafana teams.",
        after_help = ACCESS_TEAM_DIFF_HELP_TEXT
    )]
    Diff(TeamDiffArgs),
    #[command(
        about = "Delete one Grafana team.",
        after_help = ACCESS_TEAM_DELETE_HELP_TEXT
    )]
    Delete(TeamDeleteArgs),
}

/// User, organization, team, and service-account workflows.
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
    #[command(after_help = ACCESS_PLAN_HELP_TEXT)]
    Plan(AccessPlanArgs),
}

#[derive(Debug, Clone, Parser)]
#[command(
    name = "grafana-util access",
    about = "List and manage Grafana users, orgs, teams, and service accounts.",
    after_help = ACCESS_ROOT_HELP_TEXT,
    infer_long_args(true),
    styles = crate::help_styles::CLI_HELP_STYLES
)]
/// Struct definition for AccessCliRoot.
pub(crate) struct AccessCliRoot {
    #[arg(
        long,
        value_enum,
        default_value_t = CliColorChoice::Auto,
        help = "Colorize JSON output. Use auto, always, or never."
    )]
    color: CliColorChoice,
    #[command(flatten)]
    args: AccessCliArgs,
}

/// Struct definition for AccessCliArgs.
#[derive(Debug, Clone, Args)]
pub struct AccessCliArgs {
    #[command(subcommand)]
    pub command: AccessCommand,
}
