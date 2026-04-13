use std::io::{self, IsTerminal};
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use dialoguer::{theme::ColorfulTheme, MultiSelect};

use crate::common::{CliColorChoice, Result};
use crate::dashboard::CommonCliArgs;
use crate::overview::OverviewOutputFormat;

const SNAPSHOT_ROOT_HELP_TEXT: &str = "Examples:\n\n  grafana-util snapshot export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-dir ./snapshot\n\n  grafana-util snapshot export --profile prod --prompt --output-dir ./snapshot\n\n  grafana-util snapshot review --input-dir ./snapshot --output-format table\n\n  grafana-util snapshot review --input-dir ./snapshot --interactive";
const SNAPSHOT_EXPORT_HELP_TEXT: &str = "Examples:\n\n  grafana-util snapshot export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-dir ./snapshot\n  grafana-util snapshot export --profile prod --prompt --output-dir ./snapshot --overwrite";
const SNAPSHOT_REVIEW_HELP_TEXT: &str = "Examples:\n\n  grafana-util snapshot review --input-dir ./snapshot --output-format table\n  grafana-util snapshot review --input-dir ./snapshot --output-format csv\n  grafana-util snapshot review --input-dir ./snapshot --output-format text\n  grafana-util snapshot review --input-dir ./snapshot --output-format json\n  grafana-util snapshot review --input-dir ./snapshot --output-format yaml\n  grafana-util snapshot review --input-dir ./snapshot --interactive";

#[cfg(feature = "tui")]
const SNAPSHOT_REVIEW_OUTPUT_HELP: &str =
    "Render the snapshot inventory review as table, csv, text, json, yaml, or interactive browser output.";

#[cfg(not(feature = "tui"))]
const SNAPSHOT_REVIEW_OUTPUT_HELP: &str =
    "Render the snapshot inventory review as table, csv, text, json, or yaml output.";

#[derive(Debug, Clone, Args)]
pub struct SnapshotExportArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long = "output-dir",
        default_value = "snapshot",
        help = "Directory to write the snapshot export root into. The live export writes dashboard and datasource bundles under this root."
    )]
    pub output_dir: PathBuf,
    #[arg(
        long,
        help = "Replace an existing snapshot export root instead of failing when the dashboard or datasource export directories already exist."
    )]
    pub overwrite: bool,
    #[arg(
        long = "prompt",
        default_value_t = false,
        help = "Prompt for which snapshot lanes to export before starting."
    )]
    pub prompt: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SnapshotExportLane {
    Dashboards,
    Datasources,
    AccessUsers,
    AccessTeams,
    AccessOrgs,
    AccessServiceAccounts,
}

impl SnapshotExportLane {
    pub(crate) const ALL: [Self; 6] = [
        Self::Dashboards,
        Self::Datasources,
        Self::AccessUsers,
        Self::AccessTeams,
        Self::AccessOrgs,
        Self::AccessServiceAccounts,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Dashboards => "dashboards",
            Self::Datasources => "datasources",
            Self::AccessUsers => "access users",
            Self::AccessTeams => "access teams",
            Self::AccessOrgs => "access orgs",
            Self::AccessServiceAccounts => "access service-accounts",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SnapshotExportSelection {
    pub(crate) lanes: Vec<SnapshotExportLane>,
}

impl SnapshotExportSelection {
    pub(crate) fn all() -> Self {
        Self {
            lanes: SnapshotExportLane::ALL.to_vec(),
        }
    }

    pub(crate) fn contains(&self, lane: SnapshotExportLane) -> bool {
        self.lanes.contains(&lane)
    }
}

pub(crate) fn prompt_snapshot_export_selection() -> Result<Option<SnapshotExportSelection>> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(crate::common::message(
            "Snapshot export interactive lane selection requires a TTY.",
        ));
    }
    let labels: Vec<&str> = SnapshotExportLane::ALL
        .iter()
        .map(|lane| lane.label())
        .collect();
    let selections = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select snapshot lanes to export")
        .items(&labels)
        .defaults(&[true, true, true, true, true, true])
        .interact_opt()
        .map_err(|error| {
            crate::common::message(format!("Snapshot export prompt failed: {error}"))
        })?;
    let Some(indexes) = selections else {
        return Ok(None);
    };
    let lanes = indexes
        .into_iter()
        .filter_map(|index| SnapshotExportLane::ALL.get(index).copied())
        .collect::<Vec<_>>();
    if lanes.is_empty() {
        return Err(crate::common::message(
            "Select at least one snapshot lane before exporting.",
        ));
    }
    Ok(Some(SnapshotExportSelection { lanes }))
}

#[derive(Debug, Clone, Args)]
pub struct SnapshotReviewArgs {
    #[arg(
        long,
        default_value = "snapshot",
        help = "Directory containing a previously exported snapshot root. The review reads the dashboard and datasource inventory under this root."
    )]
    pub input_dir: PathBuf,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "output_format",
        help = "Shortcut for --output-format interactive."
    )]
    pub interactive: bool,
    #[arg(
        long = "output-format",
        value_enum,
        default_value_t = OverviewOutputFormat::Text,
        help = SNAPSHOT_REVIEW_OUTPUT_HELP
    )]
    pub output_format: OverviewOutputFormat,
}

#[derive(Debug, Clone, Parser)]
#[command(
    name = "grafana-util snapshot",
    about = "Export and review Grafana snapshot inventory bundles.",
    after_help = SNAPSHOT_ROOT_HELP_TEXT,
    styles = crate::help_styles::CLI_HELP_STYLES
)]
pub struct SnapshotCliArgs {
    #[arg(
        long,
        value_enum,
        default_value_t = CliColorChoice::Auto,
        help = "Colorize JSON output. Use auto, always, or never."
    )]
    pub color: CliColorChoice,
    #[command(subcommand)]
    pub command: SnapshotCommand,
}

#[derive(Debug, Clone, Subcommand)]
pub enum SnapshotCommand {
    #[command(
        name = "export",
        about = "Export dashboard and datasource inventory into a local snapshot bundle.",
        after_help = SNAPSHOT_EXPORT_HELP_TEXT
    )]
    Export(SnapshotExportArgs),
    #[command(
        name = "review",
        about = "Review a local snapshot inventory without touching Grafana.",
        after_help = SNAPSHOT_REVIEW_HELP_TEXT
    )]
    Review(SnapshotReviewArgs),
}
