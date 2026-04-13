use super::grouped::{GroupedHelpRow, GroupedHelpSection, GroupedHelpSpec};

macro_rules! row {
    ($name:literal, $summary:literal) => {
        GroupedHelpRow {
            name: $name,
            summary: $summary,
        }
    };
}

macro_rules! section {
    ($heading:literal, [$($row:expr),+ $(,)?]) => {
        GroupedHelpSection {
            heading: $heading,
            rows: &[$($row),+],
        }
    };
}

pub(crate) struct GroupedHelpEntrypoint {
    pub(crate) path: &'static [&'static str],
    pub(crate) aliases: &'static [&'static [&'static str]],
    pub(crate) spec: &'static GroupedHelpSpec,
}

pub(crate) const UNIFIED_ROOT_HELP_SPEC: GroupedHelpSpec = GroupedHelpSpec {
    usage: "grafana-util [OPTIONS] <COMMAND>",
    sections: &[
        section!(
            "Start Here",
            [
                row!("version", "Confirm the binary and version."),
                row!("completion", "Generate Bash or Zsh shell completion."),
                row!("status", "Run read-only live or staged checks."),
                row!("config", "Save and inspect connection profiles."),
            ]
        ),
        section!(
            "Read & Export",
            [
                row!(
                    "export",
                    "Back up dashboards, alerts, datasources, or access inventory."
                ),
                row!(
                    "dashboard",
                    "Browse, list, export, import, inspect, and capture dashboards."
                ),
                row!(
                    "datasource",
                    "Browse, list, change, export, import, and diff datasources."
                ),
                row!("alert", "List, export, import, author, plan, and apply alerts."),
                row!(
                    "access",
                    "Manage users, orgs, teams, service accounts, and tokens."
                ),
            ]
        ),
        section!(
            "Review & Apply",
            [row!(
                "workspace",
                "Scan, test, preview, package, and apply local workspace changes."
            )]
        ),
        section!(
            "Options",
            [
                row!("-h, --help", "Print help."),
                row!("-V, --version", "Print version."),
                row!("--color", "Colorize JSON output: auto, always, or never."),
            ]
        ),
    ],
    footer: &[
        "First 3 commands:",
        "  grafana-util --version",
        "  grafana-util completion zsh > ~/.zfunc/_grafana-util",
        "  grafana-util status live --url http://localhost:3000 --basic-user admin --prompt-password --output-format yaml",
        "",
        "More help:",
        "  grafana-util <COMMAND> --help",
        "  grafana-util --help-full",
    ],
};

pub(crate) const DASHBOARD_HELP_SPEC: GroupedHelpSpec = GroupedHelpSpec {
    usage: "grafana-util dashboard <COMMAND>",
    sections: &[
        section!(
            "Browse & Inspect",
            [
                row!("browse", "Browse dashboards interactively."),
                row!("list", "List dashboard summaries."),
                row!("get", "Fetch one dashboard JSON draft."),
                row!("variables", "List dashboard variables."),
                row!("history", "Inspect dashboard revision history."),
            ]
        ),
        section!(
            "Export & Import",
            [
                row!(
                    "export",
                    "Back up dashboards into raw/, prompt/, and provisioning/."
                ),
                row!("import", "Import raw dashboard JSON through the API."),
                row!(
                    "convert",
                    "Convert raw dashboard JSON into prompt artifacts."
                ),
            ]
        ),
        section!(
            "Review & Diff",
            [
                row!("diff", "Compare local raw dashboards against Grafana."),
                row!("review", "Check one local dashboard JSON draft."),
                row!("summary", "Analyze live or exported dashboards."),
                row!(
                    "dependencies",
                    "Show dashboard, datasource, variable, and alert dependencies."
                ),
                row!("impact", "Show datasource blast radius."),
                row!("policy", "Evaluate governance policy."),
            ]
        ),
        section!(
            "Edit & Publish",
            [
                row!("clone", "Clone one dashboard into a local draft."),
                row!("patch", "Modify one local dashboard JSON draft."),
                row!("serve", "Preview local dashboard drafts."),
                row!(
                    "edit-live",
                    "Edit one live dashboard through a local editor."
                ),
                row!("publish", "Publish one local dashboard JSON draft."),
                row!("delete", "Delete live dashboards after explicit selection."),
            ]
        ),
        section!(
            "Operate & Capture",
            [row!("screenshot", "Capture dashboard evidence.")]
        ),
    ],
    footer: &["More help:", "  grafana-util dashboard <COMMAND> --help"],
};

pub(crate) const STATUS_HELP_SPEC: GroupedHelpSpec = GroupedHelpSpec {
    usage: "grafana-util status <COMMAND>",
    sections: &[
        section!(
            "Live Read-Only",
            [
                row!("live", "Check live Grafana state."),
                row!(
                    "overview",
                    "Open a live or staged overview; use `overview live` first."
                ),
                row!("resource", "Query generic Grafana resources."),
                row!("snapshot", "Export or review live dashboard snapshots."),
            ]
        ),
        section!(
            "Staged Review",
            [
                row!("staged", "Check local staged files before apply workflows."),
                row!("overview", "Open a staged overview with `overview staged`."),
            ]
        ),
    ],
    footer: &[
        "More help:",
        "  grafana-util status <COMMAND> --help",
        "  grafana-util status --help-schema",
    ],
};

pub(crate) const EXPORT_HELP_SPEC: GroupedHelpSpec = GroupedHelpSpec {
    usage: "grafana-util export <COMMAND>",
    sections: &[section!(
        "Backups",
        [
            row!(
                "dashboard",
                "Export dashboard raw/, prompt/, and provisioning artifacts."
            ),
            row!("alert", "Export alerting resources."),
            row!(
                "datasource",
                "Export datasource inventory and provisioning files."
            ),
            row!("access", "Export users, orgs, teams, or service accounts."),
        ]
    )],
    footer: &["More help:", "  grafana-util export <COMMAND> --help"],
};

pub(crate) const DATASOURCE_HELP_SPEC: GroupedHelpSpec = GroupedHelpSpec {
    usage: "grafana-util datasource <COMMAND>",
    sections: &[
        section!(
            "Browse & Inspect",
            [
                row!("browse", "Browse live datasources interactively."),
                row!(
                    "list",
                    "List datasource inventory from live Grafana or local exports."
                ),
                row!("types", "Show supported datasource type templates."),
            ]
        ),
        section!(
            "Change Live",
            [
                row!("add", "Create one datasource."),
                row!("modify", "Modify one datasource."),
                row!("delete", "Delete one datasource."),
            ]
        ),
        section!(
            "Export & Import",
            [
                row!("export", "Back up datasource inventory."),
                row!("import", "Import datasource inventory."),
                row!("diff", "Compare local datasource exports against Grafana."),
            ]
        ),
    ],
    footer: &[
        "More help:",
        "  grafana-util datasource <COMMAND> --help",
        "  grafana-util datasource --help-full",
    ],
};

pub(crate) const ACCESS_HELP_SPEC: GroupedHelpSpec = GroupedHelpSpec {
    usage: "grafana-util access <COMMAND>",
    sections: &[
        section!(
            "People & Teams",
            [
                row!(
                    "user",
                    "List, add, modify, export, import, diff, or delete users."
                ),
                row!(
                    "team",
                    "Browse, list, add, modify, export, import, diff, or delete teams."
                ),
            ]
        ),
        section!(
            "Orgs & Automation",
            [
                row!(
                    "org",
                    "List, add, modify, export, import, diff, or delete orgs."
                ),
                row!("service-account", "Manage service accounts and tokens."),
            ]
        ),
    ],
    footer: &["More help:", "  grafana-util access <COMMAND> --help"],
};

pub(crate) const WORKSPACE_HELP_SPEC: GroupedHelpSpec = GroupedHelpSpec {
    usage: "grafana-util workspace <COMMAND>",
    sections: &[
        section!(
            "Beginner Path",
            [
                row!("scan", "Inspect discovered or explicit staged inputs."),
                row!(
                    "test",
                    "Check whether the staged workspace is structurally safe."
                ),
                row!(
                    "preview",
                    "Preview what would change before touching Grafana."
                ),
                row!("apply", "Apply a reviewed preview after explicit approval."),
            ]
        ),
        section!(
            "Packaging & CI",
            [
                row!(
                    "package",
                    "Package exported resources into one local workspace bundle."
                ),
                row!("ci", "Open lower-level CI and review-contract commands."),
            ]
        ),
    ],
    footer: &[
        "More help:",
        "  grafana-util workspace <COMMAND> --help",
        "  grafana-util workspace ci --help",
    ],
};

pub(crate) const ALERT_HELP_SPEC: GroupedHelpSpec = GroupedHelpSpec {
    usage: "grafana-util alert <COMMAND>",
    sections: &[
        section!(
            "Inventory",
            [
                row!("list-rules", "List live alert rules."),
                row!("list-contact-points", "List live contact points."),
                row!("list-mute-timings", "List live mute timings."),
                row!("list-templates", "List live notification templates."),
                row!("delete", "Delete one explicit live alert resource."),
            ]
        ),
        section!(
            "Backup & Compare",
            [
                row!("export", "Export alerting resources."),
                row!("import", "Import alerting resources."),
                row!("diff", "Compare local alert files against Grafana."),
            ]
        ),
        section!(
            "Author Desired State",
            [
                row!("init", "Initialize a staged desired-state tree."),
                row!("add-rule", "Author a managed alert rule."),
                row!("clone-rule", "Clone a staged alert rule."),
                row!("add-contact-point", "Author a managed contact point."),
                row!("set-route", "Set the managed notification route."),
                row!("preview-route", "Preview managed route matching."),
                row!("new-rule", "Seed a low-level rule scaffold."),
                row!(
                    "new-contact-point",
                    "Seed a low-level contact point scaffold."
                ),
                row!("new-template", "Seed a low-level template scaffold."),
            ]
        ),
        section!(
            "Review & Apply",
            [
                row!("plan", "Build a staged alert change plan."),
                row!("apply", "Apply a reviewed alert plan."),
            ]
        ),
    ],
    footer: &[
        "More help:",
        "  grafana-util alert <COMMAND> --help",
        "  grafana-util alert --help-full",
    ],
};

pub(crate) const CONFIG_HELP_SPEC: GroupedHelpSpec = GroupedHelpSpec {
    usage: "grafana-util config <COMMAND>",
    sections: &[section!(
        "Connection Setup",
        [row!(
            "profile",
            "Add, list, show, validate, and initialize Grafana connection profiles."
        )]
    )],
    footer: &["More help:", "  grafana-util config profile --help"],
};

pub(crate) const PROFILE_HELP_SPEC: GroupedHelpSpec = GroupedHelpSpec {
    usage: "grafana-util config profile <COMMAND>",
    sections: &[
        section!(
            "Daily Use",
            [
                row!("add", "Save one Grafana connection profile."),
                row!("list", "List configured profile names."),
                row!("current", "Show which profile would be selected."),
                row!("show", "Inspect one profile."),
                row!(
                    "validate",
                    "Validate config and optionally check live reachability."
                ),
            ]
        ),
        section!(
            "Setup & Examples",
            [
                row!("init", "Create a starting grafana-util.yaml."),
                row!("example", "Render an annotated config example."),
            ]
        ),
    ],
    footer: &[
        "More help:",
        "  grafana-util config profile <COMMAND> --help",
    ],
};

pub(crate) const GROUPED_HELP_ENTRYPOINTS: &[GroupedHelpEntrypoint] = &[
    GroupedHelpEntrypoint {
        path: &[],
        aliases: &[],
        spec: &UNIFIED_ROOT_HELP_SPEC,
    },
    GroupedHelpEntrypoint {
        path: &["status"],
        aliases: &[],
        spec: &STATUS_HELP_SPEC,
    },
    GroupedHelpEntrypoint {
        path: &["export"],
        aliases: &[],
        spec: &EXPORT_HELP_SPEC,
    },
    GroupedHelpEntrypoint {
        path: &["dashboard"],
        aliases: &[&["db"]],
        spec: &DASHBOARD_HELP_SPEC,
    },
    GroupedHelpEntrypoint {
        path: &["datasource"],
        aliases: &[&["ds"]],
        spec: &DATASOURCE_HELP_SPEC,
    },
    GroupedHelpEntrypoint {
        path: &["access"],
        aliases: &[],
        spec: &ACCESS_HELP_SPEC,
    },
    GroupedHelpEntrypoint {
        path: &["workspace"],
        aliases: &[],
        spec: &WORKSPACE_HELP_SPEC,
    },
    GroupedHelpEntrypoint {
        path: &["alert"],
        aliases: &[],
        spec: &ALERT_HELP_SPEC,
    },
    GroupedHelpEntrypoint {
        path: &["config"],
        aliases: &[],
        spec: &CONFIG_HELP_SPEC,
    },
    GroupedHelpEntrypoint {
        path: &["config", "profile"],
        aliases: &[],
        spec: &PROFILE_HELP_SPEC,
    },
];

pub(crate) fn find_grouped_help_entrypoint(
    path: &[String],
) -> Option<&'static GroupedHelpEntrypoint> {
    GROUPED_HELP_ENTRYPOINTS.iter().find(|entrypoint| {
        entrypoint.path == path || entrypoint.aliases.iter().any(|alias| *alias == path)
    })
}
