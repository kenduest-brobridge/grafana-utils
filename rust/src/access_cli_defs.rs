use clap::{Args, Command, CommandFactory, Parser, Subcommand, ValueEnum};

use super::access_pending_delete::{
    ServiceAccountDeleteArgs, ServiceAccountTokenDeleteArgs, TeamDeleteArgs,
};
use crate::common::{resolve_auth_headers, Result};
use crate::http::{JsonHttpClient, JsonHttpClientConfig};

pub const DEFAULT_URL: &str = "http://127.0.0.1:3000";
pub const DEFAULT_TIMEOUT: u64 = 30;
pub const DEFAULT_PAGE_SIZE: usize = 100;

#[derive(Debug, Clone, Args)]
pub struct CommonCliArgs {
    #[arg(long, default_value = DEFAULT_URL, help = "Grafana base URL.")]
    pub url: String,
    #[arg(
        long = "token",
        visible_alias = "api-token",
        help = "Grafana API token. Preferred flag: --token. Falls back to GRAFANA_API_TOKEN."
    )]
    pub api_token: Option<String>,
    #[arg(
        long = "basic-user",
        help = "Grafana Basic auth username. Preferred flag: --basic-user. Falls back to GRAFANA_USERNAME."
    )]
    pub username: Option<String>,
    #[arg(
        long = "basic-password",
        help = "Grafana Basic auth password. Preferred flag: --basic-password. Falls back to GRAFANA_PASSWORD."
    )]
    pub password: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Prompt for the Grafana Basic auth password."
    )]
    pub prompt_password: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Prompt for the Grafana API token without echo instead of passing --token on the command line."
    )]
    pub prompt_token: bool,
    #[arg(
        long,
        help = "Grafana organization id to send through X-Grafana-Org-Id."
    )]
    pub org_id: Option<i64>,
    #[arg(long, default_value_t = DEFAULT_TIMEOUT, help = "HTTP timeout in seconds.")]
    pub timeout: u64,
    #[arg(
        long,
        default_value_t = false,
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
    Table,
    Csv,
    Json,
}

#[derive(Debug, Clone, Args)]
pub struct UserListArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long, value_enum, default_value_t = Scope::Org, help = "List users from the current org scope or from the Grafana global admin scope.")]
    pub scope: Scope,
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
        help = "Alternative single-flag output selector. Use table, csv, or json."
    )]
    pub output_format: Option<ListOutputFormat>,
}

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
    #[arg(long = "password", help = "Initial password for the new Grafana user.")]
    pub new_user_password: String,
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
    #[arg(long, help = "Replace the user's password with this new value.")]
    pub set_password: Option<String>,
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
    #[arg(long, default_value_t = false, conflicts_with_all = ["csv", "json"], help = "Render team summaries as a table.")]
    pub table: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "json"], help = "Render team summaries as CSV.")]
    pub csv: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "csv"], help = "Render team summaries as JSON.")]
    pub json: bool,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["table", "csv", "json"],
        help = "Alternative single-flag output selector. Use table, csv, or json."
    )]
    pub output_format: Option<ListOutputFormat>,
}

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
        help = "Add one or more members by user id or login as part of team creation."
    )]
    pub members: Vec<String>,
    #[arg(
        long = "admin",
        help = "Add one or more team admins by user id or login as part of team creation."
    )]
    pub admins: Vec<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Render the create response as JSON."
    )]
    pub json: bool,
}

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
        help = "Add one or more members by user id or login."
    )]
    pub add_member: Vec<String>,
    #[arg(
        long = "remove-member",
        help = "Remove one or more members by user id or login."
    )]
    pub remove_member: Vec<String>,
    #[arg(
        long = "add-admin",
        help = "Promote one or more members to team admin."
    )]
    pub add_admin: Vec<String>,
    #[arg(
        long = "remove-admin",
        help = "Remove team-admin status from one or more members."
    )]
    pub remove_admin: Vec<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Render the modify response as JSON."
    )]
    pub json: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ServiceAccountListArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
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
    #[arg(long, default_value_t = false, conflicts_with_all = ["csv", "json"], help = "Render service-account summaries as a table.")]
    pub table: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "json"], help = "Render service-account summaries as CSV.")]
    pub csv: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "csv"], help = "Render service-account summaries as JSON.")]
    pub json: bool,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["table", "csv", "json"],
        help = "Alternative single-flag output selector. Use table, csv, or json."
    )]
    pub output_format: Option<ListOutputFormat>,
}

#[derive(Debug, Clone, Args)]
pub struct ServiceAccountAddArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long, help = "Name for the new Grafana service account.")]
    pub name: String,
    #[arg(
        long,
        default_value = "Viewer",
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

#[derive(Debug, Clone, Subcommand)]
pub enum ServiceAccountTokenCommand {
    Add(ServiceAccountTokenAddArgs),
    Delete(ServiceAccountTokenDeleteArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum ServiceAccountCommand {
    List(ServiceAccountListArgs),
    Add(ServiceAccountAddArgs),
    Delete(ServiceAccountDeleteArgs),
    Token {
        #[command(subcommand)]
        command: ServiceAccountTokenCommand,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum TeamCommand {
    List(TeamListArgs),
    Add(TeamAddArgs),
    Modify(TeamModifyArgs),
    Delete(TeamDeleteArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum UserCommand {
    List(UserListArgs),
    Add(UserAddArgs),
    Modify(UserModifyArgs),
    Delete(UserDeleteArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum AccessCommand {
    User {
        #[command(subcommand)]
        command: UserCommand,
    },
    #[command(visible_alias = "group")]
    Team {
        #[command(subcommand)]
        command: TeamCommand,
    },
    #[command(name = "service-account")]
    ServiceAccount {
        #[command(subcommand)]
        command: ServiceAccountCommand,
    },
}

#[derive(Debug, Clone, Parser)]
#[command(
    name = "grafana-access-utils",
    about = "List and manage Grafana users, teams, and service accounts."
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

pub fn parse_cli_from<I, T>(iter: I) -> AccessCliArgs
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    normalize_access_cli_args(AccessCliRoot::parse_from(iter).args)
}

fn apply_list_output_format(
    table: &mut bool,
    csv: &mut bool,
    json: &mut bool,
    output_format: &Option<ListOutputFormat>,
) {
    match output_format {
        Some(ListOutputFormat::Table) => *table = true,
        Some(ListOutputFormat::Csv) => *csv = true,
        Some(ListOutputFormat::Json) => *json = true,
        None => {}
    }
}

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

fn parse_bool_text(value: &str) -> std::result::Result<bool, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err("value must be true or false".to_string()),
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

pub fn build_http_client(common: &CommonCliArgs) -> Result<JsonHttpClient> {
    let context = build_auth_context(common)?;
    JsonHttpClient::new(JsonHttpClientConfig {
        base_url: context.url,
        headers: context.headers,
        timeout_secs: context.timeout,
        verify_ssl: context.verify_ssl,
    })
}
