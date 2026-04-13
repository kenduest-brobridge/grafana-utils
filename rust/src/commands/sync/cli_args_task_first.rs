use clap::{Args, Subcommand};
use std::path::PathBuf;

use super::super::help_texts::*;
use super::cli_args_common::{ChangeOutputArgs, ChangeStagedInputsArgs, SyncOutputFormat};

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
    pub common: crate::dashboard::CommonCliArgs,
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
        long,
        default_value_t = false,
        help = "Stamp the preview artifact as reviewed so it can flow directly into workspace apply.",
        help_heading = "Review Options"
    )]
    pub mark_reviewed: bool,
    #[arg(
        long,
        default_value = super::super::DEFAULT_REVIEW_TOKEN,
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
    pub common: crate::dashboard::CommonCliArgs,
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
    Advanced(super::cli_args_common::SyncAdvancedCliArgs),
    #[command(
        name = "package",
        about = "Package exported dashboards, alerting resources, datasource inventory, and metadata into one local workspace bundle.",
        after_help = SYNC_PACKAGE_HELP_TEXT
    )]
    Bundle(super::cli_args_bundle::SyncBundleArgs),
}
