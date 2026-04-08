//! CLI definitions for dashboard history workflows.

use crate::common::DiffOutputFormat;
use clap::{Args, Subcommand};
use std::path::PathBuf;

use super::super::cli_defs_shared::{CommonCliArgs, HistoryOutputFormat};

/// Arguments for dashboard history list.
#[derive(Debug, Clone, Args)]
pub struct HistoryListArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help = "Dashboard UID to inspect. Required for live Grafana history, optional when filtering a local export tree, and optional validation when reading one local history artifact."
    )]
    pub dashboard_uid: Option<String>,
    #[arg(
        long,
        value_name = "FILE",
        conflicts_with = "input_dir",
        help = "Read one local history artifact JSON produced by `dashboard history export` instead of calling Grafana."
    )]
    pub input: Option<PathBuf>,
    #[arg(
        long = "input-dir",
        value_name = "DIR",
        conflicts_with = "input",
        help = "Read history artifacts from a dashboard export root produced by `dashboard export --include-history` instead of calling Grafana."
    )]
    pub input_dir: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = 20,
        help = "Maximum number of recent versions to request from Grafana in live mode."
    )]
    pub limit: usize,
    #[arg(
        long,
        value_enum,
        default_value_t = HistoryOutputFormat::Table,
        help = "Render history as text, table, json, or yaml."
    )]
    pub output_format: HistoryOutputFormat,
}

/// Arguments for dashboard history restore.
#[derive(Debug, Clone, Args)]
pub struct HistoryRestoreArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long, help = "Dashboard UID to restore from Grafana history.")]
    pub dashboard_uid: String,
    #[arg(
        long,
        help = "Dashboard history version number to restore. Required unless --prompt is used."
    )]
    pub version: Option<i64>,
    #[arg(
        long,
        default_value_t = false,
        help = "Prompt for the history version, preview the restore, and confirm in the terminal."
    )]
    pub prompt: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Preview the restore without writing a new Grafana revision."
    )]
    pub dry_run: bool,
    #[arg(
        long,
        value_enum,
        default_value_t = HistoryOutputFormat::Text,
        help = "Render restore preview or result as text, table, json, or yaml."
    )]
    pub output_format: HistoryOutputFormat,
    #[arg(
        long,
        help = "Revision message to attach to the new Grafana revision. Default: 'Restored by grafana-util dashboard history to version <n>'."
    )]
    pub message: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Confirm the live restore. Required unless --dry-run is set."
    )]
    pub yes: bool,
}

/// Arguments for exporting dashboard history into a reusable JSON artifact.
#[derive(Debug, Clone, Args)]
pub struct HistoryExportArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long, help = "Dashboard UID to export from Grafana history.")]
    pub dashboard_uid: String,
    #[arg(
        long,
        value_name = "FILE",
        help = "Write the exported dashboard history artifact to this JSON file."
    )]
    pub output: PathBuf,
    #[arg(
        long,
        default_value_t = 20,
        help = "Maximum number of recent versions to include in the exported history artifact."
    )]
    pub limit: usize,
    #[arg(
        long,
        default_value_t = false,
        help = "Overwrite an existing history artifact file."
    )]
    pub overwrite: bool,
}

/// Arguments for comparing two historical dashboard revisions.
#[derive(Debug, Clone, Args)]
pub struct HistoryDiffArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long = "base-dashboard-uid",
        help = "Base-side dashboard UID for live Grafana history or for selecting one artifact under --base-input-dir."
    )]
    pub base_dashboard_uid: Option<String>,
    #[arg(
        long = "base-input",
        value_name = "FILE",
        conflicts_with = "base_input_dir",
        help = "Base-side local history artifact JSON produced by `dashboard history export`."
    )]
    pub base_input: Option<PathBuf>,
    #[arg(
        long = "base-input-dir",
        value_name = "DIR",
        conflicts_with = "base_input",
        help = "Base-side dashboard export root produced by `dashboard export --include-history`."
    )]
    pub base_input_dir: Option<PathBuf>,
    #[arg(
        long = "new-dashboard-uid",
        help = "New-side dashboard UID for live Grafana history or for selecting one artifact under --new-input-dir."
    )]
    pub new_dashboard_uid: Option<String>,
    #[arg(
        long = "new-input",
        value_name = "FILE",
        conflicts_with = "new_input_dir",
        help = "New-side local history artifact JSON produced by `dashboard history export`."
    )]
    pub new_input: Option<PathBuf>,
    #[arg(
        long = "new-input-dir",
        value_name = "DIR",
        conflicts_with = "new_input",
        help = "New-side dashboard export root produced by `dashboard export --include-history`."
    )]
    pub new_input_dir: Option<PathBuf>,
    #[arg(
        long = "base-version",
        help = "Base-side dashboard history version number."
    )]
    pub base_version: i64,
    #[arg(
        long = "new-version",
        help = "New-side dashboard history version number."
    )]
    pub new_version: i64,
    #[arg(
        long,
        value_enum,
        default_value_t = DiffOutputFormat::Text,
        help = "Render history diff as text or json."
    )]
    pub output_format: DiffOutputFormat,
    #[arg(
        long,
        default_value_t = 3,
        help = "Number of unified diff context lines."
    )]
    pub context_lines: usize,
}

/// Dashboard history subcommands.
#[derive(Debug, Clone, Subcommand)]
pub enum DashboardHistorySubcommand {
    #[command(
        name = "list",
        about = "List live dashboard revision history or review local history artifacts.",
        after_help = "Examples:\n\n  List the last 20 live versions as a table:\n    grafana-util dashboard history list --url http://localhost:3000 --basic-user admin --basic-password admin --dashboard-uid cpu-main --limit 20 --output-format table\n\n  Review one saved history artifact without calling Grafana:\n    grafana-util dashboard history list --input ./cpu-main.history.json --output-format yaml\n\n  Scan one export tree created with --include-history:\n    grafana-util dashboard history list --input-dir ./dashboards --dashboard-uid cpu-main --output-format json"
    )]
    List(HistoryListArgs),
    #[command(
        name = "restore",
        about = "Restore a previous live dashboard revision from Grafana history.",
        after_help = "Examples:\n\n  Preview a restore without changing Grafana:\n    grafana-util dashboard history restore --url http://localhost:3000 --basic-user admin --basic-password admin --dashboard-uid cpu-main --version 17 --dry-run --output-format table\n\n  Prompt for one recent version, preview it, and confirm the restore:\n    grafana-util dashboard history restore --url http://localhost:3000 --basic-user admin --basic-password admin --dashboard-uid cpu-main --prompt\n\n  Restore a historical version and record a new revision message:\n    grafana-util dashboard history restore --url http://localhost:3000 --basic-user admin --basic-password admin --dashboard-uid cpu-main --version 17 --message 'Restore known good CPU dashboard after regression' --yes"
    )]
    Restore(HistoryRestoreArgs),
    #[command(
        name = "diff",
        about = "Compare two historical dashboard revisions from live Grafana or local history artifacts.",
        after_help = "Examples:\n\n  Compare two live revisions from Grafana:\n    grafana-util dashboard history diff --url http://localhost:3000 --basic-user admin --basic-password admin --base-dashboard-uid cpu-main --base-version 17 --new-dashboard-uid cpu-main --new-version 21\n\n  Compare two versions from one local history artifact:\n    grafana-util dashboard history diff --base-input ./cpu-main.history.json --base-version 17 --new-input ./cpu-main.history.json --new-version 21 --output-format json\n\n  Compare two dated export roots for the same dashboard UID:\n    grafana-util dashboard history diff --base-input-dir ./exports-2026-04-01 --base-dashboard-uid cpu-main --base-version 17 --new-input-dir ./exports-2026-04-07 --new-dashboard-uid cpu-main --new-version 21 --output-format json"
    )]
    Diff(HistoryDiffArgs),
    #[command(
        name = "export",
        about = "Export dashboard revision history into a reusable JSON artifact.",
        after_help = "Examples:\n\n  Export the last 20 revisions to a JSON artifact:\n    grafana-util dashboard history export --url http://localhost:3000 --basic-user admin --basic-password admin --dashboard-uid cpu-main --output ./cpu-main.history.json\n\n  Overwrite an existing history artifact and raise the export limit:\n    grafana-util dashboard history export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --dashboard-uid cpu-main --limit 50 --output ./cpu-main.history.json --overwrite"
    )]
    Export(HistoryExportArgs),
}

/// Dashboard history namespace arguments.
#[derive(Debug, Clone, Args)]
pub struct DashboardHistoryArgs {
    #[command(subcommand)]
    pub command: DashboardHistorySubcommand,
}
