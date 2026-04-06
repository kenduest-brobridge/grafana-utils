//! CLI definitions for live dashboard fetch, clone, browse, delete, and diff workflows.

use clap::Args;
use std::path::PathBuf;

use super::super::super::{DEFAULT_IMPORT_MESSAGE, DEFAULT_PAGE_SIZE};
use super::super::cli_defs_shared::{CommonCliArgs, DryRunOutputFormat};
use crate::common::DiffOutputFormat;

/// Arguments for editing one live dashboard through an external editor.
#[derive(Debug, Clone, Args)]
pub struct EditLiveArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long = "dashboard-uid", help = "Live Grafana dashboard UID to edit.")]
    pub dashboard_uid: String,
    #[arg(
        long,
        help = "Write the edited dashboard draft to this file path instead of using ./<uid>.edited.json."
    )]
    pub output: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        help = "Apply the edited dashboard back to Grafana immediately instead of writing a local draft file."
    )]
    pub apply_live: bool,
    #[arg(
        long,
        default_value = DEFAULT_IMPORT_MESSAGE,
        help = "Revision message to use when --apply-live writes the edited dashboard back to Grafana."
    )]
    pub message: String,
    #[arg(
        long,
        default_value_t = false,
        help = "Acknowledge the live writeback when --apply-live is set."
    )]
    pub yes: bool,
}

/// Arguments for fetching one live dashboard into a local draft file.
#[derive(Debug, Clone, Args)]
pub struct GetArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long = "dashboard-uid", help = "Live Grafana dashboard UID to fetch.")]
    pub dashboard_uid: String,
    #[arg(long, help = "Write the fetched dashboard draft to this file path.")]
    pub output: PathBuf,
}

/// Arguments for cloning one live dashboard into a local draft file.
#[derive(Debug, Clone, Args)]
pub struct CloneLiveArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long = "source-uid", help = "Live Grafana dashboard UID to clone.")]
    pub source_uid: String,
    #[arg(long, help = "Write the cloned dashboard draft to this file path.")]
    pub output: PathBuf,
    #[arg(
        long,
        help = "Override the cloned dashboard title. Defaults to the source title."
    )]
    pub name: Option<String>,
    #[arg(
        long,
        help = "Override the cloned dashboard UID. Defaults to the source UID."
    )]
    pub uid: Option<String>,
    #[arg(
        long = "folder-uid",
        help = "Override the cloned dashboard folder UID in the preserved Grafana metadata."
    )]
    pub folder_uid: Option<String>,
}

/// Arguments for deleting live dashboards by UID or folder path.
#[derive(Debug, Clone, Args)]
pub struct DeleteArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        default_value_t = DEFAULT_PAGE_SIZE,
        help = "Dashboard search page size used to resolve delete selectors."
    )]
    pub page_size: usize,
    #[arg(
        long,
        help = "Delete dashboards from one explicit Grafana org ID instead of the current org. Use this when the same Basic auth credentials can reach multiple orgs."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        help = "Dashboard UID to delete.",
        help_heading = "Target Options"
    )]
    pub uid: Option<String>,
    #[arg(
        long,
        help = "Grafana folder path root to delete recursively, for example 'Platform / Infra'.",
        help_heading = "Target Options"
    )]
    pub path: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "With --path, also delete matched Grafana folders after deleting dashboards in the subtree.",
        help_heading = "Target Options"
    )]
    pub delete_folders: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Acknowledge the live dashboard delete. Required unless --dry-run or --interactive is set.",
        help_heading = "Safety Options"
    )]
    pub yes: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Prompt for the delete selector, preview the delete plan, and confirm interactively.",
        help_heading = "Safety Options"
    )]
    pub interactive: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Preview what dashboard delete would do without changing Grafana.",
        help_heading = "Output Options"
    )]
    pub dry_run: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "For --dry-run only, render a compact table instead of plain text.",
        help_heading = "Output Options"
    )]
    pub table: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "For --dry-run only, render one JSON document.",
        help_heading = "Output Options"
    )]
    pub json: bool,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["table", "json"],
        help = "Alternative single-flag output selector for dashboard delete dry-run output. Use text, table, or json.",
        help_heading = "Output Options"
    )]
    pub output_format: Option<DryRunOutputFormat>,
    #[arg(
        long,
        default_value_t = false,
        help = "For --dry-run --table only, omit the table header row.",
        help_heading = "Output Options"
    )]
    pub no_header: bool,
}

/// Arguments for browsing the live dashboard tree in a TUI.
#[derive(Debug, Clone, Args)]
pub struct BrowseArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long = "input-dir",
        help = "Browse dashboards from this local export tree instead of live Grafana. Point this at a raw export root, an all-orgs export root, or a provisioning root when you want to inspect files without calling Grafana."
    )]
    pub input_dir: Option<PathBuf>,
    #[arg(
        long,
        value_enum,
        default_value_t = super::DashboardImportInputFormat::Raw,
        requires = "input_dir",
        help = "Interpret --input-dir as raw export files or Grafana file-provisioning artifacts. Use provisioning to accept either the provisioning/ root or its dashboards/ subdirectory."
    )]
    pub input_format: super::DashboardImportInputFormat,
    #[arg(
        long,
        default_value_t = DEFAULT_PAGE_SIZE,
        help = "Dashboard search page size used to build the live browser tree."
    )]
    pub page_size: usize,
    #[arg(
        long,
        conflicts_with = "all_orgs",
        help = "Browse dashboards from one explicit Grafana org ID instead of the current org."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "org_id",
        help = "Enumerate all visible Grafana orgs and browse the dashboard tree across them. Prefer Basic auth when you need cross-org browse because API tokens are often scoped to one org."
    )]
    pub all_orgs: bool,
    #[arg(
        long,
        help = "Optional folder path root to open instead of the full dashboard tree, for example 'Platform / Infra'."
    )]
    pub path: Option<String>,
}

/// Arguments for deleting live dashboards by UID or folder path.
#[derive(Debug, Clone, Args)]
pub struct DiffArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long = "input-dir",
        help = "Compare dashboards from this directory against Grafana. Point this to the raw/ export directory explicitly, or use with --input-format provisioning for a provisioning root or its dashboards/ subdirectory."
    )]
    pub input_dir: PathBuf,
    #[arg(
        long,
        value_enum,
        default_value_t = super::DashboardImportInputFormat::Raw,
        help = "Interpret --input-dir as raw export files or Grafana file-provisioning artifacts. Use provisioning to accept either the provisioning/ root or its dashboards/ subdirectory."
    )]
    pub input_format: super::DashboardImportInputFormat,
    #[arg(
        long,
        help = "Override the destination Grafana folder UID when comparing imported dashboards."
    )]
    pub import_folder_uid: Option<String>,
    #[arg(
        long,
        default_value_t = 3,
        help = "Number of unified diff context lines."
    )]
    pub context_lines: usize,
    #[arg(
        long = "output-format",
        value_enum,
        default_value_t = DiffOutputFormat::Text,
        help = "Render diff output as text or json."
    )]
    pub output_format: DiffOutputFormat,
}
