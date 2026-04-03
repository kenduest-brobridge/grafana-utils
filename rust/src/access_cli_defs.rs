//! Clap schema for access-management CLI commands.
//! Centralizes CLI argument enums and parser-normalization helpers for access handlers.
use clap::{Args, Command, CommandFactory, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use super::access_pending_delete::{
    ServiceAccountDeleteArgs, ServiceAccountTokenDeleteArgs, TeamDeleteArgs,
};
use crate::common::{resolve_auth_headers, Result};
use crate::http::{JsonHttpClient, JsonHttpClientConfig};

pub const DEFAULT_URL: &str = "http://127.0.0.1:3000";
pub const DEFAULT_TIMEOUT: u64 = 30;
pub const DEFAULT_PAGE_SIZE: usize = 100;
pub const DEFAULT_ACCESS_USER_EXPORT_DIR: &str = "access-users";
pub const DEFAULT_ACCESS_TEAM_EXPORT_DIR: &str = "access-teams";
pub const DEFAULT_ACCESS_ORG_EXPORT_DIR: &str = "access-orgs";
pub const DEFAULT_ACCESS_SERVICE_ACCOUNT_EXPORT_DIR: &str = "access-service-accounts";
pub const ACCESS_EXPORT_KIND_USERS: &str = "grafana-utils-access-user-export-index";
pub const ACCESS_EXPORT_KIND_TEAMS: &str = "grafana-utils-access-team-export-index";
pub const ACCESS_EXPORT_KIND_ORGS: &str = "grafana-utils-access-org-export-index";
pub const ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS: &str =
    "grafana-utils-access-service-account-export-index";
pub const ACCESS_EXPORT_VERSION: i64 = 1;
pub const ACCESS_USER_EXPORT_FILENAME: &str = "users.json";
pub const ACCESS_TEAM_EXPORT_FILENAME: &str = "teams.json";
pub const ACCESS_ORG_EXPORT_FILENAME: &str = "orgs.json";
pub const ACCESS_SERVICE_ACCOUNT_EXPORT_FILENAME: &str = "service-accounts.json";
pub const ACCESS_EXPORT_METADATA_FILENAME: &str = "export-metadata.json";
const ACCESS_HELP_EXAMPLES: &str =
    "Examples:\n\n  List org-scoped users as a table:\n    grafana-util access user list --url http://localhost:3000 --table\n\n  Export organizations with membership data:\n    grafana-util access org export --url http://localhost:3000 --with-users --export-dir ./access-orgs --overwrite\n\n  Create a service-account token:\n    grafana-util access service-account token add --url http://localhost:3000 --name automation --token-name ci-token --seconds-to-live 3600";
const ACCESS_USER_GROUP_HELP_EXAMPLES: &str =
    "Examples:\n\n  List org users as a table:\n    grafana-util access user list --url http://localhost:3000 --table\n\n  Export global users with team memberships:\n    grafana-util access user export --url http://localhost:3000 --scope global --with-teams --export-dir ./access-users --overwrite";
const ACCESS_ORG_GROUP_HELP_EXAMPLES: &str =
    "Examples:\n\n  List organizations with memberships:\n    grafana-util access org list --url http://localhost:3000 --with-users --table\n\n  Preview org import changes:\n    grafana-util access org import --url http://localhost:3000 --import-dir ./access-orgs --replace-existing --dry-run";
const ACCESS_TEAM_GROUP_HELP_EXAMPLES: &str =
    "Examples:\n\n  List teams with members:\n    grafana-util access team list --url http://localhost:3000 --with-members --table\n\n  Apply a team import:\n    grafana-util access team import --url http://localhost:3000 --import-dir ./access-teams --replace-existing --yes";
const ACCESS_SERVICE_ACCOUNT_GROUP_HELP_EXAMPLES: &str =
    "Examples:\n\n  List service accounts as a table:\n    grafana-util access service-account list --url http://localhost:3000 --table\n\n  Create one token:\n    grafana-util access service-account token add --url http://localhost:3000 --name automation --token-name ci-token --seconds-to-live 3600";
const ACCESS_SERVICE_ACCOUNT_TOKEN_GROUP_HELP_EXAMPLES: &str =
    "Examples:\n\n  Create a service-account token:\n    grafana-util access service-account token add --url http://localhost:3000 --name automation --token-name ci-token --seconds-to-live 3600\n\n  Delete a service-account token:\n    grafana-util access service-account token delete --url http://localhost:3000 --name automation --token-name ci-token --yes";
const ACCESS_USER_LIST_HELP_EXAMPLES: &str =
    "Examples:\n\n  Table output for org users:\n    grafana-util access user list --url http://localhost:3000 --scope org --table\n\n  Global user JSON with team memberships:\n    grafana-util access user list --url http://localhost:3000 --scope global --with-teams --output-format json";
const ACCESS_USER_ADD_HELP_EXAMPLES: &str =
    "Examples:\n\n  Create a new user with an explicit password:\n    grafana-util access user add --url http://localhost:3000 --login alice --email alice@example.com --name Alice --password change-me\n\n  Prompt for the initial password and set org role:\n    grafana-util access user add --url http://localhost:3000 --login bob --email bob@example.com --name Bob --prompt-user-password --org-role Editor";
const ACCESS_USER_MODIFY_HELP_EXAMPLES: &str =
    "Examples:\n\n  Change a user email by login:\n    grafana-util access user modify --url http://localhost:3000 --login alice --set-email alice+grafana@example.com\n\n  Promote a user and reset the password:\n    grafana-util access user modify --url http://localhost:3000 --login alice --set-org-role Admin --prompt-set-password";
const ACCESS_USER_EXPORT_HELP_EXAMPLES: &str =
    "Examples:\n\n  Export current-org users:\n    grafana-util access user export --url http://localhost:3000 --export-dir ./access-users --overwrite\n\n  Export global users with team memberships:\n    grafana-util access user export --url http://localhost:3000 --scope global --with-teams --export-dir ./access-users --overwrite";
const ACCESS_USER_IMPORT_HELP_EXAMPLES: &str =
    "Examples:\n\n  Preview user import actions as a table:\n    grafana-util access user import --url http://localhost:3000 --import-dir ./access-users --replace-existing --dry-run --output-format table\n\n  Apply the import and acknowledge destructive sync:\n    grafana-util access user import --url http://localhost:3000 --import-dir ./access-users --replace-existing --yes";
const ACCESS_USER_DIFF_HELP_EXAMPLES: &str =
    "Examples:\n\n  Compare local user exports against the current org:\n    grafana-util access user diff --url http://localhost:3000 --diff-dir ./access-users\n\n  Compare the global user registry:\n    grafana-util access user diff --url http://localhost:3000 --scope global --diff-dir ./access-users";
const ACCESS_USER_DELETE_HELP_EXAMPLES: &str =
    "Examples:\n\n  Remove a user from the current org by login:\n    grafana-util access user delete --url http://localhost:3000 --login alice --scope org --yes\n\n  Delete a global user by email:\n    grafana-util access user delete --url http://localhost:3000 --email alice@example.com --scope global --yes";
const ACCESS_ORG_LIST_HELP_EXAMPLES: &str =
    "Examples:\n\n  Table output for all orgs:\n    grafana-util access org list --url http://localhost:3000 --table\n\n  JSON output with org users included:\n    grafana-util access org list --url http://localhost:3000 --with-users --output-format json";
const ACCESS_ORG_ADD_HELP_EXAMPLES: &str =
    "Examples:\n\n  Create a new organization:\n    grafana-util access org add --url http://localhost:3000 --name \"QA Org\"\n\n  Render the create response as JSON:\n    grafana-util access org add --url http://localhost:3000 --name \"Audit Org\" --json";
const ACCESS_ORG_MODIFY_HELP_EXAMPLES: &str =
    "Examples:\n\n  Rename one org by id:\n    grafana-util access org modify --url http://localhost:3000 --org-id 2 --set-name \"QA Org\"\n\n  Rename one org by exact name:\n    grafana-util access org modify --url http://localhost:3000 --name \"Audit Org\" --set-name \"Audit Org Archived\"";
const ACCESS_ORG_EXPORT_HELP_EXAMPLES: &str =
    "Examples:\n\n  Export org inventory:\n    grafana-util access org export --url http://localhost:3000 --export-dir ./access-orgs --overwrite\n\n  Export one org with its users:\n    grafana-util access org export --url http://localhost:3000 --org-id 2 --with-users --export-dir ./access-orgs --overwrite";
const ACCESS_ORG_IMPORT_HELP_EXAMPLES: &str =
    "Examples:\n\n  Preview org import changes:\n    grafana-util access org import --url http://localhost:3000 --import-dir ./access-orgs --replace-existing --dry-run\n\n  Apply the org import:\n    grafana-util access org import --url http://localhost:3000 --import-dir ./access-orgs --replace-existing --yes";
const ACCESS_ORG_DELETE_HELP_EXAMPLES: &str =
    "Examples:\n\n  Delete an org by id:\n    grafana-util access org delete --url http://localhost:3000 --org-id 4 --yes\n\n  Delete an org by exact name:\n    grafana-util access org delete --url http://localhost:3000 --name \"Audit Org\" --yes";
const ACCESS_TEAM_LIST_HELP_EXAMPLES: &str =
    "Examples:\n\n  Table output for teams:\n    grafana-util access team list --url http://localhost:3000 --table\n\n  JSON output with team members included:\n    grafana-util access team list --url http://localhost:3000 --with-members --output-format json";
const ACCESS_TEAM_ADD_HELP_EXAMPLES: &str =
    "Examples:\n\n  Create a team with initial members:\n    grafana-util access team add --url http://localhost:3000 --name platform --member alice --member bob\n\n  Create a team with an initial admin and email:\n    grafana-util access team add --url http://localhost:3000 --name sre --email sre@example.com --admin alice@example.com";
const ACCESS_TEAM_MODIFY_HELP_EXAMPLES: &str =
    "Examples:\n\n  Add a member by team name:\n    grafana-util access team modify --url http://localhost:3000 --name platform --add-member alice\n\n  Promote one member to team admin:\n    grafana-util access team modify --url http://localhost:3000 --team-id 7 --add-admin alice@example.com";
const ACCESS_TEAM_EXPORT_HELP_EXAMPLES: &str =
    "Examples:\n\n  Export teams with members:\n    grafana-util access team export --url http://localhost:3000 --export-dir ./access-teams --overwrite\n\n  Preview export paths only:\n    grafana-util access team export --url http://localhost:3000 --export-dir ./access-teams --dry-run";
const ACCESS_TEAM_IMPORT_HELP_EXAMPLES: &str =
    "Examples:\n\n  Preview team import changes as a table:\n    grafana-util access team import --url http://localhost:3000 --import-dir ./access-teams --replace-existing --dry-run --output-format table\n\n  Apply the import and acknowledge membership sync:\n    grafana-util access team import --url http://localhost:3000 --import-dir ./access-teams --replace-existing --yes";
const ACCESS_TEAM_DIFF_HELP_EXAMPLES: &str =
    "Examples:\n\n  Compare local team exports against Grafana:\n    grafana-util access team diff --url http://localhost:3000 --diff-dir ./access-teams";
const ACCESS_TEAM_DELETE_HELP_EXAMPLES: &str =
    "Examples:\n\n  Delete a team by id:\n    grafana-util access team delete --url http://localhost:3000 --team-id 7 --yes\n\n  Delete a team by exact name:\n    grafana-util access team delete --url http://localhost:3000 --name platform --yes";
const ACCESS_SERVICE_ACCOUNT_LIST_HELP_EXAMPLES: &str =
    "Examples:\n\n  Table output for service accounts:\n    grafana-util access service-account list --url http://localhost:3000 --table\n\n  JSON output for scripting:\n    grafana-util access service-account list --url http://localhost:3000 --output-format json";
const ACCESS_SERVICE_ACCOUNT_ADD_HELP_EXAMPLES: &str =
    "Examples:\n\n  Create a Viewer service account:\n    grafana-util access service-account add --url http://localhost:3000 --name automation --role Viewer\n\n  Create a disabled Editor service account:\n    grafana-util access service-account add --url http://localhost:3000 --name qa-robot --role Editor --disabled true";
const ACCESS_SERVICE_ACCOUNT_EXPORT_HELP_EXAMPLES: &str =
    "Examples:\n\n  Export service accounts:\n    grafana-util access service-account export --url http://localhost:3000 --export-dir ./access-service-accounts --overwrite";
const ACCESS_SERVICE_ACCOUNT_IMPORT_HELP_EXAMPLES: &str =
    "Examples:\n\n  Preview service-account import actions as JSON:\n    grafana-util access service-account import --url http://localhost:3000 --import-dir ./access-service-accounts --replace-existing --dry-run --output-format json\n\n  Apply the import:\n    grafana-util access service-account import --url http://localhost:3000 --import-dir ./access-service-accounts --replace-existing --yes";
const ACCESS_SERVICE_ACCOUNT_DIFF_HELP_EXAMPLES: &str =
    "Examples:\n\n  Compare local service-account exports against Grafana:\n    grafana-util access service-account diff --url http://localhost:3000 --diff-dir ./access-service-accounts";
const ACCESS_SERVICE_ACCOUNT_DELETE_HELP_EXAMPLES: &str =
    "Examples:\n\n  Delete a service account by id:\n    grafana-util access service-account delete --url http://localhost:3000 --service-account-id 9 --yes\n\n  Delete a service account by exact name:\n    grafana-util access service-account delete --url http://localhost:3000 --name automation --yes";
const ACCESS_SERVICE_ACCOUNT_TOKEN_ADD_HELP_EXAMPLES: &str =
    "Examples:\n\n  Create a token by service-account name:\n    grafana-util access service-account token add --url http://localhost:3000 --name automation --token-name ci-token --seconds-to-live 3600\n\n  Render the token response as JSON:\n    grafana-util access service-account token add --url http://localhost:3000 --service-account-id 9 --token-name bootstrap --json";
const ACCESS_SERVICE_ACCOUNT_TOKEN_DELETE_HELP_EXAMPLES: &str =
    "Examples:\n\n  Delete a token by service-account name:\n    grafana-util access service-account token delete --url http://localhost:3000 --name automation --token-id 11 --yes\n\n  Delete a token by service-account id:\n    grafana-util access service-account token delete --url http://localhost:3000 --service-account-id 9 --token-id 11 --yes";

#[derive(Debug, Clone, Args)]
pub struct CommonCliArgs {
    #[arg(
        long,
        default_value = DEFAULT_URL,
        help_heading = "Connection And Auth",
        help = "Grafana base URL."
    )]
    pub url: String,
    #[arg(
        long = "token",
        visible_alias = "api-token",
        help_heading = "Connection And Auth",
        help = "Grafana API token. Preferred flag: --token. Falls back to GRAFANA_API_TOKEN."
    )]
    pub api_token: Option<String>,
    #[arg(
        long = "basic-user",
        help_heading = "Connection And Auth",
        help = "Grafana Basic auth username. Preferred flag: --basic-user. Falls back to GRAFANA_USERNAME."
    )]
    pub username: Option<String>,
    #[arg(
        long = "basic-password",
        help_heading = "Connection And Auth",
        help = "Grafana Basic auth password. Preferred flag: --basic-password. Falls back to GRAFANA_PASSWORD."
    )]
    pub password: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help_heading = "Connection And Auth",
        help = "Prompt for the Grafana Basic auth password."
    )]
    pub prompt_password: bool,
    #[arg(
        long,
        default_value_t = false,
        help_heading = "Connection And Auth",
        help = "Prompt for the Grafana API token without echo instead of passing --token on the command line."
    )]
    pub prompt_token: bool,
    #[arg(
        long,
        help_heading = "Connection And Auth",
        help = "Grafana organization id to send through X-Grafana-Org-Id."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = DEFAULT_TIMEOUT,
        help_heading = "Connection And Auth",
        help = "HTTP timeout in seconds."
    )]
    pub timeout: u64,
    #[arg(
        long,
        default_value_t = false,
        help_heading = "Connection And Auth",
        help = "Enable TLS certificate verification. Verification is disabled by default."
    )]
    pub verify_ssl: bool,
}

#[derive(Debug, Clone, Args)]
pub struct CommonCliArgsNoOrgId {
    #[arg(
        long,
        default_value = DEFAULT_URL,
        help_heading = "Connection And Auth",
        help = "Grafana base URL."
    )]
    pub url: String,
    #[arg(
        long = "token",
        visible_alias = "api-token",
        help_heading = "Connection And Auth",
        help = "Grafana API token. Preferred flag: --token. Falls back to GRAFANA_API_TOKEN."
    )]
    pub api_token: Option<String>,
    #[arg(
        long = "basic-user",
        help_heading = "Connection And Auth",
        help = "Grafana Basic auth username. Preferred flag: --basic-user. Falls back to GRAFANA_USERNAME."
    )]
    pub username: Option<String>,
    #[arg(
        long = "basic-password",
        help_heading = "Connection And Auth",
        help = "Grafana Basic auth password. Preferred flag: --basic-password. Falls back to GRAFANA_PASSWORD."
    )]
    pub password: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help_heading = "Connection And Auth",
        help = "Prompt for the Grafana Basic auth password."
    )]
    pub prompt_password: bool,
    #[arg(
        long,
        default_value_t = false,
        help_heading = "Connection And Auth",
        help = "Prompt for the Grafana API token without echo instead of passing --token on the command line."
    )]
    pub prompt_token: bool,
    #[arg(
        long,
        default_value_t = DEFAULT_TIMEOUT,
        help_heading = "Connection And Auth",
        help = "HTTP timeout in seconds."
    )]
    pub timeout: u64,
    #[arg(
        long,
        default_value_t = false,
        help_heading = "Connection And Auth",
        help = "Enable TLS certificate verification. Verification is disabled by default."
    )]
    pub verify_ssl: bool,
}

#[derive(Debug, Clone, ValueEnum, PartialEq, Eq)]
pub enum Scope {
    Org,
    Global,
}

#[derive(Debug, Clone, ValueEnum, PartialEq, Eq)]
pub enum ListOutputFormat {
    Text,
    Table,
    Csv,
    Json,
}

#[derive(Debug, Clone, ValueEnum, PartialEq, Eq)]
pub enum DryRunOutputFormat {
    Text,
    Table,
    Json,
}

#[derive(Debug, Clone, Args)]
pub struct UserListArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help_heading = "Selection",
        value_enum,
        default_value_t = Scope::Org,
        help = "List users from the current org scope or from the Grafana global admin scope."
    )]
    pub scope: Scope,
    #[arg(
        long,
        help_heading = "Filters",
        help = "Filter users by a free-text search across login, email, or display name."
    )]
    pub query: Option<String>,
    #[arg(long, help_heading = "Filters", help = "Filter users by exact login.")]
    pub login: Option<String>,
    #[arg(
        long,
        help_heading = "Filters",
        help = "Filter users by exact email address."
    )]
    pub email: Option<String>,
    #[arg(
        long,
        help_heading = "Filters",
        help = "Filter org users by exact Grafana org role such as Viewer, Editor, or Admin."
    )]
    pub org_role: Option<String>,
    #[arg(
        long,
        help_heading = "Filters",
        value_parser = parse_bool_text,
        help = "Filter global users by Grafana server-admin status."
    )]
    pub grafana_admin: Option<bool>,
    #[arg(
        long,
        help_heading = "Output Controls",
        default_value_t = false,
        help = "Include each user's current team memberships in the list output."
    )]
    pub with_teams: bool,
    #[arg(
        long,
        help_heading = "Pagination",
        default_value_t = 1,
        help = "Result page number for paginated Grafana list APIs."
    )]
    pub page: usize,
    #[arg(
        long,
        help_heading = "Pagination",
        default_value_t = DEFAULT_PAGE_SIZE,
        help = "Number of users to request per page."
    )]
    pub per_page: usize,
    #[arg(long, default_value_t = false, conflicts_with_all = ["csv", "json"], help_heading = "Output Options", help = "Render user summaries as a table.")]
    pub table: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "json"], help_heading = "Output Options", help = "Render user summaries as CSV.")]
    pub csv: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "csv"], help_heading = "Output Options", help = "Render user summaries as JSON.")]
    pub json: bool,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["table", "csv", "json"],
        help_heading = "Output Options",
        help = "Alternative single-flag output selector. Use text, table, csv, or json."
    )]
    pub output_format: Option<ListOutputFormat>,
}

#[derive(Debug, Clone, Args)]
pub struct UserAddArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help_heading = "User Identity",
        help = "Login name for the new Grafana user."
    )]
    pub login: String,
    #[arg(
        long,
        help_heading = "User Identity",
        help = "Email address for the new Grafana user."
    )]
    pub email: String,
    #[arg(
        long,
        help_heading = "User Identity",
        help = "Display name for the new Grafana user."
    )]
    pub name: String,
    #[arg(
        long = "password",
        help_heading = "Credentials",
        conflicts_with_all = ["new_user_password_file", "prompt_user_password"],
        help = "Initial password for the new Grafana user."
    )]
    pub new_user_password: Option<String>,
    #[arg(
        long = "password-file",
        help_heading = "Credentials",
        conflicts_with_all = ["new_user_password", "prompt_user_password"],
        help = "Read the initial user password from this file."
    )]
    pub new_user_password_file: Option<PathBuf>,
    #[arg(
        long = "prompt-user-password",
        help_heading = "Credentials",
        default_value_t = false,
        conflicts_with_all = ["new_user_password", "new_user_password_file"],
        help = "Prompt for the initial user password without echo."
    )]
    pub prompt_user_password: bool,
    #[arg(
        long = "org-role",
        help_heading = "Privileges",
        help = "Optional initial org role such as Viewer, Editor, or Admin."
    )]
    pub org_role: Option<String>,
    #[arg(
        long = "grafana-admin",
        value_parser = parse_bool_text,
        help_heading = "Privileges",
        help = "Set whether the new user should be a Grafana server admin."
    )]
    pub grafana_admin: Option<bool>,
    #[arg(
        long,
        default_value_t = false,
        help_heading = "Output Options",
        help = "Render the create response as JSON."
    )]
    pub json: bool,
}

#[derive(Debug, Clone, Args)]
pub struct UserModifyArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help_heading = "Target Selection",
        conflicts_with_all = ["login", "email"],
        help = "Target one user by numeric Grafana user id."
    )]
    pub user_id: Option<String>,
    #[arg(
        long,
        help_heading = "Target Selection",
        conflicts_with_all = ["user_id", "email"],
        help = "Target one user by exact login."
    )]
    pub login: Option<String>,
    #[arg(
        long,
        help_heading = "Target Selection",
        conflicts_with_all = ["user_id", "login"],
        help = "Target one user by exact email address."
    )]
    pub email: Option<String>,
    #[arg(
        long,
        help_heading = "Profile",
        help = "Replace the user's login with this new value."
    )]
    pub set_login: Option<String>,
    #[arg(
        long,
        help_heading = "Profile",
        help = "Replace the user's email address with this new value."
    )]
    pub set_email: Option<String>,
    #[arg(
        long,
        help_heading = "Profile",
        help = "Replace the user's display name with this new value."
    )]
    pub set_name: Option<String>,
    #[arg(
        long,
        help_heading = "Security",
        conflicts_with_all = ["set_password_file", "prompt_set_password"],
        help = "Replace the user's password with this new value."
    )]
    pub set_password: Option<String>,
    #[arg(
        long = "set-password-file",
        help_heading = "Security",
        conflicts_with_all = ["set_password", "prompt_set_password"],
        help = "Read the replacement user password from this file."
    )]
    pub set_password_file: Option<PathBuf>,
    #[arg(
        long = "prompt-set-password",
        help_heading = "Security",
        default_value_t = false,
        conflicts_with_all = ["set_password", "set_password_file"],
        help = "Prompt for the replacement user password without echo."
    )]
    pub prompt_set_password: bool,
    #[arg(
        long,
        help_heading = "Privileges",
        help = "Change the user's org role to this value."
    )]
    pub set_org_role: Option<String>,
    #[arg(
        long,
        help_heading = "Privileges",
        value_parser = parse_bool_text,
        help = "Change whether the user is a Grafana server admin."
    )]
    pub set_grafana_admin: Option<bool>,
    #[arg(
        long,
        default_value_t = false,
        help_heading = "Output Options",
        help = "Render the modify response as JSON."
    )]
    pub json: bool,
}

#[derive(Debug, Clone, Args)]
pub struct UserDeleteArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help_heading = "Target Selection",
        conflicts_with_all = ["login", "email"],
        help = "Delete one user by numeric Grafana user id."
    )]
    pub user_id: Option<String>,
    #[arg(
        long,
        help_heading = "Target Selection",
        conflicts_with_all = ["user_id", "email"],
        help = "Delete one user by exact login."
    )]
    pub login: Option<String>,
    #[arg(
        long,
        help_heading = "Target Selection",
        conflicts_with_all = ["user_id", "login"],
        help = "Delete one user by exact email address."
    )]
    pub email: Option<String>,
    #[arg(
        long,
        help_heading = "Target Selection",
        value_enum,
        default_value_t = Scope::Global,
        help = "Delete from the org membership only or from the Grafana global user registry."
    )]
    pub scope: Scope,
    #[arg(
        long,
        help_heading = "Safety",
        default_value_t = false,
        help = "Skip the interactive confirmation prompt."
    )]
    pub yes: bool,
    #[arg(
        long,
        default_value_t = false,
        help_heading = "Output Options",
        help = "Render the delete response as JSON."
    )]
    pub json: bool,
}

#[derive(Debug, Clone, Args)]
pub struct UserExportArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help_heading = "Export Source",
        default_value = DEFAULT_ACCESS_USER_EXPORT_DIR,
        help = "Directory to write users.json and export-metadata.json."
    )]
    pub export_dir: PathBuf,
    #[arg(
        long,
        help_heading = "Export Controls",
        default_value_t = false,
        help = "Replace existing export files in the target directory instead of failing."
    )]
    pub overwrite: bool,
    #[arg(
        long,
        help_heading = "Export Controls",
        default_value_t = false,
        help = "Preview export paths without writing files."
    )]
    pub dry_run: bool,
    #[arg(
        long,
        help_heading = "Export Source",
        value_enum,
        default_value_t = Scope::Org,
        help = "Export org-scoped or global users (default: org)."
    )]
    pub scope: Scope,
    #[arg(
        long,
        help_heading = "Export Controls",
        default_value_t = false,
        help = "Include each user's current team memberships in the export file."
    )]
    pub with_teams: bool,
}

#[derive(Debug, Clone, Args)]
pub struct UserImportArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help_heading = "Import Source",
        help = "Import directory that contains users.json and export-metadata.json."
    )]
    pub import_dir: PathBuf,
    #[arg(
        long,
        help_heading = "Import Source",
        value_enum,
        default_value_t = Scope::Org,
        help = "Import match strategy for users: global or org scope (default: org)."
    )]
    pub scope: Scope,
    #[arg(
        long,
        help_heading = "Import Behavior",
        default_value_t = false,
        help = "Update matching existing items instead of failing import on duplicates."
    )]
    pub replace_existing: bool,
    #[arg(
        long,
        help_heading = "Import Behavior",
        default_value_t = false,
        help = "Preview import changes without writing to Grafana."
    )]
    pub dry_run: bool,
    #[arg(
        long,
        default_value_t = false,
        requires = "dry_run",
        help_heading = "Output Options",
        help = "For --dry-run only, render a compact table instead of per-record log lines."
    )]
    pub table: bool,
    #[arg(
        long,
        default_value_t = false,
        requires = "dry_run",
        help_heading = "Output Options",
        help = "For --dry-run only, render one JSON document with action rows and summary counts."
    )]
    pub json: bool,
    #[arg(
        long,
        value_enum,
        default_value_t = DryRunOutputFormat::Text,
        conflicts_with_all = ["table", "json"],
        help_heading = "Output Options",
        help = "Alternative single-flag output selector for --dry-run output. Use text, table, or json."
    )]
    pub output_format: DryRunOutputFormat,
    #[arg(
        long,
        help_heading = "Import Behavior",
        default_value_t = false,
        help = "Acknowledge destructive import operations (remove/missing sync)."
    )]
    pub yes: bool,
}

#[derive(Debug, Clone, Args)]
pub struct UserDiffArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help_heading = "Diff Source",
        default_value = "access-users",
        help = "Diff directory that contains users.json and export-metadata.json."
    )]
    pub diff_dir: PathBuf,
    #[arg(
        long,
        help_heading = "Diff Scope",
        value_enum,
        default_value_t = Scope::Org,
        help = "Compare against org-scoped or global users (default: org)."
    )]
    pub scope: Scope,
}

#[derive(Debug, Clone, Args)]
pub struct TeamListArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help_heading = "Filters",
        help = "Filter teams by a free-text search."
    )]
    pub query: Option<String>,
    #[arg(
        long,
        help_heading = "Filters",
        help = "Filter teams by exact team name."
    )]
    pub name: Option<String>,
    #[arg(
        long,
        help_heading = "Membership",
        default_value_t = false,
        help = "Include team members and admins in the rendered output."
    )]
    pub with_members: bool,
    #[arg(
        long,
        help_heading = "Pagination",
        default_value_t = 1,
        help = "Result page number for paginated Grafana list APIs."
    )]
    pub page: usize,
    #[arg(
        long,
        help_heading = "Pagination",
        default_value_t = DEFAULT_PAGE_SIZE,
        help = "Number of teams to request per page."
    )]
    pub per_page: usize,
    #[arg(long, default_value_t = false, conflicts_with_all = ["csv", "json"], help_heading = "Output Options", help = "Render team summaries as a table.")]
    pub table: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "json"], help_heading = "Output Options", help = "Render team summaries as CSV.")]
    pub csv: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "csv"], help_heading = "Output Options", help = "Render team summaries as JSON.")]
    pub json: bool,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["table", "csv", "json"],
        help_heading = "Output Options",
        help = "Alternative single-flag output selector. Use text, table, csv, or json."
    )]
    pub output_format: Option<ListOutputFormat>,
}

#[derive(Debug, Clone, Args)]
pub struct TeamAddArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help_heading = "Team Definition",
        help = "Name for the new Grafana team."
    )]
    pub name: String,
    #[arg(
        long,
        help_heading = "Team Definition",
        help = "Optional contact email for the new Grafana team."
    )]
    pub email: Option<String>,
    #[arg(
        long = "member",
        help_heading = "Team Membership",
        help = "Add one or more members by user id, exact login, or exact email as part of team creation."
    )]
    pub members: Vec<String>,
    #[arg(
        long = "admin",
        help_heading = "Team Membership",
        help = "Add one or more team admins by user id, exact login, or exact email as part of team creation."
    )]
    pub admins: Vec<String>,
    #[arg(
        long,
        default_value_t = false,
        help_heading = "Output Options",
        help = "Render the create response as JSON."
    )]
    pub json: bool,
}

#[derive(Debug, Clone, Args)]
pub struct TeamExportArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help_heading = "Export Source",
        default_value = DEFAULT_ACCESS_TEAM_EXPORT_DIR,
        help = "Directory to write teams.json and export-metadata.json."
    )]
    pub export_dir: PathBuf,
    #[arg(
        long,
        help_heading = "Export Controls",
        default_value_t = false,
        help = "Replace existing export files in the target directory instead of failing."
    )]
    pub overwrite: bool,
    #[arg(
        long,
        help_heading = "Export Controls",
        default_value_t = false,
        help = "Preview export paths without writing files."
    )]
    pub dry_run: bool,
    #[arg(
        long,
        help_heading = "Export Controls",
        default_value_t = true,
        help = "Include team members and admins in exported team records."
    )]
    pub with_members: bool,
}

#[derive(Debug, Clone, Args)]
pub struct TeamImportArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help_heading = "Import Source",
        help = "Import directory that contains teams.json and export-metadata.json."
    )]
    pub import_dir: PathBuf,
    #[arg(
        long,
        help_heading = "Import Behavior",
        default_value_t = false,
        help = "Update matching existing teams instead of failing on duplicates."
    )]
    pub replace_existing: bool,
    #[arg(
        long,
        help_heading = "Import Behavior",
        default_value_t = false,
        help = "Preview import changes without writing to Grafana."
    )]
    pub dry_run: bool,
    #[arg(
        long,
        default_value_t = false,
        requires = "dry_run",
        help_heading = "Output Options",
        help = "For --dry-run only, render a compact table instead of per-record log lines."
    )]
    pub table: bool,
    #[arg(
        long,
        default_value_t = false,
        requires = "dry_run",
        help_heading = "Output Options",
        help = "For --dry-run only, render one JSON document with action rows and summary counts."
    )]
    pub json: bool,
    #[arg(
        long,
        value_enum,
        default_value_t = DryRunOutputFormat::Text,
        conflicts_with_all = ["table", "json"],
        help_heading = "Output Options",
        help = "Alternative single-flag output selector for --dry-run output. Use text, table, or json."
    )]
    pub output_format: DryRunOutputFormat,
    #[arg(
        long,
        help_heading = "Import Behavior",
        default_value_t = false,
        help = "Acknowledge destructive team-member synchronization operations."
    )]
    pub yes: bool,
}

#[derive(Debug, Clone, Args)]
pub struct TeamDiffArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help_heading = "Diff Source",
        default_value = "access-teams",
        help = "Diff directory that contains teams.json and export-metadata.json."
    )]
    pub diff_dir: PathBuf,
}

#[derive(Debug, Clone, Args)]
pub struct TeamModifyArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help_heading = "Target Selection",
        conflicts_with = "name",
        help = "Target one team by numeric Grafana team id."
    )]
    pub team_id: Option<String>,
    #[arg(
        long,
        help_heading = "Target Selection",
        conflicts_with = "team_id",
        help = "Target one team by exact team name."
    )]
    pub name: Option<String>,
    #[arg(
        long = "add-member",
        help_heading = "Membership",
        help = "Add one or more members by user id, exact login, or exact email."
    )]
    pub add_member: Vec<String>,
    #[arg(
        long = "remove-member",
        help_heading = "Membership",
        help = "Remove one or more members by user id, exact login, or exact email."
    )]
    pub remove_member: Vec<String>,
    #[arg(
        long = "add-admin",
        help_heading = "Membership",
        help = "Promote one or more members to team admin by user id, exact login, or exact email."
    )]
    pub add_admin: Vec<String>,
    #[arg(
        long = "remove-admin",
        help_heading = "Membership",
        help = "Remove team-admin status from one or more members by user id, exact login, or exact email."
    )]
    pub remove_admin: Vec<String>,
    #[arg(
        long,
        default_value_t = false,
        help_heading = "Output Options",
        help = "Render the modify response as JSON."
    )]
    pub json: bool,
}

#[derive(Debug, Clone, Args)]
pub struct OrgListArgs {
    #[command(flatten)]
    pub common: CommonCliArgsNoOrgId,
    #[arg(
        long = "org-id",
        help_heading = "Selection",
        help = "Filter to one exact organization id."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        help_heading = "Selection",
        help = "Filter organizations by exact name."
    )]
    pub name: Option<String>,
    #[arg(
        long,
        help_heading = "Selection",
        help = "Filter organizations by a free-text search."
    )]
    pub query: Option<String>,
    #[arg(
        long,
        help_heading = "Selection",
        default_value_t = false,
        help = "Include org users and org roles in the rendered output."
    )]
    pub with_users: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["csv", "json"], help_heading = "Output Options", help = "Render org summaries as a table.")]
    pub table: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "json"], help_heading = "Output Options", help = "Render org summaries as CSV.")]
    pub csv: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "csv"], help_heading = "Output Options", help = "Render org summaries as JSON.")]
    pub json: bool,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["table", "csv", "json"],
        help_heading = "Output Options",
        help = "Alternative single-flag output selector. Use text, table, csv, or json."
    )]
    pub output_format: Option<ListOutputFormat>,
}

#[derive(Debug, Clone, Args)]
pub struct OrgAddArgs {
    #[command(flatten)]
    pub common: CommonCliArgsNoOrgId,
    #[arg(
        long,
        help_heading = "Org Identity",
        help = "Name for the new Grafana organization."
    )]
    pub name: String,
    #[arg(
        long,
        default_value_t = false,
        help_heading = "Output Options",
        help = "Render the create response as JSON."
    )]
    pub json: bool,
}

#[derive(Debug, Clone, Args)]
pub struct OrgModifyArgs {
    #[command(flatten)]
    pub common: CommonCliArgsNoOrgId,
    #[arg(
        long = "org-id",
        help_heading = "Target Selection",
        conflicts_with = "name",
        help = "Target one organization by numeric Grafana org id."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        help_heading = "Target Selection",
        conflicts_with = "org_id",
        help = "Target one organization by exact name."
    )]
    pub name: Option<String>,
    #[arg(
        long,
        help_heading = "Org Updates",
        help = "Replace the organization name with this new value."
    )]
    pub set_name: String,
    #[arg(
        long,
        default_value_t = false,
        help_heading = "Output Options",
        help = "Render the modify response as JSON."
    )]
    pub json: bool,
}

#[derive(Debug, Clone, Args)]
pub struct OrgDeleteArgs {
    #[command(flatten)]
    pub common: CommonCliArgsNoOrgId,
    #[arg(
        long = "org-id",
        help_heading = "Target Selection",
        conflicts_with = "name",
        help = "Delete one organization by numeric Grafana org id."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        help_heading = "Target Selection",
        conflicts_with = "org_id",
        help = "Delete one organization by exact name."
    )]
    pub name: Option<String>,
    #[arg(
        long,
        help_heading = "Safety",
        default_value_t = false,
        help = "Skip the interactive confirmation prompt."
    )]
    pub yes: bool,
    #[arg(
        long,
        default_value_t = false,
        help_heading = "Output Options",
        help = "Render the delete response as JSON."
    )]
    pub json: bool,
}

#[derive(Debug, Clone, Args)]
pub struct OrgExportArgs {
    #[command(flatten)]
    pub common: CommonCliArgsNoOrgId,
    #[arg(
        long = "org-id",
        help_heading = "Export Scope",
        help = "Filter export to one exact organization id."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        help_heading = "Export Source",
        default_value = DEFAULT_ACCESS_ORG_EXPORT_DIR,
        help = "Directory to write orgs.json and export-metadata.json."
    )]
    pub export_dir: PathBuf,
    #[arg(
        long,
        help_heading = "Export Controls",
        default_value_t = false,
        help = "Overwrite existing export files instead of failing."
    )]
    pub overwrite: bool,
    #[arg(
        long,
        help_heading = "Export Controls",
        default_value_t = false,
        help = "Preview export paths without writing files."
    )]
    pub dry_run: bool,
    #[arg(
        long,
        help_heading = "Export Scope",
        help = "Filter export to one exact organization name."
    )]
    pub name: Option<String>,
    #[arg(
        long,
        help_heading = "Export Controls",
        default_value_t = false,
        help = "Include org users and org roles in the export bundle."
    )]
    pub with_users: bool,
}

#[derive(Debug, Clone, Args)]
pub struct OrgImportArgs {
    #[command(flatten)]
    pub common: CommonCliArgsNoOrgId,
    #[arg(
        long,
        help_heading = "Import Source",
        help = "Import directory that contains orgs.json and export-metadata.json."
    )]
    pub import_dir: PathBuf,
    #[arg(
        long,
        help_heading = "Import Behavior",
        default_value_t = false,
        help = "Update matching existing orgs or create missing orgs instead of skipping them."
    )]
    pub replace_existing: bool,
    #[arg(
        long,
        help_heading = "Import Behavior",
        default_value_t = false,
        help = "Preview import changes without writing to Grafana."
    )]
    pub dry_run: bool,
    #[arg(
        long,
        help_heading = "Safety",
        default_value_t = false,
        help = "Acknowledge destructive import operations when required."
    )]
    pub yes: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ServiceAccountListArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help_heading = "Filters",
        help = "Filter service accounts by a free-text search."
    )]
    pub query: Option<String>,
    #[arg(
        long,
        help_heading = "Pagination",
        default_value_t = 1,
        help = "Result page number for paginated Grafana list APIs."
    )]
    pub page: usize,
    #[arg(
        long,
        help_heading = "Pagination",
        default_value_t = DEFAULT_PAGE_SIZE,
        help = "Number of service accounts to request per page."
    )]
    pub per_page: usize,
    #[arg(long, default_value_t = false, conflicts_with_all = ["csv", "json"], help_heading = "Output Options", help = "Render service-account summaries as a table.")]
    pub table: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "json"], help_heading = "Output Options", help = "Render service-account summaries as CSV.")]
    pub csv: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "csv"], help_heading = "Output Options", help = "Render service-account summaries as JSON.")]
    pub json: bool,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["table", "csv", "json"],
        help_heading = "Output Options",
        help = "Alternative single-flag output selector. Use text, table, csv, or json."
    )]
    pub output_format: Option<ListOutputFormat>,
}

#[derive(Debug, Clone, Args)]
pub struct ServiceAccountAddArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help_heading = "Service Account Identity",
        help = "Name for the new Grafana service account."
    )]
    pub name: String,
    #[arg(
        long,
        help_heading = "Service Account Identity",
        default_value = "Viewer",
        value_parser = parse_service_account_role,
        help = "Initial org role for the service account."
    )]
    pub role: String,
    #[arg(
        long,
        help_heading = "Service Account Identity",
        value_parser = parse_bool_text,
        default_value = "false",
        help = "Create the service account in a disabled state."
    )]
    pub disabled: bool,
    #[arg(
        long,
        default_value_t = false,
        help_heading = "Output Options",
        help = "Render the create response as JSON."
    )]
    pub json: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ServiceAccountExportArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help_heading = "Export Source",
        default_value = DEFAULT_ACCESS_SERVICE_ACCOUNT_EXPORT_DIR,
        help = "Directory to write service-accounts.json and export-metadata.json."
    )]
    pub export_dir: PathBuf,
    #[arg(
        long,
        help_heading = "Export Controls",
        default_value_t = false,
        help = "Overwrite existing export files instead of failing."
    )]
    pub overwrite: bool,
    #[arg(
        long,
        help_heading = "Export Controls",
        default_value_t = false,
        help = "Preview export paths without writing files."
    )]
    pub dry_run: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ServiceAccountImportArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help_heading = "Import Source",
        help = "Import directory that contains service-accounts.json and export-metadata.json."
    )]
    pub import_dir: PathBuf,
    #[arg(
        long,
        help_heading = "Import Behavior",
        default_value_t = false,
        help = "Update matching existing service accounts instead of failing on duplicates."
    )]
    pub replace_existing: bool,
    #[arg(
        long,
        help_heading = "Import Behavior",
        default_value_t = false,
        help = "Preview import changes without writing to Grafana."
    )]
    pub dry_run: bool,
    #[arg(
        long,
        default_value_t = false,
        requires = "dry_run",
        help_heading = "Output Options",
        help = "For --dry-run only, render a compact table instead of per-record log lines."
    )]
    pub table: bool,
    #[arg(
        long,
        default_value_t = false,
        requires = "dry_run",
        help_heading = "Output Options",
        help = "For --dry-run only, render one JSON document with action rows and summary counts."
    )]
    pub json: bool,
    #[arg(
        long,
        value_enum,
        default_value_t = DryRunOutputFormat::Text,
        conflicts_with_all = ["table", "json"],
        help_heading = "Output Options",
        help = "Alternative single-flag output selector for --dry-run output. Use text, table, or json."
    )]
    pub output_format: DryRunOutputFormat,
    #[arg(
        long,
        help_heading = "Import Behavior",
        default_value_t = false,
        help = "Acknowledge destructive import operations when required."
    )]
    pub yes: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ServiceAccountDiffArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help_heading = "Diff Source",
        default_value = DEFAULT_ACCESS_SERVICE_ACCOUNT_EXPORT_DIR,
        help = "Diff directory that contains service-accounts.json and export-metadata.json."
    )]
    pub diff_dir: PathBuf,
}

#[derive(Debug, Clone, Args)]
pub struct ServiceAccountTokenAddArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help_heading = "Target Selection",
        conflicts_with = "name",
        help = "Target one service account by numeric id."
    )]
    pub service_account_id: Option<String>,
    #[arg(
        long,
        help_heading = "Target Selection",
        conflicts_with = "service_account_id",
        help = "Target one service account by exact name."
    )]
    pub name: Option<String>,
    #[arg(
        long,
        help_heading = "Token Settings",
        help = "Name for the new service-account token."
    )]
    pub token_name: String,
    #[arg(
        long,
        help_heading = "Token Settings",
        value_parser = parse_positive_usize,
        help = "Optional token lifetime in seconds. Omit for a non-expiring token if Grafana allows it."
    )]
    pub seconds_to_live: Option<usize>,
    #[arg(
        long,
        default_value_t = false,
        help_heading = "Output Options",
        help = "Render the token create response as JSON."
    )]
    pub json: bool,
}

#[derive(Debug, Clone, Subcommand)]
pub enum ServiceAccountTokenCommand {
    #[command(after_help = ACCESS_SERVICE_ACCOUNT_TOKEN_ADD_HELP_EXAMPLES)]
    Add(ServiceAccountTokenAddArgs),
    #[command(after_help = ACCESS_SERVICE_ACCOUNT_TOKEN_DELETE_HELP_EXAMPLES)]
    Delete(ServiceAccountTokenDeleteArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum ServiceAccountCommand {
    #[command(after_help = ACCESS_SERVICE_ACCOUNT_LIST_HELP_EXAMPLES)]
    List(ServiceAccountListArgs),
    #[command(after_help = ACCESS_SERVICE_ACCOUNT_ADD_HELP_EXAMPLES)]
    Add(ServiceAccountAddArgs),
    #[command(after_help = ACCESS_SERVICE_ACCOUNT_EXPORT_HELP_EXAMPLES)]
    Export(ServiceAccountExportArgs),
    #[command(after_help = ACCESS_SERVICE_ACCOUNT_IMPORT_HELP_EXAMPLES)]
    Import(ServiceAccountImportArgs),
    #[command(after_help = ACCESS_SERVICE_ACCOUNT_DIFF_HELP_EXAMPLES)]
    Diff(ServiceAccountDiffArgs),
    #[command(after_help = ACCESS_SERVICE_ACCOUNT_DELETE_HELP_EXAMPLES)]
    Delete(ServiceAccountDeleteArgs),
    #[command(after_help = ACCESS_SERVICE_ACCOUNT_TOKEN_GROUP_HELP_EXAMPLES)]
    Token {
        #[command(subcommand)]
        command: ServiceAccountTokenCommand,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum OrgCommand {
    #[command(after_help = ACCESS_ORG_LIST_HELP_EXAMPLES)]
    List(OrgListArgs),
    #[command(after_help = ACCESS_ORG_ADD_HELP_EXAMPLES)]
    Add(OrgAddArgs),
    #[command(after_help = ACCESS_ORG_MODIFY_HELP_EXAMPLES)]
    Modify(OrgModifyArgs),
    #[command(after_help = ACCESS_ORG_EXPORT_HELP_EXAMPLES)]
    Export(OrgExportArgs),
    #[command(after_help = ACCESS_ORG_IMPORT_HELP_EXAMPLES)]
    Import(OrgImportArgs),
    #[command(after_help = ACCESS_ORG_DELETE_HELP_EXAMPLES)]
    Delete(OrgDeleteArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum TeamCommand {
    #[command(after_help = ACCESS_TEAM_LIST_HELP_EXAMPLES)]
    List(TeamListArgs),
    #[command(after_help = ACCESS_TEAM_ADD_HELP_EXAMPLES)]
    Add(TeamAddArgs),
    #[command(after_help = ACCESS_TEAM_MODIFY_HELP_EXAMPLES)]
    Modify(TeamModifyArgs),
    #[command(after_help = ACCESS_TEAM_EXPORT_HELP_EXAMPLES)]
    Export(TeamExportArgs),
    #[command(after_help = ACCESS_TEAM_IMPORT_HELP_EXAMPLES)]
    Import(TeamImportArgs),
    #[command(after_help = ACCESS_TEAM_DIFF_HELP_EXAMPLES)]
    Diff(TeamDiffArgs),
    #[command(after_help = ACCESS_TEAM_DELETE_HELP_EXAMPLES)]
    Delete(TeamDeleteArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum UserCommand {
    #[command(after_help = ACCESS_USER_LIST_HELP_EXAMPLES)]
    List(UserListArgs),
    #[command(after_help = ACCESS_USER_ADD_HELP_EXAMPLES)]
    Add(UserAddArgs),
    #[command(after_help = ACCESS_USER_MODIFY_HELP_EXAMPLES)]
    Modify(UserModifyArgs),
    #[command(after_help = ACCESS_USER_EXPORT_HELP_EXAMPLES)]
    Export(UserExportArgs),
    #[command(after_help = ACCESS_USER_IMPORT_HELP_EXAMPLES)]
    Import(UserImportArgs),
    #[command(after_help = ACCESS_USER_DIFF_HELP_EXAMPLES)]
    Diff(UserDiffArgs),
    #[command(after_help = ACCESS_USER_DELETE_HELP_EXAMPLES)]
    Delete(UserDeleteArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum AccessCommand {
    #[command(after_help = ACCESS_USER_GROUP_HELP_EXAMPLES)]
    User {
        #[command(subcommand)]
        command: UserCommand,
    },
    #[command(after_help = ACCESS_ORG_GROUP_HELP_EXAMPLES)]
    Org {
        #[command(subcommand)]
        command: OrgCommand,
    },
    #[command(after_help = ACCESS_TEAM_GROUP_HELP_EXAMPLES)]
    Team {
        #[command(subcommand)]
        command: TeamCommand,
    },
    #[command(
        name = "service-account",
        after_help = ACCESS_SERVICE_ACCOUNT_GROUP_HELP_EXAMPLES
    )]
    ServiceAccount {
        #[command(subcommand)]
        command: ServiceAccountCommand,
    },
}

#[derive(Debug, Clone, Parser)]
#[command(
    name = "grafana-access-utils",
    about = "List and manage Grafana users, orgs, teams, and service accounts.",
    after_help = ACCESS_HELP_EXAMPLES
)]
pub(crate) struct AccessCliRoot {
    #[command(flatten)]
    args: AccessCliArgs,
}

#[derive(Debug, Clone, Args)]
pub struct AccessCliArgs {
    #[command(subcommand)]
    pub command: AccessCommand,
}

// Parse raw argv into strongly-typed access args, then normalize output-style
// aliases so callers can rely on one boolean matrix in handlers.
pub fn parse_cli_from<I, T>(iter: I) -> AccessCliArgs
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    normalize_access_cli_args(AccessCliRoot::parse_from(iter).args)
}

// Shared list output flags can come from both legacy boolean flags and the
// enum-style alias. This helper keeps CLI compatibility while normalizing state.
fn apply_list_output_format(
    table: &mut bool,
    csv: &mut bool,
    json: &mut bool,
    output_format: &Option<ListOutputFormat>,
) {
    match output_format {
        Some(ListOutputFormat::Text) => {}
        Some(ListOutputFormat::Table) => *table = true,
        Some(ListOutputFormat::Csv) => *csv = true,
        Some(ListOutputFormat::Json) => *json = true,
        None => {}
    }
}

fn apply_dry_run_output_format(
    table: &mut bool,
    json: &mut bool,
    output_format: &DryRunOutputFormat,
) {
    match output_format {
        DryRunOutputFormat::Text => {}
        DryRunOutputFormat::Table => *table = true,
        DryRunOutputFormat::Json => *json = true,
    }
}

// Convert list output-mode aliases (table/csv/json + output_format) into a single
// canonical boolean state per command path.
pub fn normalize_access_cli_args(mut args: AccessCliArgs) -> AccessCliArgs {
    match &mut args.command {
        AccessCommand::User { command } => {
            if let UserCommand::List(list_args) = command {
                apply_list_output_format(
                    &mut list_args.table,
                    &mut list_args.csv,
                    &mut list_args.json,
                    &list_args.output_format,
                );
            }
            if let UserCommand::Import(import_args) = command {
                apply_dry_run_output_format(
                    &mut import_args.table,
                    &mut import_args.json,
                    &import_args.output_format,
                );
            }
        }
        AccessCommand::Org { command } => {
            if let OrgCommand::List(list_args) = command {
                apply_list_output_format(
                    &mut list_args.table,
                    &mut list_args.csv,
                    &mut list_args.json,
                    &list_args.output_format,
                );
            }
        }
        AccessCommand::Team { command } => {
            if let TeamCommand::List(list_args) = command {
                apply_list_output_format(
                    &mut list_args.table,
                    &mut list_args.csv,
                    &mut list_args.json,
                    &list_args.output_format,
                );
            }
            if let TeamCommand::Import(import_args) = command {
                apply_dry_run_output_format(
                    &mut import_args.table,
                    &mut import_args.json,
                    &import_args.output_format,
                );
            }
        }
        AccessCommand::ServiceAccount { command } => {
            if let ServiceAccountCommand::List(list_args) = command {
                apply_list_output_format(
                    &mut list_args.table,
                    &mut list_args.csv,
                    &mut list_args.json,
                    &list_args.output_format,
                );
            }
            if let ServiceAccountCommand::Import(import_args) = command {
                apply_dry_run_output_format(
                    &mut import_args.table,
                    &mut import_args.json,
                    &import_args.output_format,
                );
            }
        }
    }
    args
}

pub fn root_command() -> Command {
    AccessCliRoot::command()
}

#[derive(Debug, Clone)]
pub struct AccessAuthContext {
    pub url: String,
    pub timeout: u64,
    pub verify_ssl: bool,
    pub auth_mode: String,
    pub headers: Vec<(String, String)>,
}

// Parse bool-like CLI text using the explicit true/false contract used by
// back-compat flags that bypass Clap's native bool parsing.
fn parse_bool_text(value: &str) -> std::result::Result<bool, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err("value must be true or false".to_string()),
    }
}

fn parse_positive_usize(value: &str) -> std::result::Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("invalid integer value: {value}"))?;
    if parsed < 1 {
        return Err("value must be >= 1".to_string());
    }
    Ok(parsed)
}

fn parse_service_account_role(value: &str) -> std::result::Result<String, String> {
    match value {
        "Viewer" | "Editor" | "Admin" | "None" => Ok(value.to_string()),
        _ => Err("valid values: Viewer, Editor, Admin, None".to_string()),
    }
}

pub fn build_auth_context(common: &CommonCliArgs) -> Result<AccessAuthContext> {
    let mut headers = resolve_auth_headers(
        common.api_token.as_deref(),
        common.username.as_deref(),
        common.password.as_deref(),
        common.prompt_password,
        common.prompt_token,
    )?;
    if let Some(org_id) = common.org_id {
        headers.push(("X-Grafana-Org-Id".to_string(), org_id.to_string()));
    }
    let auth_mode = headers
        .iter()
        .find(|(name, _)| name == "Authorization")
        .map(|(_, value)| {
            if value.starts_with("Basic ") {
                "basic".to_string()
            } else {
                "token".to_string()
            }
        })
        .unwrap_or_else(|| "unknown".to_string());
    Ok(AccessAuthContext {
        url: common.url.clone(),
        timeout: common.timeout,
        verify_ssl: common.verify_ssl,
        auth_mode,
        headers,
    })
}

pub fn build_auth_context_no_org_id(common: &CommonCliArgsNoOrgId) -> Result<AccessAuthContext> {
    let headers = resolve_auth_headers(
        common.api_token.as_deref(),
        common.username.as_deref(),
        common.password.as_deref(),
        common.prompt_password,
        common.prompt_token,
    )?;
    let auth_mode = headers
        .iter()
        .find(|(name, _)| name == "Authorization")
        .map(|(_, value)| {
            if value.starts_with("Basic ") {
                "basic".to_string()
            } else {
                "token".to_string()
            }
        })
        .unwrap_or_else(|| "unknown".to_string());
    Ok(AccessAuthContext {
        url: common.url.clone(),
        timeout: common.timeout,
        verify_ssl: common.verify_ssl,
        auth_mode,
        headers,
    })
}

pub fn build_http_client(common: &CommonCliArgs) -> Result<JsonHttpClient> {
    let context = build_auth_context(common)?;
    JsonHttpClient::new(JsonHttpClientConfig {
        base_url: context.url,
        headers: context.headers,
        timeout_secs: context.timeout,
        verify_ssl: context.verify_ssl,
    })
}

pub fn build_http_client_no_org_id(common: &CommonCliArgsNoOrgId) -> Result<JsonHttpClient> {
    let context = build_auth_context_no_org_id(common)?;
    JsonHttpClient::new(JsonHttpClientConfig {
        base_url: context.url,
        headers: context.headers,
        timeout_secs: context.timeout,
        verify_ssl: context.verify_ssl,
    })
}
