//! Clap schema for alerting CLI commands.
//! Defines args/enums/normalization helpers used by alert dispatcher and handlers.
use clap::{Args, Command, CommandFactory, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use crate::common::{resolve_auth_headers, Result};

use super::{ALERT_HELP_TEXT, DEFAULT_OUTPUT_DIR, DEFAULT_TIMEOUT, DEFAULT_URL};

#[derive(Debug, Clone, Parser)]
#[command(
    name = "grafana-util alert",
    about = "Export, import, or diff Grafana alerting resources.",
    after_help = ALERT_HELP_TEXT,
    styles = crate::help_styles::CLI_HELP_STYLES
)]
struct AlertCliRoot {
    #[command(flatten)]
    args: AlertNamespaceArgs,
}

#[derive(Debug, Clone, Args)]
pub struct AlertCommonArgs {
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
        help = "Prompt for the Grafana Basic auth password without echo instead of passing --basic-password on the command line."
    )]
    pub prompt_password: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Prompt for the Grafana API token without echo instead of passing --token on the command line."
    )]
    pub prompt_token: bool,
    #[arg(long, default_value_t = DEFAULT_TIMEOUT, help = "HTTP timeout in seconds.")]
    pub timeout: u64,
    #[arg(
        long,
        default_value_t = false,
        help = "Enable TLS certificate verification. Verification is disabled by default."
    )]
    pub verify_ssl: bool,
}

#[derive(Debug, Clone, Args)]
pub struct AlertLegacyArgs {
    #[command(flatten)]
    pub common: AlertCommonArgs,
    #[arg(
        long,
        default_value = DEFAULT_OUTPUT_DIR,
        help = "Directory to write exported alerting resources into. Export writes files under raw/."
    )]
    pub output_dir: PathBuf,
    #[arg(
        long,
        conflicts_with = "diff_dir",
        help = "Import alerting resource JSON from this directory instead of exporting. Point this to the raw/ export directory explicitly."
    )]
    pub import_dir: Option<PathBuf>,
    #[arg(
        long,
        conflicts_with = "import_dir",
        help = "Compare alerting resource JSON from this directory against Grafana. Point this to the raw/ export directory explicitly."
    )]
    pub diff_dir: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        help = "Write rule, contact-point, mute-timing, and template files directly into their resource directories instead of nested subdirectories."
    )]
    pub flat: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Overwrite existing exported files."
    )]
    pub overwrite: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Update existing resources with the same identity instead of failing on import."
    )]
    pub replace_existing: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Show whether each import file would create or update resources without changing Grafana."
    )]
    pub dry_run: bool,
    #[arg(
        long,
        help = "JSON file that maps source dashboard UIDs to target dashboard UIDs for linked alert-rule repair during import."
    )]
    pub dashboard_uid_map: Option<PathBuf>,
    #[arg(
        long,
        help = "JSON file that maps source dashboard UID and source panel ID to a target panel ID for linked alert-rule repair during import."
    )]
    pub panel_id_map: Option<PathBuf>,
}

#[derive(Debug, Clone, Args)]
pub struct AlertExportArgs {
    #[command(flatten)]
    pub common: AlertCommonArgs,
    #[arg(
        long,
        default_value = DEFAULT_OUTPUT_DIR,
        help = "Directory to write exported alerting resources into. Export writes files under raw/."
    )]
    pub output_dir: PathBuf,
    #[arg(
        long,
        default_value_t = false,
        help = "Write rule, contact-point, mute-timing, and template files directly into their resource directories instead of nested subdirectories."
    )]
    pub flat: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Overwrite existing exported files."
    )]
    pub overwrite: bool,
}

#[derive(Debug, Clone, Args)]
pub struct AlertImportArgs {
    #[command(flatten)]
    pub common: AlertCommonArgs,
    #[arg(
        long,
        help = "Import alerting resource JSON from this directory instead of exporting. Point this to the raw/ export directory explicitly."
    )]
    pub import_dir: PathBuf,
    #[arg(
        long,
        default_value_t = false,
        help = "Update existing resources with the same identity instead of failing on import."
    )]
    pub replace_existing: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Show whether each import file would create or update resources without changing Grafana."
    )]
    pub dry_run: bool,
    #[arg(
        long,
        help = "JSON file that maps source dashboard UIDs to target dashboard UIDs for linked alert-rule repair during import."
    )]
    pub dashboard_uid_map: Option<PathBuf>,
    #[arg(
        long,
        help = "JSON file that maps source dashboard UID and source panel ID to a target panel ID for linked alert-rule repair during import."
    )]
    pub panel_id_map: Option<PathBuf>,
}

#[derive(Debug, Clone, Args)]
pub struct AlertDiffArgs {
    #[command(flatten)]
    pub common: AlertCommonArgs,
    #[arg(
        long,
        help = "Compare alerting resource JSON from this directory against Grafana. Point this to the raw/ export directory explicitly."
    )]
    pub diff_dir: PathBuf,
    #[arg(
        long,
        help = "JSON file that maps source dashboard UIDs to target dashboard UIDs for linked alert-rule repair during import."
    )]
    pub dashboard_uid_map: Option<PathBuf>,
    #[arg(
        long,
        help = "JSON file that maps source dashboard UID and source panel ID to a target panel ID for linked alert-rule repair during import."
    )]
    pub panel_id_map: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertListKind {
    Rules,
    ContactPoints,
    MuteTimings,
    Templates,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum AlertListOutputFormat {
    Table,
    Csv,
    Json,
}

#[derive(Debug, Clone, Args)]
pub struct AlertListArgs {
    #[command(flatten)]
    pub common: AlertCommonArgs,
    #[arg(
        long,
        default_value_t = false,
        help = "Render list output as a table. This is the default."
    )]
    pub table: bool,
    #[arg(long, default_value_t = false, help = "Render list output as CSV.")]
    pub csv: bool,
    #[arg(long, default_value_t = false, help = "Render list output as JSON.")]
    pub json: bool,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["table", "csv", "json"],
        help = "Alternative single-flag output selector. Use table, csv, or json."
    )]
    pub output_format: Option<AlertListOutputFormat>,
    #[arg(long, default_value_t = false, help = "Omit the table header row.")]
    pub no_header: bool,
}

#[derive(Debug, Clone, Subcommand)]
pub enum AlertGroupCommand {
    #[command(about = "Export alerting resources into raw/ JSON files.")]
    Export(AlertExportArgs),
    #[command(about = "Import alerting resource JSON files through the Grafana API.")]
    Import(AlertImportArgs),
    #[command(about = "Compare local alerting export files against live Grafana resources.")]
    Diff(AlertDiffArgs),
    #[command(name = "list-rules", about = "List live Grafana alert rules.")]
    ListRules(AlertListArgs),
    #[command(
        name = "list-contact-points",
        about = "List live Grafana alert contact points."
    )]
    ListContactPoints(AlertListArgs),
    #[command(name = "list-mute-timings", about = "List live Grafana mute timings.")]
    ListMuteTimings(AlertListArgs),
    #[command(
        name = "list-templates",
        about = "List live Grafana notification templates."
    )]
    ListTemplates(AlertListArgs),
}

#[derive(Debug, Clone, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct AlertNamespaceArgs {
    #[command(subcommand)]
    pub command: Option<AlertGroupCommand>,
    #[command(flatten)]
    pub legacy: AlertLegacyArgs,
}

#[derive(Debug, Clone)]
pub struct AlertCliArgs {
    pub url: String,
    pub api_token: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub prompt_password: bool,
    pub prompt_token: bool,
    pub output_dir: PathBuf,
    pub import_dir: Option<PathBuf>,
    pub diff_dir: Option<PathBuf>,
    pub timeout: u64,
    pub flat: bool,
    pub overwrite: bool,
    pub replace_existing: bool,
    pub dry_run: bool,
    pub dashboard_uid_map: Option<PathBuf>,
    pub panel_id_map: Option<PathBuf>,
    pub verify_ssl: bool,
    pub list_kind: Option<AlertListKind>,
    pub table: bool,
    pub csv: bool,
    pub json: bool,
    pub no_header: bool,
}

pub fn cli_args_from_common(common: AlertCommonArgs) -> AlertCliArgs {
    AlertCliArgs {
        url: common.url,
        api_token: common.api_token,
        username: common.username,
        password: common.password,
        prompt_password: common.prompt_password,
        prompt_token: common.prompt_token,
        output_dir: PathBuf::from(DEFAULT_OUTPUT_DIR),
        import_dir: None,
        diff_dir: None,
        timeout: common.timeout,
        flat: false,
        overwrite: false,
        replace_existing: false,
        dry_run: false,
        dashboard_uid_map: None,
        panel_id_map: None,
        verify_ssl: common.verify_ssl,
        list_kind: None,
        table: false,
        csv: false,
        json: false,
        no_header: false,
    }
}

fn empty_legacy_args() -> AlertLegacyArgs {
    AlertLegacyArgs {
        common: AlertCommonArgs {
            url: String::new(),
            api_token: None,
            username: None,
            password: None,
            prompt_password: false,
            prompt_token: false,
            timeout: 0,
            verify_ssl: false,
        },
        output_dir: PathBuf::new(),
        import_dir: None,
        diff_dir: None,
        flat: false,
        overwrite: false,
        replace_existing: false,
        dry_run: false,
        dashboard_uid_map: None,
        panel_id_map: None,
    }
}

#[derive(Debug, Clone)]
pub struct AlertAuthContext {
    pub url: String,
    pub timeout: u64,
    pub verify_ssl: bool,
    pub headers: Vec<(String, String)>,
}

// Parse alert argv into the namespace model and normalize it immediately into a
// flattened AlertCliArgs that downstream dispatch can execute directly.
pub fn parse_cli_from<I, T>(iter: I) -> AlertCliArgs
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    normalize_alert_namespace_args(AlertCliRoot::parse_from(iter).args)
}

pub fn root_command() -> Command {
    AlertCliRoot::command()
}

// Lift nested alert command variants into one canonical argument struct and
// apply single-output-mode migration for list commands.
pub fn normalize_alert_namespace_args(args: AlertNamespaceArgs) -> AlertCliArgs {
    fn apply_output_format(args: &mut AlertCliArgs, output_format: Option<AlertListOutputFormat>) {
        match output_format {
            Some(AlertListOutputFormat::Table) => args.table = true,
            Some(AlertListOutputFormat::Csv) => args.csv = true,
            Some(AlertListOutputFormat::Json) => args.json = true,
            None => {}
        }
    }

    match args.command {
        Some(AlertGroupCommand::Export(inner)) => {
            let mut args = cli_args_from_common(inner.common);
            args.output_dir = inner.output_dir;
            args.flat = inner.flat;
            args.overwrite = inner.overwrite;
            args
        }
        Some(AlertGroupCommand::Import(inner)) => {
            let mut args = cli_args_from_common(inner.common);
            args.import_dir = Some(inner.import_dir);
            args.replace_existing = inner.replace_existing;
            args.dry_run = inner.dry_run;
            args.dashboard_uid_map = inner.dashboard_uid_map;
            args.panel_id_map = inner.panel_id_map;
            args
        }
        Some(AlertGroupCommand::Diff(inner)) => {
            let mut args = cli_args_from_common(inner.common);
            args.diff_dir = Some(inner.diff_dir);
            args.dashboard_uid_map = inner.dashboard_uid_map;
            args.panel_id_map = inner.panel_id_map;
            args
        }
        Some(AlertGroupCommand::ListRules(inner)) => {
            let mut args = cli_args_from_common(inner.common);
            args.list_kind = Some(AlertListKind::Rules);
            args.table = inner.table;
            args.csv = inner.csv;
            args.json = inner.json;
            apply_output_format(&mut args, inner.output_format);
            args.no_header = inner.no_header;
            args
        }
        Some(AlertGroupCommand::ListContactPoints(inner)) => {
            let mut args = cli_args_from_common(inner.common);
            args.list_kind = Some(AlertListKind::ContactPoints);
            args.table = inner.table;
            args.csv = inner.csv;
            args.json = inner.json;
            apply_output_format(&mut args, inner.output_format);
            args.no_header = inner.no_header;
            args
        }
        Some(AlertGroupCommand::ListMuteTimings(inner)) => {
            let mut args = cli_args_from_common(inner.common);
            args.list_kind = Some(AlertListKind::MuteTimings);
            args.table = inner.table;
            args.csv = inner.csv;
            args.json = inner.json;
            apply_output_format(&mut args, inner.output_format);
            args.no_header = inner.no_header;
            args
        }
        Some(AlertGroupCommand::ListTemplates(inner)) => {
            let mut args = cli_args_from_common(inner.common);
            args.list_kind = Some(AlertListKind::Templates);
            args.table = inner.table;
            args.csv = inner.csv;
            args.json = inner.json;
            apply_output_format(&mut args, inner.output_format);
            args.no_header = inner.no_header;
            args
        }
        None => {
            let legacy = args.legacy;
            AlertCliArgs {
                url: legacy.common.url,
                api_token: legacy.common.api_token,
                username: legacy.common.username,
                password: legacy.common.password,
                prompt_password: legacy.common.prompt_password,
                prompt_token: legacy.common.prompt_token,
                output_dir: legacy.output_dir,
                import_dir: legacy.import_dir,
                diff_dir: legacy.diff_dir,
                timeout: legacy.common.timeout,
                flat: legacy.flat,
                overwrite: legacy.overwrite,
                replace_existing: legacy.replace_existing,
                dry_run: legacy.dry_run,
                dashboard_uid_map: legacy.dashboard_uid_map,
                panel_id_map: legacy.panel_id_map,
                verify_ssl: legacy.common.verify_ssl,
                list_kind: None,
                table: false,
                csv: false,
                json: false,
                no_header: false,
            }
        }
    }
}

// Small adapter for callers that already have a concrete group command and need
// the full normalized AlertCliArgs form.
pub fn normalize_alert_group_command(command: AlertGroupCommand) -> AlertCliArgs {
    normalize_alert_namespace_args(AlertNamespaceArgs {
        command: Some(command),
        legacy: empty_legacy_args(),
    })
}

pub fn build_auth_context(args: &AlertCliArgs) -> Result<AlertAuthContext> {
    Ok(AlertAuthContext {
        url: args.url.clone(),
        timeout: args.timeout,
        verify_ssl: args.verify_ssl,
        headers: resolve_auth_headers(
            args.api_token.as_deref(),
            args.username.as_deref(),
            args.password.as_deref(),
            args.prompt_password,
            args.prompt_token,
        )?,
    })
}
