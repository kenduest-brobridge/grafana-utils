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
    InspectExportArgs, InspectLiveArgs, ListArgs,
};
use crate::datasource::{run_datasource_cli, DatasourceGroupCommand};
use crate::sync::{run_sync_cli, SyncGroupCommand};

const UNIFIED_HELP_TEXT: &str = "Examples:\n\n  Export dashboards across all visible orgs with Basic auth:\n    grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --export-dir ./dashboards --overwrite\n\n  Preview a routed dashboard import before writing:\n    grafana-util dashboard import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards --use-export-org --create-missing-orgs --dry-run --output-format table\n\n  Inspect exported dashboards as a query tree:\n    grafana-util dashboard inspect-export --import-dir ./dashboards/raw --view query --layout tree --format table\n\n  Export alerting resources from the current org with an API token:\n    grafana-util alert export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-dir ./alerts --overwrite\n\n  List Grafana organizations with memberships:\n    grafana-util access org list --url http://localhost:3000 --basic-user admin --basic-password admin --with-users --table\n\n  List current-org teams with member details:\n    grafana-util access team list --url http://localhost:3000 --basic-user admin --basic-password admin --with-members --table\n\n  Build a local staged sync preflight document:\n    grafana-util sync preflight --desired-file ./desired.json --availability-file ./availability.json";
const DASHBOARD_LIST_HELP: &str =
    "Examples:\n\n  Table output:\n    grafana-util dashboard list --url http://localhost:3000 --table\n\n  JSON output for one folder:\n    grafana-util dashboard list --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --folder Infra --output-format json";
const DASHBOARD_EXPORT_HELP: &str =
    "Examples:\n\n  Export dashboards with Basic auth:\n    grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./dashboards --overwrite\n\n  Export into a flat directory layout:\n    grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./dashboards --flat";
const DASHBOARD_IMPORT_HELP: &str =
    "Examples:\n\n  Preview a dashboard import:\n    grafana-util dashboard import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards/raw --replace-existing --dry-run --output-format table\n\n  Replay a multi-org export into matching orgs:\n    grafana-util dashboard import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards --use-export-org --create-missing-orgs";
const DASHBOARD_DIFF_HELP: &str =
    "Examples:\n\n  Diff raw dashboard exports:\n    grafana-util dashboard diff --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --import-dir ./dashboards/raw";
const DASHBOARD_INSPECT_EXPORT_HELP: &str =
    "Examples:\n\n  Render a query tree report from exported dashboards:\n    grafana-util dashboard inspect-export --import-dir ./dashboards/raw --view query --layout tree --format table\n\n  Render a datasource report as JSON:\n    grafana-util dashboard inspect-export --import-dir ./dashboards/raw --view datasource --format json";
const DASHBOARD_INSPECT_LIVE_HELP: &str =
    "Examples:\n\n  Inspect live dashboards as a query tree:\n    grafana-util dashboard inspect-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --view query --layout tree --format text\n\n  Render a live governance report:\n    grafana-util dashboard inspect-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --view governance --format table";

#[derive(Debug, Clone, Subcommand)]
pub enum DashboardGroupCommand {
    #[command(
        about = "List dashboard summaries without writing export files.",
        after_help = DASHBOARD_LIST_HELP
    )]
    List(ListArgs),
    #[command(
        about = "Export dashboards to raw/ and prompt/ JSON files.",
        after_help = DASHBOARD_EXPORT_HELP
    )]
    Export(ExportArgs),
    #[command(
        about = "Import dashboard JSON files through the Grafana API.",
        after_help = DASHBOARD_IMPORT_HELP
    )]
    Import(ImportArgs),
    #[command(
        about = "Compare local raw dashboard files against live Grafana dashboards.",
        after_help = DASHBOARD_DIFF_HELP
    )]
    Diff(DiffArgs),
    #[command(
        about = "Analyze a raw dashboard export directory and summarize its structure.",
        after_help = DASHBOARD_INSPECT_EXPORT_HELP
    )]
    InspectExport(InspectExportArgs),
    #[command(
        about = "Analyze live Grafana dashboards without writing a persistent export.",
        after_help = DASHBOARD_INSPECT_LIVE_HELP
    )]
    InspectLive(InspectLiveArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum UnifiedCommand {
    #[command(
        about = "Dashboard [list|export|import|diff|inspect-export|inspect-live].",
        alias = "db"
    )]
    Dashboard {
        #[command(subcommand)]
        command: DashboardGroupCommand,
    },
    #[command(
        about = "Datasource [list|add|modify|delete|export|import|diff].",
        alias = "ds"
    )]
    Datasource {
        #[command(subcommand)]
        command: DatasourceGroupCommand,
    },
    #[command(
        about = "Sync [summary|preflight|plan|review|assess-alerts|bundle-preflight|apply].",
        alias = "sy"
    )]
    Sync {
        #[command(subcommand)]
        command: SyncGroupCommand,
    },
    #[command(
        about = "Alert [export|import|diff|list-rules|list-contact-points|list-mute-timings|list-templates].",
        alias = "al"
    )]
    Alert(AlertNamespaceArgs),
    #[command(about = "Access [user|team|org|service-account].", alias = "ac")]
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
    let mut argv = iter
        .into_iter()
        .map(|value| value.into())
        .collect::<Vec<std::ffi::OsString>>();
    // Keep compatibility for the removed dashboard alias and route callers onto
    // the canonical datasource namespace.
    if argv.len() > 2 {
        let command = argv[1].to_string_lossy();
        let subcommand = argv[2].to_string_lossy();
        if (command == "dashboard" || command == "db") && subcommand == "list-data-sources" {
            argv[1] = std::ffi::OsString::from("datasource");
            argv[2] = std::ffi::OsString::from("list");
        }
    }
    CliArgs::parse_from(argv)
}

fn wrap_dashboard(command: DashboardCommand) -> DashboardCliArgs {
    DashboardCliArgs { command }
}

fn wrap_dashboard_group(command: DashboardGroupCommand) -> DashboardCliArgs {
    match command {
        DashboardGroupCommand::List(inner) => wrap_dashboard(DashboardCommand::List(inner)),
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
