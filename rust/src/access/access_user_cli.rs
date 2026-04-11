//! CLI definitions for Grafana user access workflows.
//! This module owns the user-list, user-read, user-modify, user-delete, import/export,
//! and browse command wiring. It translates clap arguments into the shared access
//! workflow helpers and carries the user-facing help text for those commands.

use clap::{Args, Subcommand};
use std::path::PathBuf;

use super::access_cli_shared::{
    parse_bool_text, CommonCliArgs, DryRunOutputFormat, ListOutputFormat, Scope,
    ACCESS_USER_ADD_HELP_TEXT, ACCESS_USER_BROWSE_HELP_TEXT, ACCESS_USER_DELETE_HELP_TEXT,
    ACCESS_USER_DIFF_HELP_TEXT, ACCESS_USER_EXPORT_HELP_TEXT, ACCESS_USER_IMPORT_HELP_TEXT,
    ACCESS_USER_LIST_HELP_TEXT, ACCESS_USER_MODIFY_HELP_TEXT, DEFAULT_ACCESS_USER_EXPORT_DIR,
    DEFAULT_PAGE_SIZE,
};

fn parse_user_list_output_column(value: &str) -> std::result::Result<String, String> {
    match value {
        "all" => Ok("all".to_string()),
        "id" => Ok("id".to_string()),
        "login" => Ok("login".to_string()),
        "email" => Ok("email".to_string()),
        "name" => Ok("name".to_string()),
        "org_role" | "orgRole" => Ok("org_role".to_string()),
        "grafana_admin" | "grafanaAdmin" => Ok("grafana_admin".to_string()),
        "scope" => Ok("scope".to_string()),
        "account_scope" | "accountScope" => Ok("account_scope".to_string()),
        "origin" => Ok("origin".to_string()),
        "last_active" | "lastActive" => Ok("last_active".to_string()),
        "teams" => Ok("teams".to_string()),
        _ => Err(format!(
            "Unsupported --output-columns value '{value}'. Supported values: all, id, login, email, name, org_role, grafana_admin, scope, account_scope, origin, last_active, teams."
        )),
    }
}

/// Arguments for listing Grafana users.
#[derive(Debug, Clone, Args)]
pub struct UserListArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help = "List users from a local export bundle directory instead of live Grafana."
    )]
    pub input_dir: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = Scope::Org, help = "List users from the current org scope or from the Grafana global admin scope.")]
    pub scope: Scope,
    #[arg(
        long,
        default_value_t = false,
        help = "Human-friendly alias for --scope global when listing users across all organizations."
    )]
    pub all_orgs: bool,
    #[arg(
        long,
        help = "Filter users by a free-text search across login, email, or display name."
    )]
    pub query: Option<String>,
    #[arg(long, help = "Filter users by exact login.")]
    pub login: Option<String>,
    #[arg(long, help = "Filter users by exact email address.")]
    pub email: Option<String>,
    #[arg(
        long,
        help = "Filter org users by exact Grafana org role such as Viewer, Editor, or Admin."
    )]
    pub org_role: Option<String>,
    #[arg(long, value_parser = parse_bool_text, help = "Filter global users by Grafana server-admin status.")]
    pub grafana_admin: Option<bool>,
    #[arg(
        long,
        default_value_t = false,
        help = "Include each user's current team memberships in the list output."
    )]
    pub with_teams: bool,
    #[arg(
        long,
        value_delimiter = ',',
        value_parser = parse_user_list_output_column,
        help = "For text, table, or csv output, render only these comma-separated columns. Use all to expand every supported column. Supported values: all, id, login, email, name, org_role, grafana_admin, scope, account_scope, origin, last_active, teams. JSON-style aliases like orgRole, grafanaAdmin, accountScope, and lastActive are also accepted."
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
    #[arg(long, default_value_t = DEFAULT_PAGE_SIZE, help = "Number of users to request per page.")]
    pub per_page: usize,
    #[arg(long, default_value_t = false, conflicts_with_all = ["csv", "json", "yaml"], help = "Render user summaries as a table.")]
    pub table: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "json", "yaml"], help = "Render user summaries as CSV.")]
    pub csv: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "csv", "yaml"], help = "Render user summaries as JSON.")]
    pub json: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "csv", "json"], help = "Render user summaries as YAML.")]
    pub yaml: bool,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["table", "csv", "json", "yaml"],
        help = "Alternative single-flag output selector. Use text, table, csv, json, or yaml."
    )]
    pub output_format: Option<ListOutputFormat>,
}

/// Arguments for interactive user browsing.
#[derive(Debug, Clone, Args)]
pub struct UserBrowseArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help = "Browse users from a local export bundle directory instead of live Grafana."
    )]
    pub input_dir: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = Scope::Global, help = "Browse users from the current org scope or from the Grafana global admin scope.")]
    pub scope: Scope,
    #[arg(
        long,
        default_value_t = false,
        help = "Human-friendly alias for --scope global when browsing users across all organizations."
    )]
    pub all_orgs: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "all_orgs",
        help = "Browse only the currently selected organization instead of the default cross-org view."
    )]
    pub current_org: bool,
    #[arg(
        long,
        help = "Filter users by a free-text search across login, email, or display name."
    )]
    pub query: Option<String>,
    #[arg(long, help = "Filter users by exact login.")]
    pub login: Option<String>,
    #[arg(long, help = "Filter users by exact email address.")]
    pub email: Option<String>,
    #[arg(
        long,
        help = "Filter org users by exact Grafana org role such as Viewer, Editor, or Admin."
    )]
    pub org_role: Option<String>,
    #[arg(long, value_parser = parse_bool_text, help = "Filter global users by Grafana server-admin status.")]
    pub grafana_admin: Option<bool>,
    #[arg(
        long,
        default_value_t = false,
        hide = true,
        help = "Deprecated compatibility flag. User browse now always includes team memberships."
    )]
    pub with_teams: bool,
    #[arg(
        long,
        default_value_t = 1,
        help = "Result page number for paginated Grafana list APIs."
    )]
    pub page: usize,
    #[arg(long, default_value_t = DEFAULT_PAGE_SIZE, help = "Number of users to request per page.")]
    pub per_page: usize,
}

/// Create one Grafana user with optional initial org and admin state.
#[derive(Debug, Clone, Args)]
pub struct UserAddArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long, help = "Login name for the new Grafana user.")]
    pub login: String,
    #[arg(long, help = "Email address for the new Grafana user.")]
    pub email: String,
    #[arg(long, help = "Display name for the new Grafana user.")]
    pub name: String,
    #[arg(
        long = "password",
        conflicts_with_all = ["new_user_password_file", "prompt_user_password"],
        help = "Initial password for the new Grafana user."
    )]
    pub new_user_password: Option<String>,
    #[arg(
        long = "password-file",
        conflicts_with_all = ["new_user_password", "prompt_user_password"],
        help = "Read the initial user password from this file."
    )]
    pub new_user_password_file: Option<PathBuf>,
    #[arg(
        long = "prompt-user-password",
        default_value_t = false,
        conflicts_with_all = ["new_user_password", "new_user_password_file"],
        help = "Prompt for the initial user password without echo."
    )]
    pub prompt_user_password: bool,
    #[arg(
        long = "org-role",
        help = "Optional initial org role such as Viewer, Editor, or Admin."
    )]
    pub org_role: Option<String>,
    #[arg(long = "grafana-admin", value_parser = parse_bool_text, help = "Set whether the new user should be a Grafana server admin.")]
    pub grafana_admin: Option<bool>,
    #[arg(
        long,
        default_value_t = false,
        help = "Render the create response as JSON."
    )]
    pub json: bool,
}

/// Struct definition for UserModifyArgs.
#[derive(Debug, Clone, Args)]
pub struct UserModifyArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long, conflicts_with_all = ["login", "email"], help = "Target one user by numeric Grafana user id.")]
    pub user_id: Option<String>,
    #[arg(long, conflicts_with_all = ["user_id", "email"], help = "Target one user by exact login.")]
    pub login: Option<String>,
    #[arg(long, conflicts_with_all = ["user_id", "login"], help = "Target one user by exact email address.")]
    pub email: Option<String>,
    #[arg(long, help = "Replace the user's login with this new value.")]
    pub set_login: Option<String>,
    #[arg(long, help = "Replace the user's email address with this new value.")]
    pub set_email: Option<String>,
    #[arg(long, help = "Replace the user's display name with this new value.")]
    pub set_name: Option<String>,
    #[arg(
        long,
        conflicts_with_all = ["set_password_file", "prompt_set_password"],
        help = "Replace the user's password with this new value."
    )]
    pub set_password: Option<String>,
    #[arg(
        long = "set-password-file",
        conflicts_with_all = ["set_password", "prompt_set_password"],
        help = "Read the replacement user password from this file."
    )]
    pub set_password_file: Option<PathBuf>,
    #[arg(
        long = "prompt-set-password",
        default_value_t = false,
        conflicts_with_all = ["set_password", "set_password_file"],
        help = "Prompt for the replacement user password without echo."
    )]
    pub prompt_set_password: bool,
    #[arg(long, help = "Change the user's org role to this value.")]
    pub set_org_role: Option<String>,
    #[arg(long, value_parser = parse_bool_text, help = "Change whether the user is a Grafana server admin.")]
    pub set_grafana_admin: Option<bool>,
    #[arg(
        long,
        default_value_t = false,
        help = "Render the modify response as JSON."
    )]
    pub json: bool,
}

/// Struct definition for UserDeleteArgs.
#[derive(Debug, Clone, Args)]
pub struct UserDeleteArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long, conflicts_with_all = ["login", "email"], help = "Delete one user by numeric Grafana user id.")]
    pub user_id: Option<String>,
    #[arg(long, conflicts_with_all = ["user_id", "email"], help = "Delete one user by exact login.")]
    pub login: Option<String>,
    #[arg(long, conflicts_with_all = ["user_id", "login"], help = "Delete one user by exact email address.")]
    pub email: Option<String>,
    #[arg(
        long,
        value_enum,
        help = "Delete from the org membership or from the Grafana global user registry. Defaults to global in non-prompt mode."
    )]
    pub scope: Option<Scope>,
    #[arg(
        long,
        default_value_t = false,
        help = "Prompt for the target user, show a terminal confirmation, and then delete."
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

/// Struct definition for UserExportArgs.
#[derive(Debug, Clone, Args)]
pub struct UserExportArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long = "output-dir",
        default_value = DEFAULT_ACCESS_USER_EXPORT_DIR,
        help = "Directory to write users.json and export-metadata.json."
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
        value_enum,
        default_value_t = Scope::Org,
        help = "Export org-scoped or global users (default: org)."
    )]
    pub scope: Scope,
    #[arg(
        long,
        default_value_t = false,
        help = "Include each user's current team memberships in the export file."
    )]
    pub with_teams: bool,
}

/// Struct definition for UserImportArgs.
#[derive(Debug, Clone, Args)]
pub struct UserImportArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long = "input-dir",
        help = "Import directory that contains users.json and export-metadata.json."
    )]
    pub input_dir: PathBuf,
    #[arg(
        long,
        value_enum,
        default_value_t = Scope::Org,
        help = "Import match strategy for users: global or org scope (default: org)."
    )]
    pub scope: Scope,
    #[arg(
        long,
        default_value_t = false,
        help = "Update matching existing items instead of failing import on duplicates."
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
        help = "Acknowledge destructive import operations (remove/missing sync)."
    )]
    pub yes: bool,
}

/// Struct definition for UserDiffArgs.
#[derive(Debug, Clone, Args)]
pub struct UserDiffArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        default_value = "access-users",
        help = "Diff directory that contains users.json and export-metadata.json."
    )]
    pub diff_dir: PathBuf,
    #[arg(
        long,
        value_enum,
        default_value_t = Scope::Org,
        help = "Compare against org-scoped or global users (default: org)."
    )]
    pub scope: Scope,
}

/// User inventory and lifecycle workflows.
#[derive(Debug, Clone, Subcommand)]
pub enum UserCommand {
    #[command(after_help = ACCESS_USER_LIST_HELP_TEXT)]
    List(UserListArgs),
    #[command(after_help = ACCESS_USER_BROWSE_HELP_TEXT)]
    Browse(UserBrowseArgs),
    #[command(after_help = ACCESS_USER_ADD_HELP_TEXT)]
    Add(UserAddArgs),
    #[command(after_help = ACCESS_USER_MODIFY_HELP_TEXT)]
    Modify(UserModifyArgs),
    #[command(after_help = ACCESS_USER_EXPORT_HELP_TEXT)]
    Export(UserExportArgs),
    #[command(after_help = ACCESS_USER_IMPORT_HELP_TEXT)]
    Import(UserImportArgs),
    #[command(after_help = ACCESS_USER_DIFF_HELP_TEXT)]
    Diff(UserDiffArgs),
    #[command(after_help = ACCESS_USER_DELETE_HELP_TEXT)]
    Delete(UserDeleteArgs),
}
