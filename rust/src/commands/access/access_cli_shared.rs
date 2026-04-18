//! CLI definitions for Access command surface and option compatibility behavior.

use clap::{Args, ValueEnum};
use std::path::PathBuf;

/// Default Grafana base URL used by access commands.
pub const DEFAULT_URL: &str = "http://127.0.0.1:3000";
/// Default HTTP timeout in seconds for access commands.
pub const DEFAULT_TIMEOUT: u64 = 30;
/// Default list pagination size for access inventory commands.
pub const DEFAULT_PAGE_SIZE: usize = 100;
/// Constant for default access user export dir.
pub const DEFAULT_ACCESS_USER_EXPORT_DIR: &str = "access-users";
/// Constant for default access team export dir.
pub const DEFAULT_ACCESS_TEAM_EXPORT_DIR: &str = "access-teams";
/// Constant for default access org export dir.
pub const DEFAULT_ACCESS_ORG_EXPORT_DIR: &str = "access-orgs";
/// Constant for default access service account export dir.
pub const DEFAULT_ACCESS_SERVICE_ACCOUNT_EXPORT_DIR: &str = "access-service-accounts";
/// Constant for access export kind users.
pub const ACCESS_EXPORT_KIND_USERS: &str = "grafana-utils-access-user-export-index";
/// Constant for access export kind teams.
pub const ACCESS_EXPORT_KIND_TEAMS: &str = "grafana-utils-access-team-export-index";
/// Constant for access export kind orgs.
pub const ACCESS_EXPORT_KIND_ORGS: &str = "grafana-utils-access-org-export-index";
/// Constant for access export kind service accounts.
pub const ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS: &str =
    "grafana-utils-access-service-account-export-index";
/// Constant for access export version.
pub const ACCESS_EXPORT_VERSION: i64 = 1;
/// Constant for access user export filename.
pub const ACCESS_USER_EXPORT_FILENAME: &str = "users.json";
/// Constant for access team export filename.
pub const ACCESS_TEAM_EXPORT_FILENAME: &str = "teams.json";
/// Constant for access org export filename.
pub const ACCESS_ORG_EXPORT_FILENAME: &str = "orgs.json";
/// Constant for access service account export filename.
pub const ACCESS_SERVICE_ACCOUNT_EXPORT_FILENAME: &str = "service-accounts.json";
/// Constant for access export metadata filename.
pub const ACCESS_EXPORT_METADATA_FILENAME: &str = "export-metadata.json";

pub(crate) const ACCESS_ROOT_HELP_TEXT: &str = "Examples:\n\n  List org users as JSON:\n    grafana-util access user list --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --json\n\n  List exported users from a local bundle:\n    grafana-util access user list --input-dir ./access-users --with-teams --output-format yaml\n\n  Plan user access changes from a local bundle:\n    grafana-util access plan --profile prod --input-dir ./access-users --resource user --output-format table\n\n  Plan org access changes from a local bundle:\n    grafana-util access plan --profile prod --input-dir ./access-orgs --resource org --output-format table\n\n  Browse users across organizations interactively:\n    grafana-util access user browse --url http://localhost:3000 --basic-user admin --basic-password admin\n\n  Create a Grafana user with Basic auth:\n    grafana-util access user add --url http://localhost:3000 --basic-user admin --basic-password admin --login alice --email alice@example.com --name Alice --password secret\n\n  Import teams with destructive sync acknowledgement:\n    grafana-util access team import --url http://localhost:3000 --basic-user admin --basic-password admin --input-dir ./access-teams --replace-existing --yes\n\n  Create a service-account token:\n    grafana-util access service-account token add --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --name deploy-bot --token-name nightly";
pub(crate) const ACCESS_USER_HELP_TEXT: &str = "Examples:\n\n  grafana-util access user list --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --json\n  grafana-util access user list --input-dir ./access-users --with-teams --output-format yaml\n  grafana-util access user browse --url http://localhost:3000 --basic-user admin --basic-password admin\n  grafana-util access user browse --input-dir ./access-users\n  grafana-util access user add --url http://localhost:3000 --basic-user admin --basic-password admin --login alice --email alice@example.com --name Alice --password secret";
pub(crate) const ACCESS_USER_LIST_HELP_TEXT: &str = "Examples:\n\n  grafana-util access user list --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --scope org --output-format text\n  grafana-util access user list --url http://localhost:3000 --basic-user admin --basic-password admin --scope global --with-teams --output-format yaml\n  grafana-util access user list --input-dir ./access-users --with-teams --output-format json";
pub(crate) const ACCESS_USER_BROWSE_HELP_TEXT: &str = "Examples:\n\n  grafana-util access user browse --url http://localhost:3000 --basic-user admin --basic-password admin\n  grafana-util access user browse --url http://localhost:3000 --basic-user admin --basic-password admin --current-org --login alice\n  grafana-util access user browse --input-dir ./access-users --login alice";
pub(crate) const ACCESS_TEAM_HELP_TEXT: &str = "Examples:\n\n  grafana-util access team list --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --json\n  grafana-util access team list --input-dir ./access-teams --with-members --output-format yaml\n  grafana-util access team browse --url http://localhost:3000 --basic-user admin --basic-password admin --with-members\n  grafana-util access team browse --input-dir ./access-teams --name platform-team\n  grafana-util access team import --url http://localhost:3000 --basic-user admin --basic-password admin --input-dir ./access-teams --replace-existing --yes";
pub(crate) const ACCESS_TEAM_LIST_HELP_TEXT: &str = "Examples:\n\n  grafana-util access team list --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format text\n  grafana-util access team list --url http://localhost:3000 --basic-user admin --basic-password admin --with-members --output-format yaml\n  grafana-util access team list --input-dir ./access-teams --with-members --output-format json";
pub(crate) const ACCESS_TEAM_BROWSE_HELP_TEXT: &str = "Examples:\n\n  grafana-util access team browse --url http://localhost:3000 --basic-user admin --basic-password admin --with-members\n  grafana-util access team browse --url http://localhost:3000 --basic-user admin --basic-password admin --name platform-team\n  grafana-util access team browse --input-dir ./access-teams --name platform-team";
pub(crate) const ACCESS_ORG_HELP_TEXT: &str = "Examples:\n\n  grafana-util access org list --url http://localhost:3000 --basic-user admin --basic-password admin --json\n  grafana-util access org list --input-dir ./access-orgs --with-users --output-format yaml\n  grafana-util access org diff --url http://localhost:3000 --basic-user admin --basic-password admin --diff-dir ./access-orgs\n  grafana-util access org delete --url http://localhost:3000 --basic-user admin --basic-password admin --name platform --yes";
pub(crate) const ACCESS_ORG_LIST_HELP_TEXT: &str = "Examples:\n\n  grafana-util access org list --url http://localhost:3000 --basic-user admin --basic-password admin --output-format text\n  grafana-util access org list --url http://localhost:3000 --basic-user admin --basic-password admin --with-users --output-format yaml\n  grafana-util access org list --input-dir ./access-orgs --with-users --output-format json";
pub(crate) const ACCESS_ORG_ADD_HELP_TEXT: &str = "Examples:\n\n  grafana-util access org add --url http://localhost:3000 --basic-user admin --basic-password admin --name platform --json";
pub(crate) const ACCESS_ORG_MODIFY_HELP_TEXT: &str = "Examples:\n\n  grafana-util access org modify --url http://localhost:3000 --basic-user admin --basic-password admin --name platform --set-name platform-core --json\n  grafana-util access org modify --url http://localhost:3000 --basic-user admin --basic-password admin --org-id 7 --set-name platform-prod --json";
pub(crate) const ACCESS_ORG_EXPORT_HELP_TEXT: &str = "Examples:\n\n  grafana-util access org export --url http://localhost:3000 --basic-user admin --basic-password admin --output-dir ./access-orgs --overwrite";
pub(crate) const ACCESS_ORG_IMPORT_HELP_TEXT: &str = "Examples:\n\n  grafana-util access org import --url http://localhost:3000 --basic-user admin --basic-password admin --input-dir ./access-orgs --dry-run\n  grafana-util access org import --url http://localhost:3000 --basic-user admin --basic-password admin --input-dir ./access-orgs --replace-existing --yes";
pub(crate) const ACCESS_ORG_DIFF_HELP_TEXT: &str = "Examples:\n\n  grafana-util access org diff --basic-user admin --basic-password admin --diff-dir ./access-orgs";
pub(crate) const ACCESS_USER_ADD_HELP_TEXT: &str = "Examples:\n\n  grafana-util access user add --basic-user admin --basic-password admin --login alice --email alice@example.com --name Alice --password secret\n  grafana-util access user add --basic-user admin --basic-password admin --login bob --email bob@example.com --name Bob --prompt-user-password";
pub(crate) const ACCESS_USER_MODIFY_HELP_TEXT: &str = "Examples:\n\n  grafana-util access user modify --basic-user admin --basic-password admin --login alice --set-email alice@example.com --set-org-role Editor --json\n  grafana-util access user modify --basic-user admin --basic-password admin --user-id 7 --prompt-set-password --set-grafana-admin true --json";
pub(crate) const ACCESS_USER_EXPORT_HELP_TEXT: &str = "Examples:\n\n  grafana-util access user export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-dir ./access-users --overwrite";
pub(crate) const ACCESS_USER_IMPORT_HELP_TEXT: &str = "Examples:\n\n  grafana-util access user import --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --input-dir ./access-users --dry-run --output-format table\n  grafana-util access user import --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --input-dir ./access-users --replace-existing --yes";
pub(crate) const ACCESS_USER_DIFF_HELP_TEXT: &str = "Examples:\n\n  grafana-util access user diff --url http://localhost:3000 --basic-user admin --basic-password admin --diff-dir ./access-users --scope global";
pub(crate) const ACCESS_USER_DELETE_HELP_TEXT: &str = "Examples:\n\n  grafana-util access user delete --url http://localhost:3000 --basic-user admin --basic-password admin --login temp-user --scope global --yes --json\n  grafana-util access user delete --url http://localhost:3000 --basic-user admin --basic-password admin --scope org --prompt\n  grafana-util access user delete --url http://localhost:3000 --basic-user admin --basic-password admin --prompt";
pub(crate) const ACCESS_TEAM_ADD_HELP_TEXT: &str = "Examples:\n\n  grafana-util access team add --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --name platform-team --email platform@example.com --member alice --admin alice --json";
pub(crate) const ACCESS_TEAM_MODIFY_HELP_TEXT: &str = "Examples:\n\n  grafana-util access team modify --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --name platform-team --add-member bob --remove-admin alice --json";
pub(crate) const ACCESS_TEAM_EXPORT_HELP_TEXT: &str = "Examples:\n\n  grafana-util access team export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-dir ./access-teams --overwrite";
pub(crate) const ACCESS_TEAM_IMPORT_HELP_TEXT: &str = "Examples:\n\n  grafana-util access team import --basic-user admin --basic-password admin --input-dir ./access-teams --dry-run --output-format table\n  grafana-util access team import --basic-user admin --basic-password admin --input-dir ./access-teams --replace-existing --yes";
pub(crate) const ACCESS_TEAM_DIFF_HELP_TEXT: &str = "Examples:\n\n  grafana-util access team diff --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --diff-dir ./access-teams";
pub(crate) const ACCESS_TEAM_DELETE_HELP_TEXT: &str = "Examples:\n\n  grafana-util access team delete --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --name platform-team --yes --json\n  grafana-util access team delete --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --prompt";
pub(crate) const ACCESS_ORG_DELETE_HELP_TEXT: &str = "Examples:\n\n  grafana-util access org delete --basic-user admin --basic-password admin --name platform --yes\n  grafana-util access org delete --basic-user admin --basic-password admin --prompt\n  grafana-util access org delete --basic-user admin --basic-password admin --org-id 7 --yes --json";
pub(crate) const ACCESS_PLAN_HELP_TEXT: &str = "Examples:\n\n  grafana-util access plan --url http://localhost:3000 --basic-user admin --basic-password admin --input-dir ./access-users --output-format table\n  grafana-util access plan --profile prod --input-dir ./access-users --resource user --json\n  grafana-util access plan --profile prod --input-dir ./access-orgs --resource org --output-format table\n  grafana-util access plan --profile prod --input-dir ./access-service-accounts --resource service-account --output-format json\n  grafana-util access plan --profile prod --input-dir ./access --resource all --output-format json";

/// Shared Grafana connection/authentication arguments for org-scoped access commands.
#[derive(Debug, Clone, Args)]
pub struct CommonCliArgs {
    #[arg(
        long,
        help = "Load connection defaults from the selected repo-local profile in grafana-util.yaml.",
        help_heading = "Authentication Options"
    )]
    pub profile: Option<String>,
    #[arg(
        long,
        default_value = "",
        hide_default_value = true,
        help = "Grafana base URL. Required unless supplied by --profile or GRAFANA_URL.",
        help_heading = "Authentication Options"
    )]
    pub url: String,
    #[arg(
        long = "token",
        visible_alias = "api-token",
        help = "Grafana API token. Preferred flag: --token. Falls back to GRAFANA_API_TOKEN.",
        help_heading = "Authentication Options"
    )]
    pub api_token: Option<String>,
    #[arg(
        long = "basic-user",
        help = "Grafana Basic auth username. Preferred flag: --basic-user. Falls back to GRAFANA_USERNAME.",
        help_heading = "Authentication Options"
    )]
    pub username: Option<String>,
    #[arg(
        long = "basic-password",
        help = "Grafana Basic auth password. Preferred flag: --basic-password. Falls back to GRAFANA_PASSWORD.",
        help_heading = "Authentication Options"
    )]
    pub password: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Prompt for the Grafana Basic auth password.",
        help_heading = "Authentication Options"
    )]
    pub prompt_password: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Prompt for the Grafana API token without echo instead of passing --token on the command line.",
        help_heading = "Authentication Options"
    )]
    pub prompt_token: bool,
    #[arg(
        long,
        help = "Grafana organization id to send through X-Grafana-Org-Id.",
        help_heading = "Authentication Options"
    )]
    pub org_id: Option<i64>,
    #[arg(long, default_value_t = DEFAULT_TIMEOUT, help = "HTTP timeout in seconds.", help_heading = "Transport Options")]
    pub timeout: u64,
    #[arg(
        long,
        default_value_t = false,
        help = "Enable TLS certificate verification. Verification is disabled by default.",
        help_heading = "Transport Options"
    )]
    pub verify_ssl: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["verify_ssl", "ca_cert"],
        help = "Disable TLS certificate verification explicitly.",
        help_heading = "Transport Options"
    )]
    pub insecure: bool,
    #[arg(
        long = "ca-cert",
        value_name = "PATH",
        help = "PEM bundle file to trust for Grafana TLS verification.",
        help_heading = "Transport Options"
    )]
    pub ca_cert: Option<PathBuf>,
}

/// Shared connection/authentication arguments for global access admin commands.
#[derive(Debug, Clone, Args)]
pub struct CommonCliArgsNoOrgId {
    #[arg(
        long,
        help = "Load connection defaults from the selected repo-local profile in grafana-util.yaml.",
        help_heading = "Authentication Options"
    )]
    pub profile: Option<String>,
    #[arg(
        long,
        default_value = "",
        hide_default_value = true,
        help = "Grafana base URL. Required unless supplied by --profile or GRAFANA_URL.",
        help_heading = "Authentication Options"
    )]
    pub url: String,
    #[arg(
        long = "token",
        visible_alias = "api-token",
        help = "Grafana API token. Preferred flag: --token. Falls back to GRAFANA_API_TOKEN.",
        help_heading = "Authentication Options"
    )]
    pub api_token: Option<String>,
    #[arg(
        long = "basic-user",
        help = "Grafana Basic auth username. Preferred flag: --basic-user. Falls back to GRAFANA_USERNAME.",
        help_heading = "Authentication Options"
    )]
    pub username: Option<String>,
    #[arg(
        long = "basic-password",
        help = "Grafana Basic auth password. Preferred flag: --basic-password. Falls back to GRAFANA_PASSWORD.",
        help_heading = "Authentication Options"
    )]
    pub password: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Prompt for the Grafana Basic auth password.",
        help_heading = "Authentication Options"
    )]
    pub prompt_password: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Prompt for the Grafana API token without echo instead of passing --token on the command line.",
        help_heading = "Authentication Options"
    )]
    pub prompt_token: bool,
    #[arg(long, default_value_t = DEFAULT_TIMEOUT, help = "HTTP timeout in seconds.", help_heading = "Transport Options")]
    pub timeout: u64,
    #[arg(
        long,
        default_value_t = false,
        help = "Enable TLS certificate verification. Verification is disabled by default.",
        help_heading = "Transport Options"
    )]
    pub verify_ssl: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["verify_ssl", "ca_cert"],
        help = "Disable TLS certificate verification explicitly.",
        help_heading = "Transport Options"
    )]
    pub insecure: bool,
    #[arg(
        long = "ca-cert",
        value_name = "PATH",
        help = "PEM bundle file to trust for Grafana TLS verification.",
        help_heading = "Transport Options"
    )]
    pub ca_cert: Option<PathBuf>,
}

/// Export/diff scope selectors for access inventory workflows.
#[derive(Debug, Clone, ValueEnum, PartialEq, Eq)]
pub enum Scope {
    Org,
    Global,
}

/// Supported output formats for access listing commands.
#[derive(Debug, Clone, ValueEnum, PartialEq, Eq)]
pub enum ListOutputFormat {
    Text,
    Table,
    Csv,
    Json,
    Yaml,
}

/// Supported output formats for destructive access dry-run flows.
#[derive(Debug, Clone, ValueEnum, PartialEq, Eq)]
pub enum DryRunOutputFormat {
    Text,
    Table,
    Json,
}

/// Supported output formats for access plan review flows.
#[derive(Debug, Clone, ValueEnum, PartialEq, Eq)]
pub enum PlanOutputFormat {
    Text,
    Table,
    Json,
}

// Parse bool-like CLI text using the explicit true/false contract used by
// back-compat flags that bypass Clap's native bool parsing.
pub(crate) fn parse_bool_text(value: &str) -> std::result::Result<bool, String> {
    // Call graph (hierarchy): this function is used in related modules.
    // Upstream callers: 無
    // Downstream callees: 無

    match value.trim().to_ascii_lowercase().as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err("value must be true or false".to_string()),
    }
}

pub(crate) fn parse_positive_usize(value: &str) -> std::result::Result<usize, String> {
    // Call graph (hierarchy): this function is used in related modules.
    // Upstream callers: 無
    // Downstream callees: 無

    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("invalid integer value: {value}"))?;
    if parsed < 1 {
        return Err("value must be >= 1".to_string());
    }
    Ok(parsed)
}
