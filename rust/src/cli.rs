//! Unified CLI dispatcher for Rust entrypoints.
//!
//! Purpose:
//! - Own only command topology and domain dispatch.
//! - Keep `grafana-util` command surface in one place.
//! - Route to domain runners without carrying transport logic.

use clap::{Args, Parser, Subcommand};

use crate::access::{
    AccessCliArgs, OrgExportArgs, ServiceAccountExportArgs, TeamExportArgs, UserExportArgs,
};
use crate::alert::{
    AlertAddContactPointArgs, AlertAddRuleArgs, AlertApplyArgs, AlertCloneRuleArgs,
    AlertDeleteArgs, AlertDiffArgs, AlertExportArgs, AlertGroupCommand, AlertImportArgs,
    AlertInitArgs, AlertListArgs, AlertNewResourceArgs, AlertPlanArgs, AlertPreviewRouteArgs,
    AlertSetRouteArgs,
};
pub use crate::cli_help::{
    legacy_command_error_hint, maybe_render_unified_help_from_os_args,
    render_unified_help_full_text, render_unified_help_text, render_unified_version_text,
};
use crate::cli_help::{
    UNIFIED_ACCESS_HELP_TEXT, UNIFIED_ALERT_HELP_TEXT, UNIFIED_DATASOURCE_HELP_TEXT,
    UNIFIED_SYNC_HELP_TEXT,
};
use crate::cli_help_examples::UNIFIED_HELP_TEXT;
use crate::common::{set_json_color_choice, CliColorChoice, Result};
use crate::dashboard::{
    AnalyzeArgs, BrowseArgs, CloneLiveArgs, DashboardHistoryArgs, DeleteArgs, DiffArgs,
    EditLiveArgs, ExportArgs as DashboardExportArgs, GetArgs, GovernanceGateArgs, ImpactArgs,
    ImportArgs, InspectVarsArgs, ListArgs, PatchFileArgs, PublishArgs, RawToPromptArgs, ReviewArgs,
    ScreenshotArgs, ServeArgs, TopologyArgs,
};
use crate::datasource::{DatasourceExportArgs, DatasourceGroupCommand};
use crate::overview::{OverviewArgs, OverviewCommand};
use crate::profile_cli::ProfileCliArgs;
use crate::project_status_command::{
    ProjectStatusLiveArgs, ProjectStatusStagedArgs, PROJECT_STATUS_LIVE_HELP_TEXT,
    PROJECT_STATUS_STAGED_HELP_TEXT,
};
use crate::resource::ResourceCommand;
use crate::snapshot::SnapshotCommand;
use crate::sync::SyncGroupCommand;

const EXPORT_HELP_TEXT: &str = "Examples:\n\n  [Dashboard backup]\n    grafana-util export dashboard --output-dir ./dashboards --overwrite\n\n  [Alert backup]\n    grafana-util export alert --output-dir ./alerts --overwrite\n\n  [Datasource inventory]\n    grafana-util export datasource --output-dir ./datasources\n\n  [Access inventory]\n    grafana-util export access service-account --output-dir ./access-service-accounts";
const EXPORT_ACCESS_HELP_TEXT: &str = "Examples:\n\n  Export Grafana users into a local bundle:\n    grafana-util export access user --output-dir ./access-users --overwrite\n\n  Export Grafana teams into a local bundle:\n    grafana-util export access team --output-dir ./access-teams --overwrite\n\n  Export Grafana service accounts into a local bundle:\n    grafana-util export access service-account --output-dir ./access-service-accounts --overwrite";
const EXPORT_ACCESS_USER_HELP_TEXT: &str = "Examples:\n\n  Export Grafana users into a local bundle:\n    grafana-util export access user --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-dir ./access-users --overwrite";
const EXPORT_ACCESS_ORG_HELP_TEXT: &str = "Examples:\n\n  Export Grafana organizations into a local bundle:\n    grafana-util export access org --url http://localhost:3000 --basic-user admin --basic-password admin --output-dir ./access-orgs --overwrite";
const EXPORT_ACCESS_TEAM_HELP_TEXT: &str = "Examples:\n\n  Export Grafana teams into a local bundle:\n    grafana-util export access team --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-dir ./access-teams --overwrite";
const EXPORT_ACCESS_SERVICE_ACCOUNT_HELP_TEXT: &str = "Examples:\n\n  Export Grafana service accounts into a local bundle:\n    grafana-util export access service-account --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-dir ./access-service-accounts --overwrite";
const EXPORT_DASHBOARD_HELP_TEXT: &str = "Notes:\n  - Writes raw/, prompt/, and provisioning/ by default.\n  - Use Basic auth with --all-orgs.\n  - Use --flat for files directly under each variant directory.\n  - Use --include-history to add history/ under each exported org scope.\n  - The provider file is provisioning/provisioning/dashboards.yaml.\n  - Keep raw/ for API import or diff, prompt/ for UI import, and provisioning/ for file provisioning.\n\nExamples:\n\n  Export dashboards from the current org:\n    grafana-util export dashboard --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-dir ./dashboards --overwrite\n\n  Export dashboards across all visible orgs:\n    grafana-util export dashboard --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --output-dir ./dashboards --overwrite";
const EXPORT_ALERT_HELP_TEXT: &str = "Examples:\n\n  Export alerting resources from Grafana:\n    grafana-util export alert --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-dir ./alerts --overwrite";
const EXPORT_DATASOURCE_HELP_TEXT: &str = "Examples:\n\n  Export datasource inventory into a local artifact tree:\n    grafana-util export datasource --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-dir ./datasources --overwrite";
const ALERT_MIGRATE_HELP_TEXT: &str = "Examples:\n\n  [Import]\n    grafana-util alert migrate import --input-dir ./alerts/raw --replace-existing --dry-run --json\n\n  [Export]\n    grafana-util alert migrate export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-dir ./alerts --overwrite\n\n  [Diff]\n    grafana-util alert migrate diff --url http://localhost:3000 --diff-dir ./alerts/raw --output-format json";
const ALERT_MIGRATE_EXPORT_HELP_TEXT: &str = "Examples:\n\n  Export alerting resources with overwrite enabled:\n    grafana-util alert migrate export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-dir ./alerts --overwrite";
const ALERT_MIGRATE_IMPORT_HELP_TEXT: &str = "Examples:\n\n  Preview a replace-existing import before execution as structured JSON:\n    grafana-util alert migrate import --url http://localhost:3000 --input-dir ./alerts/raw --replace-existing --dry-run --json\n\n  Re-map linked dashboards and panels during import:\n    grafana-util alert migrate import --url http://localhost:3000 --input-dir ./alerts/raw --replace-existing --dashboard-uid-map ./dashboard-map.json --panel-id-map ./panel-map.json";
const ALERT_MIGRATE_DIFF_HELP_TEXT: &str = "Examples:\n\n  Compare a local export against Grafana as structured JSON:\n    grafana-util alert migrate diff --url http://localhost:3000 --diff-dir ./alerts/raw --output-format json";
const VERSION_HELP_TEXT: &str =
    "Examples:\n\n  Print the human-readable version:\n    grafana-util version\n\n  Print machine-readable version details:\n    grafana-util version --json";
const DASHBOARD_RAW_TO_PROMPT_HELP_TEXT: &str = "Examples:\n\n  Convert one raw dashboard JSON file into a prompt artifact:\n    grafana-util dashboard convert raw-to-prompt --input ./dashboards/raw/cpu-main.json --output ./dashboards/prompt/cpu-main.prompt.json\n\n  Convert a raw export tree into the prompt lane:\n    grafana-util dashboard convert raw-to-prompt --input-dir ./dashboards/raw --output-dir ./dashboards/prompt --output-format table";
#[derive(Debug, Clone, Subcommand)]
pub enum StatusCommand {
    #[command(
        about = "Render shared project-wide live status.",
        after_help = PROJECT_STATUS_LIVE_HELP_TEXT
    )]
    Live(ProjectStatusLiveArgs),
    #[command(
        about = "Render shared project-wide staged status.",
        after_help = PROJECT_STATUS_STAGED_HELP_TEXT
    )]
    Staged(ProjectStatusStagedArgs),
    #[command(about = "Render project-wide staged or live overview.")]
    Overview {
        #[command(flatten)]
        staged: OverviewArgs,
        #[command(subcommand)]
        command: Option<OverviewCommand>,
    },
    #[command(about = "Export or review live dashboard snapshots.")]
    Snapshot {
        #[command(subcommand)]
        command: SnapshotCommand,
    },
    #[command(about = "Run generic read-only Grafana resource queries.")]
    Resource {
        #[command(subcommand)]
        command: ResourceCommand,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum ConfigCommand {
    #[command(about = "Manage repo-local Grafana connection profiles.")]
    Profile(ProfileCliArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum ExportAccessCommand {
    #[command(
        about = "Export Grafana users into a local reviewable bundle.",
        after_help = EXPORT_ACCESS_USER_HELP_TEXT
    )]
    User(UserExportArgs),
    #[command(
        about = "Export Grafana org inventory into a local reviewable bundle.",
        after_help = EXPORT_ACCESS_ORG_HELP_TEXT
    )]
    Org(OrgExportArgs),
    #[command(
        about = "Export Grafana teams into a local reviewable bundle.",
        after_help = EXPORT_ACCESS_TEAM_HELP_TEXT
    )]
    Team(TeamExportArgs),
    #[command(
        name = "service-account",
        about = "Export Grafana service accounts into a local reviewable bundle.",
        after_help = EXPORT_ACCESS_SERVICE_ACCOUNT_HELP_TEXT
    )]
    ServiceAccount(ServiceAccountExportArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum ExportCommand {
    #[command(
        about = "Export dashboards into a local artifact tree for review or backup.",
        after_help = EXPORT_DASHBOARD_HELP_TEXT
    )]
    Dashboard(DashboardExportArgs),
    #[command(
        about = "Export alerting resources into a local artifact tree for review or backup.",
        after_help = EXPORT_ALERT_HELP_TEXT
    )]
    Alert(AlertExportArgs),
    #[command(
        about = "Export datasource inventory into a local artifact tree for review or backup.",
        after_help = EXPORT_DATASOURCE_HELP_TEXT
    )]
    Datasource(DatasourceExportArgs),
    #[command(
        about = "Export access inventory into a local artifact tree for review or backup.",
        after_help = EXPORT_ACCESS_HELP_TEXT
    )]
    Access {
        #[command(subcommand)]
        command: ExportAccessCommand,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum DashboardConvertCommand {
    #[command(
        name = "raw-to-prompt",
        about = "Convert raw dashboard exports into prompt lane artifacts.",
        after_help = DASHBOARD_RAW_TO_PROMPT_HELP_TEXT
    )]
    RawToPrompt(RawToPromptArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum DashboardRootCommand {
    #[command(
        name = "browse",
        about = "Browse the live dashboard tree in an interactive terminal UI."
    )]
    Browse(BrowseArgs),
    #[command(
        name = "list",
        about = "List dashboard summaries without writing export files."
    )]
    List(ListArgs),
    #[command(
        name = "variables",
        alias = "vars",
        about = "List dashboard templating variables from live Grafana or local artifacts."
    )]
    Variables(InspectVarsArgs),
    #[command(
        name = "get",
        about = "Fetch one live dashboard into an API-safe local JSON draft."
    )]
    Get(GetArgs),
    #[command(
        name = "history",
        about = "List, restore, diff, or export live dashboard revision history."
    )]
    History(DashboardHistoryArgs),
    #[command(
        name = "clone",
        about = "Clone one live dashboard into a local draft with optional overrides."
    )]
    Clone(CloneLiveArgs),
    #[command(
        name = "edit-live",
        about = "Edit one live dashboard through an external editor."
    )]
    EditLive(EditLiveArgs),
    #[command(
        name = "delete",
        about = "Delete live dashboards by UID or folder path."
    )]
    Delete(DeleteArgs),
    #[command(
        name = "export",
        about = "Export dashboards to raw/ and prompt/ JSON files."
    )]
    Export(DashboardExportArgs),
    #[command(
        name = "import",
        about = "Import dashboard JSON files through the Grafana API."
    )]
    Import(ImportArgs),
    #[command(
        name = "diff",
        about = "Compare local raw dashboard files against live Grafana dashboards."
    )]
    Diff(DiffArgs),
    #[command(name = "convert", about = "Run dashboard format conversion workflows.")]
    Convert {
        #[command(subcommand)]
        command: DashboardConvertCommand,
    },
    #[command(
        name = "review",
        about = "Review one local dashboard JSON file without touching Grafana."
    )]
    Review(ReviewArgs),
    #[command(
        name = "patch",
        about = "Patch one local dashboard JSON file in place or to a new path."
    )]
    Patch(PatchFileArgs),
    #[command(
        name = "serve",
        about = "Serve dashboard drafts through a local preview server."
    )]
    Serve(ServeArgs),
    #[command(
        name = "publish",
        about = "Publish one local dashboard JSON file through the existing dashboard import pipeline."
    )]
    Publish(PublishArgs),
    #[command(
        name = "summary",
        about = "Analyze dashboards from live Grafana or a local export tree."
    )]
    Summary(AnalyzeArgs),
    #[command(
        name = "dependencies",
        about = "Show which dashboards, variables, data sources, and alerts depend on each other."
    )]
    Dependencies(TopologyArgs),
    #[command(
        name = "impact",
        about = "Show which dashboards and alert resources would be affected by one data source."
    )]
    Impact(ImpactArgs),
    #[command(
        name = "policy",
        about = "Evaluate governance policy against dashboard inspect JSON artifacts."
    )]
    Policy(GovernanceGateArgs),
    #[command(
        name = "screenshot",
        about = "Open one dashboard in a headless browser and capture image or PDF output."
    )]
    Screenshot(ScreenshotArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum AlertLiveCommand {
    #[command(name = "list-rules", about = "List live Grafana alert rules.")]
    ListRules(AlertListArgs),
    #[command(
        name = "list-contact-points",
        about = "List live Grafana alert contact points."
    )]
    ListContactPoints(AlertListArgs),
    #[command(name = "list-mute-timings", about = "List live Grafana mute timings.")]
    ListMuteTimings(AlertListArgs),
    #[command(
        name = "list-templates",
        about = "List live Grafana notification templates."
    )]
    ListTemplates(AlertListArgs),
    #[command(
        name = "delete",
        about = "Delete one explicit alert resource identity."
    )]
    Delete(AlertDeleteArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum AlertMigrateCommand {
    #[command(
        name = "export",
        about = "Export alerting resources into raw/ JSON files.",
        after_help = ALERT_MIGRATE_EXPORT_HELP_TEXT
    )]
    Export(AlertExportArgs),
    #[command(
        name = "import",
        about = "Import alerting resource JSON files through the Grafana API.",
        after_help = ALERT_MIGRATE_IMPORT_HELP_TEXT
    )]
    Import(AlertImportArgs),
    #[command(
        name = "diff",
        about = "Compare local alerting export files against live Grafana resources.",
        after_help = ALERT_MIGRATE_DIFF_HELP_TEXT
    )]
    Diff(AlertDiffArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum AlertAuthorRuleCommand {
    #[command(
        name = "add",
        about = "Add a managed alert rule into the staged desired tree."
    )]
    Add(AlertAddRuleArgs),
    #[command(
        name = "clone",
        about = "Clone an existing staged alert rule into a new authoring target."
    )]
    Clone(AlertCloneRuleArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum AlertAuthorContactPointCommand {
    #[command(
        name = "add",
        about = "Add a managed contact point into the staged desired tree."
    )]
    Add(AlertAddContactPointArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum AlertAuthorRouteCommand {
    #[command(
        name = "set",
        about = "Set the tool-owned managed route inside the desired tree."
    )]
    Set(AlertSetRouteArgs),
    #[command(
        name = "preview",
        about = "Preview how the staged managed route would match labels and severity."
    )]
    Preview(AlertPreviewRouteArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum AlertAuthorCommand {
    #[command(name = "init", about = "Initialize a staged desired-state alert tree.")]
    Init(AlertInitArgs),
    #[command(name = "rule", about = "Author or clone managed alert rules.")]
    Rule {
        #[command(subcommand)]
        command: AlertAuthorRuleCommand,
    },
    #[command(name = "contact-point", about = "Author managed alert contact points.")]
    ContactPoint {
        #[command(subcommand)]
        command: AlertAuthorContactPointCommand,
    },
    #[command(
        name = "route",
        about = "Author or preview the managed alert routing subtree."
    )]
    Route {
        #[command(subcommand)]
        command: AlertAuthorRouteCommand,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum AlertScaffoldCommand {
    #[command(
        name = "rule",
        about = "Seed a low-level rule scaffold into the desired tree."
    )]
    Rule(AlertNewResourceArgs),
    #[command(
        name = "contact-point",
        about = "Seed a low-level contact-point scaffold into the desired tree."
    )]
    ContactPoint(AlertNewResourceArgs),
    #[command(
        name = "template",
        about = "Seed a low-level template scaffold into the desired tree."
    )]
    Template(AlertNewResourceArgs),
}

#[derive(Debug, Clone, Subcommand)]
pub enum AlertChangeCommand {
    #[command(
        name = "plan",
        about = "Build a staged alert change plan from desired files."
    )]
    Plan(AlertPlanArgs),
    #[command(
        name = "apply",
        about = "Apply a reviewed alert plan after explicit approval."
    )]
    Apply(AlertApplyArgs),
}

#[derive(Debug, Clone, Args)]
pub struct AlertSurfaceArgs {
    #[command(subcommand)]
    pub command: AlertCommandSurface,
}

#[derive(Debug, Clone, Subcommand)]
pub enum AlertCommandSurface {
    #[command(about = "Read live alert inventory or delete one live alert resource.")]
    Live {
        #[command(subcommand)]
        command: AlertLiveCommand,
    },
    #[command(
        about = "Move alert resources between local artifacts and Grafana.",
        after_help = ALERT_MIGRATE_HELP_TEXT
    )]
    Migrate {
        #[command(subcommand)]
        command: AlertMigrateCommand,
    },
    #[command(about = "Author managed alert desired-state resources.")]
    Author {
        #[command(subcommand)]
        command: AlertAuthorCommand,
    },
    #[command(about = "Seed low-level alert resource scaffolds.")]
    Scaffold {
        #[command(subcommand)]
        command: AlertScaffoldCommand,
    },
    #[command(about = "Plan or apply staged alert changes.")]
    Change {
        #[command(subcommand)]
        command: AlertChangeCommand,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum UnifiedCommand {
    #[command(
        about = "Print the current grafana-util version.",
        after_help = VERSION_HELP_TEXT
    )]
    Version(VersionArgs),
    #[command(
        name = "status",
        about = "Read live and staged Grafana state through a shared status surface."
    )]
    Status {
        #[command(subcommand)]
        command: StatusCommand,
    },
    #[command(
        about = "Run common export and backup flows without learning domain-heavy subtrees.",
        after_help = EXPORT_HELP_TEXT
    )]
    Export {
        #[command(subcommand)]
        command: ExportCommand,
    },
    #[command(
        about = "Canonical dashboard root for browse, list, authoring, export, import, analysis, and capture workflows."
    )]
    Dashboard {
        #[command(subcommand)]
        command: DashboardRootCommand,
    },
    #[command(
        about = "Manage datasource list, browse, export, import, and diff workflows.",
        visible_alias = "ds",
        after_help = UNIFIED_DATASOURCE_HELP_TEXT
    )]
    Datasource {
        #[arg(
            long,
            global = true,
            value_enum,
            help = "Override JSON/YAML/table color for the datasource namespace."
        )]
        color: Option<CliColorChoice>,
        #[command(subcommand)]
        command: DatasourceGroupCommand,
    },
    #[command(
        about = "Manage alert inventory, backup, authoring, and apply workflows.",
        after_help = UNIFIED_ALERT_HELP_TEXT
    )]
    Alert {
        #[arg(
            long,
            global = true,
            value_enum,
            help = "Override JSON/YAML/table color for the alert namespace."
        )]
        color: Option<CliColorChoice>,
        #[command(subcommand)]
        command: AlertGroupCommand,
    },
    #[command(
        about = "Manage Grafana users, teams, and service accounts.",
        after_help = UNIFIED_ACCESS_HELP_TEXT
    )]
    Access(AccessCliArgs),
    #[command(
        name = "workspace",
        about = "Review a local Grafana workspace before preview and apply.",
        after_help = UNIFIED_SYNC_HELP_TEXT
    )]
    Workspace {
        #[command(subcommand)]
        command: SyncGroupCommand,
    },
    #[command(about = "Run repo-local Grafana configuration and profile workflows.")]
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
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
    about = "Task-first Grafana CLI for status, export, dashboard, workspace review, alert, access, datasource, and config workflows.",
    after_help = UNIFIED_HELP_TEXT,
    infer_subcommands(true),
    styles = crate::help_styles::CLI_HELP_STYLES
)]
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

pub fn parse_cli_from<I, T>(iter: I) -> CliArgs
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    CliArgs::parse_from(iter)
}

pub fn run_cli(args: CliArgs) -> Result<()> {
    set_json_color_choice(args.color);
    dispatch_with_handlers(
        args,
        crate::dashboard::run_dashboard_cli,
        crate::datasource::run_datasource_cli,
        crate::sync::run_sync_cli,
        crate::alert::run_alert_cli,
        crate::access::run_access_cli,
        crate::profile_cli::run_profile_cli,
        crate::snapshot::run_snapshot_cli,
        crate::resource::run_resource_cli,
        crate::overview::run_overview_cli,
        crate::project_status_command::run_project_status_cli,
    )
}

pub(crate) use crate::cli_dispatch::dispatch_with_handlers;

#[cfg(test)]
#[path = "cli_rust_tests.rs"]
mod cli_rust_tests;
