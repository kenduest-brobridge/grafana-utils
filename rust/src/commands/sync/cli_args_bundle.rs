use clap::Args;
use std::path::PathBuf;

use super::cli_args_common::SyncOutputFormat;

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
