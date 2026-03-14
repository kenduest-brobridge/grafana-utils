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
        visible_alias = "username",
        help = "Grafana Basic auth username. Preferred flag: --basic-user. Falls back to GRAFANA_USERNAME."
    )]
    pub username: Option<String>,
    #[arg(
        long = "basic-password",
        visible_alias = "password",
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

#[derive(Debug, Clone, Args)]
pub struct UserListArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long, value_enum, default_value_t = Scope::Org)]
    pub scope: Scope,
    #[arg(long)]
    pub query: Option<String>,
    #[arg(long)]
    pub login: Option<String>,
    #[arg(long)]
    pub email: Option<String>,
    #[arg(long)]
    pub org_role: Option<String>,
    #[arg(long, value_parser = parse_bool_text)]
    pub grafana_admin: Option<bool>,
    #[arg(long, default_value_t = false)]
    pub with_teams: bool,
    #[arg(long, default_value_t = 1)]
    pub page: usize,
    #[arg(long, default_value_t = DEFAULT_PAGE_SIZE)]
    pub per_page: usize,
    #[arg(long, default_value_t = false, conflicts_with_all = ["csv", "json"])]
    pub table: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "json"])]
    pub csv: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "csv"])]
    pub json: bool,
}

#[derive(Debug, Clone, Args)]
pub struct UserAddArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long)]
    pub login: String,
    #[arg(long)]
    pub email: String,
    #[arg(long)]
    pub name: String,
    #[arg(long = "password")]
    pub new_user_password: String,
    #[arg(long = "org-role")]
    pub org_role: Option<String>,
    #[arg(long = "grafana-admin", value_parser = parse_bool_text)]
    pub grafana_admin: Option<bool>,
    #[arg(long, default_value_t = false)]
    pub json: bool,
}

#[derive(Debug, Clone, Args)]
pub struct UserModifyArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long, conflicts_with_all = ["login", "email"])]
    pub user_id: Option<String>,
    #[arg(long, conflicts_with_all = ["user_id", "email"])]
    pub login: Option<String>,
    #[arg(long, conflicts_with_all = ["user_id", "login"])]
    pub email: Option<String>,
    #[arg(long)]
    pub set_login: Option<String>,
    #[arg(long)]
    pub set_email: Option<String>,
    #[arg(long)]
    pub set_name: Option<String>,
    #[arg(long)]
    pub set_password: Option<String>,
    #[arg(long)]
    pub set_org_role: Option<String>,
    #[arg(long, value_parser = parse_bool_text)]
    pub set_grafana_admin: Option<bool>,
    #[arg(long, default_value_t = false)]
    pub json: bool,
}

#[derive(Debug, Clone, Args)]
pub struct UserDeleteArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long, conflicts_with_all = ["login", "email"])]
    pub user_id: Option<String>,
    #[arg(long, conflicts_with_all = ["user_id", "email"])]
    pub login: Option<String>,
    #[arg(long, conflicts_with_all = ["user_id", "login"])]
    pub email: Option<String>,
    #[arg(long, value_enum, default_value_t = Scope::Global)]
    pub scope: Scope,
    #[arg(long, default_value_t = false)]
    pub yes: bool,
    #[arg(long, default_value_t = false)]
    pub json: bool,
}

#[derive(Debug, Clone, Args)]
pub struct TeamListArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long)]
    pub query: Option<String>,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long, default_value_t = false)]
    pub with_members: bool,
    #[arg(long, default_value_t = 1)]
    pub page: usize,
    #[arg(long, default_value_t = DEFAULT_PAGE_SIZE)]
    pub per_page: usize,
    #[arg(long, default_value_t = false, conflicts_with_all = ["csv", "json"])]
    pub table: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "json"])]
    pub csv: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "csv"])]
    pub json: bool,
}

#[derive(Debug, Clone, Args)]
pub struct TeamAddArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long)]
    pub name: String,
    #[arg(long)]
    pub email: Option<String>,
    #[arg(long = "member")]
    pub members: Vec<String>,
    #[arg(long = "admin")]
    pub admins: Vec<String>,
    #[arg(long, default_value_t = false)]
    pub json: bool,
}

#[derive(Debug, Clone, Args)]
pub struct TeamModifyArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long, conflicts_with = "name")]
    pub team_id: Option<String>,
    #[arg(long, conflicts_with = "team_id")]
    pub name: Option<String>,
    #[arg(long = "add-member")]
    pub add_member: Vec<String>,
    #[arg(long = "remove-member")]
    pub remove_member: Vec<String>,
    #[arg(long = "add-admin")]
    pub add_admin: Vec<String>,
    #[arg(long = "remove-admin")]
    pub remove_admin: Vec<String>,
    #[arg(long, default_value_t = false)]
    pub json: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ServiceAccountListArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long)]
    pub query: Option<String>,
    #[arg(long, default_value_t = 1)]
    pub page: usize,
    #[arg(long, default_value_t = DEFAULT_PAGE_SIZE)]
    pub per_page: usize,
    #[arg(long, default_value_t = false, conflicts_with_all = ["csv", "json"])]
    pub table: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "json"])]
    pub csv: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "csv"])]
    pub json: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ServiceAccountAddArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long)]
    pub name: String,
    #[arg(long, default_value = "Viewer")]
    pub role: String,
    #[arg(long, value_parser = parse_bool_text, default_value = "false")]
    pub disabled: bool,
    #[arg(long, default_value_t = false)]
    pub json: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ServiceAccountTokenAddArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long, conflicts_with = "name")]
    pub service_account_id: Option<String>,
    #[arg(long, conflicts_with = "service_account_id")]
    pub name: Option<String>,
    #[arg(long)]
    pub token_name: String,
    #[arg(long)]
    pub seconds_to_live: Option<usize>,
    #[arg(long, default_value_t = false)]
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
struct AccessCliRoot {
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
    AccessCliRoot::parse_from(iter).args
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
