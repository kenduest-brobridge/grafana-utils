//! Unified CLI dispatcher for Rust entrypoints.
//!
//! Purpose:
//! - Own only command topology, legacy alias normalization, and domain dispatch.
//! - Keep `grafana-util` and compatibility aliases in one place.
//! - Route to domain runners (`dashboard`, `alert`, `access`, `datasource`) without
//!   carrying transport/request behavior.
//!
//! Flow:
//! - Parse into `CliArgs` via Clap.
//! - Normalize legacy and namespaced command forms into one domain command enum.
//! - Delegate execution to the selected domain runner function.
//!
//! Caveats:
//! - Do not add domain logic or HTTP transport details here.
//! - Keep compatibility aliases minimal so deprecation windows are easy to track.
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

const UNIFIED_HELP_TEXT: &str = "Examples:\n\n  Export dashboards:\n    grafana-util export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --export-dir ./dashboards --overwrite\n\n  Export alerting resources through the unified binary:\n    grafana-util alert export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-dir ./alerts --overwrite\n\n  List org users through the unified binary:\n    grafana-util access user list --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --json\n\nCompatibility shim remains available:\n  grafana-access-utils ...";

#[derive(Debug, Clone, Subcommand)]
pub enum DashboardGroupCommand {
    #[command(
        visible_alias = "list-dashboard",
        about = "List dashboard summaries without writing export files."
    )]
    List(ListArgs),
    #[command(
        name = "list-data-sources",
        about = "Compatibility command for datasource inventory; prefer `grafana-util datasource list`."
    )]
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
    #[command(about = "Compatibility direct form; prefer `grafana-util dashboard list`.")]
    List(ListArgs),
    #[command(
        name = "list-data-sources",
        about = "Compatibility direct form; prefer `grafana-util datasource list`."
    )]
    ListDataSources(ListDataSourcesArgs),
    #[command(about = "Compatibility direct form; prefer `grafana-util dashboard export`.")]
    Export(ExportArgs),
    #[command(about = "Compatibility direct form; prefer `grafana-util dashboard import`.")]
    Import(ImportArgs),
    #[command(about = "Compatibility direct form; prefer `grafana-util dashboard diff`.")]
    Diff(DiffArgs),
    #[command(
        about = "Compatibility direct form; prefer `grafana-util dashboard inspect-export`."
    )]
    InspectExport(InspectExportArgs),
    #[command(about = "Compatibility direct form; prefer `grafana-util dashboard inspect-live`.")]
    InspectLive(InspectLiveArgs),
    #[command(about = "Export, import, or diff Grafana alerting resources.")]
    Alert(AlertNamespaceArgs),
    #[command(
        name = "export-alert",
        about = "Compatibility direct form; prefer `grafana-util alert export`."
    )]
    ExportAlert(AlertExportArgs),
    #[command(
        name = "import-alert",
        about = "Compatibility direct form; prefer `grafana-util alert import`."
    )]
    ImportAlert(AlertImportArgs),
    #[command(
        name = "diff-alert",
        about = "Compatibility direct form; prefer `grafana-util alert diff`."
    )]
    DiffAlert(AlertDiffArgs),
    #[command(
        name = "list-alert-rules",
        about = "Compatibility direct form; prefer `grafana-util alert list-rules`."
    )]
    ListAlertRules(AlertListArgs),
    #[command(
        name = "list-alert-contact-points",
        about = "Compatibility direct form; prefer `grafana-util alert list-contact-points`."
    )]
    ListAlertContactPoints(AlertListArgs),
    #[command(
        name = "list-alert-mute-timings",
        about = "Compatibility direct form; prefer `grafana-util alert list-mute-timings`."
    )]
    ListAlertMuteTimings(AlertListArgs),
    #[command(
        name = "list-alert-templates",
        about = "Compatibility direct form; prefer `grafana-util alert list-templates`."
    )]
    ListAlertTemplates(AlertListArgs),
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
    }
}

// Centralized command fan-out before invoking domain runners.
// Every unified CLI variant is normalized into one of dashboard/alert/datasource/access runners here.
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

/// Runtime entrypoint for unified execution.
///
/// Keeping handler execution injectable via `dispatch_with_handlers` allows tests to
/// validate dispatch logic without touching network transport.
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
