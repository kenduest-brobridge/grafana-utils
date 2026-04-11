use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use super::help_texts::*;
use super::DEFAULT_REVIEW_TOKEN;
use crate::dashboard::CommonCliArgs;

/// Output formats shared by staged sync document commands.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum SyncOutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Args)]
pub struct ChangeStagedInputsArgs {
    #[arg(
        index = 1,
        default_value = ".",
        help = "Workspace root used for auto-discovery when explicit staged inputs are omitted.",
        help_heading = "Input Options"
    )]
    pub workspace: PathBuf,
    #[arg(
        long,
        help = "Explicit JSON file containing the desired sync resource list.",
        help_heading = "Input Options"
    )]
    pub desired_file: Option<PathBuf>,
    #[arg(
        long,
        help = "Existing staged source bundle JSON file to use instead of per-surface export discovery.",
        help_heading = "Input Options"
    )]
    pub source_bundle: Option<PathBuf>,
    #[arg(
        long,
        conflicts_with = "dashboard_provisioning_dir",
        help = "Path to one existing dashboard raw export directory such as ./dashboards/raw.",
        help_heading = "Input Options"
    )]
    pub dashboard_export_dir: Option<PathBuf>,
    #[arg(
        long,
        conflicts_with = "dashboard_export_dir",
        help = "Path to one existing dashboard provisioning root or dashboards/ directory such as ./dashboards/provisioning.",
        help_heading = "Input Options"
    )]
    pub dashboard_provisioning_dir: Option<PathBuf>,
    #[arg(
        long,
        help = "Path to one existing alert raw export directory such as ./alerts/raw.",
        help_heading = "Input Options"
    )]
    pub alert_export_dir: Option<PathBuf>,
    #[arg(
        long,
        conflicts_with = "datasource_provisioning_file",
        help = "Standalone datasource inventory JSON file to include or prefer over dashboards/raw/datasources.json.",
        help_heading = "Input Options"
    )]
    pub datasource_export_file: Option<PathBuf>,
    #[arg(
        long,
        conflicts_with = "datasource_export_file",
        help = "Datasource provisioning YAML file to include instead of dashboards/raw/datasources.json.",
        help_heading = "Input Options"
    )]
    pub datasource_provisioning_file: Option<PathBuf>,
    #[arg(
        long,
        help = "Access user export directory to include from staged artifacts.",
        help_heading = "Input Options"
    )]
    pub access_user_export_dir: Option<PathBuf>,
    #[arg(
        long,
        help = "Access team export directory to include from staged artifacts.",
        help_heading = "Input Options"
    )]
    pub access_team_export_dir: Option<PathBuf>,
    #[arg(
        long,
        help = "Access org export directory to include from staged artifacts.",
        help_heading = "Input Options"
    )]
    pub access_org_export_dir: Option<PathBuf>,
    #[arg(
        long,
        help = "Access service-account export directory to include from staged artifacts.",
        help_heading = "Input Options"
    )]
    pub access_service_account_export_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Args)]
pub struct ChangeOutputArgs {
    #[arg(
        long = "output-format",
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help = "Render the document as text or json.",
        help_heading = "Output Options"
    )]
    pub output_format: SyncOutputFormat,
    #[arg(
        long,
        help = "Optional file path to write the rendered artifact.",
        help_heading = "Output Options"
    )]
    pub output_file: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        requires = "output_file",
        help = "When --output-file is set, also print the rendered artifact to stdout.",
        help_heading = "Output Options"
    )]
    pub also_stdout: bool,
}

/// CI-oriented workspace workflow namespace under `grafana-util workspace ci`.
#[derive(Debug, Clone, Args)]
#[command(
    name = "grafana-util workspace ci",
    about = "CI-oriented workspace workflows and lower-level review contracts.",
    after_help = SYNC_CI_HELP_TEXT,
    styles = crate::help_styles::CLI_HELP_STYLES
)]
pub struct SyncAdvancedCliArgs {
    #[command(subcommand)]
    pub command: SyncAdvancedCommand,
}

#[derive(Debug, Clone, Parser)]
#[command(
    name = "grafana-util workspace",
    about = "Task-first workspace workflow for scan, test, preview, package, apply, and CI paths.",
    after_help = SYNC_ROOT_HELP_TEXT,
    styles = crate::help_styles::CLI_HELP_STYLES
)]
/// Root `grafana-util workspace` parser wrapper.
pub struct SyncCliArgs {
    #[command(subcommand)]
    pub command: SyncGroupCommand,
}

#[derive(Debug, Clone, Args)]
pub struct ChangeInspectArgs {
    #[command(flatten)]
    pub inputs: ChangeStagedInputsArgs,
    #[command(flatten)]
    pub output: ChangeOutputArgs,
}

#[derive(Debug, Clone, Args)]
pub struct ChangeCheckArgs {
    #[command(flatten)]
    pub inputs: ChangeStagedInputsArgs,
    #[arg(
        long,
        help = "Optional JSON object file containing staged availability hints.",
        help_heading = "Input Options"
    )]
    pub availability_file: Option<PathBuf>,
    #[arg(
        long,
        help = "Optional JSON file containing the target inventory snapshot for bundle or promotion checks.",
        help_heading = "Input Options"
    )]
    pub target_inventory: Option<PathBuf>,
    #[arg(
        long,
        help = "Optional JSON object file containing explicit promotion mappings.",
        help_heading = "Input Options"
    )]
    pub mapping_file: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        help = "Fetch availability hints from Grafana instead of relying only on --availability-file.",
        help_heading = "Live Options"
    )]
    pub fetch_live: bool,
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help = "Optional Grafana org id used when --fetch-live is active.",
        help_heading = "Live Options"
    )]
    pub org_id: Option<i64>,
    #[command(flatten)]
    pub output: ChangeOutputArgs,
}

#[derive(Debug, Clone, Args)]
pub struct ChangePreviewArgs {
    #[command(flatten)]
    pub inputs: ChangeStagedInputsArgs,
    #[arg(
        long,
        help = "Optional staged target inventory JSON used by bundle or promotion preview.",
        help_heading = "Input Options"
    )]
    pub target_inventory: Option<PathBuf>,
    #[arg(
        long,
        help = "Optional staged promotion mapping JSON for promotion preview.",
        help_heading = "Input Options"
    )]
    pub mapping_file: Option<PathBuf>,
    #[arg(
        long,
        help = "Optional staged availability JSON reused by preview builders.",
        help_heading = "Input Options"
    )]
    pub availability_file: Option<PathBuf>,
    #[arg(
        long,
        help = "JSON file containing the live sync resource list.",
        help_heading = "Input Options"
    )]
    pub live_file: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        help = "Read the current live state directly from Grafana instead of --live-file.",
        help_heading = "Live Options"
    )]
    pub fetch_live: bool,
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help = "Optional Grafana org id used when --fetch-live is active.",
        help_heading = "Live Options"
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = 500usize,
        help = "Dashboard search page size when --fetch-live is active.",
        help_heading = "Live Options"
    )]
    pub page_size: usize,
    #[arg(
        long,
        default_value_t = false,
        help = "Mark live-only resources as would-delete instead of unmanaged.",
        help_heading = "Planning Options"
    )]
    pub allow_prune: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Stamp the preview artifact as reviewed so it can flow directly into workspace apply.",
        help_heading = "Review Options"
    )]
    pub mark_reviewed: bool,
    #[arg(
        long,
        default_value = DEFAULT_REVIEW_TOKEN,
        help = "Review token recorded when --mark-reviewed is used.",
        help_heading = "Review Options"
    )]
    pub review_token: String,
    #[arg(
        long,
        help = "Optional reviewer identity to record when --mark-reviewed is used.",
        help_heading = "Review Options"
    )]
    pub reviewed_by: Option<String>,
    #[arg(
        long,
        help = "Optional staged reviewed-at value to record when --mark-reviewed is used.",
        help_heading = "Review Options"
    )]
    pub reviewed_at: Option<String>,
    #[arg(
        long,
        help = "Optional review note to record when --mark-reviewed is used.",
        help_heading = "Review Options"
    )]
    pub review_note: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        requires = "mark_reviewed",
        help = "Open an interactive terminal review before stamping the preview reviewed.",
        help_heading = "Review Options"
    )]
    pub interactive_review: bool,
    #[arg(
        long,
        help = "Optional stable trace id to carry through preview and apply files.",
        help_heading = "Planning Options"
    )]
    pub trace_id: Option<String>,
    #[command(flatten)]
    pub output: ChangeOutputArgs,
}

/// Arguments for summarizing local desired sync resources.
#[derive(Debug, Clone, Args)]
pub struct SyncSummaryArgs {
    #[arg(
        long,
        help = "JSON file containing the desired sync resource list.",
        help_heading = "Input Options"
    )]
    pub desired_file: PathBuf,
    #[arg(
        long = "output-format",
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help = "Render the summary document as text or json.",
        help_heading = "Output Options"
    )]
    pub output_format: SyncOutputFormat,
}

/// Arguments for building a staged sync plan from desired and live state.
#[derive(Debug, Clone, Args)]
pub struct SyncPlanArgs {
    #[arg(
        long,
        help = "JSON file containing the desired sync resource list.",
        help_heading = "Input Options"
    )]
    pub desired_file: PathBuf,
    #[arg(
        long,
        help = "JSON file containing the live sync resource list.",
        help_heading = "Input Options"
    )]
    pub live_file: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        help = "Read the current live state directly from Grafana instead of --live-file.",
        help_heading = "Live Options"
    )]
    pub fetch_live: bool,
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help = "Optional Grafana org id used when --fetch-live is active.",
        help_heading = "Live Options"
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = 500usize,
        help = "Dashboard search page size when --fetch-live is active.",
        help_heading = "Live Options"
    )]
    pub page_size: usize,
    #[arg(
        long,
        default_value_t = false,
        help = "Mark live-only resources as would-delete instead of unmanaged.",
        help_heading = "Planning Options"
    )]
    pub allow_prune: bool,
    #[arg(
        long = "output-format",
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help = "Render the plan document as text or json.",
        help_heading = "Output Options"
    )]
    pub output_format: SyncOutputFormat,
    #[arg(
        long,
        help = "Optional stable trace id to carry through staged plan/review/apply files."
    )]
    pub trace_id: Option<String>,
}

/// Arguments for marking a staged sync plan as reviewed.
#[derive(Debug, Clone, Args)]
pub struct SyncReviewArgs {
    #[arg(
        long,
        help = "JSON file containing the staged sync plan document.",
        help_heading = "Input Options"
    )]
    pub plan_file: PathBuf,
    #[arg(
        long,
        default_value = DEFAULT_REVIEW_TOKEN,
        help = "Explicit review token required to mark the plan reviewed.",
        help_heading = "Review Options"
    )]
    pub review_token: String,
    #[arg(
        long = "output-format",
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help = "Render the reviewed plan document as text or json.",
        help_heading = "Output Options"
    )]
    pub output_format: SyncOutputFormat,
    #[arg(
        long,
        help = "Optional reviewer identity to record in the reviewed plan."
    )]
    pub reviewed_by: Option<String>,
    #[arg(
        long,
        help = "Optional staged reviewed-at value to record in the reviewed plan."
    )]
    pub reviewed_at: Option<String>,
    #[arg(long, help = "Optional review note to record in the reviewed plan.")]
    pub review_note: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Open an interactive terminal review to select which actionable sync operations stay enabled before the plan is marked reviewed."
    )]
    pub interactive: bool,
}

/// Arguments for producing or executing an apply step from a reviewed plan.
#[derive(Debug, Clone, Args)]
pub struct SyncApplyArgs {
    #[arg(
        long = "preview-file",
        alias = "plan-file",
        help = "Optional JSON file containing the staged preview/plan document. When omitted, workspace apply looks for a common preview path such as ./workspace-preview.json or ./sync-plan-reviewed.json.",
        help_heading = "Input Options"
    )]
    pub plan_file: Option<PathBuf>,
    #[arg(
        long = "input-test-file",
        alias = "preflight-file",
        help = "Optional JSON file containing a staged workspace input-test document."
    )]
    pub preflight_file: Option<PathBuf>,
    #[arg(
        long = "package-test-file",
        alias = "bundle-preflight-file",
        help = "Optional JSON file containing a staged workspace package-test document."
    )]
    pub bundle_preflight_file: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        help = "Explicit acknowledgement required before a local apply intent is emitted.",
        help_heading = "Approval Options"
    )]
    pub approve: bool,
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help = "Optional Grafana org id used when --execute-live is active.",
        help_heading = "Live Options"
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = false,
        help = "Apply supported sync operations to Grafana after review and approval checks pass.",
        help_heading = "Live Options"
    )]
    pub execute_live: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Allow live deletion of folders when a reviewed plan includes would-delete folder operations.",
        help_heading = "Approval Options"
    )]
    pub allow_folder_delete: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Allow live reset of the notification policy tree when a reviewed plan includes would-delete alert-policy operations.",
        help_heading = "Approval Options"
    )]
    pub allow_policy_reset: bool,
    #[arg(
        long = "output-format",
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help = "Render the apply intent document as text or json.",
        help_heading = "Output Options"
    )]
    pub output_format: SyncOutputFormat,
    #[arg(
        long,
        help = "Optional apply actor identity to record in the apply intent."
    )]
    pub applied_by: Option<String>,
    #[arg(
        long,
        help = "Optional staged applied-at value to record in the apply intent."
    )]
    pub applied_at: Option<String>,
    #[arg(long, help = "Optional approval reason to record in the apply intent.")]
    pub approval_reason: Option<String>,
    #[arg(long, help = "Optional apply note to record in the apply intent.")]
    pub apply_note: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub struct SyncAuditArgs {
    #[arg(
        long,
        help = "Optional JSON file containing the managed desired sync resource list used to define audit scope and managed fields.",
        help_heading = "Input Options"
    )]
    pub managed_file: Option<PathBuf>,
    #[arg(
        long,
        help = "Optional JSON file containing a staged sync lock document to compare against.",
        help_heading = "Input Options"
    )]
    pub lock_file: Option<PathBuf>,
    #[arg(
        long,
        help = "Optional JSON file containing the current live sync resource list.",
        help_heading = "Input Options"
    )]
    pub live_file: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        help = "Fetch the current live state directly from Grafana instead of --live-file.",
        help_heading = "Live Options"
    )]
    pub fetch_live: bool,
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help = "Optional Grafana org id used when --fetch-live is active.",
        help_heading = "Live Options"
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = 500usize,
        help = "Dashboard search page size when --fetch-live is active.",
        help_heading = "Live Options"
    )]
    pub page_size: usize,
    #[arg(
        long,
        help = "Optional JSON file path to write the newly generated lock snapshot.",
        help_heading = "Output Options"
    )]
    pub write_lock: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        help = "Fail the command when the audit detects drift.",
        help_heading = "Output Options"
    )]
    pub fail_on_drift: bool,
    #[arg(
        long = "output-format",
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help = "Render the audit document as text or json.",
        help_heading = "Output Options"
    )]
    pub output_format: SyncOutputFormat,
    #[arg(
        long,
        default_value_t = false,
        help = "Open an interactive terminal browser over drift rows.",
        help_heading = "Output Options"
    )]
    pub interactive: bool,
}

/// Struct definition for SyncPreflightArgs.
#[derive(Debug, Clone, Args)]
pub struct SyncPreflightArgs {
    #[arg(
        long,
        help = "JSON file containing the desired sync resource list.",
        help_heading = "Input Options"
    )]
    pub desired_file: PathBuf,
    #[arg(
        long,
        help = "Optional JSON object file containing staged availability hints."
    )]
    pub availability_file: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        help = "Fetch availability hints from Grafana instead of relying only on --availability-file.",
        help_heading = "Live Options"
    )]
    pub fetch_live: bool,
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help = "Optional Grafana org id used when --fetch-live is active.",
        help_heading = "Live Options"
    )]
    pub org_id: Option<i64>,
    #[arg(
        long = "output-format",
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help = "Render the input-test document as text or json."
    )]
    pub output_format: SyncOutputFormat,
}

/// Struct definition for SyncAssessAlertsArgs.
#[derive(Debug, Clone, Args)]
pub struct SyncAssessAlertsArgs {
    #[arg(
        long,
        help = "JSON file containing the alert workspace resource list.",
        help_heading = "Input Options"
    )]
    pub alerts_file: PathBuf,
    #[arg(
        long = "output-format",
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help = "Render the alert assessment document as text or json."
    )]
    pub output_format: SyncOutputFormat,
}

/// Struct definition for SyncBundlePreflightArgs.
#[derive(Debug, Clone, Args)]
pub struct SyncBundlePreflightArgs {
    #[arg(
        long,
        help = "JSON file containing the staged workspace package document.",
        help_heading = "Input Options"
    )]
    pub source_bundle: PathBuf,
    #[arg(
        long,
        help = "JSON file containing the staged target inventory snapshot.",
        help_heading = "Input Options"
    )]
    pub target_inventory: PathBuf,
    #[arg(
        long,
        help = "Optional JSON object file containing staged availability hints."
    )]
    pub availability_file: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        help = "Fetch availability hints from Grafana instead of relying only on --availability-file.",
        help_heading = "Live Options"
    )]
    pub fetch_live: bool,
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help = "Optional Grafana org id used when --fetch-live is active."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long = "output-format",
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help = "Render the package-test document as text or json."
    )]
    pub output_format: SyncOutputFormat,
}

/// Struct definition for SyncPromotionPreflightArgs.
#[derive(Debug, Clone, Args)]
pub struct SyncPromotionPreflightArgs {
    #[arg(
        long,
        help = "JSON file containing the staged workspace package document.",
        help_heading = "Input Options"
    )]
    pub source_bundle: PathBuf,
    #[arg(
        long,
        help = "JSON file containing the staged target inventory snapshot.",
        help_heading = "Input Options"
    )]
    pub target_inventory: PathBuf,
    #[arg(
        long,
        help = "Optional JSON object file containing explicit cross-environment promotion mappings."
    )]
    pub mapping_file: Option<PathBuf>,
    #[arg(
        long,
        help = "Optional JSON object file containing staged availability hints."
    )]
    pub availability_file: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        help = "Fetch availability hints from Grafana instead of relying only on --availability-file.",
        help_heading = "Live Options"
    )]
    pub fetch_live: bool,
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help = "Optional Grafana org id used when --fetch-live is active."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long = "output-format",
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help = "Render the promote-test document as text or json."
    )]
    pub output_format: SyncOutputFormat,
}

/// Struct definition for SyncBundleArgs.
#[derive(Debug, Clone, Args)]
pub struct SyncBundleArgs {
    #[arg(
        index = 1,
        help = "Optional workspace root used for auto-discovery when per-surface bundle inputs are omitted."
    )]
    pub workspace: Option<PathBuf>,
    #[arg(
        long,
        help = "Path to one existing dashboard raw export directory such as ./dashboards/raw."
    )]
    pub dashboard_export_dir: Option<PathBuf>,
    #[arg(
        long,
        conflicts_with = "dashboard_export_dir",
        help = "Path to one existing dashboard provisioning root or dashboards/ directory such as ./dashboards/provisioning."
    )]
    pub dashboard_provisioning_dir: Option<PathBuf>,
    #[arg(
        long,
        help = "Path to one existing alert raw export directory such as ./alerts/raw."
    )]
    pub alert_export_dir: Option<PathBuf>,
    #[arg(
        long,
        conflicts_with = "datasource_provisioning_file",
        help = "Optional standalone datasource inventory JSON file to include or prefer over dashboards/raw/datasources.json."
    )]
    pub datasource_export_file: Option<PathBuf>,
    #[arg(
        long,
        conflicts_with = "datasource_export_file",
        help = "Optional datasource provisioning YAML file to include instead of dashboards/raw/datasources.json."
    )]
    pub datasource_provisioning_file: Option<PathBuf>,
    #[arg(
        long,
        help = "Optional JSON object file containing extra bundle metadata."
    )]
    pub metadata_file: Option<PathBuf>,
    #[arg(
        long,
        help = "Optional JSON file path to write the workspace package artifact."
    )]
    pub output_file: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        requires = "output_file",
        help = "When --output-file is set, also print the workspace package document to stdout."
    )]
    pub also_stdout: bool,
    #[arg(
        long = "output-format",
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help = "Render the workspace package document as text or json."
    )]
    pub output_format: SyncOutputFormat,
}

/// CI-oriented workspace subcommands under `grafana-util workspace ci`.
#[derive(Debug, Clone, Subcommand)]
pub enum SyncAdvancedCommand {
    #[command(
        name = "summary",
        about = "Summarize local desired workspace resources from JSON.",
        after_help = SYNC_SUMMARY_HELP_TEXT
    )]
    Summary(SyncSummaryArgs),
    #[command(
        name = "plan",
        about = "Build a staged workspace plan from local desired and live JSON files.",
        after_help = SYNC_PLAN_HELP_TEXT
    )]
    Plan(SyncPlanArgs),
    #[command(
        name = "mark-reviewed",
        about = "Mark a staged workspace plan JSON document reviewed.",
        after_help = SYNC_REVIEW_HELP_TEXT
    )]
    Review(SyncReviewArgs),
    #[command(
        name = "input-test",
        about = "Build a staged workspace input-test document from local JSON.",
        after_help = SYNC_PREFLIGHT_HELP_TEXT
    )]
    Preflight(SyncPreflightArgs),
    #[command(
        name = "audit",
        about = "Audit managed Grafana resources against a checksum lock and current live state.",
        after_help = SYNC_AUDIT_HELP_TEXT
    )]
    Audit(SyncAuditArgs),
    #[command(
        name = "alert-readiness",
        about = "Assess alert sync specs for candidate, plan-only, and blocked states.",
        after_help = SYNC_ASSESS_ALERTS_HELP_TEXT
    )]
    AssessAlerts(SyncAssessAlertsArgs),
    #[command(
        name = "package-test",
        about = "Build a staged workspace package-test document from local JSON.",
        after_help = SYNC_BUNDLE_PREFLIGHT_HELP_TEXT
    )]
    BundlePreflight(SyncBundlePreflightArgs),
    #[command(
        name = "promote-test",
        about = "Build a staged promotion review handoff from a workspace package and target inventory.",
        after_help = SYNC_PROMOTION_PREFLIGHT_HELP_TEXT
    )]
    PromotionPreflight(SyncPromotionPreflightArgs),
}

/// Top-level sync subcommands exposed under `grafana-util workspace`.
#[derive(Debug, Clone, Subcommand)]
pub enum SyncGroupCommand {
    #[command(
        name = "scan",
        about = "Scan the staged workspace package from discovered or explicit inputs.",
        after_help = SYNC_SCAN_HELP_TEXT
    )]
    Inspect(ChangeInspectArgs),
    #[command(
        name = "test",
        about = "Test whether the staged workspace package looks structurally safe to continue.",
        after_help = SYNC_TEST_HELP_TEXT
    )]
    Check(ChangeCheckArgs),
    #[command(
        name = "preview",
        about = "Preview what would change in the workspace from discovered or explicit staged inputs.",
        after_help = SYNC_PREVIEW_HELP_TEXT
    )]
    Preview(ChangePreviewArgs),
    #[command(
        name = "apply",
        about = "Apply a reviewed staged workspace with explicit approval.",
        after_help = SYNC_APPLY_HELP_TEXT
    )]
    Apply(SyncApplyArgs),
    #[command(
        name = "ci",
        about = "Open CI-oriented workspace workflows and lower-level review contracts."
    )]
    Advanced(SyncAdvancedCliArgs),
    #[command(
        name = "package",
        about = "Package exported dashboards, alerting resources, datasource inventory, and metadata into one local workspace bundle.",
        after_help = SYNC_PACKAGE_HELP_TEXT
    )]
    Bundle(SyncBundleArgs),
}
