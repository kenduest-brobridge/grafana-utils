//! CLI definitions for dashboard review-first plan workflows.
use clap::{Args, ValueEnum};
use std::path::PathBuf;

use super::super::cli_defs_shared::CommonCliArgs;
use super::super::dashboard_runtime::parse_dashboard_plan_output_column;
use super::InspectExportInputType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DashboardPlanOutputFormat {
    Text,
    Table,
    Json,
}

#[derive(Debug, Clone, Args)]
pub struct PlanArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long = "input-dir",
        value_name = "DIR",
        help = "Build a dashboard plan from this local export root or dashboard variant directory.",
        help_heading = "Input Options"
    )]
    pub input_dir: PathBuf,
    #[arg(
        long = "input-type",
        value_enum,
        default_value_t = InspectExportInputType::Raw,
        help = "Interpret --input-dir as raw or source export files. Use source for prompt-lane exports.",
        help_heading = "Input Options"
    )]
    pub input_type: InspectExportInputType,
    #[arg(
        long,
        conflicts_with = "use_export_org",
        help = "Plan against one explicit Grafana org ID instead of the current org. Use this when the same credentials can reach one target org only.",
        help_heading = "Selection Options"
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "org_id",
        help = "Route a combined multi-org export root back into matching target orgs. Requires Basic auth.",
        help_heading = "Selection Options"
    )]
    pub use_export_org: bool,
    #[arg(
        long = "only-org-id",
        requires = "use_export_org",
        help = "With --use-export-org, limit review to these exported source org IDs. Repeat the flag to select multiple orgs.",
        help_heading = "Selection Options"
    )]
    pub only_org_id: Vec<i64>,
    #[arg(
        long,
        default_value_t = false,
        requires = "use_export_org",
        help = "With --use-export-org, treat missing destination orgs as would-create targets in the review plan instead of blocking them.",
        help_heading = "Selection Options"
    )]
    pub create_missing_orgs: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Mark remote-only dashboards as would-delete candidates instead of only showing them as unmanaged review items.",
        help_heading = "Review Options"
    )]
    pub prune: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Show unchanged dashboards in text and table output.",
        help_heading = "Review Options"
    )]
    pub show_same: bool,
    #[arg(
        long = "output-columns",
        value_delimiter = ',',
        value_parser = parse_dashboard_plan_output_column,
        help = "Render only these comma-separated plan columns. Use all to expand the supported review columns.",
        help_heading = "Output Options"
    )]
    pub output_columns: Vec<String>,
    #[arg(
        long = "list-columns",
        default_value_t = false,
        help = "Print the supported --output-columns values and exit.",
        help_heading = "Output Options"
    )]
    pub list_columns: bool,
    #[arg(
        long = "no-header",
        default_value_t = false,
        help = "Do not print table headers when rendering table output.",
        help_heading = "Output Options"
    )]
    pub no_header: bool,
    #[arg(
        long,
        value_enum,
        default_value_t = DashboardPlanOutputFormat::Text,
        help = "Render plan output as text, table, or json.",
        help_heading = "Output Options"
    )]
    pub output_format: DashboardPlanOutputFormat,
}
