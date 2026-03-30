//! Artifact-driven project overview assembly.
//!
//! This module stays pure and local: it loads staged artifacts, reuses existing
//! dashboard, access, and sync summary builders, and renders a single overview
//! document for text, JSON, or interactive output.

use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;

pub use crate::project_status::{
    ProjectDomainStatus as OverviewProjectStatusDomain, ProjectStatus as OverviewProjectStatus,
    ProjectStatusAction as OverviewProjectStatusAction,
    ProjectStatusFinding as OverviewProjectBlocker,
    ProjectStatusFreshness as OverviewProjectStatusFreshness,
    ProjectStatusOverall as OverviewProjectStatusOverall,
    ProjectStatusRankedFinding as OverviewProjectStatusRankedFinding,
};

use crate::access::{
    ACCESS_EXPORT_KIND_ORGS, ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS, ACCESS_EXPORT_KIND_TEAMS,
    ACCESS_EXPORT_KIND_USERS,
};
use crate::common::Result;
use crate::project_status_command::run_project_status_live;

pub use crate::project_status_command::{
    ProjectStatusLiveArgs as OverviewLiveArgs,
    ProjectStatusOutputFormat as OverviewLiveOutputFormat,
};

#[cfg(feature = "tui")]
#[path = "overview_tui.rs"]
mod overview_tui;

#[path = "overview_artifacts.rs"]
mod overview_artifacts;

#[path = "overview_kind.rs"]
mod overview_kind;

#[path = "overview_summary_projection.rs"]
mod overview_summary_projection;

#[path = "overview_section_rows.rs"]
mod overview_section_rows;

#[path = "overview_sections.rs"]
mod overview_sections;

#[path = "overview_support.rs"]
mod overview_support;

#[path = "overview_document.rs"]
mod overview_document;

pub const OVERVIEW_KIND: &str = "grafana-utils-overview";
pub const OVERVIEW_SCHEMA_VERSION: i64 = 1;
pub const OVERVIEW_ARTIFACT_DASHBOARD_EXPORT_KIND: &str = "dashboard-export";
pub const OVERVIEW_ARTIFACT_DATASOURCE_EXPORT_KIND: &str = "datasource-export";
pub const OVERVIEW_ARTIFACT_ALERT_EXPORT_KIND: &str = "alert-export";
pub const OVERVIEW_ARTIFACT_ACCESS_USER_EXPORT_KIND: &str = ACCESS_EXPORT_KIND_USERS;
pub const OVERVIEW_ARTIFACT_ACCESS_TEAM_EXPORT_KIND: &str = ACCESS_EXPORT_KIND_TEAMS;
pub const OVERVIEW_ARTIFACT_ACCESS_ORG_EXPORT_KIND: &str = ACCESS_EXPORT_KIND_ORGS;
pub const OVERVIEW_ARTIFACT_ACCESS_SERVICE_ACCOUNT_EXPORT_KIND: &str =
    ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS;
pub const OVERVIEW_ARTIFACT_SYNC_SUMMARY_KIND: &str = "sync-summary";
pub const OVERVIEW_ARTIFACT_BUNDLE_PREFLIGHT_KIND: &str = "bundle-preflight";
pub const OVERVIEW_ARTIFACT_PROMOTION_PREFLIGHT_KIND: &str = "promotion-preflight";
const DATASOURCE_EXPORT_FILENAME: &str = "datasources.json";
const DATASOURCE_EXPORT_METADATA_FILENAME: &str = "export-metadata.json";
const DATASOURCE_ROOT_KIND: &str = "grafana-utils-datasource-export-index";
const DATASOURCE_ROOT_SCHEMA_VERSION: i64 = 1;

/// Output formats for the overview renderer.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum OverviewOutputFormat {
    Text,
    Json,
    #[cfg(feature = "tui")]
    Interactive,
}

/// Command arguments for the artifact-driven overview runner.
#[derive(Debug, Clone, Args)]
#[command(next_help_heading = "Staged Input Options")]
pub struct OverviewArgs {
    #[arg(
        long,
        help = "Dashboard export directory to summarize with the existing inspect contract.",
        help_heading = "Input Options"
    )]
    pub dashboard_export_dir: Option<PathBuf>,
    #[arg(
        long,
        help = "Datasource export directory to summarize with the stable inventory contract.",
        help_heading = "Input Options"
    )]
    pub datasource_export_dir: Option<PathBuf>,
    #[arg(
        long,
        help = "Access user export directory to summarize with the stable export bundle contract.",
        help_heading = "Input Options"
    )]
    pub access_user_export_dir: Option<PathBuf>,
    #[arg(
        long,
        help = "Access team export directory to summarize with the stable export bundle contract.",
        help_heading = "Input Options"
    )]
    pub access_team_export_dir: Option<PathBuf>,
    #[arg(
        long,
        help = "Access org export directory to summarize with the stable export bundle contract.",
        help_heading = "Input Options"
    )]
    pub access_org_export_dir: Option<PathBuf>,
    #[arg(
        long,
        help = "Access service-account export directory to summarize with the stable export bundle contract.",
        help_heading = "Input Options"
    )]
    pub access_service_account_export_dir: Option<PathBuf>,
    #[arg(
        long,
        help = "Desired sync file to summarize with the existing sync summary builder.",
        help_heading = "Input Options"
    )]
    pub desired_file: Option<PathBuf>,
    #[arg(
        long,
        help = "Source bundle JSON file to analyze with the existing bundle preflight builder.",
        help_heading = "Input Options"
    )]
    pub source_bundle: Option<PathBuf>,
    #[arg(
        long,
        help = "Target inventory JSON file used by bundle and promotion preflight builders.",
        help_heading = "Input Options"
    )]
    pub target_inventory: Option<PathBuf>,
    #[arg(
        long,
        help = "Alert export directory to summarize with the stable root index contract.",
        help_heading = "Input Options"
    )]
    pub alert_export_dir: Option<PathBuf>,
    #[arg(
        long,
        help = "Optional availability hints JSON file reused by the bundle and promotion preflight builders.",
        help_heading = "Input Options"
    )]
    pub availability_file: Option<PathBuf>,
    #[arg(
        long,
        help = "Optional promotion mapping JSON file reused by the promotion preflight builder.",
        help_heading = "Input Options"
    )]
    pub mapping_file: Option<PathBuf>,
    #[arg(
        long,
        value_enum,
        default_value_t = OverviewOutputFormat::Text,
        help = "Render the overview document as text, json, or interactive output.",
        help_heading = "Output Options"
    )]
    pub output: OverviewOutputFormat,
}

/// CLI shape for `grafana-util overview`.
#[derive(Debug, Clone, Parser)]
#[command(
    name = "grafana-util overview",
    about = "Render a project-wide overview from staged artifacts, or use `live` as a thin entrypoint into shared live project status.",
    args_conflicts_with_subcommands = true
)]
pub struct OverviewCliArgs {
    #[command(flatten)]
    pub staged: OverviewArgs,
    #[command(subcommand)]
    pub command: Option<OverviewCommand>,
}

/// Overview subcommands exposed through the unified root CLI.
#[derive(Debug, Clone, Subcommand)]
pub enum OverviewCommand {
    #[command(
        about = "Render a live overview by delegating to the shared project-status live path."
    )]
    Live(OverviewLiveArgs),
}

/// Stable input field used by overview artifact entries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverviewInputField {
    pub name: String,
    pub value: String,
}

/// Stable overview artifact entry.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverviewArtifact {
    pub kind: String,
    pub title: String,
    pub inputs: Vec<OverviewInputField>,
    pub document: Value,
}

/// Stable overview section fact used by the TUI summary cards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverviewSectionFact {
    pub label: String,
    pub value: String,
}

/// Stable overview section item aligned with the shared browser/workbench shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverviewSectionItem {
    pub kind: String,
    pub title: String,
    pub meta: String,
    pub facts: Vec<OverviewSectionFact>,
    pub details: Vec<String>,
}

/// Stable overview section view aligned with the shared workbench vocabulary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverviewSectionView {
    pub label: String,
    pub items: Vec<OverviewSectionItem>,
}

/// Stable overview section entry used by the TUI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverviewSection {
    pub artifact_index: usize,
    pub kind: String,
    pub label: String,
    pub subtitle: String,
    pub views: Vec<OverviewSectionView>,
}

/// Stable overview summary block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OverviewSummary {
    pub artifact_count: usize,
    pub dashboard_export_count: usize,
    pub datasource_export_count: usize,
    pub alert_export_count: usize,
    pub access_user_export_count: usize,
    pub access_team_export_count: usize,
    pub access_org_export_count: usize,
    pub access_service_account_export_count: usize,
    pub sync_summary_count: usize,
    pub bundle_preflight_count: usize,
    pub promotion_preflight_count: usize,
}

/// Stable overview document emitted by the runner.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverviewDocument {
    pub kind: String,
    #[serde(rename = "schemaVersion")]
    pub schema_version: i64,
    #[serde(rename = "toolVersion")]
    pub tool_version: String,
    pub summary: OverviewSummary,
    pub project_status: OverviewProjectStatus,
    pub artifacts: Vec<OverviewArtifact>,
    pub selected_section_index: usize,
    pub sections: Vec<OverviewSection>,
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn build_overview_artifacts(args: &OverviewArgs) -> Result<Vec<OverviewArtifact>> {
    overview_artifacts::build_overview_artifacts(args)
}

pub fn build_overview_document(artifacts: Vec<OverviewArtifact>) -> Result<OverviewDocument> {
    overview_document::build_overview_document(artifacts)
}

pub fn render_overview_text(document: &OverviewDocument) -> Result<Vec<String>> {
    overview_document::render_overview_text(document)
}

#[cfg(feature = "tui")]
pub(crate) fn run_overview_interactive(document: OverviewDocument) -> Result<()> {
    overview_tui::run_overview_interactive(document)
}

#[cfg(not(feature = "tui"))]
pub(crate) fn run_overview_interactive(_document: OverviewDocument) -> Result<()> {
    Err(crate::common::tui(
        "Overview interactive mode requires the `tui` feature.",
    ))
}

/// Run the overview command using staged artifact inputs and the requested output format.
pub fn run_overview(args: OverviewArgs) -> Result<()> {
    let artifacts = overview_artifacts::build_overview_artifacts(&args)?;
    let document = build_overview_document(artifacts)?;
    match args.output {
        OverviewOutputFormat::Text => {
            for line in render_overview_text(&document)? {
                println!("{line}");
            }
            Ok(())
        }
        OverviewOutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&document)?);
            Ok(())
        }
        #[cfg(feature = "tui")]
        OverviewOutputFormat::Interactive => run_overview_interactive(document),
    }
}

/// Backward-compatible CLI entrypoint for the existing dispatcher wiring.
pub fn run_overview_cli(args: OverviewCliArgs) -> Result<()> {
    match args.command {
        Some(OverviewCommand::Live(live_args)) => run_overview_live(live_args),
        None => run_overview(args.staged),
    }
}

type OverviewLiveRunner = fn(OverviewLiveArgs) -> Result<()>;

fn overview_live_runner() -> OverviewLiveRunner {
    run_project_status_live
}

/// Run the overview live alias by delegating to the shared project-status live path.
pub fn run_overview_live(args: OverviewLiveArgs) -> Result<()> {
    overview_live_runner()(args)
}

#[cfg(test)]
mod tests {
    use super::{overview_live_runner, OverviewLiveRunner};
    use crate::project_status_command::run_project_status_live;

    #[test]
    fn overview_live_alias_dispatches_to_project_status_live_runner() {
        let runner = overview_live_runner();
        let project_status_runner: OverviewLiveRunner = run_project_status_live;
        assert!(std::ptr::fn_addr_eq(runner, project_status_runner));
    }
}
