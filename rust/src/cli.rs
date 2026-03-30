//! Unified CLI dispatcher for Rust entrypoints.
//!
//! Purpose:
//! - Own only command topology and domain dispatch.
//! - Keep `grafana-util` command surface in one place.
//! - Route to domain runners (`dashboard`, `alert`, `access`, `datasource`, `overview`, `status`) without
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
use clap::{ColorChoice, CommandFactory, Parser, Subcommand};

use crate::access::{root_command as access_root_command, run_access_cli, AccessCliArgs};
use crate::alert::{
    normalize_alert_namespace_args, root_command as alert_root_command, run_alert_cli,
    AlertCliArgs, AlertNamespaceArgs,
};
use crate::cli_help_examples::{
    colorize_help_examples, inject_help_full_hint, ACCESS_HELP_FULL_TEXT, ALERT_HELP_FULL_TEXT,
    DATASOURCE_HELP_FULL_TEXT, OVERVIEW_HELP_FULL_TEXT, PROJECT_STATUS_HELP_FULL_TEXT,
    SYNC_HELP_FULL_TEXT, UNIFIED_HELP_FULL_TEXT, UNIFIED_HELP_TEXT,
};
use crate::common::Result;
use crate::dashboard::{
    run_dashboard_cli, BrowseArgs, DashboardCliArgs, DashboardCommand, DeleteArgs, DiffArgs,
    ExportArgs, GovernanceGateArgs, ImportArgs, InspectExportArgs, InspectLiveArgs,
    InspectVarsArgs, ListArgs, ScreenshotArgs, TopologyArgs,
};
use crate::datasource::{run_datasource_cli, DatasourceCliArgs, DatasourceGroupCommand};
use crate::overview::{run_overview_cli, OverviewCliArgs};
use crate::project_status_command::{run_project_status_cli, ProjectStatusCliArgs};
use crate::sync::{run_sync_cli, SyncCliArgs, SyncGroupCommand};

const UNIFIED_DASHBOARD_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard browse --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\"\n  grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./dashboards --overwrite\n  grafana-util dashboard diff --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards/raw";
const UNIFIED_DATASOURCE_HELP_TEXT: &str = "Examples:\n\n  grafana-util datasource browse --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\"\n  grafana-util datasource list --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --json\n  grafana-util datasource import --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --import-dir ./datasources --dry-run --json";
const UNIFIED_SYNC_HELP_TEXT: &str = "Examples:\n\n  grafana-util change summary --desired-file ./desired.json\n  grafana-util change plan --desired-file ./desired.json --fetch-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\"\n  grafana-util change apply --plan-file ./sync-plan-reviewed.json --approve --execute-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\"";
const UNIFIED_ALERT_HELP_TEXT: &str = "Examples:\n\n  grafana-util alert export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-dir ./alerts --overwrite\n  grafana-util alert import --url http://localhost:3000 --import-dir ./alerts/raw --replace-existing --dry-run --json\n  grafana-util alert list-rules --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --json";
const UNIFIED_ACCESS_HELP_TEXT: &str = "Examples:\n\n  grafana-util access user list --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --json\n  grafana-util access team import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./access-teams --replace-existing --yes\n  grafana-util access service-account token add --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --name deploy-bot --token-name nightly";
const DASHBOARD_BROWSE_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard browse --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\"\n  grafana-util dashboard browse --url http://localhost:3000 --basic-user admin --basic-password admin --path 'Platform / Infra'\n  grafana-util dashboard browse --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs";
const DASHBOARD_LIST_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard list --url http://localhost:3000 --basic-user admin --basic-password admin\n  grafana-util dashboard list --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --json\n  grafana-util dashboard list --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --json";
const DASHBOARD_EXPORT_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./dashboards --overwrite\n  grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --export-dir ./dashboards --overwrite\n  grafana-util dashboard export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --export-dir ./dashboards --overwrite";
const DASHBOARD_IMPORT_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards/raw --replace-existing\n  grafana-util dashboard import --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --import-dir ./dashboards/raw --dry-run --table\n  grafana-util dashboard import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards/raw --interactive --replace-existing";
const DASHBOARD_DELETE_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard delete --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --uid cpu-main --dry-run --json\n  grafana-util dashboard delete --url http://localhost:3000 --basic-user admin --basic-password admin --path 'Platform / Infra' --yes\n  grafana-util dashboard delete --url http://localhost:3000 --interactive";
const DASHBOARD_DIFF_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard diff --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards/raw\n  grafana-util dashboard diff --url http://localhost:3000 --basic-user admin --basic-password admin --org-id 2 --import-dir ./dashboards/raw --json";
const DASHBOARD_INSPECT_EXPORT_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard inspect-export --import-dir ./dashboards/raw --table\n  grafana-util dashboard inspect-export --import-dir ./dashboards/raw --interactive\n  grafana-util dashboard inspect-export --import-dir ./dashboards/raw --report governance-json";
const DASHBOARD_INSPECT_LIVE_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard inspect-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format governance-json\n  grafana-util dashboard inspect-live --url http://localhost:3000 --basic-user admin --basic-password admin --interactive";
const DASHBOARD_INSPECT_VARS_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard inspect-vars --dashboard-url 'https://grafana.example.com/d/cpu-main/cpu-overview?var-cluster=prod-a' --token \"$GRAFANA_API_TOKEN\" --output-format table\n  grafana-util dashboard inspect-vars --url https://grafana.example.com --dashboard-uid cpu-main --vars-query 'var-cluster=prod-a&var-instance=node01' --token \"$GRAFANA_API_TOKEN\" --output-format json";
const DASHBOARD_GOVERNANCE_GATE_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard governance-gate --policy-source file --policy ./policy.yaml --governance ./governance.json --queries ./queries.json\n  grafana-util dashboard governance-gate --policy-source builtin --builtin-policy default --governance ./governance.json --queries ./queries.json --output-format json --json-output ./governance-check.json";
const DASHBOARD_TOPOLOGY_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard topology --governance ./governance.json --queries ./queries.json --alert-contract ./alert-contract.json --output-format mermaid\n  grafana-util dashboard graph --governance ./governance.json --queries ./queries.json --alert-contract ./alert-contract.json --output-format dot --output-file ./dashboard-topology.dot";
const DASHBOARD_SCREENSHOT_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard screenshot --dashboard-url 'https://grafana.example.com/d/cpu-main/cpu-overview?var-cluster=prod-a' --token \"$GRAFANA_API_TOKEN\" --output ./cpu-main.png --full-page --header-title --header-url --header-captured-at\n  grafana-util dashboard screenshot --url https://grafana.example.com --dashboard-uid rYdddlPWk --panel-id 20 --vars-query 'var-datasource=prom-main&var-job=node-exporter&var-node=host01:9100' --token \"$GRAFANA_API_TOKEN\" --output ./panel.png --header-title 'CPU Busy' --header-text 'Solo panel debug capture'";

fn render_long_help_with_color_choice(command: &mut clap::Command, colorize: bool) -> String {
    let configured = std::mem::take(command).color(if colorize {
        ColorChoice::Always
    } else {
        ColorChoice::Never
    });
    *command = configured;
    let rendered = command.render_long_help();
    if colorize {
        rendered.ansi().to_string()
    } else {
        rendered.to_string()
    }
}

/// Render unified help text and apply compact "full examples" markers.
///
/// This keeps default help stable while allowing operators to discover the
/// extended example section when needed.
pub fn render_unified_help_text(colorize: bool) -> String {
    let mut command = CliArgs::command();
    let help = inject_help_full_hint(render_long_help_with_color_choice(&mut command, colorize));
    let mut help = if colorize {
        help.replace(
            UNIFIED_HELP_TEXT,
            &colorize_help_examples(UNIFIED_HELP_TEXT),
        )
    } else {
        help
    };
    help.push_str(OVERVIEW_HELP_SHAPE_NOTE);
    help
}

fn render_domain_help_text(mut command: clap::Command, colorize: bool) -> String {
    inject_help_full_hint(render_long_help_with_color_choice(&mut command, colorize))
}

fn render_domain_help_full_text(
    mut command: clap::Command,
    extended_examples: &str,
    colorize: bool,
) -> String {
    let mut help = render_long_help_with_color_choice(&mut command, colorize);
    if colorize {
        help.push_str(&colorize_help_examples(extended_examples));
    } else {
        help.push_str(extended_examples);
    }
    help
}

const OVERVIEW_HELP_SHAPE_NOTE: &str =
    "\nStaged overview is the default. Use `grafana-util overview live` to route into shared live status.\n";

fn render_overview_help_text(colorize: bool) -> String {
    let mut help = render_domain_help_text(OverviewCliArgs::command(), colorize);
    help.push_str(OVERVIEW_HELP_SHAPE_NOTE);
    help
}

fn render_overview_help_full_text(colorize: bool) -> String {
    let mut help = render_domain_help_full_text(
        OverviewCliArgs::command(),
        OVERVIEW_HELP_FULL_TEXT,
        colorize,
    );
    help.push_str(OVERVIEW_HELP_SHAPE_NOTE);
    help
}

/// Render the unified help text with the longer `--help-full` example block.
pub fn render_unified_help_full_text(colorize: bool) -> String {
    let mut help = render_unified_help_text(colorize);
    if colorize {
        help.push_str(&colorize_help_examples(UNIFIED_HELP_FULL_TEXT));
    } else {
        help.push_str(UNIFIED_HELP_FULL_TEXT);
    }
    help
}

/// maybe render unified help from os args.
pub fn maybe_render_unified_help_from_os_args<I, T>(iter: I, colorize: bool) -> Option<String>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    // Fast path for `-h/--help` and `--help-full` before command parsing.
    // This avoids constructing a full `CliArgs` value for top-level help usage.
    let args = iter
        .into_iter()
        .map(|value| value.into().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    match args.as_slice() {
        [_binary] => Some(render_unified_help_text(colorize)),
        [_binary, flag] if flag == "--help" || flag == "-h" => {
            Some(render_unified_help_text(colorize))
        }
        [_binary, flag] if flag == "--help-full" => Some(render_unified_help_full_text(colorize)),
        [_binary, command, flag] if command == "alert" && (flag == "--help" || flag == "-h") => {
            Some(render_domain_help_text(alert_root_command(), colorize))
        }
        [_binary, command, flag]
            if command == "datasource" && (flag == "--help" || flag == "-h") =>
        {
            Some(render_domain_help_text(
                DatasourceCliArgs::command(),
                colorize,
            ))
        }
        [_binary, command, flag] if command == "access" && (flag == "--help" || flag == "-h") => {
            Some(render_domain_help_text(access_root_command(), colorize))
        }
        [_binary, command, flag] if command == "overview" && (flag == "--help" || flag == "-h") => {
            Some(render_overview_help_text(colorize))
        }
        [_binary, command, flag] if command == "status" && (flag == "--help" || flag == "-h") => {
            Some(render_domain_help_text(
                ProjectStatusCliArgs::command(),
                colorize,
            ))
        }
        [_binary, command, flag] if command == "change" && (flag == "--help" || flag == "-h") => {
            Some(render_domain_help_text(SyncCliArgs::command(), colorize))
        }
        [_binary, command, flag] if command == "alert" && flag == "--help-full" => Some(
            render_domain_help_full_text(alert_root_command(), ALERT_HELP_FULL_TEXT, colorize),
        ),
        [_binary, command, flag] if command == "datasource" && flag == "--help-full" => {
            Some(render_domain_help_full_text(
                DatasourceCliArgs::command(),
                DATASOURCE_HELP_FULL_TEXT,
                colorize,
            ))
        }
        [_binary, command, flag] if command == "access" && flag == "--help-full" => Some(
            render_domain_help_full_text(access_root_command(), ACCESS_HELP_FULL_TEXT, colorize),
        ),
        [_binary, command, flag] if command == "overview" && flag == "--help-full" => {
            Some(render_overview_help_full_text(colorize))
        }
        [_binary, command, flag] if command == "status" && flag == "--help-full" => {
            Some(render_domain_help_full_text(
                ProjectStatusCliArgs::command(),
                PROJECT_STATUS_HELP_FULL_TEXT,
                colorize,
            ))
        }
        [_binary, command, flag] if command == "change" && flag == "--help-full" => Some(
            render_domain_help_full_text(SyncCliArgs::command(), SYNC_HELP_FULL_TEXT, colorize),
        ),
        _ => None,
    }
}

/// Dashboard subcommands exposed through the unified root CLI.
#[derive(Debug, Clone, Subcommand)]
pub enum DashboardGroupCommand {
    #[command(
        about = "Browse the live dashboard tree in an interactive terminal UI.",
        after_help = DASHBOARD_BROWSE_HELP_TEXT
    )]
    Browse(BrowseArgs),
    #[command(
        about = "List dashboard summaries without writing export files.",
        after_help = DASHBOARD_LIST_HELP_TEXT
    )]
    List(ListArgs),
    #[command(
        about = "Export dashboards to raw/ and prompt/ JSON files.",
        after_help = DASHBOARD_EXPORT_HELP_TEXT
    )]
    Export(ExportArgs),
    #[command(
        about = "Import dashboard JSON files through the Grafana API.",
        after_help = DASHBOARD_IMPORT_HELP_TEXT
    )]
    Import(ImportArgs),
    #[command(
        about = "Delete live dashboards by UID or folder path.",
        after_help = DASHBOARD_DELETE_HELP_TEXT
    )]
    Delete(DeleteArgs),
    #[command(
        about = "Compare local raw dashboard files against live Grafana dashboards.",
        after_help = DASHBOARD_DIFF_HELP_TEXT
    )]
    Diff(DiffArgs),
    #[command(
        about = "Analyze a raw dashboard export directory and summarize its structure.",
        after_help = DASHBOARD_INSPECT_EXPORT_HELP_TEXT
    )]
    InspectExport(InspectExportArgs),
    #[command(
        about = "Analyze live Grafana dashboards without writing a persistent export.",
        after_help = DASHBOARD_INSPECT_LIVE_HELP_TEXT
    )]
    InspectLive(InspectLiveArgs),
    #[command(
        about = "List dashboard templating variables from live Grafana.",
        after_help = DASHBOARD_INSPECT_VARS_HELP_TEXT
    )]
    InspectVars(InspectVarsArgs),
    #[command(
        about = "Evaluate governance policy against dashboard inspect JSON artifacts.",
        after_help = DASHBOARD_GOVERNANCE_GATE_HELP_TEXT
    )]
    GovernanceGate(GovernanceGateArgs),
    #[command(
        name = "topology",
        visible_alias = "graph",
        about = "Build a deterministic dashboard topology graph from JSON artifacts.",
        after_help = DASHBOARD_TOPOLOGY_HELP_TEXT
    )]
    Topology(TopologyArgs),
    #[command(
        about = "Open one dashboard in a headless browser and capture image or PDF output.",
        after_help = DASHBOARD_SCREENSHOT_HELP_TEXT
    )]
    Screenshot(ScreenshotArgs),
}

/// Namespaced root commands handled by the Rust `grafana-util` binary.
#[derive(Debug, Clone, Subcommand)]
pub enum UnifiedCommand {
    #[command(
        about = "Run dashboard export, list, import, and diff workflows.",
        visible_alias = "db",
        after_help = UNIFIED_DASHBOARD_HELP_TEXT
    )]
    Dashboard {
        #[command(subcommand)]
        command: DashboardGroupCommand,
    },
    #[command(
        about = "Run datasource browse, list, export, import, and diff workflows.",
        visible_alias = "ds",
        after_help = UNIFIED_DATASOURCE_HELP_TEXT
    )]
    Datasource {
        #[command(subcommand)]
        command: DatasourceGroupCommand,
    },
    #[command(
        name = "change",
        about = "Run review-first change workflows with optional live Grafana fetch/apply paths.",
        after_help = UNIFIED_SYNC_HELP_TEXT
    )]
    Change {
        #[command(subcommand)]
        command: SyncGroupCommand,
    },
    #[command(
        about = "Export, import, or diff Grafana alerting resources.",
        after_help = UNIFIED_ALERT_HELP_TEXT
    )]
    Alert(AlertNamespaceArgs),
    #[command(
        about = "List and manage Grafana users, teams, and service accounts.",
        after_help = UNIFIED_ACCESS_HELP_TEXT
    )]
    Access(AccessCliArgs),
    #[command(
        about = "Summarize project artifacts into a project-wide overview. Staged exports are the default; use `overview live` to route into shared live status."
    )]
    Overview(OverviewCliArgs),
    #[command(
        name = "status",
        about = "Render shared project-wide staged or live status. Staged subcommands use exported artifacts; live subcommands query Grafana."
    )]
    Status(ProjectStatusCliArgs),
}

#[derive(Debug, Clone, Parser)]
#[command(
    name = "grafana-util",
    about = "Unified Grafana dashboard, alerting, and access utility.",
    after_help = UNIFIED_HELP_TEXT,
    styles = crate::help_styles::CLI_HELP_STYLES
)]
/// Parsed root CLI arguments for the Rust unified binary.
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
    // Keep parser invocation in one place so runtime entrypoints all share identical
    // argument normalization and Clap error handling.
    CliArgs::parse_from(iter)
}

fn wrap_dashboard(command: DashboardCommand) -> DashboardCliArgs {
    DashboardCliArgs { command }
}

fn wrap_dashboard_group(command: DashboardGroupCommand) -> DashboardCliArgs {
    match command {
        DashboardGroupCommand::Browse(inner) => wrap_dashboard(DashboardCommand::Browse(inner)),
        DashboardGroupCommand::List(inner) => wrap_dashboard(DashboardCommand::List(inner)),
        DashboardGroupCommand::Export(inner) => wrap_dashboard(DashboardCommand::Export(inner)),
        DashboardGroupCommand::Import(inner) => wrap_dashboard(DashboardCommand::Import(inner)),
        DashboardGroupCommand::Delete(inner) => wrap_dashboard(DashboardCommand::Delete(inner)),
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
        DashboardGroupCommand::GovernanceGate(inner) => {
            wrap_dashboard(DashboardCommand::GovernanceGate(inner))
        }
        DashboardGroupCommand::Topology(inner) => wrap_dashboard(DashboardCommand::Topology(inner)),
        DashboardGroupCommand::Screenshot(inner) => {
            wrap_dashboard(DashboardCommand::Screenshot(inner))
        }
    }
}

// Centralized command fan-out before invoking domain runners.
// Every unified CLI variant is normalized into one of dashboard/alert/datasource/access/overview/status runners here.
/// Dispatch the normalized root command into exactly one domain handler.
///
/// Handlers are injected as callables so tests can assert routing without
/// triggering network-heavy domain execution.
fn dispatch_with_handlers<FD, FS, FY, FA, FX, FO, FP>(
    args: CliArgs,
    mut run_dashboard: FD,
    mut run_datasource: FS,
    mut run_sync: FY,
    mut run_alert: FA,
    mut run_access: FX,
    mut run_overview: FO,
    mut run_project_status: FP,
) -> Result<()>
where
    FD: FnMut(DashboardCliArgs) -> Result<()>,
    FS: FnMut(DatasourceGroupCommand) -> Result<()>,
    FY: FnMut(SyncGroupCommand) -> Result<()>,
    FA: FnMut(AlertCliArgs) -> Result<()>,
    FX: FnMut(AccessCliArgs) -> Result<()>,
    FO: FnMut(OverviewCliArgs) -> Result<()>,
    FP: FnMut(ProjectStatusCliArgs) -> Result<()>,
{
    match args.command {
        UnifiedCommand::Dashboard { command } => run_dashboard(wrap_dashboard_group(command)),
        UnifiedCommand::Datasource { command } => run_datasource(command),
        UnifiedCommand::Change { command } => run_sync(command),
        UnifiedCommand::Alert(inner) => run_alert(normalize_alert_namespace_args(inner)),
        UnifiedCommand::Access(inner) => run_access(inner),
        UnifiedCommand::Overview(inner) => run_overview(inner),
        UnifiedCommand::Status(inner) => run_project_status(inner),
    }
}

/// Runtime entrypoint for unified execution.
///
/// Keeping handler execution injectable via `dispatch_with_handlers` allows tests to
/// validate dispatch logic without touching network transport.
pub fn run_cli(args: CliArgs) -> Result<()> {
    // Keep one executable boundary: parse-independent dispatch + injected runners.
    dispatch_with_handlers(
        args,
        run_dashboard_cli,
        run_datasource_cli,
        run_sync_cli,
        run_alert_cli,
        run_access_cli,
        run_overview_cli,
        run_project_status_cli,
    )
}

#[cfg(test)]
#[path = "cli_rust_tests.rs"]
mod cli_rust_tests;
