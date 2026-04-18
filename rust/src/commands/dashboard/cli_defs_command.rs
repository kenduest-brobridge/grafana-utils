//! CLI definitions for Dashboard command surface and option compatibility behavior.

use crate::common::CliColorChoice;
use clap::{Parser, Subcommand, ValueEnum};

use super::cli_defs_inspect::{
    GovernanceGateArgs, ImpactArgs, InspectExportArgs, InspectLiveArgs, InspectVarsArgs,
    ScreenshotArgs, SummaryArgs, TopologyArgs, ValidateExportArgs,
};
use super::{
    DASHBOARD_BROWSE_AFTER_HELP, DASHBOARD_CLI_AFTER_HELP, DASHBOARD_CLONE_LIVE_AFTER_HELP,
    DASHBOARD_DELETE_AFTER_HELP, DASHBOARD_DIFF_AFTER_HELP, DASHBOARD_EDIT_LIVE_AFTER_HELP,
    DASHBOARD_EXPORT_AFTER_HELP, DASHBOARD_FETCH_LIVE_AFTER_HELP,
    DASHBOARD_GOVERNANCE_GATE_AFTER_HELP, DASHBOARD_IMPACT_AFTER_HELP, DASHBOARD_IMPORT_AFTER_HELP,
    DASHBOARD_LIST_AFTER_HELP, DASHBOARD_LIST_VARS_AFTER_HELP, DASHBOARD_PATCH_FILE_AFTER_HELP,
    DASHBOARD_PLAN_AFTER_HELP, DASHBOARD_PUBLISH_AFTER_HELP, DASHBOARD_REVIEW_AFTER_HELP,
    DASHBOARD_SCREENSHOT_AFTER_HELP, DASHBOARD_SERVE_AFTER_HELP, DASHBOARD_SUMMARY_AFTER_HELP,
    DASHBOARD_SUMMARY_EXPORT_AFTER_HELP, DASHBOARD_SUMMARY_LIVE_AFTER_HELP,
    DASHBOARD_TOPOLOGY_AFTER_HELP, DASHBOARD_VALIDATE_EXPORT_AFTER_HELP,
};

#[path = "cli_defs_command_export.rs"]
mod cli_defs_command_export;
#[path = "cli_defs_command_history.rs"]
mod cli_defs_command_history;
#[path = "cli_defs_command_list.rs"]
mod cli_defs_command_list;
#[path = "cli_defs_command_live.rs"]
mod cli_defs_command_live;
#[path = "cli_defs_command_local.rs"]
mod cli_defs_command_local;
#[path = "cli_defs_command_plan.rs"]
mod cli_defs_command_plan;

pub use cli_defs_command_export::{
    ExportArgs, ExportLayoutArgs, ExportLayoutOutputFormat, ExportLayoutVariant, RawToPromptArgs,
};
pub use cli_defs_command_history::{
    DashboardHistoryArgs, DashboardHistorySubcommand, HistoryDiffArgs, HistoryExportArgs,
    HistoryListArgs, HistoryRestoreArgs,
};
pub use cli_defs_command_list::ListArgs;
pub use cli_defs_command_live::{
    BrowseArgs, CloneLiveArgs, DeleteArgs, DiffArgs, EditLiveArgs, GetArgs,
};
pub use cli_defs_command_local::{ImportArgs, PatchFileArgs, PublishArgs, ReviewArgs, ServeArgs};
pub use cli_defs_command_plan::{DashboardPlanOutputFormat, PlanArgs};

/// Arguments for importing dashboards from a local export directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DashboardImportInputFormat {
    Raw,
    Provisioning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum InspectExportInputType {
    Raw,
    Source,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DashboardServeScriptFormat {
    Json,
    Yaml,
}

/// Enum definition for DashboardCommand.
#[derive(Debug, Clone, Subcommand)]
pub enum DashboardCommand {
    #[command(
        name = "list",
        about = "List dashboard summaries without writing export files.",
        after_help = DASHBOARD_LIST_AFTER_HELP
    )]
    List(ListArgs),
    #[command(
        name = "get",
        about = "Fetch one live dashboard into an API-safe local JSON draft.",
        after_help = DASHBOARD_FETCH_LIVE_AFTER_HELP
    )]
    Get(GetArgs),
    #[command(
        name = "clone",
        about = "Clone one live dashboard into a local draft with optional overrides.",
        after_help = DASHBOARD_CLONE_LIVE_AFTER_HELP
    )]
    CloneLive(CloneLiveArgs),
    #[command(
        name = "serve",
        about = "Serve dashboard drafts through a local preview server.",
        after_help = DASHBOARD_SERVE_AFTER_HELP
    )]
    Serve(ServeArgs),
    #[command(
        name = "edit-live",
        about = "Fetch one live dashboard into an external editor, review the edited draft, and either preview, save, or apply the result.",
        after_help = DASHBOARD_EDIT_LIVE_AFTER_HELP
    )]
    EditLive(EditLiveArgs),
    #[command(
        name = "export",
        about = "Export dashboards to raw/, prompt/, provisioning/, and optional history/ files.",
        after_help = DASHBOARD_EXPORT_AFTER_HELP
    )]
    Export(ExportArgs),
    #[command(
        name = "import",
        about = "Import dashboard JSON files through the Grafana API.",
        after_help = DASHBOARD_IMPORT_AFTER_HELP
    )]
    Import(ImportArgs),
    #[command(
        name = "browse",
        about = "Browse live Grafana or a local export tree in an interactive terminal UI.",
        after_help = DASHBOARD_BROWSE_AFTER_HELP
    )]
    Browse(BrowseArgs),
    #[command(
        name = "history",
        about = "List or restore dashboard revision history from Grafana."
    )]
    History(DashboardHistoryArgs),
    #[command(
        name = "delete",
        about = "Delete live dashboards by UID or folder path.",
        after_help = DASHBOARD_DELETE_AFTER_HELP
    )]
    Delete(DeleteArgs),
    #[command(
        about = "Compare local dashboard files against live Grafana dashboards.",
        after_help = DASHBOARD_DIFF_AFTER_HELP
    )]
    Diff(DiffArgs),
    #[command(
        name = "plan",
        about = "Build a review-first dashboard reconcile plan from a local export tree against live Grafana.",
        after_help = DASHBOARD_PLAN_AFTER_HELP
    )]
    Plan(PlanArgs),
    #[command(
        name = "patch",
        about = "Patch one local dashboard JSON file in place or to a new path.",
        after_help = DASHBOARD_PATCH_FILE_AFTER_HELP
    )]
    PatchFile(PatchFileArgs),
    #[command(
        name = "review",
        about = "Review one local dashboard JSON file without touching Grafana.",
        after_help = DASHBOARD_REVIEW_AFTER_HELP
    )]
    Review(ReviewArgs),
    #[command(
        name = "publish",
        about = "Publish one local dashboard JSON file through the existing dashboard import pipeline.",
        after_help = DASHBOARD_PUBLISH_AFTER_HELP
    )]
    Publish(PublishArgs),
    #[command(
        name = "summary",
        about = "Summarize dashboards from live Grafana or a local export tree and build summary or governance artifacts.",
        after_help = DASHBOARD_SUMMARY_AFTER_HELP
    )]
    Summary(SummaryArgs),
    #[command(
        name = "summary-export",
        hide = true,
        about = "Summarize dashboards from local export directories.",
        after_help = DASHBOARD_SUMMARY_EXPORT_AFTER_HELP
    )]
    InspectExport(InspectExportArgs),
    #[command(
        name = "summary-live",
        hide = true,
        about = "Summarize dashboards from live Grafana.",
        after_help = DASHBOARD_SUMMARY_LIVE_AFTER_HELP
    )]
    InspectLive(InspectLiveArgs),
    #[command(
        name = "variables",
        about = "List dashboard templating variables and datasource-like choices from live Grafana or a local dashboard file.",
        after_help = DASHBOARD_LIST_VARS_AFTER_HELP
    )]
    InspectVars(InspectVarsArgs),
    #[command(
        name = "policy",
        about = "Check dashboard findings against a policy from live Grafana or a local export tree.",
        after_help = DASHBOARD_GOVERNANCE_GATE_AFTER_HELP
    )]
    GovernanceGate(GovernanceGateArgs),
    #[command(
        name = "dependencies",
        about = "Show dashboard dependencies directly from live Grafana or a local export tree.",
        after_help = DASHBOARD_TOPOLOGY_AFTER_HELP
    )]
    Topology(TopologyArgs),
    #[command(
        name = "impact",
        about = "Show which dashboards and alert resources would be affected by one data source from live Grafana, an export tree, or saved artifacts.",
        after_help = DASHBOARD_IMPACT_AFTER_HELP
    )]
    Impact(ImpactArgs),
    #[command(
        name = "validate-export",
        about = "Run strict schema validation against dashboard raw export files before GitOps sync.",
        after_help = DASHBOARD_VALIDATE_EXPORT_AFTER_HELP
    )]
    ValidateExport(ValidateExportArgs),
    #[command(
        name = "screenshot",
        about = "Open one Grafana dashboard in a headless browser and capture PNG, JPEG, or PDF output.",
        after_help = DASHBOARD_SCREENSHOT_AFTER_HELP
    )]
    Screenshot(ScreenshotArgs),
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn dashboard_history_restore_help_mentions_same_dashboard() {
        let command = DashboardCliArgs::command();
        let history = command.find_subcommand("history").unwrap();
        let restore = history.find_subcommand("restore").unwrap();
        let about = restore.get_about().unwrap().to_string();
        assert!(about.contains("previous live dashboard revision"));
    }
}

#[derive(Debug, Clone, Parser)]
#[command(
    about = "Export or import Grafana dashboards.",
    after_help = DASHBOARD_CLI_AFTER_HELP,
    styles = crate::help_styles::CLI_HELP_STYLES
)]
/// Struct definition for DashboardCliArgs.
pub struct DashboardCliArgs {
    #[arg(
        long,
        value_enum,
        default_value_t = CliColorChoice::Auto,
        help = "Colorize JSON output. Use auto, always, or never."
    )]
    pub color: CliColorChoice,
    #[command(subcommand)]
    pub command: DashboardCommand,
}
