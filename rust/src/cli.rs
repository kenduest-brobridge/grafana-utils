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
use clap::{CommandFactory, Parser, Subcommand};

use crate::access::{root_command as access_root_command, run_access_cli, AccessCliArgs};
use crate::alert::{
    normalize_alert_namespace_args, root_command as alert_root_command, run_alert_cli,
    AlertCliArgs, AlertNamespaceArgs,
};
use crate::common::Result;
use crate::dashboard::{
    run_dashboard_cli, DashboardCliArgs, DashboardCommand, DiffArgs, ExportArgs, ImportArgs,
    InspectExportArgs, InspectLiveArgs, InspectVarsArgs, ListArgs, ListDataSourcesArgs,
    ScreenshotArgs,
};
use crate::datasource::{run_datasource_cli, DatasourceCliArgs, DatasourceGroupCommand};
use crate::sync::{run_sync_cli, SyncCliArgs, SyncGroupCommand};

const UNIFIED_HELP_TEXT: &str = "Examples:\n\n  [Dashboard Export] Export dashboards with Basic auth:\n    grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./dashboards --overwrite\n\n  [Dashboard Export] Export dashboards across all visible orgs:\n    grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --export-dir ./dashboards --overwrite\n\n  [Dashboard Capture] Capture a dashboard screenshot from browser-like state:\n    grafana-util dashboard screenshot --dashboard-url 'https://grafana.example.com/d/cpu-main/cpu-overview?var-cluster=prod-a' --token \"$GRAFANA_API_TOKEN\" --output ./cpu-main.png --full-page\n\n  [Dashboard Capture] Inspect dashboard variables before capture:\n    grafana-util dashboard inspect-vars --dashboard-url 'https://grafana.example.com/d/cpu-main/cpu-overview?var-cluster=prod-a' --token \"$GRAFANA_API_TOKEN\"\n\n  [Alert Export] Export alerting resources through the unified binary:\n    grafana-util alert export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-dir ./alerts --overwrite\n\n  [Datasource Inventory] List datasource inventory through the unified binary:\n    grafana-util datasource list --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --json\n\n  [Access Inventory] List org users through the unified binary:\n    grafana-util access user list --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --json\n\n  [Sync Planning] Build a sync plan directly from live Grafana state:\n    grafana-util sync plan --desired-file ./desired.json --fetch-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\"\n\n  [Sync Apply] Apply a reviewed sync plan back to Grafana:\n    grafana-util sync apply --plan-file ./sync-plan-reviewed.json --approve --execute-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\"";
const UNIFIED_HELP_FULL_TEXT: &str = "\nExtended Examples:\n\n  [Dashboard Inspect Export] Render a grouped dashboard dependency table from raw exports:\n    grafana-util dashboard inspect-export --import-dir ./dashboards/raw --output-format report-tree-table --report-columns dashboard_uid,panel_title,datasource_uid,query\n\n  [Dashboard Inspect Live] Render datasource governance JSON directly from live Grafana:\n    grafana-util dashboard inspect-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format governance-json\n\n  [Datasource Import] Dry-run a datasource import and keep the result machine-readable:\n    grafana-util datasource import --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --import-dir ./datasources --dry-run --json\n\n  [Access Team Import] Preview a destructive team sync before confirming:\n    grafana-util access team import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./access-teams --replace-existing --dry-run --output-format table\n\n  [Alert Import] Re-map linked alert dashboards during import:\n    grafana-util alert import --url http://localhost:3000 --import-dir ./alerts/raw --replace-existing --dashboard-uid-map ./dashboard-map.json --panel-id-map ./panel-map.json\n\n  [Sync Review] Stamp a plan as reviewed before apply:\n    grafana-util sync review --plan-file ./sync-plan.json --review-note 'peer-reviewed' --output json\n";
const ALERT_HELP_FULL_TEXT: &str = "\nExtended Examples:\n\n  [Alert Export] Export alerting resources with overwrite enabled:\n    grafana-util alert export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-dir ./alerts --overwrite\n\n  [Alert Import] Preview a replace-existing import before execution:\n    grafana-util alert import --url http://localhost:3000 --import-dir ./alerts/raw --replace-existing --dry-run\n\n  [Alert Import] Re-map linked dashboards and panels during import:\n    grafana-util alert import --url http://localhost:3000 --import-dir ./alerts/raw --replace-existing --dashboard-uid-map ./dashboard-map.json --panel-id-map ./panel-map.json\n\n  [Alert List] Render live alert rules as JSON:\n    grafana-util alert list-rules --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --json\n";
const DATASOURCE_HELP_FULL_TEXT: &str = "\nExtended Examples:\n\n  [Datasource List] Enumerate all visible org datasources as CSV:\n    grafana-util datasource list --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --output-format csv\n\n  [Datasource Add] Preview a new datasource contract as JSON:\n    grafana-util datasource add --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --name prometheus-main --type prometheus --datasource-url http://prometheus:9090 --dry-run --json\n\n  [Datasource Import] Import one exported org bundle with create-missing-orgs:\n    grafana-util datasource import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./datasources --use-export-org --only-org-id 2 --create-missing-orgs --dry-run --json\n\n  [Datasource Diff] Compare a local export directory with live Grafana:\n    grafana-util datasource diff --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --diff-dir ./datasources\n";
const ACCESS_HELP_FULL_TEXT: &str = "\nExtended Examples:\n\n  [Access User Diff] Compare exported users against the Grafana global scope:\n    grafana-util access user diff --url http://localhost:3000 --basic-user admin --basic-password admin --diff-dir ./access-users --scope global\n\n  [Access Team Import] Preview a destructive team sync as a table:\n    grafana-util access team import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./access-teams --replace-existing --dry-run --output-format table\n\n  [Access Org Delete] Delete one org by explicit org id:\n    grafana-util access org delete --url http://localhost:3000 --basic-user admin --basic-password admin --org-id 7 --yes --json\n\n  [Access Token Add] Issue a short-lived service-account token:\n    grafana-util access service-account token add --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --service-account-id 7 --token-name nightly --seconds-to-live 3600\n";
const SYNC_HELP_FULL_TEXT: &str = "\nExtended Examples:\n\n  [Sync Summary] Render the desired resource summary as JSON:\n    grafana-util sync summary --desired-file ./desired.json --output json\n\n  [Sync Plan] Build a live-backed plan with prune candidates:\n    grafana-util sync plan --desired-file ./desired.json --fetch-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --allow-prune --output json\n\n  [Sync Review] Stamp a reviewed plan with reviewer metadata:\n    grafana-util sync review --plan-file ./sync-plan.json --review-note 'peer-reviewed' --reviewed-by ops-user --output json\n\n  [Sync Apply] Emit a reviewed local apply intent:\n    grafana-util sync apply --plan-file ./sync-plan-reviewed.json --approve\n";

const HELP_COLOR_RESET: &str = "\x1b[0m";
const HELP_COLOR_DASHBOARD: &str = "\x1b[1;36m";
const HELP_COLOR_ALERT: &str = "\x1b[1;31m";
const HELP_COLOR_DATASOURCE: &str = "\x1b[1;32m";
const HELP_COLOR_ACCESS: &str = "\x1b[1;33m";
const HELP_COLOR_SYNC: &str = "\x1b[1;34m";

const UNIFIED_HELP_LABELS: [(&str, &str); 21] = [
    ("[Dashboard Export]", HELP_COLOR_DASHBOARD),
    ("[Dashboard Capture]", HELP_COLOR_DASHBOARD),
    ("[Dashboard Inspect Export]", HELP_COLOR_DASHBOARD),
    ("[Dashboard Inspect Live]", HELP_COLOR_DASHBOARD),
    ("[Alert Export]", HELP_COLOR_ALERT),
    ("[Alert Import]", HELP_COLOR_ALERT),
    ("[Alert List]", HELP_COLOR_ALERT),
    ("[Datasource Inventory]", HELP_COLOR_DATASOURCE),
    ("[Datasource List]", HELP_COLOR_DATASOURCE),
    ("[Datasource Add]", HELP_COLOR_DATASOURCE),
    ("[Datasource Import]", HELP_COLOR_DATASOURCE),
    ("[Datasource Diff]", HELP_COLOR_DATASOURCE),
    ("[Access Inventory]", HELP_COLOR_ACCESS),
    ("[Access User Diff]", HELP_COLOR_ACCESS),
    ("[Access Team Import]", HELP_COLOR_ACCESS),
    ("[Access Org Delete]", HELP_COLOR_ACCESS),
    ("[Access Token Add]", HELP_COLOR_ACCESS),
    ("[Sync Planning]", HELP_COLOR_SYNC),
    ("[Sync Summary]", HELP_COLOR_SYNC),
    ("[Sync Plan]", HELP_COLOR_SYNC),
    ("[Sync Review]", HELP_COLOR_SYNC),
];

fn colorize_unified_help_examples(text: &str) -> String {
    let mut colored = text.to_string();
    for (label, color) in UNIFIED_HELP_LABELS {
        let colored_label = format!("{color}{label}{HELP_COLOR_RESET}");
        colored = colored.replace(label, &colored_label);
    }
    colored.replace(
        "[Sync Apply]",
        &format!("{HELP_COLOR_SYNC}[Sync Apply]{HELP_COLOR_RESET}"),
    )
}

fn render_long_help(command: &mut clap::Command) -> String {
    let mut output = Vec::new();
    command.write_long_help(&mut output).unwrap();
    String::from_utf8(output).unwrap()
}

pub fn render_unified_help_text(colorize: bool) -> String {
    let mut command = CliArgs::command();
    let help = render_long_help(&mut command);
    if colorize {
        help.replace(
            UNIFIED_HELP_TEXT,
            &colorize_unified_help_examples(UNIFIED_HELP_TEXT),
        )
    } else {
        help
    }
}

fn render_domain_help_full_text(
    mut command: clap::Command,
    extended_examples: &str,
    colorize: bool,
) -> String {
    let mut help = render_long_help(&mut command);
    if colorize {
        help.push_str(&colorize_unified_help_examples(extended_examples));
    } else {
        help.push_str(extended_examples);
    }
    help
}

pub fn render_unified_help_full_text(colorize: bool) -> String {
    let mut help = render_unified_help_text(colorize);
    if colorize {
        help.push_str(&colorize_unified_help_examples(UNIFIED_HELP_FULL_TEXT));
    } else {
        help.push_str(UNIFIED_HELP_FULL_TEXT);
    }
    help
}

pub fn maybe_render_unified_help_from_os_args<I, T>(iter: I, colorize: bool) -> Option<String>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let args = iter
        .into_iter()
        .map(|value| value.into().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    match args.as_slice() {
        [_binary, flag] if flag == "--help" || flag == "-h" => {
            Some(render_unified_help_text(colorize))
        }
        [_binary, flag] if flag == "--help-full" => Some(render_unified_help_full_text(colorize)),
        [_binary, command, flag] if command == "alert" && flag == "--help-full" => Some(
            render_domain_help_full_text(alert_root_command(), ALERT_HELP_FULL_TEXT, colorize),
        ),
        [_binary, command, flag] if command == "datasource" && flag == "--help-full" => Some(
            render_domain_help_full_text(
                DatasourceCliArgs::command(),
                DATASOURCE_HELP_FULL_TEXT,
                colorize,
            ),
        ),
        [_binary, command, flag] if command == "access" && flag == "--help-full" => Some(
            render_domain_help_full_text(access_root_command(), ACCESS_HELP_FULL_TEXT, colorize),
        ),
        [_binary, command, flag] if command == "sync" && flag == "--help-full" => Some(
            render_domain_help_full_text(SyncCliArgs::command(), SYNC_HELP_FULL_TEXT, colorize),
        ),
        _ => None,
    }
}

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
        about = "Run staged sync planning workflows with optional live Grafana fetch/apply paths.",
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
