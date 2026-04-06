//! CLI definitions for Dashboard command surface and option compatibility behavior.

use crate::common::CliColorChoice;
use clap::{Parser, Subcommand, ValueEnum};

use super::cli_defs_inspect::{
    AnalyzeArgs, GovernanceGateArgs, ImpactArgs, InspectExportArgs, InspectLiveArgs,
    InspectVarsArgs, ScreenshotArgs, TopologyArgs, ValidateExportArgs,
};

#[path = "cli_defs_command_export.rs"]
mod cli_defs_command_export;
#[path = "cli_defs_command_list.rs"]
mod cli_defs_command_list;
#[path = "cli_defs_command_live.rs"]
mod cli_defs_command_live;
#[path = "cli_defs_command_local.rs"]
mod cli_defs_command_local;
#[path = "cli_defs_command_history.rs"]
mod cli_defs_command_history;

pub use cli_defs_command_export::{ExportArgs, RawToPromptArgs};
pub use cli_defs_command_history::{
    DashboardHistoryArgs, DashboardHistorySubcommand, HistoryExportArgs, HistoryListArgs,
    HistoryRestoreArgs,
};
pub use cli_defs_command_list::ListArgs;
pub use cli_defs_command_live::{BrowseArgs, CloneLiveArgs, DeleteArgs, DiffArgs, EditLiveArgs, GetArgs};
pub use cli_defs_command_local::{ImportArgs, PatchFileArgs, PublishArgs, ReviewArgs, ServeArgs};

/// Arguments for importing dashboards from a local export directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DashboardImportInputFormat {
    Raw,
    Provisioning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum InspectExportInputType {
    Raw,
    Source,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DashboardServeScriptFormat {
    Json,
    Yaml,
}

/// Enum definition for DashboardCommand.
#[derive(Debug, Clone, Subcommand)]
pub enum DashboardCommand {
    #[command(
        name = "list",
        about = "List dashboard summaries without writing export files.",
        after_help = "Examples:\n\n  List dashboards from the current org with Basic auth:\n    grafana-util dashboard list --url http://localhost:3000 --basic-user admin --basic-password admin\n\n  List dashboards across all visible orgs with Basic auth:\n    grafana-util dashboard list --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --json\n\n  List dashboards from one explicit org ID:\n    grafana-util dashboard list --url http://localhost:3000 --basic-user admin --basic-password admin --org-id 2 --csv\n\n  List dashboards from the current org with an API token:\n    grafana-util dashboard list --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --json"
    )]
    List(ListArgs),
    #[command(
        name = "fetch-live",
        about = "Fetch one live dashboard into an API-safe local JSON draft.",
        after_help = "What it does:\n  Fetch one live dashboard and write an API-safe local draft file without mutating Grafana.\n\nWhen to use:\n  - Start a local edit or review flow from the current live dashboard.\n  - Capture one dashboard before patching, diffing, or publishing locally.\n\nRelated commands:\n  - dashboard clone-live  Fetch then override title, UID, or folder metadata.\n  - dashboard review      Inspect one local draft before publish.\n  - dashboard publish     Send one reviewed local draft back to Grafana.\n\nExamples:\n\n  Fetch one live dashboard and write a local draft file:\n    grafana-util dashboard fetch-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --dashboard-uid cpu-main --output ./cpu-main.json\n\n  Fetch one dashboard with Basic auth and a saved profile:\n    grafana-util dashboard fetch-live --profile prod --url http://localhost:3000 --basic-user admin --basic-password admin --dashboard-uid cpu-main --output ./cpu-main.json"
    )]
    Get(GetArgs),
    #[command(
        name = "clone-live",
        about = "Clone one live dashboard into a local draft with optional overrides.",
        after_help = "What it does:\n  Fetch one live dashboard into a local draft and optionally override title, UID, or folder metadata before saving it.\n\nWhen to use:\n  - Fork a live dashboard into a new draft for another folder, environment, or owner.\n  - Prepare a publishable variant without mutating the source dashboard first.\n\nRelated commands:\n  - dashboard fetch-live  Fetch the live dashboard without changing any metadata.\n  - dashboard patch-file  Adjust the local draft after the initial clone step.\n  - dashboard publish     Push the reviewed clone into Grafana.\n\nExamples:\n\n  Clone one live dashboard, keep the source UID and title, and write a local draft:\n    grafana-util dashboard clone-live --url http://localhost:3000 --basic-user admin --basic-password admin --source-uid cpu-main --output ./cpu-main-clone.json\n\n  Clone a live dashboard with a new title, UID, and folder UID:\n    grafana-util dashboard clone-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --source-uid cpu-main --name 'CPU Clone' --uid cpu-main-clone --folder-uid infra --output ./cpu-main-clone.json"
    )]
    CloneLive(CloneLiveArgs),
    #[command(
        name = "serve",
        about = "Serve dashboard drafts through a local preview server.",
        after_help = "Examples:\n\n  Serve one local draft file and open the browser:\n    grafana-util dashboard serve --input ./drafts/cpu-main.json --open-browser\n\n  Serve a directory of dashboard drafts:\n    grafana-util dashboard serve --input ./dashboards/raw\n\n  Serve one generated dashboard and watch the generator inputs:\n    grafana-util dashboard serve --script 'jsonnet dashboards/cpu.jsonnet' --watch ./dashboards --watch ./lib"
    )]
    Serve(ServeArgs),
    #[command(
        name = "edit-live",
        about = "Fetch one live dashboard into an external editor, review the edited draft, and save the result as a local draft or explicit live writeback.",
        after_help = "Examples:\n\n  Edit one live dashboard and write the result to the default local draft path:\n    grafana-util dashboard edit-live --url http://localhost:3000 --basic-user admin --basic-password admin --dashboard-uid cpu-main\n\n  Edit one live dashboard into an explicit output file:\n    grafana-util dashboard edit-live --profile prod --dashboard-uid cpu-main --output ./drafts/cpu-main.edited.json\n\n  Edit one live dashboard and write it back to Grafana after explicit acknowledgement:\n    grafana-util dashboard edit-live --profile prod --dashboard-uid cpu-main --apply-live --yes --message 'Hotfix CPU dashboard'\n\n  Edit one live dashboard and inspect the review output before deciding whether to publish:\n    grafana-util dashboard edit-live --profile prod --dashboard-uid cpu-main"
    )]
    EditLive(EditLiveArgs),
    #[command(
        name = "export",
        about = "Export dashboards to raw/, prompt/, provisioning/, and optional history/ files.",
        after_help = "The provisioning export writes a Grafana file-provisioning provider file at provisioning/provisioning/dashboards.yaml. Override the provider name, org ID, path, or update behavior when you need a different on-disk deployment target. Add --include-history when you also want per-dashboard revision history under history/ for each exported scope.\n\nExamples:\n\n  Export dashboards from the current org with Basic auth and history artifacts:\n    grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --output-dir ./dashboards --overwrite --include-history\n\n  Export dashboards across all visible orgs with Basic auth and history artifacts:\n    grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --output-dir ./dashboards --overwrite --include-history\n\n  Export dashboards with a custom provisioning provider path:\n    grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --output-dir ./dashboards --overwrite --provisioning-provider-name grafana-utils-prod --provisioning-provider-org-id 2 --provisioning-provider-path /srv/grafana/dashboards --provisioning-provider-disable-deletion --provisioning-provider-update-interval-seconds 60\n\n  Export dashboards from one explicit org ID:\n    grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --org-id 2 --output-dir ./dashboards --overwrite\n\n  Export dashboards from the current org with an API token:\n    export GRAFANA_API_TOKEN='your-token'\n    grafana-util dashboard export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-dir ./dashboards --overwrite"
    )]
    Export(ExportArgs),
    #[command(
        name = "raw-to-prompt",
        about = "Convert raw dashboard exports into prompt lane artifacts.",
        after_help = "Examples:\n\n  Convert one raw dashboard file and rely on the sibling .prompt.json target:\n    grafana-util dashboard raw-to-prompt --input-file ./dashboards/raw/cpu-main.json\n\n  Convert one raw export root into a sibling prompt/ lane:\n    grafana-util dashboard raw-to-prompt --input-dir ./dashboards/raw --output-dir ./dashboards/prompt --overwrite\n\n  Convert a raw file with explicit datasource resolution settings:\n    grafana-util dashboard raw-to-prompt --input-file ./dashboards/raw/cpu-main.json --datasource-map ./datasource-map.json --resolution exact --output-format json\n\n  Augment datasource resolution with live lookup from a profile:\n    grafana-util dashboard raw-to-prompt --input-file ./dashboards/raw/cpu-main.json --profile prod --org-id 2"
    )]
    RawToPrompt(RawToPromptArgs),
    #[command(
        name = "import",
        about = "Import dashboard JSON files through the Grafana API.",
        after_help = "Examples:\n\n  Import one raw export directory into the current org:\n    grafana-util dashboard import --url http://localhost:3000 --basic-user admin --basic-password admin --input-dir ./dashboards/raw --replace-existing\n\n  Preview import actions without changing Grafana:\n    grafana-util dashboard import --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --input-dir ./dashboards/raw --dry-run --table\n\n  Interactively choose exported dashboards to restore/import:\n    grafana-util dashboard import --url http://localhost:3000 --basic-user admin --basic-password admin --input-dir ./dashboards/raw --interactive --replace-existing"
    )]
    Import(ImportArgs),
    #[command(
        name = "browse",
        about = "Browse live Grafana or a local export tree in an interactive terminal UI.",
        after_help = "Examples:\n\n  Browse the full dashboard tree from the current org:\n    grafana-util dashboard browse --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\"\n\n  Open the browser at one folder subtree:\n    grafana-util dashboard browse --url http://localhost:3000 --basic-user admin --basic-password admin --path 'Platform / Infra'\n\n  Browse a raw export tree from disk:\n    grafana-util dashboard browse --input-dir ./dashboards/raw --path 'Platform / Infra'\n\n  Browse one explicit org:\n    grafana-util dashboard browse --url http://localhost:3000 --basic-user admin --basic-password admin --org-id 2\n\n  Browse all visible orgs with Basic auth:\n    grafana-util dashboard browse --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs"
    )]
    Browse(BrowseArgs),
    #[command(
        name = "history",
        about = "List or restore dashboard revision history from Grafana."
    )]
    History(DashboardHistoryArgs),
    #[command(
        name = "delete",
        about = "Delete live dashboards by UID or folder path.",
        after_help = "Examples:\n\n  Dry-run one dashboard delete by UID:\n    grafana-util dashboard delete --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --uid cpu-main --dry-run --json\n\n  Delete all dashboards under one folder subtree:\n    grafana-util dashboard delete --url http://localhost:3000 --basic-user admin --basic-password admin --path 'Platform / Infra' --yes\n\n  Interactively preview and confirm a folder delete:\n    grafana-util dashboard delete --url http://localhost:3000 --interactive"
    )]
    Delete(DeleteArgs),
    #[command(
        about = "Compare local dashboard files against live Grafana dashboards.",
        after_help = "Examples:\n\n  Compare one raw export directory against the current org:\n    grafana-util dashboard diff --url http://localhost:3000 --basic-user admin --basic-password admin --input-dir ./dashboards/raw\n\n  Compare a provisioning export root against the current org:\n    grafana-util dashboard diff --url http://localhost:3000 --basic-user admin --basic-password admin --input-dir ./dashboards/provisioning --input-format provisioning\n\n  Compare against one explicit org as structured JSON:\n    grafana-util dashboard diff --url http://localhost:3000 --basic-user admin --basic-password admin --org-id 2 --input-dir ./dashboards/raw --json"
    )]
    Diff(DiffArgs),
    #[command(
        name = "patch-file",
        about = "Patch one local dashboard JSON file in place or to a new path.",
        after_help = "Examples:\n\n  Patch a raw export file in place:\n    grafana-util dashboard patch-file --input ./dashboards/raw/cpu-main.json --name 'CPU Overview' --folder-uid infra --tag prod --tag sre\n\n  Patch one draft file into a new output path:\n    grafana-util dashboard patch-file --input ./drafts/cpu-main.json --output ./drafts/cpu-main-patched.json --uid cpu-main --message 'Add folder metadata before publish'\n\n  Patch one dashboard from standard input into an explicit output file:\n    jsonnet dashboards/cpu.jsonnet | grafana-util dashboard patch-file --input - --output ./drafts/cpu-main.json --folder-uid infra"
    )]
    PatchFile(PatchFileArgs),
    #[command(
        name = "review",
        about = "Review one local dashboard JSON file without touching Grafana.",
        after_help = "What it does:\n  Review one local dashboard draft without touching Grafana and render the draft in text, YAML, or JSON form.\n\nWhen to use:\n  - Check a generated or edited draft before publish.\n  - Confirm folder, tags, UID, panels, and datasource references in CI or local review.\n\nRelated commands:\n  - dashboard fetch-live  Fetch a live dashboard into a local draft first.\n  - dashboard patch-file  Adjust the local draft before review.\n  - dashboard publish     Send the reviewed draft to Grafana.\n\nExamples:\n\n  Review one local dashboard file in text mode:\n    grafana-util dashboard review --input ./drafts/cpu-main.json\n\n  Review one local dashboard file as YAML:\n    grafana-util dashboard review --input ./drafts/cpu-main.json --output-format yaml\n\n  Review one generated dashboard from standard input:\n    jsonnet dashboards/cpu.jsonnet | grafana-util dashboard review --input - --output-format json"
    )]
    Review(ReviewArgs),
    #[command(
        name = "publish",
        about = "Publish one local dashboard JSON file through the existing dashboard import pipeline.",
        after_help = "What it does:\n  Publish one local dashboard draft through the import pipeline, with dry-run support before any live write.\n\nWhen to use:\n  - Promote a reviewed draft back into Grafana.\n  - Reuse the same import semantics for one-off dashboard edits or generated drafts.\n\nRelated commands:\n  - dashboard review      Inspect the local draft before publish.\n  - dashboard fetch-live  Start from the current live dashboard state.\n  - dashboard clone-live  Prepare a new variant before publish.\n\nExamples:\n\n  Publish one draft file to the current Grafana org:\n    grafana-util dashboard publish --url http://localhost:3000 --basic-user admin --basic-password admin --input ./drafts/cpu-main.json --folder-uid infra --message 'Promote CPU dashboard'\n\n  Preview the same publish without writing to Grafana:\n    grafana-util dashboard publish --url http://localhost:3000 --basic-user admin --basic-password admin --input ./drafts/cpu-main.json --dry-run --table\n\n  Publish one generated dashboard from standard input:\n    jsonnet dashboards/cpu.jsonnet | grafana-util dashboard publish --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --input - --replace-existing\n\n  Watch one local draft file and rerun dry-run after each save:\n    grafana-util dashboard publish --url http://localhost:3000 --basic-user admin --basic-password admin --input ./drafts/cpu-main.json --dry-run --watch"
    )]
    Publish(PublishArgs),
    #[command(
        name = "analyze",
        about = "Analyze dashboards from live Grafana or a local export tree and build summary or governance artifacts.",
        after_help = "What it does:\n  Analyze dashboards from live Grafana or a local export tree and render summary, governance, dependency, or queries-json outputs.\n\nWhen to use:\n  - Inspect a live environment before topology, governance-gate, or impact checks.\n  - Reuse a local export tree in CI without calling Grafana again.\n\nRelated commands:\n  - dashboard topology         Show which dashboards, variables, data sources, and alerts depend on each other.\n  - dashboard governance-gate  Check dashboard findings against a policy.\n  - dashboard list-vars        List one dashboard's current variables only.\n\nExamples:\n\n  Analyze live Grafana and render governance JSON:\n    grafana-util dashboard analyze --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format governance-json\n\n  Analyze a raw export tree without calling Grafana:\n    grafana-util dashboard analyze --input-dir ./dashboards/raw --input-format raw --output-format tree-table\n\n  Analyze a provisioning export tree:\n    grafana-util dashboard analyze --input-dir ./dashboards/provisioning --input-format provisioning --output-format governance\n"
    )]
    Analyze(AnalyzeArgs),
    #[command(
        name = "analyze-export",
        hide = true,
        alias = "inspect-export",
        about = "Analyze dashboard export directories with operator-summary, governance, dependency, and queries-json views.",
        after_help = "Examples:\n\n  Render an operator-summary table from raw exports:\n    grafana-util dashboard analyze-export --input-dir ./dashboards/raw --input-format raw --output-format table\n\n  Open the interactive inspect workbench over raw exports:\n    grafana-util dashboard analyze-export --input-dir ./dashboards/raw --input-format raw --interactive\n\n  Render the machine-readable governance artifact from raw exports:\n    grafana-util dashboard analyze-export --input-dir ./dashboards/raw --input-format raw --output-format governance-json\n\n  Render the queries-json artifact from raw exports:\n    grafana-util dashboard analyze-export --input-dir ./dashboards/raw --input-format raw --output-format queries-json\n\n  Inspect a file-provisioning tree from the provisioning root:\n    grafana-util dashboard analyze-export --input-dir ./dashboards/provisioning --input-format provisioning --output-format tree-table"
    )]
    InspectExport(InspectExportArgs),
    #[command(
        name = "analyze-live",
        hide = true,
        alias = "inspect-live",
        about = "Analyze live Grafana dashboards via a temporary raw-export snapshot.",
        after_help = "Examples:\n\n  Render governance JSON from live Grafana:\n    grafana-util dashboard analyze-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format governance-json\n\n  Render the queries-json artifact from live Grafana:\n    grafana-util dashboard analyze-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format queries-json\n\n  Open the interactive inspect workbench over live Grafana:\n    grafana-util dashboard analyze-live --url http://localhost:3000 --basic-user admin --basic-password admin --interactive"
    )]
    InspectLive(InspectLiveArgs),
    #[command(
        name = "list-vars",
        alias = "inspect-vars",
        about = "List dashboard templating variables and datasource-like choices from live Grafana or a local dashboard file.",
        after_help = "Examples:\n\n  List variables from a browser URL directly:\n    grafana-util dashboard list-vars --dashboard-url 'https://grafana.example.com/d/cpu-main/cpu-overview?var-cluster=prod-a' --token \"$GRAFANA_API_TOKEN\" --output-format table\n\n  List one dashboard UID with a vars-query fragment:\n    grafana-util dashboard list-vars --url https://grafana.example.com --dashboard-uid cpu-main --vars-query 'var-cluster=prod-a&var-instance=node01' --token \"$GRAFANA_API_TOKEN\" --output-format json\n\n  List variables from one local dashboard JSON file:\n    grafana-util dashboard list-vars --input ./dashboards/raw/cpu-main.json --output-format yaml\n\n  List variables from one local export tree:\n    grafana-util dashboard list-vars --input-dir ./dashboards/raw --dashboard-uid cpu-main --output-format table"
    )]
    InspectVars(InspectVarsArgs),
    #[command(
        name = "governance-gate",
        about = "Check dashboard findings against a policy from live Grafana or a local export tree.",
        after_help = "Examples:\n\n  Check live Grafana directly with a JSON/YAML policy file:\n    grafana-util dashboard governance-gate --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --policy-source file --policy ./policy.yaml\n\n  Check an export tree without calling Grafana:\n    grafana-util dashboard governance-gate --policy-source builtin --builtin-policy default --input-dir ./dashboards/raw --input-format raw\n\n  Advanced reuse: recheck saved analysis artifacts and write normalized JSON:\n    grafana-util dashboard governance-gate --policy-source builtin --builtin-policy default --governance ./governance.json --queries ./queries.json --output-format json --json-output ./governance-check.json"
    )]
    GovernanceGate(GovernanceGateArgs),
    #[command(
        name = "topology",
        visible_alias = "graph",
        about = "Show dashboard dependencies directly from live Grafana or a local export tree.",
        after_help = "Examples:\n\n  Analyze live Grafana directly and render Mermaid:\n    grafana-util dashboard topology --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-format mermaid\n\n  Analyze an export tree without calling Grafana:\n    grafana-util dashboard topology --input-dir ./dashboards/raw --input-format raw --output-format text\n\n  Advanced reuse: render Graphviz DOT through the graph alias from saved analysis artifacts:\n    grafana-util dashboard graph --governance ./governance.json --queries ./queries.json --alert-contract ./alert-contract.json --output-format dot --output-file ./dashboard-topology.dot"
    )]
    Topology(TopologyArgs),
    #[command(
        name = "impact",
        about = "Show which dashboards and alert resources would be affected by one data source from live Grafana, an export tree, or saved artifacts.",
        after_help = "Examples:\n\n  Check blast radius directly from live Grafana:\n    grafana-util dashboard impact --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --datasource-uid prom-main --output-format text\n\n  Check blast radius from an export tree:\n    grafana-util dashboard impact --input-dir ./dashboards/raw --input-format raw --datasource-uid prom-main --output-format json\n\n  Reuse saved artifacts and add alert contract context:\n    grafana-util dashboard impact --governance ./governance.json --queries ./queries.json --datasource-uid prom-main --alert-contract ./alert-contract.json --output-format json"
    )]
    Impact(ImpactArgs),
    #[command(
        name = "validate-export",
        about = "Run strict schema validation against dashboard raw export files before GitOps sync.",
        after_help = "Examples:\n\n  Validate a raw export and fail on migration or plugin issues:\n    grafana-util dashboard validate-export --input-dir ./dashboards/raw --reject-custom-plugins --reject-legacy-properties --target-schema-version 39\n\n  Validate a provisioning export root explicitly:\n    grafana-util dashboard validate-export --input-dir ./dashboards/provisioning --input-format provisioning --reject-custom-plugins\n\n  Write the validation report as JSON:\n    grafana-util dashboard validate-export --input-dir ./dashboards/raw --output-format json --output-file ./dashboard-validation.json"
    )]
    ValidateExport(ValidateExportArgs),
    #[command(
        name = "screenshot",
        about = "Open one Grafana dashboard in a headless browser and capture PNG, JPEG, or PDF output.",
        after_help = "Examples:\n\n  Capture a full dashboard from a browser URL and add an auto title/header block:\n    grafana-util dashboard screenshot --dashboard-url 'https://grafana.example.com/d/cpu-main/cpu-overview?var-cluster=prod-a' --token \"$GRAFANA_API_TOKEN\" --output ./cpu-main.png --full-page --header-title --header-url --header-captured-at\n\n  Capture a solo panel with a vars-query fragment and custom header note:\n    grafana-util dashboard screenshot --url https://grafana.example.com --dashboard-uid rYdddlPWk --panel-id 20 --vars-query 'var-datasource=prom-main&var-job=node-exporter&var-node=host01:9100' --token \"$GRAFANA_API_TOKEN\" --output ./panel.png --header-title 'CPU Busy' --header-text 'Solo panel debug capture'"
    )]
    Screenshot(ScreenshotArgs),
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn dashboard_history_restore_help_mentions_same_dashboard() {
        let command = DashboardCliArgs::command();
        let history = command.find_subcommand("history").unwrap();
        let restore = history.find_subcommand("restore").unwrap();
        let about = restore.get_about().unwrap().to_string();
        assert!(about.contains("same dashboard"));
    }
}

#[derive(Debug, Clone, Parser)]
#[command(
    about = "Export or import Grafana dashboards.",
    after_help = "Examples:\n\n  Fetch one live dashboard into a local draft:\n    grafana-util dashboard fetch-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --dashboard-uid cpu-main --output ./cpu-main.json\n\n  Clone one live dashboard with a new UID and folder:\n    grafana-util dashboard clone-live --url http://localhost:3000 --basic-user admin --basic-password admin --source-uid cpu-main --uid cpu-main-clone --folder-uid infra --output ./cpu-main-clone.json\n\n  Export dashboards from local Grafana with Basic auth:\n    grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --output-dir ./dashboards --overwrite\n\n  Export dashboards across all visible orgs with Basic auth:\n    grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --output-dir ./dashboards --overwrite\n\n  List dashboards across all visible orgs with Basic auth:\n    grafana-util dashboard list --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --json\n\n  Export dashboards with an API token from the current org:\n    export GRAFANA_API_TOKEN='your-token'\n    grafana-util dashboard export --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output-dir ./dashboards --overwrite\n\n  Compare raw dashboard exports against local Grafana:\n    grafana-util dashboard diff --url http://localhost:3000 --basic-user admin --basic-password admin --input-dir ./dashboards/raw\n\n  Patch a local dashboard file before publishing:\n    grafana-util dashboard patch-file --input ./dashboards/raw/cpu-main.json --name 'CPU Overview' --folder-uid infra --tag prod --tag sre\n\n  Publish one local draft to Grafana:\n    grafana-util dashboard publish --url http://localhost:3000 --basic-user admin --basic-password admin --input ./drafts/cpu-main.json --dry-run --table\n\n  Capture a browser-rendered dashboard screenshot:\n    grafana-util dashboard screenshot --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --dashboard-uid cpu-main --output ./cpu-main.png --from now-6h --to now",
    styles = crate::help_styles::CLI_HELP_STYLES
)]
/// Struct definition for DashboardCliArgs.
pub struct DashboardCliArgs {
    #[arg(
        long,
        value_enum,
        default_value_t = CliColorChoice::Auto,
        help = "Colorize JSON output. Use auto, always, or never."
    )]
    pub color: CliColorChoice,
    #[command(subcommand)]
    pub command: DashboardCommand,
}
