//! Unified CLI dispatcher for Rust entrypoints.
//!
//! Purpose:
//! - Own only command topology and domain dispatch.
//! - Keep `grafana-util` command surface in one place.
//! - Route to domain runners (`dashboard`, `alert`, `access`, `datasource`) without
//!   carrying transport/request behavior.
//!
//! Flow:
//! - Parse into `CliArgs` via Clap.
//! - Normalize namespaced command forms into one domain command enum.
//! - Delegate execution to the selected domain runner function.
//!
//! Caveats:
//! - Do not add domain logic or HTTP transport details here.
//! - Keep help output canonical-first so users discover formal commands.
use clap::{Parser, Subcommand};

use crate::access::{run_access_cli, AccessCliArgs};
use crate::alert::{
    normalize_alert_namespace_args, run_alert_cli, AlertCliArgs, AlertNamespaceArgs,
};
use crate::common::Result;
use crate::dashboard::{
    run_dashboard_cli, DashboardCliArgs, DashboardCommand, DiffArgs, ExportArgs, ImportArgs,
    InspectExportArgs, InspectLiveArgs, InspectVarsArgs, ListArgs, ListDataSourcesArgs,
    ScreenshotArgs,
};
use crate::datasource::{run_datasource_cli, DatasourceGroupCommand};
use crate::sync::{run_sync_cli, SyncGroupCommand};

const UNIFIED_HELP_TEXT: &str = "Examples:\n\n  Export dashboards:\n    grafana-util dashboard export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --export-dir ./dashboards --overwrite\n\n  Capture a dashboard screenshot from browser-like state:\n    grafana-util dashboard screenshot --dashboard-url 'https://grafana.example.com/d/cpu-main/cpu-overview?var-cluster=prod-a' --token \"$GRAFANA_API_TOKEN\" --output ./cpu-main.png --full-page\n\n  Inspect dashboard variables before capture:\n    grafana-util dashboard inspect-vars --dashboard-url 'https://grafana.example.com/d/cpu-main/cpu-overview?var-cluster=prod-a' --token \"$GRAFANA_API_TOKEN\"\n\n  Export alerting resources through the unified binary:\n    grafana-util alert export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-dir ./alerts --overwrite\n\n  List org users through the unified binary:\n    grafana-util access user list --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --json\n\n  Build a local staged sync preflight document:\n    grafana-util sync preflight --desired-file ./desired.json --availability-file ./availability.json\n\nCompatibility shim remains available:\n  grafana-access-utils ...";

#[derive(Debug, Clone, Subcommand)]
pub enum DashboardGroupCommand {
    #[command(about = "List dashboard summaries without writing export files.")]
    List(ListArgs),
    #[command(
        name = "list-data-sources",
        about = "List datasource inventory under the dashboard command surface."
    )]
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
    #[command(about = "List dashboard templating variables from live Grafana.")]
    InspectVars(InspectVarsArgs),
    #[command(about = "Open one dashboard in a headless browser and capture image or PDF output.")]
    Screenshot(ScreenshotArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum UnifiedCommand {
    #[command(
        about = "Run dashboard export, list, import, and diff workflows.",
        visible_alias = "db"
    )]
    Dashboard {
        #[command(subcommand)]
        command: DashboardGroupCommand,
    },
    #[command(
        about = "Run datasource list, export, import, and diff workflows.",
        visible_alias = "ds"
    )]
    Datasource {
        #[command(subcommand)]
        command: DatasourceGroupCommand,
    },
    #[command(
        about = "Run local/document-only sync summary and preflight workflows.",
        visible_alias = "sy"
    )]
    Sync {
        #[command(subcommand)]
        command: SyncGroupCommand,
    },
    #[command(about = "Export, import, or diff Grafana alerting resources.")]
    Alert(AlertNamespaceArgs),
    #[command(about = "List and manage Grafana users, teams, and service accounts.")]
    Access(AccessCliArgs),
}

#[derive(Debug, Clone, Parser)]
#[command(
    name = "grafana-util",
    about = "Unified Grafana dashboard, alerting, and access utility.",
    after_help = UNIFIED_HELP_TEXT
)]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: UnifiedCommand,
}

/// Parse raw argv into the unified command tree.
///
/// This is intentionally side-effect-free and should only validate CLI shape.
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
        DashboardGroupCommand::InspectVars(inner) => {
            wrap_dashboard(DashboardCommand::InspectVars(inner))
        }
        DashboardGroupCommand::Screenshot(inner) => {
            wrap_dashboard(DashboardCommand::Screenshot(inner))
        }
    }
}

// Centralized command fan-out before invoking domain runners.
// Every unified CLI variant is normalized into one of dashboard/alert/datasource/access runners here.
fn dispatch_with_handlers<FD, FS, FY, FA, FX>(
    args: CliArgs,
    mut run_dashboard: FD,
    mut run_datasource: FS,
    mut run_sync: FY,
    mut run_alert: FA,
    mut run_access: FX,
) -> Result<()>
where
    FD: FnMut(DashboardCliArgs) -> Result<()>,
    FS: FnMut(DatasourceGroupCommand) -> Result<()>,
    FY: FnMut(SyncGroupCommand) -> Result<()>,
    FA: FnMut(AlertCliArgs) -> Result<()>,
    FX: FnMut(AccessCliArgs) -> Result<()>,
{
    match args.command {
        UnifiedCommand::Dashboard { command } => run_dashboard(wrap_dashboard_group(command)),
        UnifiedCommand::Datasource { command } => run_datasource(command),
        UnifiedCommand::Sync { command } => run_sync(command),
        UnifiedCommand::Alert(inner) => run_alert(normalize_alert_namespace_args(inner)),
        UnifiedCommand::Access(inner) => run_access(inner),
    }
}

/// Runtime entrypoint for unified execution.
///
/// Keeping handler execution injectable via `dispatch_with_handlers` allows tests to
/// validate dispatch logic without touching network transport.
pub fn run_cli(args: CliArgs) -> Result<()> {
    dispatch_with_handlers(
        args,
        run_dashboard_cli,
        run_datasource_cli,
        run_sync_cli,
        run_alert_cli,
        run_access_cli,
    )
}

#[cfg(test)]
#[path = "cli_rust_tests.rs"]
mod cli_rust_tests;
