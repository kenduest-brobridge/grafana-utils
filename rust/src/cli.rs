//! Unified CLI dispatcher for Rust entrypoints.
//!
//! Purpose:
//! - Own only command topology and domain dispatch.
//! - Keep `grafana-util` command surface in one place.
//! - Route to domain runners (`dashboard`, `alert`, `access`, `datasource`, `snapshot`, `overview`, `status`) without
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
use clap::{Args, Parser, Subcommand};

use crate::access::{run_access_cli, AccessCliArgs};
use crate::alert::{
    normalize_alert_namespace_args, run_alert_cli, AlertCliArgs, AlertNamespaceArgs,
};
pub use crate::cli_help::{
    maybe_render_unified_help_from_os_args, render_unified_help_full_text,
    render_unified_help_text, render_unified_version_text,
};
use crate::cli_help::{
    DASHBOARD_ANALYZE_HELP_TEXT, DASHBOARD_BROWSE_HELP_TEXT, DASHBOARD_CLONE_LIVE_HELP_TEXT,
    DASHBOARD_DELETE_HELP_TEXT, DASHBOARD_DIFF_HELP_TEXT, DASHBOARD_EXPORT_HELP_TEXT,
    DASHBOARD_GET_HELP_TEXT, DASHBOARD_GOVERNANCE_GATE_HELP_TEXT, DASHBOARD_IMPORT_HELP_TEXT,
    DASHBOARD_INSPECT_EXPORT_HELP_TEXT, DASHBOARD_INSPECT_LIVE_HELP_TEXT,
    DASHBOARD_INSPECT_VARS_HELP_TEXT, DASHBOARD_LIST_HELP_TEXT, DASHBOARD_PATCH_FILE_HELP_TEXT,
    DASHBOARD_PUBLISH_HELP_TEXT, DASHBOARD_REVIEW_HELP_TEXT, DASHBOARD_SCREENSHOT_HELP_TEXT,
    DASHBOARD_TOPOLOGY_HELP_TEXT, SNAPSHOT_HELP_TEXT, UNIFIED_ACCESS_HELP_TEXT,
    UNIFIED_ALERT_HELP_TEXT, UNIFIED_DASHBOARD_HELP_TEXT, UNIFIED_DATASOURCE_HELP_TEXT,
    UNIFIED_MIGRATE_HELP_TEXT, UNIFIED_PROFILE_HELP_TEXT, UNIFIED_SYNC_HELP_TEXT,
};
use crate::cli_help_examples::UNIFIED_HELP_TEXT;
use crate::common::{json_color_choice, set_json_color_choice, CliColorChoice, Result};
use crate::dashboard::{
    run_dashboard_cli, AnalyzeArgs, BrowseArgs, CloneLiveArgs, DashboardCliArgs, DashboardCommand,
    DashboardHistoryArgs, DeleteArgs, DiffArgs, EditLiveArgs, ExportArgs, GetArgs,
    GovernanceGateArgs, ImpactArgs, ImportArgs, InspectExportArgs, InspectLiveArgs,
    InspectVarsArgs, ListArgs, PatchFileArgs, PublishArgs, ReviewArgs, ScreenshotArgs, ServeArgs,
    TopologyArgs, ValidateExportArgs,
};
use crate::datasource::{run_datasource_cli, DatasourceGroupCommand};
use crate::migrate::{run_migrate_cli, MigrateCliArgs};
use crate::overview::{run_overview_cli, OverviewCliArgs};
use crate::profile_cli::{run_profile_cli, ProfileCliArgs};
use crate::project_status_command::{run_project_status_cli, ProjectStatusCliArgs};
use crate::resource::{run_resource_cli, ResourceCliArgs};
use crate::snapshot::{run_snapshot_cli, SnapshotCommand};
use crate::sync::{run_sync_cli, SyncGroupCommand};

/// Dashboard subcommands exposed through the unified root CLI.
#[derive(Debug, Clone, Subcommand)]
pub enum DashboardGroupCommand {
    #[command(
        about = "Browse the live dashboard tree in an interactive terminal UI.",
        after_help = DASHBOARD_BROWSE_HELP_TEXT
    )]
    Browse(BrowseArgs),
    #[command(
        name = "fetch-live",
        about = "Fetch one live dashboard into an API-safe local JSON draft.",
        after_help = DASHBOARD_GET_HELP_TEXT
    )]
    Get(GetArgs),
    #[command(
        about = "Clone one live dashboard into a local draft with optional overrides.",
        after_help = DASHBOARD_CLONE_LIVE_HELP_TEXT
    )]
    CloneLive(CloneLiveArgs),
    #[command(about = "Serve dashboard drafts through a local preview server.")]
    Serve(ServeArgs),
    #[command(about = "Edit one live dashboard through an external editor.")]
    EditLive(EditLiveArgs),
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
        name = "patch-file",
        about = "Patch one local dashboard JSON file in place or to a new path.",
        after_help = DASHBOARD_PATCH_FILE_HELP_TEXT
    )]
    PatchFile(PatchFileArgs),
    #[command(
        name = "review",
        about = "Review one local dashboard JSON file without touching Grafana.",
        after_help = DASHBOARD_REVIEW_HELP_TEXT
    )]
    Review(ReviewArgs),
    #[command(
        about = "Publish one local dashboard JSON file through the existing dashboard import pipeline.",
        after_help = DASHBOARD_PUBLISH_HELP_TEXT
    )]
    Publish(PublishArgs),
    #[command(
        about = "Analyze dashboards from live Grafana or a local export tree and build summary or governance artifacts."
        ,
        after_help = DASHBOARD_ANALYZE_HELP_TEXT
    )]
    Analyze(AnalyzeArgs),
    #[command(
        name = "analyze-export",
        alias = "inspect-export",
        hide = true,
        about = "Analyze a raw dashboard export directory and summarize its structure.",
        after_help = DASHBOARD_INSPECT_EXPORT_HELP_TEXT
    )]
    InspectExport(InspectExportArgs),
    #[command(
        name = "analyze-live",
        alias = "inspect-live",
        hide = true,
        about = "Analyze live Grafana dashboards without writing a persistent export.",
        after_help = DASHBOARD_INSPECT_LIVE_HELP_TEXT
    )]
    InspectLive(InspectLiveArgs),
    #[command(
        name = "list-vars",
        alias = "inspect-vars",
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
        about = "Show which dashboards, variables, data sources, and alerts depend on each other.",
        after_help = DASHBOARD_TOPOLOGY_HELP_TEXT
    )]
    Topology(TopologyArgs),
    #[command(
        about = "Show which dashboards and alert resources would be affected by one data source from live Grafana, an export tree, or saved artifacts."
    )]
    Impact(ImpactArgs),
    #[command(
        name = "validate-export",
        about = "Run strict schema validation against dashboard raw export files before GitOps sync."
    )]
    ValidateExport(ValidateExportArgs),
    #[command(
        about = "List, restore, diff, or export live dashboard revision history.",
        after_help = "Examples:\n\n  List recent revisions from live Grafana for one dashboard:\n    grafana-util dashboard history list --url http://localhost:3000 --basic-user admin --basic-password admin --dashboard-uid cpu-main --output-format table\n\n  Review one local history artifact without calling Grafana:\n    grafana-util dashboard history list --input ./cpu-main.history.json --output-format yaml\n\n  Compare two live or local historical revisions:\n    grafana-util dashboard history diff --base-input ./cpu-main.history.json --base-version 17 --new-input ./cpu-main.history.json --new-version 21 --output-format json\n\n  Restore one historical revision as a new latest Grafana version:\n    grafana-util dashboard history restore --url http://localhost:3000 --basic-user admin --basic-password admin --dashboard-uid cpu-main --version 17 --dry-run\n\n  Export recent revision history into a reusable JSON artifact:\n    grafana-util dashboard history export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --dashboard-uid cpu-main --output ./cpu-main.history.json"
    )]
    History(DashboardHistoryArgs),
    #[command(
        about = "Open one dashboard in a headless browser and capture image or PDF output.",
        after_help = DASHBOARD_SCREENSHOT_HELP_TEXT
    )]
    Screenshot(ScreenshotArgs),
}

/// Namespaced root commands handled by the Rust `grafana-util` binary.
#[derive(Debug, Clone, Subcommand)]
pub enum UnifiedCommand {
    #[command(about = "Print the current grafana-util version.")]
    Version(VersionArgs),
    #[command(
        about = "Run dashboard browse, authoring, export, import, diff, patch-file, review, publish, and migration workflows.",
        visible_alias = "db",
        after_help = UNIFIED_DASHBOARD_HELP_TEXT
    )]
    Dashboard {
        #[command(subcommand)]
        command: DashboardGroupCommand,
    },
    #[command(
        about = "Run datasource list, browse-live, export, import, and diff workflows.",
        visible_alias = "ds",
        after_help = UNIFIED_DATASOURCE_HELP_TEXT
    )]
    Datasource {
        #[arg(
            long,
            global = true,
            value_enum,
            help = "Override JSON/YAML/table color for the datasource namespace. Use auto, always, never, none, or off."
        )]
        color: Option<CliColorChoice>,
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
        about = "Run profile list, show, add, example, and init workflows.",
        after_help = UNIFIED_PROFILE_HELP_TEXT
    )]
    Profile(ProfileCliArgs),
    #[command(
        about = "Run resource describe, kinds, list, and get workflows through a generic read-only query surface."
    )]
    Resource(ResourceCliArgs),
    #[command(
        about = "Run migration and repair transforms across Grafana artifact formats.",
        after_help = UNIFIED_MIGRATE_HELP_TEXT
    )]
    Migrate(MigrateCliArgs),
    #[command(
        about = "Export and review live dashboard snapshots.",
        after_help = SNAPSHOT_HELP_TEXT
    )]
    Snapshot {
        #[command(subcommand)]
        command: SnapshotCommand,
    },
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

#[derive(Debug, Clone, Args)]
pub struct VersionArgs {
    #[arg(long, help = "Render version details as JSON for external tooling.")]
    pub json: bool,
}

#[derive(Debug, Clone, Parser)]
#[command(
    name = "grafana-util",
    version = crate::common::TOOL_VERSION_DETAILS,
    about = "Unified Grafana dashboard, alerting, access, and profile utility.",
    after_help = UNIFIED_HELP_TEXT,
    styles = crate::help_styles::CLI_HELP_STYLES
)]
/// Parsed root CLI arguments for the Rust unified binary.
pub struct CliArgs {
    #[arg(
        long,
        value_enum,
        default_value_t = CliColorChoice::Auto,
        help = "Colorize JSON output. Use auto, always, or never."
    )]
    pub color: CliColorChoice,
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
    DashboardCliArgs {
        color: json_color_choice(),
        command,
    }
}

fn wrap_dashboard_group(command: DashboardGroupCommand) -> DashboardCliArgs {
    match command {
        DashboardGroupCommand::Browse(inner) => wrap_dashboard(DashboardCommand::Browse(inner)),
        DashboardGroupCommand::Get(inner) => wrap_dashboard(DashboardCommand::Get(inner)),
        DashboardGroupCommand::CloneLive(inner) => {
            wrap_dashboard(DashboardCommand::CloneLive(inner))
        }
        DashboardGroupCommand::Serve(inner) => wrap_dashboard(DashboardCommand::Serve(inner)),
        DashboardGroupCommand::EditLive(inner) => wrap_dashboard(DashboardCommand::EditLive(inner)),
        DashboardGroupCommand::List(inner) => wrap_dashboard(DashboardCommand::List(inner)),
        DashboardGroupCommand::Export(inner) => wrap_dashboard(DashboardCommand::Export(inner)),
        DashboardGroupCommand::Import(inner) => wrap_dashboard(DashboardCommand::Import(inner)),
        DashboardGroupCommand::Delete(inner) => wrap_dashboard(DashboardCommand::Delete(inner)),
        DashboardGroupCommand::Diff(inner) => wrap_dashboard(DashboardCommand::Diff(inner)),
        DashboardGroupCommand::PatchFile(inner) => {
            wrap_dashboard(DashboardCommand::PatchFile(inner))
        }
        DashboardGroupCommand::Review(inner) => wrap_dashboard(DashboardCommand::Review(inner)),
        DashboardGroupCommand::Publish(inner) => wrap_dashboard(DashboardCommand::Publish(inner)),
        DashboardGroupCommand::Analyze(inner) => wrap_dashboard(DashboardCommand::Analyze(inner)),
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
        DashboardGroupCommand::Impact(inner) => wrap_dashboard(DashboardCommand::Impact(inner)),
        DashboardGroupCommand::ValidateExport(inner) => {
            wrap_dashboard(DashboardCommand::ValidateExport(inner))
        }
        DashboardGroupCommand::History(inner) => wrap_dashboard(DashboardCommand::History(inner)),
        DashboardGroupCommand::Screenshot(inner) => {
            wrap_dashboard(DashboardCommand::Screenshot(inner))
        }
    }
}

// Centralized command fan-out before invoking domain runners.
// Every unified CLI variant is normalized into one of dashboard/alert/datasource/access/snapshot/overview/status runners here.
/// Dispatch the normalized root command into exactly one domain handler.
///
/// Handlers are injected as callables so tests can assert routing without
/// triggering network-heavy domain execution.
#[allow(clippy::too_many_arguments)]
fn dispatch_with_handlers<FD, FS, FY, FA, FX, FM, FP, FR, FO, FQ>(
    args: CliArgs,
    mut run_dashboard: FD,
    mut run_datasource: FS,
    mut run_sync: FY,
    mut run_alert: FA,
    mut run_access: FX,
    mut run_migrate: FM,
    mut run_profile: FP,
    mut run_snapshot: FR,
    mut run_overview: FO,
    mut run_project_status: FQ,
) -> Result<()>
where
    FD: FnMut(DashboardCliArgs) -> Result<()>,
    FS: FnMut(DatasourceGroupCommand) -> Result<()>,
    FY: FnMut(SyncGroupCommand) -> Result<()>,
    FA: FnMut(AlertCliArgs) -> Result<()>,
    FX: FnMut(AccessCliArgs) -> Result<()>,
    FM: FnMut(MigrateCliArgs) -> Result<()>,
    FP: FnMut(ProfileCliArgs) -> Result<()>,
    FR: FnMut(SnapshotCommand) -> Result<()>,
    FO: FnMut(OverviewCliArgs) -> Result<()>,
    FQ: FnMut(ProjectStatusCliArgs) -> Result<()>,
{
    let default_color = args.color;
    match args.command {
        UnifiedCommand::Version(args) => {
            if args.json {
                let payload = serde_json::json!({
                    "schemaVersion": crate::common::TOOL_VERSION_SCHEMA_VERSION,
                    "name": "grafana-util",
                    "version": crate::common::tool_version(),
                    "commit": crate::common::tool_git_commit(),
                    "buildTime": crate::common::tool_build_time(),
                });
                print!(
                    "{}",
                    crate::common::render_json_value_with_choice(&payload, default_color, false)?
                );
            } else {
                print!("{}", render_unified_version_text());
            }
            Ok(())
        }
        UnifiedCommand::Dashboard { command } => run_dashboard(wrap_dashboard_group(command)),
        UnifiedCommand::Datasource { color, command } => {
            set_json_color_choice(color.unwrap_or(default_color));
            run_datasource(command)
        }
        UnifiedCommand::Change { command } => run_sync(command),
        UnifiedCommand::Alert(inner) => run_alert(normalize_alert_namespace_args(inner)),
        UnifiedCommand::Access(inner) => run_access(inner),
        UnifiedCommand::Migrate(inner) => run_migrate(inner),
        UnifiedCommand::Profile(inner) => run_profile(inner),
        UnifiedCommand::Resource(inner) => run_resource_cli(inner),
        UnifiedCommand::Snapshot { command } => run_snapshot(command),
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
    set_json_color_choice(args.color);
    dispatch_with_handlers(
        args,
        run_dashboard_cli,
        run_datasource_cli,
        run_sync_cli,
        run_alert_cli,
        run_access_cli,
        run_migrate_cli,
        run_profile_cli,
        run_snapshot_cli,
        run_overview_cli,
        run_project_status_cli,
    )
}

#[cfg(test)]
#[path = "cli_rust_tests.rs"]
mod cli_rust_tests;
