//! Local/document-only sync CLI wrapper.
//!
//! Maintainer note:
//! - Raw JSON inputs are normalized into `SyncResourceSpec` before staged
//!   planning, review, audit, and apply documents are built.
//! - Local contract changes usually start in `sync/workbench.rs`; live request
//!   mapping starts in `sync/live_fetch.rs` or `sync/live_apply.rs`; routing
//!   stays in `sync/cli.rs`.
//! - Keep dry-run/reviewable sync artifacts available even when optional live
//!   fetch/apply wiring is enabled.

use clap::{Args, Parser, Subcommand, ValueEnum};
use serde_json::{Map, Value};
use std::fs;
use std::path::PathBuf;

mod apply_builder;
pub mod audit;
pub mod audit_tui;
pub(crate) mod blocked_reasons;
pub mod bundle_alert_contracts;
mod bundle_builder;
mod bundle_inputs;
pub mod bundle_preflight;
pub mod cli;
mod json;
pub mod live;
mod live_project_status;
mod live_project_status_promotion;
mod live_project_status_sync;
mod plan_builder;
pub mod preflight;
mod project_status;
mod project_status_promotion;
pub mod promotion_preflight;
pub mod review_tui;
mod staged_documents;
mod summary_builder;
pub mod workbench;

use self::audit::{
    build_sync_audit_document, build_sync_lock_document, build_sync_lock_document_from_lock,
    render_sync_audit_text,
};
use self::audit_tui::run_sync_audit_interactive;
use self::bundle_preflight::{
    build_sync_bundle_preflight_document, render_sync_bundle_preflight_text,
};
use self::preflight::{build_sync_preflight_document, render_sync_preflight_text};
pub(crate) use self::project_status::{build_sync_domain_status, SyncDomainStatusInputs};
pub(crate) use self::project_status_promotion::build_promotion_domain_status;
use self::promotion_preflight::{
    build_sync_promotion_preflight_document, render_sync_promotion_preflight_text,
};
use self::workbench::{
    build_sync_apply_intent_document, build_sync_plan_document, build_sync_source_bundle_document,
    build_sync_summary_document, render_sync_source_bundle_text,
};
use crate::alert_sync::assess_alert_sync_specs;
use crate::common::{message, Result};
use crate::dashboard::CommonCliArgs;
/// Constant for default review token.
pub const DEFAULT_REVIEW_TOKEN: &str = "reviewed-sync-plan";
const SYNC_ROOT_HELP_TEXT: &str = "Examples:\n\n  Summarize desired resources:\n    grafana-util sync summary --desired-file ./desired.json\n\n  Audit managed resources against a staged checksum lock:\n    grafana-util sync audit --lock-file ./sync-lock.json --fetch-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --fail-on-drift --output json\n\n  Package local exports into one source bundle:\n    grafana-util sync bundle --dashboard-export-dir ./dashboards/raw --alert-export-dir ./alerts/raw --output-file ./sync-source-bundle.json\n\n  Compare a source bundle against target inventory before apply:\n    grafana-util sync bundle-preflight --source-bundle ./sync-source-bundle.json --target-inventory ./target-inventory.json --output json\n\n  Assess staged promotion review handoff:\n    grafana-util sync promotion-preflight --source-bundle ./sync-source-bundle.json --target-inventory ./target-inventory.json --mapping-file ./promotion-map.json --output json\n\n  Build a live-backed sync plan:\n    grafana-util sync plan --desired-file ./desired.json --fetch-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\"\n\n  Apply a reviewed plan back to Grafana:\n    grafana-util sync apply --plan-file ./sync-plan-reviewed.json --approve --execute-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\"";
const SYNC_SUMMARY_HELP_TEXT: &str = "Examples:\n\n  grafana-util sync summary --desired-file ./desired.json\n  grafana-util sync summary --desired-file ./desired.json --output json";
const SYNC_PLAN_HELP_TEXT: &str = "Examples:\n\n  grafana-util sync plan --desired-file ./desired.json --live-file ./live.json\n  grafana-util sync plan --desired-file ./desired.json --fetch-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --allow-prune --output json";
const SYNC_REVIEW_HELP_TEXT: &str = "Examples:\n\n  grafana-util sync review --plan-file ./sync-plan.json\n  grafana-util sync review --plan-file ./sync-plan.json --review-note 'peer-reviewed' --output json";
const SYNC_APPLY_HELP_TEXT: &str = "Examples:\n\n  grafana-util sync apply --plan-file ./sync-plan-reviewed.json --approve\n  grafana-util sync apply --plan-file ./sync-plan-reviewed.json --approve --execute-live --allow-folder-delete --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\"\n  grafana-util sync apply --plan-file ./sync-plan-reviewed.json --approve --execute-live --allow-policy-reset --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\"";
const SYNC_AUDIT_HELP_TEXT: &str = "Examples:\n\n  grafana-util sync audit --managed-file ./desired.json --live-file ./live.json --write-lock ./sync-lock.json\n  grafana-util sync audit --lock-file ./sync-lock.json --fetch-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --fail-on-drift --output json";
const SYNC_PREFLIGHT_HELP_TEXT: &str = "Examples:\n\n  grafana-util sync preflight --desired-file ./desired.json --availability-file ./availability.json\n  grafana-util sync preflight --desired-file ./desired.json --fetch-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output json";
const SYNC_ASSESS_ALERTS_HELP_TEXT: &str = "Examples:\n\n  grafana-util sync assess-alerts --alerts-file ./alerts.json\n  grafana-util sync assess-alerts --alerts-file ./alerts.json --output json";
const SYNC_BUNDLE_PREFLIGHT_HELP_TEXT: &str = "Examples:\n\n  grafana-util sync bundle-preflight --source-bundle ./bundle.json --target-inventory ./target.json\n  grafana-util sync bundle-preflight --source-bundle ./bundle.json --target-inventory ./target.json --availability-file ./availability.json --output json\n\n  Example availability file:\n    {\n      \"providerNames\": [\"vault\"],\n      \"secretPlaceholderNames\": [\"prom-basic-auth\"]\n    }";
const SYNC_PROMOTION_PREFLIGHT_HELP_TEXT: &str = "This command is a staged review handoff for promotion; it stays read-only and does not apply live changes.\n\nExamples:\n\n  grafana-util sync promotion-preflight --source-bundle ./bundle.json --target-inventory ./target.json\n  grafana-util sync promotion-preflight --source-bundle ./bundle.json --target-inventory ./target.json --mapping-file ./promotion-mapping.json --availability-file ./availability.json --output json\n\n  Minimal promotion mapping file:\n    {\n      \"kind\": \"grafana-utils-sync-promotion-mapping\",\n      \"schemaVersion\": 1,\n      \"metadata\": {\n        \"sourceEnvironment\": \"staging\",\n        \"targetEnvironment\": \"prod\"\n      },\n      \"folders\": {\n        \"ops-src\": \"ops-prod\"\n      },\n      \"datasources\": {\n        \"uids\": {\n          \"prom-src\": \"prom-prod\"\n        },\n        \"names\": {\n          \"Prometheus Source\": \"Prometheus Prod\"\n        }\n      }\n    }\n\n  Example availability file:\n    {\n      \"providerNames\": [\"vault\"],\n      \"secretPlaceholderNames\": [\"prom-basic-auth\"]\n    }";
const SYNC_BUNDLE_HELP_TEXT: &str = "Examples:\n\n  grafana-util sync bundle --dashboard-export-dir ./dashboards/raw --alert-export-dir ./alerts/raw --output-file ./sync-source-bundle.json\n  grafana-util sync bundle --dashboard-export-dir ./dashboards/raw --datasource-export-file ./datasources/datasources.json --output json";

/// Output formats shared by staged sync document commands.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum SyncOutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Parser)]
#[command(
    name = "grafana-util sync",
    about = "Reviewable sync workflows with optional live Grafana fetch/apply paths.",
    after_help = SYNC_ROOT_HELP_TEXT,
    styles = crate::help_styles::CLI_HELP_STYLES
)]
/// Root `grafana-util sync` parser wrapper.
pub struct SyncCliArgs {
    #[command(subcommand)]
    pub command: SyncGroupCommand,
}

#[cfg(test)]
pub(crate) use audit_tui::{build_sync_audit_tui_groups, build_sync_audit_tui_rows};

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
        long,
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help = "Render the summary document as text or json.",
        help_heading = "Output Options"
    )]
    pub output: SyncOutputFormat,
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
        long,
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help = "Render the plan document as text or json.",
        help_heading = "Output Options"
    )]
    pub output: SyncOutputFormat,
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
        long,
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help = "Render the reviewed plan document as text or json.",
        help_heading = "Output Options"
    )]
    pub output: SyncOutputFormat,
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
        long,
        help = "JSON file containing the reviewed sync plan document.",
        help_heading = "Input Options"
    )]
    pub plan_file: PathBuf,
    #[arg(
        long,
        help = "Optional JSON file containing a staged sync preflight document."
    )]
    pub preflight_file: Option<PathBuf>,
    #[arg(
        long,
        help = "Optional JSON file containing a staged sync bundle-preflight document."
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
        long,
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help = "Render the apply intent document as text or json.",
        help_heading = "Output Options"
    )]
    pub output: SyncOutputFormat,
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
        long,
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help = "Render the audit document as text or json.",
        help_heading = "Output Options"
    )]
    pub output: SyncOutputFormat,
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
        long,
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help = "Render the preflight document as text or json."
    )]
    pub output: SyncOutputFormat,
}

/// Struct definition for SyncAssessAlertsArgs.
#[derive(Debug, Clone, Args)]
pub struct SyncAssessAlertsArgs {
    #[arg(
        long,
        help = "JSON file containing the alert sync resource list.",
        help_heading = "Input Options"
    )]
    pub alerts_file: PathBuf,
    #[arg(
        long,
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help = "Render the alert assessment document as text or json."
    )]
    pub output: SyncOutputFormat,
}

/// Struct definition for SyncBundlePreflightArgs.
#[derive(Debug, Clone, Args)]
pub struct SyncBundlePreflightArgs {
    #[arg(
        long,
        help = "JSON file containing the staged multi-resource source bundle.",
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
        long,
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help = "Render the bundle preflight document as text or json."
    )]
    pub output: SyncOutputFormat,
}

/// Struct definition for SyncPromotionPreflightArgs.
#[derive(Debug, Clone, Args)]
pub struct SyncPromotionPreflightArgs {
    #[arg(
        long,
        help = "JSON file containing the staged multi-resource source bundle.",
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
        long,
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help = "Render the promotion preflight document as text or json."
    )]
    pub output: SyncOutputFormat,
}

/// Struct definition for SyncBundleArgs.
#[derive(Debug, Clone, Args)]
pub struct SyncBundleArgs {
    #[arg(
        long,
        help = "Path to one existing dashboard raw export directory such as ./dashboards/raw."
    )]
    pub dashboard_export_dir: Option<PathBuf>,
    #[arg(
        long,
        help = "Path to one existing alert raw export directory such as ./alerts/raw."
    )]
    pub alert_export_dir: Option<PathBuf>,
    #[arg(
        long,
        help = "Optional standalone datasource inventory JSON file to include or prefer over dashboards/raw/datasources.json."
    )]
    pub datasource_export_file: Option<PathBuf>,
    #[arg(
        long,
        help = "Optional JSON object file containing extra bundle metadata."
    )]
    pub metadata_file: Option<PathBuf>,
    #[arg(
        long,
        help = "Optional JSON file path to write the source bundle artifact."
    )]
    pub output_file: Option<PathBuf>,
    #[arg(
        long,
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help = "Render the source bundle document as text or json."
    )]
    pub output: SyncOutputFormat,
}

/// Top-level sync subcommands exposed under `grafana-util sync`.
#[derive(Debug, Clone, Subcommand)]
pub enum SyncGroupCommand {
    #[command(about = "Build a staged sync plan from local desired and live JSON files.", after_help = SYNC_PLAN_HELP_TEXT)]
    Plan(SyncPlanArgs),
    #[command(about = "Mark a staged sync plan JSON document reviewed.", after_help = SYNC_REVIEW_HELP_TEXT)]
    Review(SyncReviewArgs),
    #[command(about = "Build a gated local apply intent from a reviewed sync plan.", after_help = SYNC_APPLY_HELP_TEXT)]
    Apply(SyncApplyArgs),
    #[command(about = "Audit managed Grafana resources against a checksum lock and current live state.", after_help = SYNC_AUDIT_HELP_TEXT)]
    Audit(SyncAuditArgs),
    #[command(about = "Summarize local desired sync resources from JSON.", after_help = SYNC_SUMMARY_HELP_TEXT)]
    Summary(SyncSummaryArgs),
    #[command(about = "Build a staged sync preflight document from local JSON.", after_help = SYNC_PREFLIGHT_HELP_TEXT)]
    Preflight(SyncPreflightArgs),
    #[command(about = "Assess alert sync specs for candidate, plan-only, and blocked states.", after_help = SYNC_ASSESS_ALERTS_HELP_TEXT)]
    AssessAlerts(SyncAssessAlertsArgs),
    #[command(about = "Build a staged bundle-level sync preflight document from local JSON.", after_help = SYNC_BUNDLE_PREFLIGHT_HELP_TEXT)]
    BundlePreflight(SyncBundlePreflightArgs),
    #[command(about = "Build a staged promotion review handoff from a source bundle and target inventory.", after_help = SYNC_PROMOTION_PREFLIGHT_HELP_TEXT)]
    PromotionPreflight(SyncPromotionPreflightArgs),
    #[command(
        about = "Package exported dashboards, alerting resources, datasource inventory, and metadata into one local source bundle.",
        after_help = SYNC_BUNDLE_HELP_TEXT
    )]
    Bundle(SyncBundleArgs),
}

pub(crate) use bundle_inputs::{
    build_alert_sync_specs, load_alerting_bundle_section, load_dashboard_bundle_sections,
    normalize_alert_managed_fields, normalize_alert_resource_identity_and_title,
    normalize_datasource_bundle_item,
};
pub(crate) use json::{
    append_unique_strings, load_json_array_file, load_json_value, load_optional_json_object_file,
    require_json_object,
};
pub(crate) use live::{
    execute_live_apply, fetch_live_availability, fetch_live_resource_specs, merge_availability,
};
#[allow(unused_imports)]
pub(crate) use live_project_status::{
    build_live_promotion_domain_status as build_live_promotion_domain_status_transport,
    build_live_sync_domain_status as build_live_sync_domain_status_transport,
};
pub(crate) use live_project_status_promotion::{
    build_live_promotion_project_status, LivePromotionProjectStatusInputs,
};
pub(crate) use live_project_status_sync::{
    build_live_sync_domain_status, SyncLiveProjectStatusInputs,
};
// Lineage helpers stay separate from staged document mutation helpers so the
// review/apply flow is easy to trace at the call site.
pub(crate) use staged_documents::{
    attach_apply_audit, attach_bundle_preflight_summary, attach_lineage, attach_preflight_summary,
    attach_review_audit, attach_trace_id, mark_plan_reviewed, require_matching_optional_trace_id,
    require_optional_stage, require_trace_id, validate_apply_bundle_preflight,
    validate_apply_preflight,
};
// Renderers remain public because the CLI and tests consume their stable text
// output directly.
pub use staged_documents::{
    render_alert_sync_assessment_text, render_sync_apply_intent_text, render_sync_plan_text,
    render_sync_summary_text,
};
#[cfg(feature = "tui")]
pub(crate) use staged_documents::{
    sync_audit_drift_cmp, sync_audit_drift_details, sync_audit_drift_meta, sync_audit_drift_title,
};

/// Entrypoint for sync command execution after Clap parsing.
///
/// The heavy runtime logic lives in `sync/cli.rs`; this module keeps the parser
/// surface and re-exports discoverable from one place.
pub fn run_sync_cli(command: SyncGroupCommand) -> Result<()> {
    cli::run_sync_cli(command)
}

#[cfg(test)]
#[path = "cli_render_rust_tests.rs"]
mod cli_render_rust_tests;

#[cfg(test)]
#[path = "cli_rust_tests.rs"]
mod sync_cli_rust_tests;

#[cfg(test)]
#[path = "cli_apply_review_rust_tests.rs"]
mod cli_apply_review_rust_tests;

#[cfg(test)]
#[path = "cli_apply_review_exec_rust_tests.rs"]
mod cli_apply_review_exec_rust_tests;

#[cfg(test)]
#[path = "cli_review_tui_rust_tests.rs"]
mod cli_review_tui_rust_tests;

#[cfg(test)]
#[path = "cli_audit_preflight_rust_tests.rs"]
mod cli_audit_preflight_rust_tests;

#[cfg(test)]
#[path = "cli_help_rust_tests.rs"]
mod cli_help_rust_tests;

#[cfg(test)]
#[path = "live_rust_tests.rs"]
mod sync_live_rust_tests;

#[cfg(test)]
#[path = "rust_tests.rs"]
mod sync_rust_tests;
