//! Clap schema for alerting CLI commands.
//! Defines args used by alert dispatcher and handlers.

use clap::{ArgAction, Args, Command, CommandFactory, Parser};
use std::path::PathBuf;

use crate::common::{set_json_color_choice, CliColorChoice, DiffOutputFormat};

use super::super::{ALERT_HELP_TEXT, DEFAULT_OUTPUT_DIR, DEFAULT_TIMEOUT, DEFAULT_URL};
use super::alert_cli_commands::{
    AlertAuthoringCommandKind, AlertCommandKind, AlertCommandOutputFormat, AlertGroupCommand,
    AlertListKind, AlertListOutputFormat, AlertResourceKind,
};

#[derive(Debug, Clone, Parser)]
#[command(
    name = "grafana-util alert",
    about = "Export, manage, and author Grafana alerting resources.",
    after_help = ALERT_HELP_TEXT,
    styles = crate::help_styles::CLI_HELP_STYLES
)]
struct AlertCliRoot {
    #[arg(
        long,
        value_enum,
        default_value_t = CliColorChoice::Auto,
        help = "Colorize JSON output. Use auto, always, or never."
    )]
    color: CliColorChoice,
    #[command(flatten)]
    args: AlertNamespaceArgs,
}

/// Shared Grafana connection/authentication arguments for alert commands.
#[derive(Debug, Clone, Args)]
pub struct AlertCommonArgs {
    #[arg(
        long,
        help = "Load connection defaults from the selected repo-local profile in grafana-util.yaml.",
        help_heading = "Connection Options"
    )]
    pub profile: Option<String>,
    #[arg(
        long,
        default_value = "",
        hide_default_value = true,
        help = "Grafana base URL. Required unless supplied by --profile or GRAFANA_URL.",
        help_heading = "Connection Options"
    )]
    pub url: String,
    #[arg(
        long = "token",
        visible_alias = "api-token",
        help = "Grafana API token. Preferred flag: --token. Falls back to GRAFANA_API_TOKEN.",
        help_heading = "Connection Options"
    )]
    pub api_token: Option<String>,
    #[arg(
        long = "basic-user",
        help = "Grafana Basic auth username. Preferred flag: --basic-user. Falls back to GRAFANA_USERNAME.",
        help_heading = "Connection Options"
    )]
    pub username: Option<String>,
    #[arg(
        long = "basic-password",
        help = "Grafana Basic auth password. Preferred flag: --basic-password. Falls back to GRAFANA_PASSWORD.",
        help_heading = "Connection Options"
    )]
    pub password: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Prompt for the Grafana Basic auth password without echo instead of passing --basic-password on the command line.",
        help_heading = "Connection Options"
    )]
    pub prompt_password: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Prompt for the Grafana API token without echo instead of passing --token on the command line.",
        help_heading = "Connection Options"
    )]
    pub prompt_token: bool,
    #[arg(
        long,
        default_value_t = DEFAULT_TIMEOUT,
        help = "HTTP timeout in seconds.",
        help_heading = "Transport Options"
    )]
    pub timeout: u64,
    #[arg(
        long,
        default_value_t = false,
        help = "Enable TLS certificate verification. Verification is disabled by default.",
        help_heading = "Transport Options"
    )]
    pub verify_ssl: bool,
}

/// Legacy flat alert CLI shape kept for compatibility with older invocation styles.
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
        long = "input-dir",
        conflicts_with = "diff_dir",
        help = "Import alerting resource JSON from this directory instead of exporting. Point this to the raw/ export directory explicitly."
    )]
    pub input_dir: Option<PathBuf>,
    #[arg(
        long,
        conflicts_with = "input_dir",
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

/// Arguments for exporting alerting resources from Grafana.
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

/// Arguments for importing alerting resources from a local export directory.
#[derive(Debug, Clone, Args)]
pub struct AlertImportArgs {
    #[command(flatten)]
    pub common: AlertCommonArgs,
    #[arg(
        long = "input-dir",
        help = "Import alerting resource JSON from this directory instead of exporting. Point this to the raw/ export directory explicitly."
    )]
    pub input_dir: PathBuf,
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
        default_value_t = false,
        help = "Render dry-run import output as structured JSON. Only supported with --dry-run."
    )]
    pub json: bool,
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

/// Arguments for diffing local alert exports against live Grafana state.
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
        default_value_t = false,
        help = "Deprecated compatibility flag. Equivalent to --output-format json."
    )]
    pub json: bool,
    #[arg(
        long = "output-format",
        value_enum,
        default_value_t = DiffOutputFormat::Text,
        help = "Render diff output as text or json."
    )]
    pub output_format: DiffOutputFormat,
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

/// Struct definition for AlertListArgs.
#[derive(Debug, Clone, Args)]
pub struct AlertListArgs {
    #[command(flatten)]
    pub common: AlertCommonArgs,
    #[arg(
        long,
        conflicts_with = "all_orgs",
        help = "List alerting resources from this Grafana org ID. This requires Basic auth."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "org_id",
        help = "Enumerate all visible Grafana orgs and aggregate alerting inventory across them. This requires Basic auth."
    )]
    pub all_orgs: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["table", "csv", "json", "yaml", "output_format"],
        help = "Render list output as plain text.",
        help_heading = "Output Options"
    )]
    pub text: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["text", "csv", "json", "yaml", "output_format"],
        help = "Render list output as a table. This is the default.",
        help_heading = "Output Options"
    )]
    pub table: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["text", "table", "json", "yaml", "output_format"],
        help = "Render list output as CSV.",
        help_heading = "Output Options"
    )]
    pub csv: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["text", "table", "csv", "yaml", "output_format"],
        help = "Render list output as JSON.",
        help_heading = "Output Options"
    )]
    pub json: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["text", "table", "csv", "json", "output_format"],
        help = "Render list output as YAML.",
        help_heading = "Output Options"
    )]
    pub yaml: bool,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["text", "table", "csv", "json", "yaml"],
        help = "Alternative single-flag output selector. Use text, table, csv, json, or yaml.",
        help_heading = "Output Options"
    )]
    pub output_format: Option<AlertListOutputFormat>,
    #[arg(
        long,
        default_value_t = false,
        help = "Omit the table header row.",
        help_heading = "Output Options"
    )]
    pub no_header: bool,
}

/// Arguments for building a staged alert apply plan.
#[derive(Debug, Clone, Args)]
pub struct AlertPlanArgs {
    #[command(flatten)]
    pub common: AlertCommonArgs,
    #[arg(
        long,
        help = "Directory containing the desired alert resource definitions to plan from."
    )]
    pub desired_dir: PathBuf,
    #[arg(
        long,
        default_value_t = false,
        help = "Mark live-only alert resources as delete candidates in the staged plan."
    )]
    pub prune: bool,
    #[arg(
        long,
        help = "JSON file that maps source dashboard UIDs to target dashboard UIDs for linked alert-rule repair during planning."
    )]
    pub dashboard_uid_map: Option<PathBuf>,
    #[arg(
        long,
        help = "JSON file that maps source dashboard UID and source panel ID to a target panel ID for linked alert-rule repair during planning."
    )]
    pub panel_id_map: Option<PathBuf>,
    #[arg(
        long = "output-format",
        value_enum,
        default_value_t = AlertCommandOutputFormat::Text,
        help = "Render plan output as text or json."
    )]
    pub output_format: AlertCommandOutputFormat,
}

/// Arguments for applying a reviewed alert plan.
#[derive(Debug, Clone, Args)]
pub struct AlertApplyArgs {
    #[command(flatten)]
    pub common: AlertCommonArgs,
    #[arg(long, help = "JSON file containing the reviewed alert plan document.")]
    pub plan_file: PathBuf,
    #[arg(
        long,
        action = ArgAction::SetTrue,
        required = true,
        help = "Explicit acknowledgement required before alert apply execution is allowed."
    )]
    pub approve: bool,
    #[arg(
        long = "output-format",
        value_enum,
        default_value_t = AlertCommandOutputFormat::Text,
        help = "Render apply output as text or json."
    )]
    pub output_format: AlertCommandOutputFormat,
}

/// Arguments for deleting one managed alert resource.
#[derive(Debug, Clone, Args)]
pub struct AlertDeleteArgs {
    #[command(flatten)]
    pub common: AlertCommonArgs,
    #[arg(long, value_enum, help = "Alert resource kind to delete.")]
    pub kind: AlertResourceKind,
    #[arg(
        long,
        help = "Explicit resource identity for the selected delete kind."
    )]
    pub identity: String,
    #[arg(
        long,
        default_value_t = false,
        help = "Allow notification policy tree reset when deleting the policy-tree resource kind."
    )]
    pub allow_policy_reset: bool,
    #[arg(
        long = "output-format",
        value_enum,
        default_value_t = AlertCommandOutputFormat::Text,
        help = "Render delete preview or execution output as text or json."
    )]
    pub output_format: AlertCommandOutputFormat,
}

/// Arguments for initializing a staged alert desired-state layout.
#[derive(Debug, Clone, Args)]
pub struct AlertInitArgs {
    #[arg(
        long,
        help = "Directory to initialize with staged alert desired-state scaffolding."
    )]
    pub desired_dir: PathBuf,
}

/// Shared arguments for new staged alert resource scaffolds.
#[derive(Debug, Clone, Args)]
pub struct AlertNewResourceArgs {
    #[arg(
        long,
        help = "Directory containing the staged alert desired-state layout."
    )]
    pub desired_dir: PathBuf,
    #[arg(long, help = "Resource name to seed into the new scaffold.")]
    pub name: String,
}

/// Shared authoring inputs for desired-state alert resource commands.
#[derive(Debug, Clone, Args)]
pub struct AlertAuthoringBaseArgs {
    #[arg(
        long,
        help = "Directory containing the staged alert desired-state layout."
    )]
    pub desired_dir: PathBuf,
}

/// Authoring inputs for high-level add-rule.
#[derive(Debug, Clone, Args)]
pub struct AlertAddRuleArgs {
    #[command(flatten)]
    pub base: AlertAuthoringBaseArgs,
    #[arg(long, help = "Rule name to author.")]
    pub name: String,
    #[arg(long, help = "Folder that will own the authored rule.")]
    pub folder: String,
    #[arg(long = "rule-group", help = "Rule group name inside the folder.")]
    pub rule_group: String,
    #[arg(
        long,
        required_unless_present = "no_route",
        help = "Receiver name to route the authored rule to."
    )]
    pub receiver: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "receiver",
        help = "Skip route authoring for this rule."
    )]
    pub no_route: bool,
    #[arg(
        long = "label",
        help = "Rule label in key=value form. Repeat for more labels."
    )]
    pub labels: Vec<String>,
    #[arg(
        long = "annotation",
        help = "Rule annotation in key=value form. Repeat for more annotations."
    )]
    pub annotations: Vec<String>,
    #[arg(long, help = "Convenience severity label value for the authored rule.")]
    pub severity: Option<String>,
    #[arg(long = "for", help = "Pending duration before the rule starts firing.")]
    pub for_duration: Option<String>,
    #[arg(long, help = "Simple-rule expression reference or expression text.")]
    pub expr: Option<String>,
    #[arg(long, help = "Simple-rule threshold value.")]
    pub threshold: Option<f64>,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "below",
        help = "Fire when the evaluated value is above the threshold."
    )]
    pub above: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "above",
        help = "Fire when the evaluated value is below the threshold."
    )]
    pub below: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Render the planned authored rule output without writing files."
    )]
    pub dry_run: bool,
}

/// Authoring inputs for rule cloning.
#[derive(Debug, Clone, Args)]
pub struct AlertCloneRuleArgs {
    #[command(flatten)]
    pub base: AlertAuthoringBaseArgs,
    #[arg(
        long,
        help = "Existing rule identity to clone from the desired-state tree."
    )]
    pub source: String,
    #[arg(long, help = "New rule name for the cloned rule.")]
    pub name: String,
    #[arg(long, help = "Optional replacement folder for the cloned rule.")]
    pub folder: Option<String>,
    #[arg(
        long = "rule-group",
        help = "Optional replacement rule group for the cloned rule."
    )]
    pub rule_group: Option<String>,
    #[arg(
        long,
        help = "Optional replacement receiver for the cloned rule route."
    )]
    pub receiver: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "receiver",
        help = "Clear route authoring while cloning."
    )]
    pub no_route: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Render the planned cloned rule output without writing files."
    )]
    pub dry_run: bool,
}

/// Authoring inputs for contact point creation.
#[derive(Debug, Clone, Args)]
pub struct AlertAddContactPointArgs {
    #[command(flatten)]
    pub base: AlertAuthoringBaseArgs,
    #[arg(long, help = "Contact point name to author.")]
    pub name: String,
    #[arg(
        long,
        default_value_t = false,
        help = "Render the planned authored contact point output without writing files."
    )]
    pub dry_run: bool,
}

/// Authoring inputs for notification route updates.
#[derive(Debug, Clone, Args)]
pub struct AlertSetRouteArgs {
    #[command(flatten)]
    pub base: AlertAuthoringBaseArgs,
    #[arg(long, help = "Receiver name for the route.")]
    pub receiver: String,
    #[arg(
        long = "label",
        help = "Route matcher in key=value form. Repeat for more matchers."
    )]
    pub labels: Vec<String>,
    #[arg(long, help = "Convenience severity matcher value for the route.")]
    pub severity: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Render the managed route document that would replace the tool-owned route without writing files."
    )]
    pub dry_run: bool,
}

/// Authoring inputs for route previews.
#[derive(Debug, Clone, Args)]
pub struct AlertPreviewRouteArgs {
    #[command(flatten)]
    pub base: AlertAuthoringBaseArgs,
    #[arg(
        long = "label",
        help = "Preview label in key=value form. Repeat for more labels."
    )]
    pub labels: Vec<String>,
    #[arg(long, help = "Convenience severity label value for route preview.")]
    pub severity: Option<String>,
}

/// Struct definition for AlertNamespaceArgs.
#[derive(Debug, Clone, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct AlertNamespaceArgs {
    #[command(subcommand)]
    pub command: Option<AlertGroupCommand>,
    #[command(flatten)]
    pub legacy: AlertLegacyArgs,
}

/// Struct definition for AlertCliArgs.
#[derive(Debug, Clone)]
pub struct AlertCliArgs {
    pub command_kind: Option<AlertCommandKind>,
    pub authoring_command_kind: Option<AlertAuthoringCommandKind>,
    pub profile: Option<String>,
    pub url: String,
    pub api_token: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub prompt_password: bool,
    pub prompt_token: bool,
    pub output_dir: PathBuf,
    pub input_dir: Option<PathBuf>,
    pub diff_dir: Option<PathBuf>,
    pub timeout: u64,
    pub flat: bool,
    pub overwrite: bool,
    pub replace_existing: bool,
    pub dry_run: bool,
    pub dashboard_uid_map: Option<PathBuf>,
    pub panel_id_map: Option<PathBuf>,
    pub verify_ssl: bool,
    pub org_id: Option<i64>,
    pub all_orgs: bool,
    pub list_kind: Option<AlertListKind>,
    pub text: bool,
    pub table: bool,
    pub csv: bool,
    pub json: bool,
    pub yaml: bool,
    pub no_header: bool,
    pub desired_dir: Option<PathBuf>,
    pub prune: bool,
    pub plan_file: Option<PathBuf>,
    pub approve: bool,
    pub allow_policy_reset: bool,
    pub resource_kind: Option<AlertResourceKind>,
    pub resource_identity: Option<String>,
    pub command_output: Option<AlertCommandOutputFormat>,
    pub diff_output: Option<DiffOutputFormat>,
    pub scaffold_name: Option<String>,
    pub source_name: Option<String>,
    pub folder: Option<String>,
    pub rule_group: Option<String>,
    pub receiver: Option<String>,
    pub no_route: bool,
    pub labels: Vec<String>,
    pub annotations: Vec<String>,
    pub severity: Option<String>,
    pub for_duration: Option<String>,
    pub expr: Option<String>,
    pub threshold: Option<f64>,
    pub above: bool,
    pub below: bool,
}

/// cli args from common.
pub fn cli_args_from_common(common: AlertCommonArgs) -> AlertCliArgs {
    AlertCliArgs {
        command_kind: None,
        authoring_command_kind: None,
        profile: common.profile,
        url: common.url,
        api_token: common.api_token,
        username: common.username,
        password: common.password,
        prompt_password: common.prompt_password,
        prompt_token: common.prompt_token,
        output_dir: PathBuf::from(DEFAULT_OUTPUT_DIR),
        input_dir: None,
        diff_dir: None,
        timeout: common.timeout,
        flat: false,
        overwrite: false,
        replace_existing: false,
        dry_run: false,
        dashboard_uid_map: None,
        panel_id_map: None,
        verify_ssl: common.verify_ssl,
        org_id: None,
        all_orgs: false,
        list_kind: None,
        text: false,
        table: false,
        csv: false,
        json: false,
        yaml: false,
        no_header: false,
        desired_dir: None,
        prune: false,
        plan_file: None,
        approve: false,
        allow_policy_reset: false,
        resource_kind: None,
        resource_identity: None,
        command_output: None,
        diff_output: None,
        scaffold_name: None,
        source_name: None,
        folder: None,
        rule_group: None,
        receiver: None,
        no_route: false,
        labels: Vec::new(),
        annotations: Vec::new(),
        severity: None,
        for_duration: None,
        expr: None,
        threshold: None,
        above: false,
        below: false,
    }
}

pub(crate) fn cli_args_from_defaults() -> AlertCliArgs {
    cli_args_from_common(AlertCommonArgs {
        profile: None,
        url: DEFAULT_URL.to_string(),
        api_token: None,
        username: None,
        password: None,
        prompt_password: false,
        prompt_token: false,
        timeout: DEFAULT_TIMEOUT,
        verify_ssl: false,
    })
}

pub(crate) fn empty_legacy_args() -> AlertLegacyArgs {
    AlertLegacyArgs {
        common: AlertCommonArgs {
            profile: None,
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
        input_dir: None,
        diff_dir: None,
        flat: false,
        overwrite: false,
        replace_existing: false,
        dry_run: false,
        dashboard_uid_map: None,
        panel_id_map: None,
    }
}

/// Parse alert argv into the namespace model and normalize it immediately into a
/// flattened AlertCliArgs that downstream dispatch can execute directly.
pub fn parse_cli_from<I, T>(iter: I) -> AlertCliArgs
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let root = AlertCliRoot::parse_from(iter);
    set_json_color_choice(root.color);
    super::alert_cli_normalize::normalize_alert_namespace_args(root.args)
}

/// root command.
pub fn root_command() -> Command {
    AlertCliRoot::command()
}
