use clap::{Args, Subcommand};
use std::path::PathBuf;

use super::access_cli_shared::{
    parse_bool_text, CommonCliArgs, DryRunOutputFormat, ListOutputFormat, Scope,
    ACCESS_USER_ADD_HELP_TEXT, DEFAULT_ACCESS_USER_EXPORT_DIR, DEFAULT_PAGE_SIZE,
};

/// Arguments for listing Grafana users.
#[derive(Debug, Clone, Args)]
pub struct UserListArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
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
        default_value_t = 1,
        help = "Result page number for paginated Grafana list APIs."
    )]
    pub page: usize,
    #[arg(long, default_value_t = DEFAULT_PAGE_SIZE, help = "Number of users to request per page.")]
    pub per_page: usize,
    #[arg(long, default_value_t = false, conflicts_with_all = ["csv", "json"], help = "Render user summaries as a table.")]
    pub table: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "json"], help = "Render user summaries as CSV.")]
    pub csv: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "csv"], help = "Render user summaries as JSON.")]
    pub json: bool,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["table", "csv", "json"],
        help = "Alternative single-flag output selector. Use text, table, csv, or json."
    )]
    pub output_format: Option<ListOutputFormat>,
}

/// Arguments for interactive user browsing.
#[derive(Debug, Clone, Args)]
pub struct UserBrowseArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
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

/// Struct definition for UserAddArgs.
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
    #[arg(long, value_enum, default_value_t = Scope::Global, help = "Delete from the org membership only or from the Grafana global user registry.")]
    pub scope: Scope,
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

/// Struct definition for UserExportArgs.
#[derive(Debug, Clone, Args)]
pub struct UserExportArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        default_value = DEFAULT_ACCESS_USER_EXPORT_DIR,
        help = "Directory to write users.json and export-metadata.json."
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
        long,
        help = "Import directory that contains users.json and export-metadata.json."
    )]
    pub import_dir: PathBuf,
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

/// Enum definition for UserCommand.
#[derive(Debug, Clone, Subcommand)]
pub enum UserCommand {
    List(UserListArgs),
    Browse(UserBrowseArgs),
    #[command(after_help = ACCESS_USER_ADD_HELP_TEXT)]
    Add(UserAddArgs),
    Modify(UserModifyArgs),
    Export(UserExportArgs),
    Import(UserImportArgs),
    Diff(UserDiffArgs),
    Delete(UserDeleteArgs),
}
