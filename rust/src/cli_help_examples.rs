//! Structured help/example text for the unified Rust CLI.

macro_rules! help_item {
    (($label:literal, $summary:literal, $command:literal)) => {
        concat!("  ", $label, " ", $summary, "\n    ", $command)
    };
}

macro_rules! help_block {
    ($heading:literal, $first:tt $(, $rest:tt)+ $(,)?) => {
        concat!(
            $heading,
            "\n\n",
            help_item!($first),
            $(
                "\n\n",
                help_item!($rest),
            )+
        )
    };
    ($heading:literal, $only:tt $(,)?) => {
        concat!($heading, "\n\n", help_item!($only))
    };
}

pub(crate) const HELP_COLOR_RESET: &str = "\x1b[0m";
pub(crate) const HELP_COLOR_DASHBOARD: &str = "\x1b[1;36m";
pub(crate) const HELP_COLOR_ALERT: &str = "\x1b[1;31m";
pub(crate) const HELP_COLOR_DATASOURCE: &str = "\x1b[1;32m";
pub(crate) const HELP_COLOR_ACCESS: &str = "\x1b[1;33m";
pub(crate) const HELP_COLOR_SYNC: &str = "\x1b[1;34m";
pub(crate) const HELP_COLOR_PROFILE: &str = "\x1b[1;35m";

pub(crate) const HELP_FULL_HINT: &str =
    "Extended Help:\n  --help-full\n          Print help with extended examples\n";

pub(crate) const OVERVIEW_HELP_FULL_TEXT: &str = help_block!(
    "Extended Examples:",
    (
        "[Overview Staged]",
        "Summarize staged dashboard, alert, and sync exports into one overview:",
        "grafana-util overview --dashboard-export-dir ./dashboards/raw --alert-export-dir ./alerts/raw --desired-file ./desired.json --output json"
    ),
    (
        "[Overview Live]",
        "Open the live project-home overview through the shared status live path:",
        "grafana-util overview live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output interactive"
    ),
    (
        "[Overview Bundle]",
        "Build an overview from staged bundle and promotion inputs:",
        "grafana-util overview --source-bundle ./sync-source-bundle.json --target-inventory ./target-inventory.json --availability-file ./availability.json --mapping-file ./mapping.json --output text"
    )
);

pub(crate) const PROJECT_STATUS_HELP_FULL_TEXT: &str = help_block!(
    "Extended Examples:",
    (
        "[Status Staged]",
        "Inspect staged artifacts with a machine-readable summary:",
        "grafana-util status staged --dashboard-export-dir ./dashboards/raw --desired-file ./desired.json --output json"
    ),
    (
        "[Status Live]",
        "Check live Grafana status while layering staged sync context:",
        "grafana-util status live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --sync-summary-file ./sync-summary.json --bundle-preflight-file ./bundle-preflight.json --output json"
    )
);

pub(crate) const UNIFIED_HELP_TEXT: &str = help_block!(
    "Examples:",
    (
        "[Dashboard Export]",
        "Export dashboards with Basic auth:",
        "grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./dashboards --overwrite"
    ),
    (
        "[Dashboard Raw To Prompt]",
        "Convert one raw dashboard file into its sibling .prompt.json target:",
        "grafana-util dashboard raw-to-prompt --input-file ./dashboards/raw/cpu-main.json"
    ),
    (
        "[Dashboard Export]",
        "Export dashboards across all visible orgs:",
        "grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --export-dir ./dashboards --overwrite"
    ),
    (
        "[Dashboard Capture]",
        "Capture a dashboard screenshot from browser-like state:",
        r#"grafana-util dashboard screenshot --dashboard-url 'https://grafana.example.com/d/cpu-main/cpu-overview?var-cluster=prod-a' --token "$GRAFANA_API_TOKEN" --output ./cpu-main.png --full-page"#
    ),
    (
        "[Dashboard Capture]",
        "Inspect dashboard variables before capture:",
        r#"grafana-util dashboard inspect-vars --dashboard-url 'https://grafana.example.com/d/cpu-main/cpu-overview?var-cluster=prod-a' --token "$GRAFANA_API_TOKEN""#
    ),
    (
        "[Dashboard Review]",
        "Review a local dashboard file before publish:",
        "grafana-util dashboard review --input ./drafts/cpu-main.json --output-format yaml"
    ),
    (
        "[Snapshot Export]",
        "Capture a live snapshot into a local export root:",
        r#"grafana-util snapshot export --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --export-dir ./snapshot"#
    ),
    (
        "[Alert Export]",
        "Export alerting resources through the unified binary:",
        r#"grafana-util alert export --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-dir ./alerts --overwrite"#
    ),
    (
        "[Datasource Inventory]",
        "List datasource inventory through the unified binary:",
        r#"grafana-util datasource list --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --json"#
    ),
    (
        "[Datasource Inspect Export]",
        "Inspect a local datasource export root without Grafana access:",
        r#"grafana-util datasource inspect-export --input-dir ./datasources --json"#
    ),
    (
        "[Access Inventory]",
        "List org users through the unified binary:",
        r#"grafana-util access user list --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --json"#
    ),
    (
        "[Profile Show]",
        "Inspect a selected profile from the repo-local config file:",
        "grafana-util profile show --profile prod --output-format yaml"
    ),
    (
        "[Profile Add]",
        "Create or replace one repo-local profile entry:",
        "grafana-util profile add prod --url https://grafana.example.com --basic-user admin --prompt-password --store-secret encrypted-file"
    ),
    (
        "[Profile Example]",
        "Render a full annotated config example for reference editing:",
        "grafana-util profile example --mode full"
    ),
    (
        "[Snapshot Review]",
        "Review a local snapshot inventory as JSON:",
        "grafana-util snapshot review --input-dir ./snapshot --output json"
    ),
    (
        "[Change Planning]",
        "Build a change plan directly from live Grafana state:",
        r#"grafana-util change plan --desired-file ./desired.json --fetch-live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN"#
    ),
    (
        "[Change Apply]",
        "Apply a reviewed change plan back to Grafana:",
        r#"grafana-util change apply --plan-file ./sync-plan-reviewed.json --approve --execute-live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN"#
    )
);

pub(crate) const UNIFIED_HELP_FULL_TEXT: &str = help_block!(
    "Extended Examples:",
    (
        "[Dashboard Inspect Export]",
        "Render a grouped dashboard dependency table from raw exports:",
        "grafana-util dashboard inspect-export --import-dir ./dashboards/raw --input-format raw --output-format report-tree-table --report-columns dashboard_uid,panel_title,datasource_uid,query"
    ),
    (
        "[Dashboard Raw To Prompt]",
        "Generate a prompt/ lane from a raw export root:",
        "grafana-util dashboard raw-to-prompt --input-dir ./dashboards/raw --output-dir ./dashboards/prompt --overwrite"
    ),
    (
        "[Dashboard Inspect Export]",
        "Inspect a provisioning tree from the file-provisioning root:",
        "grafana-util dashboard inspect-export --import-dir ./dashboards/provisioning --input-format provisioning --report tree-table"
    ),
    (
        "[Dashboard Inspect Live]",
        "Render datasource governance JSON directly from live Grafana:",
        r#"grafana-util dashboard inspect-live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format governance-json"#
    ),
    (
        "[Datasource Import]",
        "Dry-run a datasource import and keep the result machine-readable:",
        r#"grafana-util datasource import --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --import-dir ./datasources --dry-run --json"#
    ),
    (
        "[Datasource Diff]",
        "Compare a provisioning datasource tree against live Grafana:",
        r#"grafana-util datasource diff --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --diff-dir ./datasources/provisioning --input-format provisioning"#
    ),
    (
        "[Access Team Import]",
        "Preview a destructive team sync before confirming:",
        "grafana-util access team import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./access-teams --replace-existing --dry-run --output-format table"
    ),
    (
        "[Profile Show]",
        "Inspect a selected profile from the repo-local config file:",
        "grafana-util profile show --profile prod --output-format yaml"
    ),
    (
        "[Profile Init]",
        "Seed grafana-util.yaml in the current directory from the built-in template:",
        "grafana-util profile init --overwrite"
    ),
    (
        "[Profile Add]",
        "Create or replace one repo-local profile entry:",
        "grafana-util profile add prod --url https://grafana.example.com --basic-user admin --prompt-password --store-secret encrypted-file"
    ),
    (
        "[Profile Example]",
        "Render a full annotated config example for reference editing:",
        "grafana-util profile example --mode full"
    ),
    (
        "[Snapshot Export]",
        "Capture a live snapshot into a local export root:",
        r#"grafana-util snapshot export --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --export-dir ./snapshot"#
    ),
    (
        "[Alert Import]",
        "Re-map linked alert dashboards during import:",
        "grafana-util alert import --url http://localhost:3000 --import-dir ./alerts/raw --replace-existing --dashboard-uid-map ./dashboard-map.json --panel-id-map ./panel-map.json"
    ),
    (
        "[Change Review]",
        "Stamp a plan as reviewed before apply:",
        "grafana-util change review --plan-file ./sync-plan.json --review-note 'peer-reviewed' --output json"
    ),
    (
        "[Overview Staged]",
        "Summarize staged dashboard, alert, and sync exports into one overview:",
        "grafana-util overview --dashboard-export-dir ./dashboards/raw --alert-export-dir ./alerts/raw --desired-file ./desired.json --output json"
    ),
    (
        "[Status Staged]",
        "Inspect staged artifacts with a machine-readable summary:",
        "grafana-util status staged --dashboard-export-dir ./dashboards/raw --desired-file ./desired.json --output json"
    ),
    (
        "[Snapshot Review]",
        "Open a local snapshot inventory in the interactive browser with `--interactive`:",
        "grafana-util snapshot review --input-dir ./snapshot --interactive"
    )
);

pub(crate) const ALERT_HELP_FULL_TEXT: &str = help_block!(
    "Extended Examples:",
    (
        "[Alert Export]",
        "Export alerting resources with overwrite enabled:",
        r#"grafana-util alert export --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-dir ./alerts --overwrite"#
    ),
    (
        "[Alert Import]",
        "Preview a replace-existing import before execution as structured JSON:",
        r#"grafana-util alert import --url http://localhost:3000 --import-dir ./alerts/raw --replace-existing --dry-run --json"#
    ),
    (
        "[Alert Diff]",
        "Compare a local export against Grafana as structured JSON:",
        r#"grafana-util alert diff --url http://localhost:3000 --diff-dir ./alerts/raw --json"#
    ),
    (
        "[Alert Plan]",
        "Build a staged alert plan from desired files with linkage maps:",
        "grafana-util alert plan --desired-dir ./alerts/desired --prune --dashboard-uid-map ./dashboard-map.json --panel-id-map ./panel-map.json --output json"
    ),
    (
        "[Alert Apply]",
        "Apply a reviewed alert plan only after explicit approval:",
        "grafana-util alert apply --plan-file ./alert-plan-reviewed.json --approve"
    ),
    (
        "[Alert Delete]",
        "Delete one explicit alert resource and allow policy reset only when requested:",
        "grafana-util alert delete --kind policy-tree --identity default --allow-policy-reset"
    ),
    (
        "[Alert Add Rule]",
        "Dry-run a managed rule with routing, labels, and a simple threshold contract before writing files:",
        "grafana-util alert add-rule --desired-dir ./alerts/desired --name cpu-high --folder platform-alerts --rule-group cpu --receiver pagerduty-primary --severity critical --expr 'A' --threshold 80 --above --for 5m --label team=platform --annotation summary='CPU high' --dry-run"
    ),
    (
        "[Alert Clone Rule]",
        "Dry-run a clone into a new target identity before writing files:",
        "grafana-util alert clone-rule --desired-dir ./alerts/desired --source cpu-high --name cpu-high-staging --folder staging-alerts --rule-group cpu --receiver slack-platform --dry-run"
    ),
    (
        "[Alert Add Contact Point]",
        "Dry-run a managed contact point entry before wiring routes:",
        "grafana-util alert add-contact-point --desired-dir ./alerts/desired --name pagerduty-primary --dry-run"
    ),
    (
        "[Alert Set Route]",
        "Dry-run the tool-owned managed route that will be fully replaced on rerun instead of merged field-by-field:",
        "grafana-util alert set-route --desired-dir ./alerts/desired --receiver pagerduty-primary --label team=platform --severity critical --dry-run"
    ),
    (
        "[Alert Preview Route]",
        "Preview route matching inputs from the staged desired tree under the same replace-not-merge managed-route model:",
        "grafana-util alert preview-route --desired-dir ./alerts/desired --label team=platform --severity critical"
    ),
    (
        "[Alert Import]",
        "Re-map linked dashboards and panels during import:",
        "grafana-util alert import --url http://localhost:3000 --import-dir ./alerts/raw --replace-existing --dashboard-uid-map ./dashboard-map.json --panel-id-map ./panel-map.json"
    ),
    (
        "[Alert List]",
        "Render live alert rules as JSON:",
        r#"grafana-util alert list-rules --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --json"#
    ),
    (
        "[Alert Init]",
        "Initialize a desired-state tree for staged alert management:",
        "grafana-util alert init --desired-dir ./alerts/desired"
    ),
    (
        "[Alert New Rule]",
        "Seed a low-level rule scaffold into the desired-state tree when the higher-level authoring surface is not enough:",
        "grafana-util alert new-rule --desired-dir ./alerts/desired --name cpu-main"
    ),
    (
        "[Alert New Contact Point]",
        "Seed a low-level contact point scaffold directly:",
        "grafana-util alert new-contact-point --desired-dir ./alerts/desired --name pagerduty-primary"
    )
);

pub(crate) const DATASOURCE_HELP_FULL_TEXT: &str = help_block!(
    "Extended Examples:",
    (
        "[Datasource Browse]",
        "Open a live datasource browser:",
        r#"grafana-util datasource browse --url http://localhost:3000 --token "$GRAFANA_API_TOKEN""#
    ),
    (
        "[Datasource List]",
        "Enumerate all visible org datasources as CSV:",
        r#"grafana-util datasource list --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --output-format csv"#
    ),
    (
        "[Datasource Add]",
        "Preview a new datasource contract as JSON:",
        r#"grafana-util datasource add --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --name prometheus-main --type prometheus --datasource-url http://prometheus:9090 --dry-run --json"#
    ),
    (
        "[Datasource Import]",
        "Import one exported org bundle with create-missing-orgs:",
        r#"grafana-util datasource import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./datasources --use-export-org --only-org-id 2 --create-missing-orgs --dry-run --json"#
    ),
    (
        "[Datasource Diff]",
        "Compare a local export directory with live Grafana:",
        r#"grafana-util datasource diff --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --diff-dir ./datasources"#
    ),
    (
        "[Datasource Diff]",
        "Compare a provisioning datasource tree against live Grafana:",
        r#"grafana-util datasource diff --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --diff-dir ./datasources/provisioning --input-format provisioning"#
    )
);

pub(crate) const ACCESS_HELP_FULL_TEXT: &str = help_block!(
    "Extended Examples:",
    (
        "[Access User Diff]",
        "Compare exported users against the Grafana global scope:",
        "grafana-util access user diff --url http://localhost:3000 --basic-user admin --basic-password admin --diff-dir ./access-users --scope global"
    ),
    (
        "[Access Team Import]",
        "Preview a destructive team sync as a table:",
        "grafana-util access team import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./access-teams --replace-existing --dry-run --output-format table"
    ),
    (
        "[Access Org Delete]",
        "Delete one org by explicit org id:",
        "grafana-util access org delete --url http://localhost:3000 --basic-user admin --basic-password admin --org-id 7 --yes --json"
    ),
    (
        "[Access Token Add]",
        "Issue a short-lived service-account token:",
        r#"grafana-util access service-account token add --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --service-account-id 7 --token-name nightly --seconds-to-live 3600"#
    )
);

pub(crate) const SYNC_HELP_FULL_TEXT: &str = help_block!(
    "Extended Examples:",
    (
        "[Change Summary]",
        "Render the desired resource summary as JSON:",
        "grafana-util change summary --desired-file ./desired.json --output json"
    ),
    (
        "[Change Audit]",
        "Compare the current live state against a staged checksum lock:",
        r#"grafana-util change audit --lock-file ./sync-lock.json --fetch-live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --fail-on-drift --output json"#
    ),
    (
        "[Change Bundle]",
        "Package exported dashboard and alert artifacts into one source bundle:",
        "grafana-util change bundle --dashboard-provisioning-dir ./dashboards/provisioning --alert-export-dir ./alerts/raw --output-file ./sync-source-bundle.json"
    ),
    (
        "[Change Bundle Preflight]",
        "Compare a source bundle against a target inventory snapshot:",
        "grafana-util change bundle-preflight --source-bundle ./sync-source-bundle.json --target-inventory ./target-inventory.json --output json"
    ),
    (
        "[Change Plan]",
        "Build a live-backed plan with prune candidates:",
        r#"grafana-util change plan --desired-file ./desired.json --fetch-live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --allow-prune --output json"#
    ),
    (
        "[Change Review]",
        "Stamp a reviewed plan with reviewer metadata:",
        "grafana-util change review --plan-file ./sync-plan.json --review-note 'peer-reviewed' --reviewed-by ops-user --output json"
    ),
    (
        "[Change Apply]",
        "Emit a reviewed local apply intent:",
        "grafana-util change apply --plan-file ./sync-plan-reviewed.json --approve"
    )
);

pub(crate) const HELP_EXAMPLE_LABELS: [(&str, &str); 31] = [
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
    ("[Datasource Inspect Export]", HELP_COLOR_DATASOURCE),
    ("[Datasource Diff]", HELP_COLOR_DATASOURCE),
    ("[Access Inventory]", HELP_COLOR_ACCESS),
    ("[Access User Diff]", HELP_COLOR_ACCESS),
    ("[Access Team Import]", HELP_COLOR_ACCESS),
    ("[Access Org Delete]", HELP_COLOR_ACCESS),
    ("[Access Token Add]", HELP_COLOR_ACCESS),
    ("[Profile Show]", HELP_COLOR_PROFILE),
    ("[Profile Init]", HELP_COLOR_PROFILE),
    ("[Profile Add]", HELP_COLOR_PROFILE),
    ("[Profile Example]", HELP_COLOR_PROFILE),
    ("[Change Planning]", HELP_COLOR_SYNC),
    ("[Change Summary]", HELP_COLOR_SYNC),
    ("[Change Plan]", HELP_COLOR_SYNC),
    ("[Change Review]", HELP_COLOR_SYNC),
    ("[Change Apply]", HELP_COLOR_SYNC),
    ("[Overview Staged]", HELP_COLOR_SYNC),
    ("[Overview Bundle]", HELP_COLOR_SYNC),
    ("[Project Status Staged]", HELP_COLOR_SYNC),
    ("[Project Status Live]", HELP_COLOR_SYNC),
];

pub(crate) fn colorize_help_examples(text: &str) -> String {
    let mut colored = text.to_string();
    for (label, color) in HELP_EXAMPLE_LABELS {
        let colored_label = format!("{color}{label}{HELP_COLOR_RESET}");
        colored = colored.replace(label, &colored_label);
    }
    colored
}

pub(crate) fn inject_help_full_hint(help: String) -> String {
    help.replace("\nExamples:\n", &format!("\n{HELP_FULL_HINT}\nExamples:\n"))
}
