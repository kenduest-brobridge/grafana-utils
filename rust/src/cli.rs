use clap::{Parser, Subcommand};

use crate::access::{run_access_cli, AccessCliArgs};
use crate::alert::{
    normalize_alert_group_command, normalize_alert_namespace_args, run_alert_cli, AlertCliArgs,
    AlertDiffArgs, AlertExportArgs, AlertImportArgs, AlertListArgs, AlertNamespaceArgs,
};
use crate::common::Result;
use crate::dashboard::{
    run_dashboard_cli, DashboardCliArgs, DashboardCommand, DiffArgs, ExportArgs, ImportArgs,
    InspectExportArgs, InspectLiveArgs, ListArgs, ListDataSourcesArgs,
};
use crate::datasource::{run_datasource_cli, DatasourceGroupCommand};

const UNIFIED_HELP_TEXT: &str = "Examples:\n\n  Export dashboards:\n    grafana-utils export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --export-dir ./dashboards --overwrite\n\n  Export alerting resources through the unified binary:\n    grafana-utils alert export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-dir ./alerts --overwrite\n\n  List org users through the unified binary:\n    grafana-utils access user list --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --json\n\nCompatibility shim remains available:\n  grafana-access-utils ...";

#[derive(Debug, Clone, Subcommand)]
pub enum DashboardGroupCommand {
    #[command(
        visible_alias = "list-dashboard",
        about = "List dashboard summaries without writing export files."
    )]
    List(ListArgs),
    #[command(name = "list-data-sources", about = "List Grafana data sources.")]
    ListDataSources(ListDataSourcesArgs),
    #[command(
        visible_alias = "export-dashboard",
        about = "Export dashboards to raw/ and prompt/ JSON files."
    )]
    Export(ExportArgs),
    #[command(
        visible_alias = "import-dashboard",
        about = "Import dashboard JSON files through the Grafana API."
    )]
    Import(ImportArgs),
    #[command(about = "Compare local raw dashboard files against live Grafana dashboards.")]
    Diff(DiffArgs),
    #[command(about = "Analyze a raw dashboard export directory and summarize its structure.")]
    InspectExport(InspectExportArgs),
    #[command(about = "Analyze live Grafana dashboards without writing a persistent export.")]
    InspectLive(InspectLiveArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum UnifiedCommand {
    #[command(about = "Run dashboard export, list, import, and diff workflows.")]
    Dashboard {
        #[command(subcommand)]
        command: DashboardGroupCommand,
    },
    #[command(about = "Run datasource list, export, import, and diff workflows.")]
    Datasource {
        #[command(subcommand)]
        command: DatasourceGroupCommand,
    },
    #[command(about = "List dashboard summaries without writing export files.")]
    List(ListArgs),
    #[command(name = "list-data-sources", about = "List Grafana data sources.")]
    ListDataSources(ListDataSourcesArgs),
    #[command(about = "Export dashboards to raw/ and prompt/ JSON files.")]
    Export(ExportArgs),
    #[command(about = "Import dashboard JSON files through the Grafana API.")]
    Import(ImportArgs),
    #[command(about = "Compare local raw dashboard files against live Grafana dashboards.")]
    Diff(DiffArgs),
    #[command(about = "Analyze a raw dashboard export directory and summarize its structure.")]
    InspectExport(InspectExportArgs),
    #[command(about = "Analyze live Grafana dashboards without writing a persistent export.")]
    InspectLive(InspectLiveArgs),
    #[command(about = "Export, import, or diff Grafana alerting resources.")]
    Alert(AlertNamespaceArgs),
    #[command(
        name = "export-alert",
        about = "Export alerting resources into raw/ JSON files."
    )]
    ExportAlert(AlertExportArgs),
    #[command(
        name = "import-alert",
        about = "Import alerting resource JSON files through the Grafana API."
    )]
    ImportAlert(AlertImportArgs),
    #[command(
        name = "diff-alert",
        about = "Compare local alerting export files against live Grafana resources."
    )]
    DiffAlert(AlertDiffArgs),
    #[command(name = "list-alert-rules", about = "List live Grafana alert rules.")]
    ListAlertRules(AlertListArgs),
    #[command(
        name = "list-alert-contact-points",
        about = "List live Grafana alert contact points."
    )]
    ListAlertContactPoints(AlertListArgs),
    #[command(
        name = "list-alert-mute-timings",
        about = "List live Grafana mute timings."
    )]
    ListAlertMuteTimings(AlertListArgs),
    #[command(
        name = "list-alert-templates",
        about = "List live Grafana notification templates."
    )]
    ListAlertTemplates(AlertListArgs),
    #[command(about = "List and manage Grafana users, teams, and service accounts.")]
    Access(AccessCliArgs),
}

#[derive(Debug, Clone, Parser)]
#[command(
    name = "grafana-utils",
    about = "Unified Grafana dashboard, alerting, and access utility.",
    after_help = UNIFIED_HELP_TEXT
)]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: UnifiedCommand,
}

pub fn parse_cli_from<I, T>(iter: I) -> CliArgs
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    CliArgs::parse_from(iter)
}

fn wrap_dashboard(command: DashboardCommand) -> DashboardCliArgs {
    DashboardCliArgs { command }
}

fn wrap_dashboard_group(command: DashboardGroupCommand) -> DashboardCliArgs {
    match command {
        DashboardGroupCommand::List(inner) => wrap_dashboard(DashboardCommand::List(inner)),
        DashboardGroupCommand::ListDataSources(inner) => {
            wrap_dashboard(DashboardCommand::ListDataSources(inner))
        }
        DashboardGroupCommand::Export(inner) => wrap_dashboard(DashboardCommand::Export(inner)),
        DashboardGroupCommand::Import(inner) => wrap_dashboard(DashboardCommand::Import(inner)),
        DashboardGroupCommand::Diff(inner) => wrap_dashboard(DashboardCommand::Diff(inner)),
        DashboardGroupCommand::InspectExport(inner) => {
            wrap_dashboard(DashboardCommand::InspectExport(inner))
        }
        DashboardGroupCommand::InspectLive(inner) => {
            wrap_dashboard(DashboardCommand::InspectLive(inner))
        }
    }
}

fn dispatch_with_handlers<FD, FS, FA, FX>(
    args: CliArgs,
    mut run_dashboard: FD,
    mut run_datasource: FS,
    mut run_alert: FA,
    mut run_access: FX,
) -> Result<()>
where
    FD: FnMut(DashboardCliArgs) -> Result<()>,
    FS: FnMut(DatasourceGroupCommand) -> Result<()>,
    FA: FnMut(AlertCliArgs) -> Result<()>,
    FX: FnMut(AccessCliArgs) -> Result<()>,
{
    match args.command {
        UnifiedCommand::Dashboard { command } => run_dashboard(wrap_dashboard_group(command)),
        UnifiedCommand::Datasource { command } => run_datasource(command),
        UnifiedCommand::List(inner) => run_dashboard(wrap_dashboard(DashboardCommand::List(inner))),
        UnifiedCommand::ListDataSources(inner) => {
            run_dashboard(wrap_dashboard(DashboardCommand::ListDataSources(inner)))
        }
        UnifiedCommand::Export(inner) => {
            run_dashboard(wrap_dashboard(DashboardCommand::Export(inner)))
        }
        UnifiedCommand::Import(inner) => {
            run_dashboard(wrap_dashboard(DashboardCommand::Import(inner)))
        }
        UnifiedCommand::Diff(inner) => run_dashboard(wrap_dashboard(DashboardCommand::Diff(inner))),
        UnifiedCommand::InspectExport(inner) => {
            run_dashboard(wrap_dashboard(DashboardCommand::InspectExport(inner)))
        }
        UnifiedCommand::InspectLive(inner) => {
            run_dashboard(wrap_dashboard(DashboardCommand::InspectLive(inner)))
        }
        UnifiedCommand::Alert(inner) => run_alert(normalize_alert_namespace_args(inner)),
        UnifiedCommand::ExportAlert(inner) => run_alert(normalize_alert_group_command(
            crate::alert::AlertGroupCommand::Export(inner),
        )),
        UnifiedCommand::ImportAlert(inner) => run_alert(normalize_alert_group_command(
            crate::alert::AlertGroupCommand::Import(inner),
        )),
        UnifiedCommand::DiffAlert(inner) => run_alert(normalize_alert_group_command(
            crate::alert::AlertGroupCommand::Diff(inner),
        )),
        UnifiedCommand::ListAlertRules(inner) => run_alert(normalize_alert_group_command(
            crate::alert::AlertGroupCommand::ListRules(inner),
        )),
        UnifiedCommand::ListAlertContactPoints(inner) => run_alert(normalize_alert_group_command(
            crate::alert::AlertGroupCommand::ListContactPoints(inner),
        )),
        UnifiedCommand::ListAlertMuteTimings(inner) => run_alert(normalize_alert_group_command(
            crate::alert::AlertGroupCommand::ListMuteTimings(inner),
        )),
        UnifiedCommand::ListAlertTemplates(inner) => run_alert(normalize_alert_group_command(
            crate::alert::AlertGroupCommand::ListTemplates(inner),
        )),
        UnifiedCommand::Access(inner) => run_access(inner),
    }
}

pub fn run_cli(args: CliArgs) -> Result<()> {
    dispatch_with_handlers(
        args,
        run_dashboard_cli,
        run_datasource_cli,
        run_alert_cli,
        run_access_cli,
    )
}

#[cfg(test)]
#[path = "cli_rust_tests.rs"]
mod cli_rust_tests;
