use clap::{Subcommand, ValueEnum};

use super::alert_cli_args::{
    AlertAddContactPointArgs, AlertAddRuleArgs, AlertApplyArgs, AlertCloneRuleArgs,
    AlertDeleteArgs, AlertDiffArgs, AlertExportArgs, AlertImportArgs, AlertInitArgs, AlertListArgs,
    AlertNewResourceArgs, AlertPlanArgs, AlertPreviewRouteArgs, AlertSetRouteArgs,
};
use super::alert_help_texts::*;

/// Resource categories supported by alert list operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertListKind {
    Rules,
    ContactPoints,
    MuteTimings,
    Templates,
}

/// Enum definition for AlertListOutputFormat.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum AlertListOutputFormat {
    Text,
    Table,
    Csv,
    Json,
    Yaml,
}

/// Single-flag output selector for plan-oriented alert commands.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum AlertCommandOutputFormat {
    Text,
    Json,
}

/// Resource categories supported by alert delete scaffolding.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum AlertResourceKind {
    Rule,
    ContactPoint,
    MuteTiming,
    PolicyTree,
    Template,
}

/// Canonical subcommand identity for normalized alert CLI routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertCommandKind {
    Export,
    Import,
    Diff,
    Plan,
    Apply,
    Delete,
    Init,
    NewRule,
    NewContactPoint,
    NewTemplate,
    ListRules,
    ListContactPoints,
    ListMuteTimings,
    ListTemplates,
}

/// Authoring-focused subcommand identity carried only by parser/help surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertAuthoringCommandKind {
    AddRule,
    CloneRule,
    AddContactPoint,
    SetRoute,
    PreviewRoute,
}

/// Enum definition for AlertGroupCommand.
#[derive(Debug, Clone, Subcommand)]
pub enum AlertGroupCommand {
    #[command(
        about = "Export alerting resources into raw/ JSON files.",
        after_help = ALERT_EXPORT_HELP_TEXT
    )]
    Export(AlertExportArgs),
    #[command(
        about = "Import alerting resource JSON files through the Grafana API.",
        after_help = ALERT_IMPORT_HELP_TEXT
    )]
    Import(AlertImportArgs),
    #[command(
        about = "Compare local alerting export files against live Grafana resources.",
        after_help = ALERT_DIFF_HELP_TEXT
    )]
    Diff(AlertDiffArgs),
    #[command(
        about = "Build a staged alert management plan from desired alert resources.",
        after_help = ALERT_PLAN_HELP_TEXT
    )]
    Plan(AlertPlanArgs),
    #[command(
        about = "Apply a reviewed alert management plan.",
        after_help = ALERT_APPLY_HELP_TEXT
    )]
    Apply(AlertApplyArgs),
    #[command(
        about = "Delete one explicit alert resource identity.",
        after_help = ALERT_DELETE_HELP_TEXT
    )]
    Delete(AlertDeleteArgs),
    #[command(
        about = "Initialize a staged alert desired-state layout.",
        after_help = ALERT_INIT_HELP_TEXT
    )]
    Init(AlertInitArgs),
    #[command(
        name = "add-rule",
        about = "Author a staged alert rule from the higher-level authoring surface.",
        after_help = ALERT_ADD_RULE_HELP_TEXT
    )]
    AddRule(AlertAddRuleArgs),
    #[command(
        name = "clone-rule",
        about = "Clone an existing staged alert rule into a new authoring target.",
        after_help = ALERT_CLONE_RULE_HELP_TEXT
    )]
    CloneRule(AlertCloneRuleArgs),
    #[command(
        name = "add-contact-point",
        about = "Author a staged alert contact point from the higher-level authoring surface.",
        after_help = ALERT_ADD_CONTACT_POINT_HELP_TEXT
    )]
    AddContactPoint(AlertAddContactPointArgs),
    #[command(
        name = "set-route",
        about = "Author or replace the tool-owned staged notification route. Re-running fully replaces that managed route instead of merging fields.",
        after_help = ALERT_SET_ROUTE_HELP_TEXT
    )]
    SetRoute(AlertSetRouteArgs),
    #[command(
        name = "preview-route",
        about = "Preview the managed route inputs without changing runtime behavior. The corresponding set-route command fully replaces the tool-owned route on rerun.",
        after_help = ALERT_PREVIEW_ROUTE_HELP_TEXT
    )]
    PreviewRoute(AlertPreviewRouteArgs),
    #[command(
        name = "new-rule",
        about = "Create a low-level staged alert rule scaffold.",
        after_help = ALERT_NEW_RULE_HELP_TEXT
    )]
    NewRule(AlertNewResourceArgs),
    #[command(
        name = "new-contact-point",
        about = "Create a low-level staged alert contact point scaffold.",
        after_help = ALERT_NEW_CONTACT_POINT_HELP_TEXT
    )]
    NewContactPoint(AlertNewResourceArgs),
    #[command(
        name = "new-template",
        about = "Create a low-level staged alert template scaffold.",
        after_help = ALERT_NEW_TEMPLATE_HELP_TEXT
    )]
    NewTemplate(AlertNewResourceArgs),
    #[command(
        name = "list-rules",
        about = "List live Grafana alert rules.",
        after_help = ALERT_LIST_RULES_HELP_TEXT
    )]
    ListRules(AlertListArgs),
    #[command(
        name = "list-contact-points",
        about = "List live Grafana alert contact points.",
        after_help = ALERT_LIST_CONTACT_POINTS_HELP_TEXT
    )]
    ListContactPoints(AlertListArgs),
    #[command(
        name = "list-mute-timings",
        about = "List live Grafana mute timings.",
        after_help = ALERT_LIST_MUTE_TIMINGS_HELP_TEXT
    )]
    ListMuteTimings(AlertListArgs),
    #[command(
        name = "list-templates",
        about = "List live Grafana notification templates.",
        after_help = ALERT_LIST_TEMPLATES_HELP_TEXT
    )]
    ListTemplates(AlertListArgs),
}
