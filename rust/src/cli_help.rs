//! Unified CLI help examples and rendering helpers.
//!
//! Keeping the large example blocks and help rendering here lets `cli.rs`
//! stay focused on command topology and dispatch.

use clap::{ColorChoice, CommandFactory};

use crate::access::root_command as access_root_command;
use crate::alert::root_command as alert_root_command;
use crate::cli::CliArgs;
use crate::cli_help_examples::{
    colorize_dashboard_short_help, colorize_help_examples, inject_help_full_hint,
    ACCESS_HELP_FULL_TEXT, ALERT_HELP_FULL_TEXT, DATASOURCE_HELP_FULL_TEXT,
    OVERVIEW_HELP_FULL_TEXT, PROJECT_STATUS_HELP_FULL_TEXT, SYNC_HELP_FULL_TEXT,
    UNIFIED_HELP_FULL_TEXT, UNIFIED_HELP_TEXT,
};
use crate::datasource::root_command as datasource_root_command;
use crate::migrate::{maybe_render_migrate_help_from_os_args, MigrateCliArgs};
use crate::overview::OverviewCliArgs;
use crate::profile_cli::root_command as profile_root_command;
use crate::project_status_command::ProjectStatusCliArgs;
use crate::snapshot::root_command as snapshot_root_command;
use crate::sync::SyncCliArgs;

pub(crate) const UNIFIED_DASHBOARD_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard browse --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\"\n  grafana-util dashboard fetch-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --dashboard-uid cpu-main --output ./cpu-main.json\n  grafana-util dashboard clone-live --url http://localhost:3000 --basic-user admin --basic-password admin --source-uid cpu-main --output ./cpu-main-clone.json\n  grafana-util dashboard analyze --url http://localhost:3000 --basic-user admin --basic-password admin --output-format governance-json\n  grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --output-dir ./dashboards --overwrite --include-history\n  grafana-util dashboard diff --url http://localhost:3000 --basic-user admin --basic-password admin --input-dir ./dashboards/raw\n  grafana-util dashboard topology --url http://localhost:3000 --basic-user admin --basic-password admin --output-format mermaid\n  grafana-util dashboard patch-file --input ./dashboards/raw/cpu-main.json --name 'CPU Overview' --folder-uid infra --tag prod --tag sre\n  grafana-util dashboard review --input ./drafts/cpu-main.json --output-format yaml\n  grafana-util dashboard publish --url http://localhost:3000 --basic-user admin --basic-password admin --input ./drafts/cpu-main.json --dry-run --table";
pub(crate) const UNIFIED_DASHBOARD_SHORT_HELP_TEXT: &str = "Usage: grafana-util dashboard <COMMAND>\n\nChoose the task first:\n  work with dashboard trees        browse, list, list-vars, fetch-live, clone-live, edit-live, screenshot\n  work with local drafts           review, patch-file, serve, publish\n  move dashboards                  export, import, diff, delete\n  analyze and review risk          analyze, topology, governance-gate, history\n\nWork with dashboard trees:\n  browse           Browse the live dashboard tree or a local export tree in an interactive terminal UI.\n  list             List dashboard summaries without writing export files.\n  list-vars        List dashboard templating variables from live Grafana, a local dashboard file, or a local export tree.\n  fetch-live       Fetch one live dashboard into an API-safe local JSON draft.\n  clone-live       Clone one live dashboard into a local draft with optional overrides.\n  edit-live        Edit one live dashboard through an external editor.\n  screenshot       Open one dashboard in a headless browser and capture image or PDF output.\n\nWork with local drafts:\n  review           Review one local dashboard JSON file without touching Grafana.\n  patch-file       Patch one local dashboard JSON file in place or to a new path.\n  serve            Serve dashboard drafts through a local preview server.\n  publish          Publish one local dashboard JSON file through the existing dashboard import pipeline.\n\nMove dashboards:\n  export           Export dashboards to raw/ and prompt/ JSON files.\n  import           Import dashboard JSON files through the Grafana API.\n  diff             Compare local raw dashboard files against live Grafana dashboards.\n  delete           Delete live dashboards by UID or folder path.\n\nAnalyze and review risk:\n  analyze          Analyze live Grafana or a local export tree and render summary, dependency, governance, or query-analysis outputs.\n  topology         Show which dashboards, variables, data sources, and alerts depend on each other.\n  governance-gate  Check dashboard query and governance findings against a policy.\n  history          List, restore, or export live dashboard revision history.\n\nMore help:\n  grafana-util dashboard <COMMAND> --help\n  grafana-util dashboard <COMMAND> --help-full\n";
pub(crate) const UNIFIED_DATASOURCE_HELP_TEXT: &str = "Examples:\n\n  grafana-util datasource browse --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\"\n  grafana-util datasource list --input-dir ./datasources --json\n  grafana-util datasource list --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --json\n  grafana-util datasource import --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --input-dir ./datasources --dry-run --json";
pub(crate) const UNIFIED_MIGRATE_HELP_TEXT: &str = "Examples:\n\n  grafana-util migrate dashboard raw-to-prompt --input-file ./dashboards/raw/cpu-main.json\n  grafana-util migrate dashboard raw-to-prompt --input-dir ./dashboards/raw --output-dir ./dashboards/prompt --overwrite\n  grafana-util migrate dashboard raw-to-prompt --input-file ./dashboards/raw/cpu-main.json --datasource-map ./datasource-map.json --resolution exact";
pub(crate) const UNIFIED_SYNC_HELP_TEXT: &str = "Examples:\n\n  grafana-util change inspect --workspace ./grafana-oac-repo --output-format table\n  grafana-util change preview --workspace ./grafana-oac-repo --fetch-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format json\n  grafana-util change apply --preview-file ./change-preview.json --approve --execute-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\"";
pub(crate) const UNIFIED_ALERT_HELP_TEXT: &str = "Examples:\n\n  grafana-util alert export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-dir ./alerts --overwrite\n  grafana-util alert import --url http://localhost:3000 --input-dir ./alerts/raw --replace-existing --dry-run --json\n  grafana-util alert list-rules --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --json";
pub(crate) const UNIFIED_ACCESS_HELP_TEXT: &str = "Examples:\n\n  grafana-util access user list --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --json\n  grafana-util access user list --input-dir ./access-users --json\n  grafana-util access team import --url http://localhost:3000 --basic-user admin --basic-password admin --input-dir ./access-teams --replace-existing --yes\n  grafana-util access service-account token add --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --name deploy-bot --token-name nightly";
pub(crate) const UNIFIED_PROFILE_HELP_TEXT: &str = "Examples:\n\n  grafana-util profile list\n  grafana-util profile show --profile prod --output-format yaml\n  grafana-util profile add prod --url https://grafana.example.com --basic-user admin --prompt-password --store-secret encrypted-file\n  grafana-util profile example --mode basic\n  grafana-util profile example --mode full\n  grafana-util profile init --overwrite";
pub(crate) const DASHBOARD_BROWSE_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard browse --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\"\n  grafana-util dashboard browse --url http://localhost:3000 --basic-user admin --basic-password admin --path 'Platform / Infra'\n  grafana-util dashboard browse --input-dir ./dashboards/raw --path 'Platform / Infra'\n  grafana-util dashboard browse --workspace ./grafana-oac-repo --path 'Platform / Infra'\n  grafana-util dashboard browse --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs";
pub(crate) const DASHBOARD_DIFF_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard diff --url http://localhost:3000 --basic-user admin --basic-password admin --input-dir ./dashboards/raw\n  grafana-util dashboard diff --url http://localhost:3000 --basic-user admin --basic-password admin --org-id 2 --input-dir ./dashboards/raw --json";
pub(crate) const DASHBOARD_GET_HELP_TEXT: &str = "What it does:\n  Fetch one live dashboard and write an API-safe local draft file without mutating Grafana.\n\nWhen to use:\n  - Start a local edit or review flow from the current live dashboard.\n  - Capture one dashboard before patching, diffing, or publishing locally.\n\nRelated commands:\n  - dashboard clone-live  Fetch then override title, UID, or folder metadata.\n  - dashboard review      Inspect one local draft before publish.\n  - dashboard publish     Send one reviewed local draft back to Grafana.\n\nExamples:\n\n  grafana-util dashboard fetch-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --dashboard-uid cpu-main --output ./cpu-main.json\n  grafana-util dashboard fetch-live --profile prod --url http://localhost:3000 --basic-user admin --basic-password admin --dashboard-uid cpu-main --output ./cpu-main.json";
pub(crate) const DASHBOARD_CLONE_LIVE_HELP_TEXT: &str = "What it does:\n  Fetch one live dashboard into a local draft and optionally override title, UID, or folder metadata before saving it.\n\nWhen to use:\n  - Fork a live dashboard into a new draft for another folder, environment, or owner.\n  - Prepare a publishable variant without mutating the source dashboard first.\n\nRelated commands:\n  - dashboard fetch-live  Fetch the live dashboard without changing any metadata.\n  - dashboard patch-file  Adjust the local draft after the initial clone step.\n  - dashboard publish     Push the reviewed clone into Grafana.\n\nExamples:\n\n  grafana-util dashboard clone-live --url http://localhost:3000 --basic-user admin --basic-password admin --source-uid cpu-main --output ./cpu-main-clone.json\n  grafana-util dashboard clone-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --source-uid cpu-main --name 'CPU Clone' --uid cpu-main-clone --folder-uid infra --output ./cpu-main-clone.json";
pub(crate) const DASHBOARD_LIST_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard list --url http://localhost:3000 --basic-user admin --basic-password admin\n  grafana-util dashboard list --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --json\n  grafana-util dashboard list --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --org-id 2 --json";
pub(crate) const DASHBOARD_EXPORT_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --output-dir ./dashboards --overwrite --include-history\n  grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --output-dir ./dashboards --overwrite --include-history\n  grafana-util dashboard export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-dir ./dashboards --overwrite";
pub(crate) const DASHBOARD_IMPORT_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard import --url http://localhost:3000 --basic-user admin --basic-password admin --input-dir ./dashboards/raw --replace-existing\n  grafana-util dashboard import --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --input-dir ./dashboards/raw --dry-run --table\n  grafana-util dashboard import --url http://localhost:3000 --basic-user admin --basic-password admin --input-dir ./dashboards/raw --interactive --replace-existing";
pub(crate) const DASHBOARD_ANALYZE_HELP_TEXT: &str = "What it does:\n  Analyze dashboards from live Grafana or a local export tree and render summary, dependency, governance, or queries-json outputs.\n\nWhen to use:\n  - Inspect a live environment before topology, governance-gate, or impact checks.\n  - Reuse a local export tree in CI without calling Grafana again.\n\nRelated commands:\n  - dashboard topology         Show which dashboards, variables, data sources, and alerts depend on each other.\n  - dashboard governance-gate  Check dashboard findings against a policy.\n  - dashboard list-vars        List one dashboard's current variables only.\n\nExamples:\n\n  Analyze live Grafana and render governance JSON:\n    grafana-util dashboard analyze --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format governance-json\n\n  Analyze a raw export tree without calling Grafana:\n    grafana-util dashboard analyze --input-dir ./dashboards/raw --input-format raw --output-format tree-table\n\n  Analyze a provisioning export tree:\n    grafana-util dashboard analyze --input-dir ./dashboards/provisioning --input-format provisioning --output-format governance";
pub(crate) const DASHBOARD_PATCH_FILE_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard patch-file --input ./dashboards/raw/cpu-main.json --name 'CPU Overview' --folder-uid infra --tag prod --tag sre\n  grafana-util dashboard patch-file --input ./drafts/cpu-main.json --output ./drafts/cpu-main-patched.json --uid cpu-main --message 'Add folder metadata before publish'";
pub(crate) const DASHBOARD_REVIEW_HELP_TEXT: &str = "What it does:\n  Review one local dashboard draft without touching Grafana and render the draft in text, YAML, or JSON form.\n\nWhen to use:\n  - Check a generated or edited draft before publish.\n  - Confirm folder, tags, UID, panels, and datasource references in CI or local review.\n\nRelated commands:\n  - dashboard fetch-live  Fetch a live dashboard into a local draft first.\n  - dashboard patch-file  Adjust the local draft before review.\n  - dashboard publish     Send the reviewed draft to Grafana.\n\nExamples:\n\n  grafana-util dashboard review --input ./drafts/cpu-main.json\n  grafana-util dashboard review --input ./drafts/cpu-main.json --output-format yaml";
pub(crate) const DASHBOARD_PUBLISH_HELP_TEXT: &str = "What it does:\n  Publish one local dashboard draft through the import pipeline, with dry-run support before any live write.\n\nWhen to use:\n  - Promote a reviewed draft back into Grafana.\n  - Reuse the same import semantics for one-off dashboard edits or generated drafts.\n\nRelated commands:\n  - dashboard review      Inspect the local draft before publish.\n  - dashboard fetch-live  Start from the current live dashboard state.\n  - dashboard clone-live  Prepare a new variant before publish.\n\nExamples:\n\n  grafana-util dashboard publish --url http://localhost:3000 --basic-user admin --basic-password admin --input ./drafts/cpu-main.json --folder-uid infra --message 'Promote CPU dashboard'\n  grafana-util dashboard publish --url http://localhost:3000 --basic-user admin --basic-password admin --input ./drafts/cpu-main.json --dry-run --table";
pub(crate) const DASHBOARD_DELETE_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard delete --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --uid cpu-main --dry-run --json\n  grafana-util dashboard delete --url http://localhost:3000 --basic-user admin --basic-password admin --path 'Platform / Infra' --yes\n  grafana-util dashboard delete --url http://localhost:3000 --interactive";
pub(crate) const DASHBOARD_INSPECT_EXPORT_HELP_TEXT: &str = "What it does:\n  Analyze a local dashboard export tree and build summary, dependency, governance, or query-analysis artifacts without calling live Grafana.\n\nWhen to use:\n  - Review exported dashboards in CI before import or publish.\n  - Feed topology, governance-gate, or impact workflows from an export tree.\n\nRelated commands:\n  - dashboard analyze          Build the same analysis artifacts directly from live Grafana or an export tree.\n  - dashboard topology         Render dependency graphs from the generated artifacts.\n  - dashboard governance-gate  Check policy against the generated artifacts.\n\nExamples:\n\n  grafana-util dashboard analyze --input-dir ./dashboards/raw --input-format raw --table\n  grafana-util dashboard analyze --input-dir ./dashboards/raw --input-format raw --interactive\n  grafana-util dashboard analyze --input-dir ./dashboards/raw --input-format raw --output-format governance-json\n  grafana-util dashboard analyze --input-dir ./dashboards/provisioning --input-format provisioning --output-format tree-table";
pub(crate) const DASHBOARD_INSPECT_LIVE_HELP_TEXT: &str = "What it does:\n  Analyze live Grafana dashboards and emit summary or machine-readable analysis artifacts without writing a persistent export tree.\n\nWhen to use:\n  - Inspect one live environment before governance, topology, or impact checks.\n  - Produce CI-friendly analysis artifacts straight from Grafana.\n\nRelated commands:\n  - dashboard analyze          Run the same analysis against live Grafana or an export tree.\n  - dashboard list            List lightweight dashboard inventory only.\n  - dashboard topology        Turn analysis artifacts into a dependency graph.\n\nExamples:\n\n  grafana-util dashboard analyze --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format governance-json\n  grafana-util dashboard analyze --url http://localhost:3000 --basic-user admin --basic-password admin --interactive";
pub(crate) const DASHBOARD_INSPECT_VARS_HELP_TEXT: &str = "What it does:\n  List dashboard templating variables and their current values from a live dashboard URL or UID, a local dashboard file, or a local export tree.\n\nWhen to use:\n  - Confirm which vars a dashboard expects before screenshot, publish, or troubleshooting.\n  - Check URL state and override values before browser-based capture.\n  - Inspect a rendered local dashboard file or export tree without calling Grafana.\n\nRelated commands:\n  - dashboard screenshot    Capture the dashboard after confirming vars.\n  - dashboard analyze       Build broader analysis artifacts for the same dashboard set.\n\nExamples:\n\n  grafana-util dashboard list-vars --dashboard-url 'https://grafana.example.com/d/cpu-main/cpu-overview?var-cluster=prod-a' --token \"$GRAFANA_API_TOKEN\" --output-format table\n  grafana-util dashboard list-vars --url https://grafana.example.com --dashboard-uid cpu-main --vars-query 'var-cluster=prod-a&var-instance=node01' --token \"$GRAFANA_API_TOKEN\" --output-format json\n  grafana-util dashboard list-vars --input ./dashboards/raw/cpu-main.json --output-format yaml\n  grafana-util dashboard list-vars --input-dir ./dashboards/raw --dashboard-uid cpu-main --output-format table";
pub(crate) const DASHBOARD_GOVERNANCE_GATE_HELP_TEXT: &str = "Examples:\n\n  Check live Grafana directly with a JSON/YAML policy file:\n    grafana-util dashboard governance-gate --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --policy-source file --policy ./policy.yaml\n\n  Check an export tree without calling Grafana:\n    grafana-util dashboard governance-gate --policy-source builtin --builtin-policy default --input-dir ./dashboards/raw --input-format raw\n\n  Reuse saved artifacts and write normalized JSON:\n    grafana-util dashboard governance-gate --policy-source builtin --builtin-policy default --governance ./governance.json --queries ./queries.json --output-format json --json-output ./governance-check.json";
pub(crate) const DASHBOARD_TOPOLOGY_HELP_TEXT: &str = "Examples:\n\n  Analyze live Grafana directly and render Mermaid:\n    grafana-util dashboard topology --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format mermaid\n\n  Analyze an export tree without calling Grafana:\n    grafana-util dashboard topology --input-dir ./dashboards/raw --input-format raw --output-format text\n\n  Reuse saved artifacts and render Graphviz DOT with the graph alias:\n    grafana-util dashboard graph --governance ./governance.json --queries ./queries.json --alert-contract ./alert-contract.json --output-format dot --output-file ./dashboard-topology.dot";
pub(crate) const DASHBOARD_SCREENSHOT_HELP_TEXT: &str = "Examples:\n\n  grafana-util dashboard screenshot --dashboard-url 'https://grafana.example.com/d/cpu-main/cpu-overview?var-cluster=prod-a' --token \"$GRAFANA_API_TOKEN\" --output ./cpu-main.png --full-page --header-title --header-url --header-captured-at\n  grafana-util dashboard screenshot --url https://grafana.example.com --dashboard-uid rYdddlPWk --panel-id 20 --vars-query 'var-datasource=prom-main&var-job=node-exporter&var-node=host01:9100' --token \"$GRAFANA_API_TOKEN\" --output ./panel.png --header-title 'CPU Busy' --header-text 'Solo panel debug capture'";
pub(crate) const SNAPSHOT_HELP_TEXT: &str = "Examples:\n\n  grafana-util snapshot export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-dir ./snapshot\n  grafana-util snapshot export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-dir ./snapshot --overwrite\n  grafana-util snapshot review --input-dir ./snapshot --output-format text\n  grafana-util snapshot review --input-dir ./snapshot --output-format json\n  grafana-util snapshot review --input-dir ./snapshot --interactive";

const OVERVIEW_HELP_SHAPE_NOTE: &str =
    "\nStaged overview is the default. Use `grafana-util overview live` to route into shared live status.\n";

const DASHBOARD_DIFF_SCHEMA_HELP_TEXT: &str = include_str!("../../schemas/help/diff/dashboard.txt");
const ALERT_DIFF_SCHEMA_HELP_TEXT: &str = include_str!("../../schemas/help/diff/alert.txt");
const DATASOURCE_DIFF_SCHEMA_HELP_TEXT: &str =
    include_str!("../../schemas/help/diff/datasource.txt");
const STATUS_SCHEMA_ROOT_HELP_TEXT: &str = include_str!("../../schemas/help/status/root.txt");
const STATUS_SCHEMA_STAGED_HELP_TEXT: &str = include_str!("../../schemas/help/status/staged.txt");
const STATUS_SCHEMA_LIVE_HELP_TEXT: &str = include_str!("../../schemas/help/status/live.txt");

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

fn render_migrate_help_text(colorize: bool) -> String {
    render_domain_help_text(MigrateCliArgs::command(), colorize)
}

fn render_migrate_help_full_text(colorize: bool) -> String {
    render_domain_help_text(MigrateCliArgs::command(), colorize)
}

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

pub fn render_unified_help_full_text(colorize: bool) -> String {
    let mut help = render_unified_help_text(colorize);
    if colorize {
        help.push_str(&colorize_help_examples(UNIFIED_HELP_FULL_TEXT));
    } else {
        help.push_str(UNIFIED_HELP_FULL_TEXT);
    }
    help
}

pub fn render_unified_version_text() -> String {
    crate::common::TOOL_VERSION_TEXT.to_string()
}

fn render_change_schema_help(target: Option<&str>) -> Option<String> {
    match target {
        None => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/change/root.help.txt"
            ))
            .to_string(),
        ),
        Some("summary") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/change/summary.help.txt"
            ))
            .to_string(),
        ),
        Some("plan") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/change/plan.help.txt"
            ))
            .to_string(),
        ),
        Some("review") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/change/review.help.txt"
            ))
            .to_string(),
        ),
        Some("apply") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/change/apply.help.txt"
            ))
            .to_string(),
        ),
        Some("audit") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/change/audit.help.txt"
            ))
            .to_string(),
        ),
        Some("preflight") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/change/preflight.help.txt"
            ))
            .to_string(),
        ),
        Some("assess-alerts") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/change/assess-alerts.help.txt"
            ))
            .to_string(),
        ),
        Some("bundle-preflight") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/change/bundle-preflight.help.txt"
            ))
            .to_string(),
        ),
        Some("promotion-preflight") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/change/promotion-preflight.help.txt"
            ))
            .to_string(),
        ),
        Some("bundle") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/change/bundle.help.txt"
            ))
            .to_string(),
        ),
        _ => None,
    }
}

fn render_dashboard_history_schema_help(target: Option<&str>) -> Option<String> {
    match target {
        None => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/dashboard-history/root.help.txt"
            ))
            .to_string(),
        ),
        Some("list") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/dashboard-history/list.help.txt"
            ))
            .to_string(),
        ),
        Some("restore") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/dashboard-history/restore.help.txt"
            ))
            .to_string(),
        ),
        Some("diff") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/dashboard-history/diff.help.txt"
            ))
            .to_string(),
        ),
        Some("export") => Some(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../schemas/help/dashboard-history/export.help.txt"
            ))
            .to_string(),
        ),
        _ => None,
    }
}

fn render_diff_schema_help(domain: &str) -> Option<String> {
    match domain {
        "dashboard" => Some(DASHBOARD_DIFF_SCHEMA_HELP_TEXT.to_string()),
        "alert" => Some(ALERT_DIFF_SCHEMA_HELP_TEXT.to_string()),
        "datasource" => Some(DATASOURCE_DIFF_SCHEMA_HELP_TEXT.to_string()),
        _ => None,
    }
}

fn render_status_schema_help(target: Option<&str>) -> Option<String> {
    match target {
        None => Some(STATUS_SCHEMA_ROOT_HELP_TEXT.to_string()),
        Some("staged") => Some(STATUS_SCHEMA_STAGED_HELP_TEXT.to_string()),
        Some("live") => Some(STATUS_SCHEMA_LIVE_HELP_TEXT.to_string()),
        _ => None,
    }
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
    if args.get(1).map(String::as_str) == Some("migrate") {
        if let Some(rendered) = maybe_render_migrate_help_from_os_args(args.clone(), colorize) {
            return Some(rendered);
        }
    }
    if args.len() >= 3
        && args.get(1).map(String::as_str) == Some("change")
        && args.iter().any(|value| value == "--help-schema")
    {
        let target = args
            .get(2)
            .filter(|value| !value.starts_with('-'))
            .map(String::as_str);
        return render_change_schema_help(target);
    }
    if args.len() >= 4
        && args.get(1).map(String::as_str) == Some("dashboard")
        && args.get(2).map(String::as_str) == Some("history")
        && args.iter().any(|value| value == "--help-schema")
    {
        let target = args
            .get(3)
            .filter(|value| !value.starts_with('-'))
            .map(String::as_str);
        return render_dashboard_history_schema_help(target);
    }
    if args.len() >= 4
        && args.get(1).map(String::as_str) == Some("dashboard")
        && args.get(2).map(String::as_str) == Some("diff")
        && args.iter().any(|value| value == "--help-schema")
    {
        return render_diff_schema_help("dashboard");
    }
    if args.len() >= 4
        && args.get(1).map(String::as_str) == Some("alert")
        && args.get(2).map(String::as_str) == Some("diff")
        && args.iter().any(|value| value == "--help-schema")
    {
        return render_diff_schema_help("alert");
    }
    if args.len() >= 4
        && args.get(1).map(String::as_str) == Some("datasource")
        && args.get(2).map(String::as_str) == Some("diff")
        && args.iter().any(|value| value == "--help-schema")
    {
        return render_diff_schema_help("datasource");
    }
    if args.len() >= 3
        && args.get(1).map(String::as_str) == Some("status")
        && args.iter().any(|value| value == "--help-schema")
    {
        let target = args
            .get(2)
            .filter(|value| !value.starts_with('-'))
            .map(String::as_str);
        return render_status_schema_help(target);
    }
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
            Some(render_domain_help_text(datasource_root_command(), colorize))
        }
        [_binary, command, flag] if command == "access" && (flag == "--help" || flag == "-h") => {
            Some(render_domain_help_text(access_root_command(), colorize))
        }
        [_binary, command, flag] if command == "profile" && (flag == "--help" || flag == "-h") => {
            Some(render_domain_help_text(profile_root_command(), colorize))
        }
        [_binary, command, flag] if command == "snapshot" && (flag == "--help" || flag == "-h") => {
            Some(render_domain_help_text(snapshot_root_command(), colorize))
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
        [_binary, command, flag] if command == "migrate" && (flag == "--help" || flag == "-h") => {
            Some(render_migrate_help_text(colorize))
        }
        [_binary, command, flag]
            if command == "dashboard" && (flag == "--help" || flag == "-h") =>
        {
            Some(if colorize {
                colorize_dashboard_short_help(UNIFIED_DASHBOARD_SHORT_HELP_TEXT)
            } else {
                UNIFIED_DASHBOARD_SHORT_HELP_TEXT.to_string()
            })
        }
        [_binary, command, flag] if command == "alert" && flag == "--help-full" => Some(
            render_domain_help_full_text(alert_root_command(), ALERT_HELP_FULL_TEXT, colorize),
        ),
        [_binary, command, flag] if command == "datasource" && flag == "--help-full" => {
            Some(render_domain_help_full_text(
                datasource_root_command(),
                DATASOURCE_HELP_FULL_TEXT,
                colorize,
            ))
        }
        [_binary, command, flag] if command == "access" && flag == "--help-full" => Some(
            render_domain_help_full_text(access_root_command(), ACCESS_HELP_FULL_TEXT, colorize),
        ),
        [_binary, command, flag] if command == "profile" && flag == "--help-full" => {
            Some(render_domain_help_text(profile_root_command(), colorize))
        }
        [_binary, command, flag] if command == "snapshot" && flag == "--help-full" => {
            Some(render_domain_help_text(snapshot_root_command(), colorize))
        }
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
        [_binary, command, flag] if command == "migrate" && flag == "--help-full" => {
            Some(render_migrate_help_full_text(colorize))
        }
        _ => None,
    }
}
