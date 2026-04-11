//! Clap schema for alerting CLI commands.
//! Defines args/enums/normalization helpers used by alert dispatcher and handlers.
use clap::{ArgAction, Args, Command, CommandFactory, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use crate::common::{set_json_color_choice, CliColorChoice, DiffOutputFormat, Result};
use crate::grafana_api::{AuthInputs, GrafanaConnection};
use crate::profile_config::ConnectionMergeInput;

#[path = "alert_help_texts.rs"]
mod alert_help_texts;
use self::alert_help_texts::*;

use super::{ALERT_HELP_TEXT, DEFAULT_OUTPUT_DIR, DEFAULT_TIMEOUT, DEFAULT_URL};

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

/// Resource categories supported by alert list operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertListKind {
    Rules,
    ContactPoints,
    MuteTimings,
    Templates,
}

/// Enum definition for AlertListOutputFormat.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum AlertListOutputFormat {
    Text,
    Table,
    Csv,
    Json,
    Yaml,
}

/// Single-flag output selector for plan-oriented alert commands.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum AlertCommandOutputFormat {
    Text,
    Json,
}

/// Resource categories supported by alert delete scaffolding.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum AlertResourceKind {
    Rule,
    ContactPoint,
    MuteTiming,
    PolicyTree,
    Template,
}

/// Canonical subcommand identity for normalized alert CLI routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertCommandKind {
    Export,
    Import,
    Diff,
    Plan,
    Apply,
    Delete,
    Init,
    NewRule,
    NewContactPoint,
    NewTemplate,
    ListRules,
    ListContactPoints,
    ListMuteTimings,
    ListTemplates,
}

/// Authoring-focused subcommand identity carried only by parser/help surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertAuthoringCommandKind {
    AddRule,
    CloneRule,
    AddContactPoint,
    SetRoute,
    PreviewRoute,
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

/// Enum definition for AlertGroupCommand.
#[derive(Debug, Clone, Subcommand)]
pub enum AlertGroupCommand {
    #[command(
        about = "Export alerting resources into raw/ JSON files.",
        after_help = ALERT_EXPORT_HELP_TEXT
    )]
    Export(AlertExportArgs),
    #[command(
        about = "Import alerting resource JSON files through the Grafana API.",
        after_help = ALERT_IMPORT_HELP_TEXT
    )]
    Import(AlertImportArgs),
    #[command(
        about = "Compare local alerting export files against live Grafana resources.",
        after_help = ALERT_DIFF_HELP_TEXT
    )]
    Diff(AlertDiffArgs),
    #[command(
        about = "Build a staged alert management plan from desired alert resources.",
        after_help = ALERT_PLAN_HELP_TEXT
    )]
    Plan(AlertPlanArgs),
    #[command(
        about = "Apply a reviewed alert management plan.",
        after_help = ALERT_APPLY_HELP_TEXT
    )]
    Apply(AlertApplyArgs),
    #[command(
        about = "Delete one explicit alert resource identity.",
        after_help = ALERT_DELETE_HELP_TEXT
    )]
    Delete(AlertDeleteArgs),
    #[command(
        about = "Initialize a staged alert desired-state layout.",
        after_help = ALERT_INIT_HELP_TEXT
    )]
    Init(AlertInitArgs),
    #[command(
        name = "add-rule",
        about = "Author a staged alert rule from the higher-level authoring surface.",
        after_help = ALERT_ADD_RULE_HELP_TEXT
    )]
    AddRule(AlertAddRuleArgs),
    #[command(
        name = "clone-rule",
        about = "Clone an existing staged alert rule into a new authoring target.",
        after_help = ALERT_CLONE_RULE_HELP_TEXT
    )]
    CloneRule(AlertCloneRuleArgs),
    #[command(
        name = "add-contact-point",
        about = "Author a staged alert contact point from the higher-level authoring surface.",
        after_help = ALERT_ADD_CONTACT_POINT_HELP_TEXT
    )]
    AddContactPoint(AlertAddContactPointArgs),
    #[command(
        name = "set-route",
        about = "Author or replace the tool-owned staged notification route. Re-running fully replaces that managed route instead of merging fields.",
        after_help = ALERT_SET_ROUTE_HELP_TEXT
    )]
    SetRoute(AlertSetRouteArgs),
    #[command(
        name = "preview-route",
        about = "Preview the managed route inputs without changing runtime behavior. The corresponding set-route command fully replaces the tool-owned route on rerun.",
        after_help = ALERT_PREVIEW_ROUTE_HELP_TEXT
    )]
    PreviewRoute(AlertPreviewRouteArgs),
    #[command(
        name = "new-rule",
        about = "Create a low-level staged alert rule scaffold.",
        after_help = ALERT_NEW_RULE_HELP_TEXT
    )]
    NewRule(AlertNewResourceArgs),
    #[command(
        name = "new-contact-point",
        about = "Create a low-level staged alert contact point scaffold.",
        after_help = ALERT_NEW_CONTACT_POINT_HELP_TEXT
    )]
    NewContactPoint(AlertNewResourceArgs),
    #[command(
        name = "new-template",
        about = "Create a low-level staged alert template scaffold.",
        after_help = ALERT_NEW_TEMPLATE_HELP_TEXT
    )]
    NewTemplate(AlertNewResourceArgs),
    #[command(
        name = "list-rules",
        about = "List live Grafana alert rules.",
        after_help = ALERT_LIST_RULES_HELP_TEXT
    )]
    ListRules(AlertListArgs),
    #[command(
        name = "list-contact-points",
        about = "List live Grafana alert contact points.",
        after_help = ALERT_LIST_CONTACT_POINTS_HELP_TEXT
    )]
    ListContactPoints(AlertListArgs),
    #[command(
        name = "list-mute-timings",
        about = "List live Grafana mute timings.",
        after_help = ALERT_LIST_MUTE_TIMINGS_HELP_TEXT
    )]
    ListMuteTimings(AlertListArgs),
    #[command(
        name = "list-templates",
        about = "List live Grafana notification templates.",
        after_help = ALERT_LIST_TEMPLATES_HELP_TEXT
    )]
    ListTemplates(AlertListArgs),
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

fn cli_args_from_defaults() -> AlertCliArgs {
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

fn empty_legacy_args() -> AlertLegacyArgs {
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

/// Struct definition for AlertAuthContext.
#[derive(Debug, Clone)]
pub struct AlertAuthContext {
    pub url: String,
    pub timeout: u64,
    pub verify_ssl: bool,
    pub headers: Vec<(String, String)>,
}

/// Parse alert argv into the namespace model and normalize it immediately into a
/// flattened AlertCliArgs that downstream dispatch can execute directly.
pub fn parse_cli_from<I, T>(iter: I) -> AlertCliArgs
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    // Call graph (hierarchy): this function is used in related modules.
    // Upstream callers: 無
    // Downstream callees: alert_cli_defs.rs:normalize_alert_namespace_args

    let root = AlertCliRoot::parse_from(iter);
    set_json_color_choice(root.color);
    normalize_alert_namespace_args(root.args)
}

/// root command.
pub fn root_command() -> Command {
    // Call graph (hierarchy): this function is used in related modules.
    // Upstream callers: 無
    // Downstream callees: 無

    AlertCliRoot::command()
}

/// Lift nested alert command variants into one canonical argument struct and
/// apply single-output-mode migration for list commands.
pub fn normalize_alert_namespace_args(args: AlertNamespaceArgs) -> AlertCliArgs {
    fn apply_output_format(args: &mut AlertCliArgs, output_format: Option<AlertListOutputFormat>) {
        match output_format {
            Some(AlertListOutputFormat::Text) => args.text = true,
            Some(AlertListOutputFormat::Table) => args.table = true,
            Some(AlertListOutputFormat::Csv) => args.csv = true,
            Some(AlertListOutputFormat::Json) => args.json = true,
            Some(AlertListOutputFormat::Yaml) => args.yaml = true,
            None => {}
        }
    }

    match args.command {
        Some(AlertGroupCommand::Export(inner)) => {
            let mut args = cli_args_from_common(inner.common);
            args.command_kind = Some(AlertCommandKind::Export);
            args.output_dir = inner.output_dir;
            args.flat = inner.flat;
            args.overwrite = inner.overwrite;
            args
        }
        Some(AlertGroupCommand::Import(inner)) => {
            let mut args = cli_args_from_common(inner.common);
            args.command_kind = Some(AlertCommandKind::Import);
            args.input_dir = Some(inner.input_dir);
            args.replace_existing = inner.replace_existing;
            args.dry_run = inner.dry_run;
            args.json = inner.json;
            args.dashboard_uid_map = inner.dashboard_uid_map;
            args.panel_id_map = inner.panel_id_map;
            args
        }
        Some(AlertGroupCommand::Diff(inner)) => {
            let mut args = cli_args_from_common(inner.common);
            args.command_kind = Some(AlertCommandKind::Diff);
            args.diff_dir = Some(inner.diff_dir);
            args.diff_output = Some(if inner.json {
                DiffOutputFormat::Json
            } else {
                inner.output_format
            });
            args.json = inner.json || matches!(inner.output_format, DiffOutputFormat::Json);
            args.dashboard_uid_map = inner.dashboard_uid_map;
            args.panel_id_map = inner.panel_id_map;
            args
        }
        Some(AlertGroupCommand::Plan(inner)) => {
            let mut args = cli_args_from_common(inner.common);
            args.command_kind = Some(AlertCommandKind::Plan);
            args.desired_dir = Some(inner.desired_dir);
            args.prune = inner.prune;
            args.dashboard_uid_map = inner.dashboard_uid_map;
            args.panel_id_map = inner.panel_id_map;
            args.command_output = Some(inner.output_format);
            args.json = matches!(inner.output_format, AlertCommandOutputFormat::Json);
            args
        }
        Some(AlertGroupCommand::Apply(inner)) => {
            let mut args = cli_args_from_common(inner.common);
            args.command_kind = Some(AlertCommandKind::Apply);
            args.plan_file = Some(inner.plan_file);
            args.approve = inner.approve;
            args.command_output = Some(inner.output_format);
            args.json = matches!(inner.output_format, AlertCommandOutputFormat::Json);
            args
        }
        Some(AlertGroupCommand::Delete(inner)) => {
            let mut args = cli_args_from_common(inner.common);
            args.command_kind = Some(AlertCommandKind::Delete);
            args.resource_kind = Some(inner.kind);
            args.resource_identity = Some(inner.identity);
            args.allow_policy_reset = inner.allow_policy_reset;
            args.command_output = Some(inner.output_format);
            args.json = matches!(inner.output_format, AlertCommandOutputFormat::Json);
            args
        }
        Some(AlertGroupCommand::Init(inner)) => {
            let mut args = cli_args_from_defaults();
            args.command_kind = Some(AlertCommandKind::Init);
            args.desired_dir = Some(inner.desired_dir);
            args
        }
        Some(AlertGroupCommand::AddRule(inner)) => {
            let mut args = cli_args_from_defaults();
            args.authoring_command_kind = Some(AlertAuthoringCommandKind::AddRule);
            args.desired_dir = Some(inner.base.desired_dir);
            args.scaffold_name = Some(inner.name);
            args.folder = Some(inner.folder);
            args.rule_group = Some(inner.rule_group);
            args.receiver = inner.receiver;
            args.no_route = inner.no_route;
            args.labels = inner.labels;
            args.annotations = inner.annotations;
            args.severity = inner.severity;
            args.for_duration = inner.for_duration;
            args.expr = inner.expr;
            args.threshold = inner.threshold;
            args.above = inner.above;
            args.below = inner.below;
            args.dry_run = inner.dry_run;
            args
        }
        Some(AlertGroupCommand::CloneRule(inner)) => {
            let mut args = cli_args_from_defaults();
            args.authoring_command_kind = Some(AlertAuthoringCommandKind::CloneRule);
            args.desired_dir = Some(inner.base.desired_dir);
            args.source_name = Some(inner.source);
            args.scaffold_name = Some(inner.name);
            args.folder = inner.folder;
            args.rule_group = inner.rule_group;
            args.receiver = inner.receiver;
            args.no_route = inner.no_route;
            args.dry_run = inner.dry_run;
            args
        }
        Some(AlertGroupCommand::AddContactPoint(inner)) => {
            let mut args = cli_args_from_defaults();
            args.authoring_command_kind = Some(AlertAuthoringCommandKind::AddContactPoint);
            args.desired_dir = Some(inner.base.desired_dir);
            args.scaffold_name = Some(inner.name);
            args.dry_run = inner.dry_run;
            args
        }
        Some(AlertGroupCommand::SetRoute(inner)) => {
            let mut args = cli_args_from_defaults();
            args.authoring_command_kind = Some(AlertAuthoringCommandKind::SetRoute);
            args.desired_dir = Some(inner.base.desired_dir);
            args.receiver = Some(inner.receiver);
            args.labels = inner.labels;
            args.severity = inner.severity;
            args.dry_run = inner.dry_run;
            args
        }
        Some(AlertGroupCommand::PreviewRoute(inner)) => {
            let mut args = cli_args_from_defaults();
            args.authoring_command_kind = Some(AlertAuthoringCommandKind::PreviewRoute);
            args.desired_dir = Some(inner.base.desired_dir);
            args.labels = inner.labels;
            args.severity = inner.severity;
            args
        }
        Some(AlertGroupCommand::NewRule(inner)) => {
            let mut args = cli_args_from_defaults();
            args.command_kind = Some(AlertCommandKind::NewRule);
            args.desired_dir = Some(inner.desired_dir);
            args.scaffold_name = Some(inner.name);
            args
        }
        Some(AlertGroupCommand::NewContactPoint(inner)) => {
            let mut args = cli_args_from_defaults();
            args.command_kind = Some(AlertCommandKind::NewContactPoint);
            args.desired_dir = Some(inner.desired_dir);
            args.scaffold_name = Some(inner.name);
            args
        }
        Some(AlertGroupCommand::NewTemplate(inner)) => {
            let mut args = cli_args_from_defaults();
            args.command_kind = Some(AlertCommandKind::NewTemplate);
            args.desired_dir = Some(inner.desired_dir);
            args.scaffold_name = Some(inner.name);
            args
        }
        Some(AlertGroupCommand::ListRules(inner)) => {
            let mut args = cli_args_from_common(inner.common);
            args.command_kind = Some(AlertCommandKind::ListRules);
            args.list_kind = Some(AlertListKind::Rules);
            args.org_id = inner.org_id;
            args.all_orgs = inner.all_orgs;
            args.text = inner.text;
            args.table = inner.table;
            args.csv = inner.csv;
            args.json = inner.json;
            args.yaml = inner.yaml;
            apply_output_format(&mut args, inner.output_format);
            args.no_header = inner.no_header;
            args
        }
        Some(AlertGroupCommand::ListContactPoints(inner)) => {
            let mut args = cli_args_from_common(inner.common);
            args.command_kind = Some(AlertCommandKind::ListContactPoints);
            args.list_kind = Some(AlertListKind::ContactPoints);
            args.org_id = inner.org_id;
            args.all_orgs = inner.all_orgs;
            args.text = inner.text;
            args.table = inner.table;
            args.csv = inner.csv;
            args.json = inner.json;
            args.yaml = inner.yaml;
            apply_output_format(&mut args, inner.output_format);
            args.no_header = inner.no_header;
            args
        }
        Some(AlertGroupCommand::ListMuteTimings(inner)) => {
            let mut args = cli_args_from_common(inner.common);
            args.command_kind = Some(AlertCommandKind::ListMuteTimings);
            args.list_kind = Some(AlertListKind::MuteTimings);
            args.org_id = inner.org_id;
            args.all_orgs = inner.all_orgs;
            args.text = inner.text;
            args.table = inner.table;
            args.csv = inner.csv;
            args.json = inner.json;
            args.yaml = inner.yaml;
            apply_output_format(&mut args, inner.output_format);
            args.no_header = inner.no_header;
            args
        }
        Some(AlertGroupCommand::ListTemplates(inner)) => {
            let mut args = cli_args_from_common(inner.common);
            args.command_kind = Some(AlertCommandKind::ListTemplates);
            args.list_kind = Some(AlertListKind::Templates);
            args.org_id = inner.org_id;
            args.all_orgs = inner.all_orgs;
            args.text = inner.text;
            args.table = inner.table;
            args.csv = inner.csv;
            args.json = inner.json;
            args.yaml = inner.yaml;
            apply_output_format(&mut args, inner.output_format);
            args.no_header = inner.no_header;
            args
        }
        None => {
            let legacy = args.legacy;
            AlertCliArgs {
                command_kind: None,
                authoring_command_kind: None,
                profile: legacy.common.profile,
                url: legacy.common.url,
                api_token: legacy.common.api_token,
                username: legacy.common.username,
                password: legacy.common.password,
                prompt_password: legacy.common.prompt_password,
                prompt_token: legacy.common.prompt_token,
                output_dir: legacy.output_dir,
                input_dir: legacy.input_dir,
                diff_dir: legacy.diff_dir,
                timeout: legacy.common.timeout,
                flat: legacy.flat,
                overwrite: legacy.overwrite,
                replace_existing: legacy.replace_existing,
                dry_run: legacy.dry_run,
                dashboard_uid_map: legacy.dashboard_uid_map,
                panel_id_map: legacy.panel_id_map,
                verify_ssl: legacy.common.verify_ssl,
                org_id: None,
                all_orgs: false,
                list_kind: None,
                text: false,
                table: false,
                csv: false,
                json: false,
                yaml: false,
                no_header: false,
                diff_output: None,
                desired_dir: None,
                prune: false,
                plan_file: None,
                approve: false,
                allow_policy_reset: false,
                resource_kind: None,
                resource_identity: None,
                command_output: None,
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
    }
}

/// Small adapter for callers that already have a concrete group command and need
/// the full normalized AlertCliArgs form.
pub fn normalize_alert_group_command(command: AlertGroupCommand) -> AlertCliArgs {
    // Call graph (hierarchy): this function is used in related modules.
    // Upstream callers: 無
    // Downstream callees: alert_cli_defs.rs:empty_legacy_args, alert_cli_defs.rs:normalize_alert_namespace_args

    normalize_alert_namespace_args(AlertNamespaceArgs {
        command: Some(command),
        legacy: empty_legacy_args(),
    })
}

/// Purpose: implementation note.
///
/// Args: see function signature.
/// Returns: see implementation.
pub fn build_auth_context(args: &AlertCliArgs) -> Result<AlertAuthContext> {
    let connection = GrafanaConnection::resolve(
        args.profile.as_deref(),
        ConnectionMergeInput {
            url: &args.url,
            url_default: "",
            api_token: args.api_token.as_deref(),
            username: args.username.as_deref(),
            password: args.password.as_deref(),
            org_id: args.org_id,
            timeout: args.timeout,
            timeout_default: DEFAULT_TIMEOUT,
            verify_ssl: args.verify_ssl,
            insecure: false,
            ca_cert: None,
        },
        AuthInputs {
            api_token: args.api_token.as_deref(),
            username: args.username.as_deref(),
            password: args.password.as_deref(),
            prompt_password: args.prompt_password,
            prompt_token: args.prompt_token,
        },
        false,
    )?;
    Ok(AlertAuthContext {
        url: connection.base_url,
        timeout: connection.timeout_secs,
        verify_ssl: connection.verify_ssl,
        headers: connection.headers,
    })
}
