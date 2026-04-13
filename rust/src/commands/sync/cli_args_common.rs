use clap::{Args, Parser, ValueEnum};
use std::path::PathBuf;

use super::super::help_texts::*;
use super::cli_args_ci::SyncAdvancedCommand;
use super::cli_args_task_first::SyncGroupCommand;

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

/// Root `grafana-util workspace` parser wrapper.
#[derive(Debug, Clone, Parser)]
#[command(
    name = "grafana-util workspace",
    about = "Task-first workspace workflow for scan, test, preview, package, apply, and CI paths.",
    after_help = SYNC_ROOT_HELP_TEXT,
    styles = crate::help_styles::CLI_HELP_STYLES
)]
pub struct SyncCliArgs {
    #[command(subcommand)]
    pub command: SyncGroupCommand,
}

/// CI-oriented workspace namespace under `grafana-util workspace ci`.
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
