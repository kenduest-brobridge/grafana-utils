use clap::{Args, Subcommand};
use std::path::PathBuf;

use super::super::help_texts::*;
use super::cli_args_common::SyncOutputFormat;

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
        help = "JSON file containing the current live sync resource list.",
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
    pub common: crate::dashboard::CommonCliArgs,
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
        default_value = super::super::DEFAULT_REVIEW_TOKEN,
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
    pub common: crate::dashboard::CommonCliArgs,
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
    pub common: crate::dashboard::CommonCliArgs,
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
    pub common: crate::dashboard::CommonCliArgs,
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
    pub common: crate::dashboard::CommonCliArgs,
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
