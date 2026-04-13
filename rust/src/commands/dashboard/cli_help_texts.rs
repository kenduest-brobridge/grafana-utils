//! Long-form dashboard CLI help text constants.

pub(crate) const DASHBOARD_LIST_AFTER_HELP: &str = r#"Examples:

  List dashboards from the current org with Basic auth:
    grafana-util dashboard list --url http://localhost:3000 --basic-user admin --basic-password admin

  List dashboards across all visible orgs with Basic auth:
    grafana-util dashboard list --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --json

  List dashboards from one explicit org ID:
    grafana-util dashboard list --url http://localhost:3000 --basic-user admin --basic-password admin --org-id 2 --csv

  List dashboards from the current org with an API token:
    grafana-util dashboard list --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --json"#;

pub(crate) const DASHBOARD_FETCH_LIVE_AFTER_HELP: &str = r#"What it does:
  Fetch one live dashboard and write an API-safe local draft file without mutating Grafana.

When to use:
  - Start a local edit or review flow from the current live dashboard.
  - Capture one dashboard before patching, diffing, or publishing locally.

Related commands:
  - dashboard clone  Fetch then override title, UID, or folder metadata.
  - dashboard review      Inspect one local draft before publish.
  - dashboard publish     Send one reviewed local draft back to Grafana.

Examples:

  Fetch one live dashboard and write a local draft file:
    grafana-util dashboard get --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --dashboard-uid cpu-main --output ./cpu-main.json

  Fetch one dashboard with Basic auth and a saved profile:
    grafana-util dashboard get --profile prod --url http://localhost:3000 --basic-user admin --basic-password admin --dashboard-uid cpu-main --output ./cpu-main.json"#;

pub(crate) const DASHBOARD_CLONE_LIVE_AFTER_HELP: &str = r#"What it does:
  Fetch one live dashboard into a local draft and optionally override title, UID, or folder metadata before saving it.

When to use:
  - Fork a live dashboard into a new draft for another folder, environment, or owner.
  - Prepare a publishable variant without mutating the source dashboard first.

Related commands:
  - dashboard get    Fetch the live dashboard without changing any metadata.
  - dashboard patch  Adjust the local draft after the initial clone step.
  - dashboard publish     Push the reviewed clone into Grafana.

Examples:

  Clone one live dashboard, keep the source UID and title, and write a local draft:
    grafana-util dashboard clone --url http://localhost:3000 --basic-user admin --basic-password admin --source-uid cpu-main --output ./cpu-main-clone.json

  Clone a live dashboard with a new title, UID, and folder UID:
    grafana-util dashboard clone --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --source-uid cpu-main --name 'CPU Clone' --uid cpu-main-clone --folder-uid infra --output ./cpu-main-clone.json"#;

pub(crate) const DASHBOARD_SERVE_AFTER_HELP: &str = r#"Examples:

  Serve one local draft file and open the browser:
    grafana-util dashboard serve --input ./drafts/cpu-main.json --open-browser

  Serve a directory of dashboard drafts:
    grafana-util dashboard serve --input ./dashboards/raw

  Serve one generated dashboard and watch the generator inputs:
    grafana-util dashboard serve --script 'jsonnet dashboards/cpu.jsonnet' --watch ./dashboards --watch ./lib"#;

pub(crate) const DASHBOARD_EDIT_LIVE_AFTER_HELP: &str = r#"What it does:
  Fetch one live dashboard into an external editor, then return a review output that drives preview, save, or live apply.

Examples:

  Edit one live dashboard and immediately preview the live publish without saving a draft file:
    grafana-util dashboard edit-live --url http://localhost:3000 --basic-user admin --basic-password admin --dashboard-uid cpu-main

  Edit one live dashboard into an explicit output file:
    grafana-util dashboard edit-live --profile prod --dashboard-uid cpu-main --output ./drafts/cpu-main.edited.json

  Edit one live dashboard, save the local draft, and immediately preview the publish:
    grafana-util dashboard edit-live --profile prod --dashboard-uid cpu-main --output ./drafts/cpu-main.edited.json --publish-dry-run

  Edit one live dashboard and write it back to Grafana after explicit acknowledgement:
    grafana-util dashboard edit-live --profile prod --dashboard-uid cpu-main --apply-live --yes --message 'Hotfix CPU dashboard'

  Edit one live dashboard, inspect the preview, then decide whether to publish manually:
    grafana-util dashboard edit-live --profile prod --dashboard-uid cpu-main"#;

pub(crate) const DASHBOARD_EXPORT_AFTER_HELP: &str = r#"Notes:
  - Writes raw/, prompt/, and provisioning/ by default.
  - Use Basic auth with --all-orgs.
  - Use --flat for files directly under each variant directory.
  - Use --include-history to add history/ under each exported org scope.
  - The provider file is provisioning/provisioning/dashboards.yaml.
  - Keep raw/ for API import or diff, prompt/ for UI import, and provisioning/ for file provisioning.

Examples:

  Export dashboards from the current org with Basic auth and history artifacts:
    grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --output-dir ./dashboards --overwrite --include-history

  Export dashboards across all visible orgs with Basic auth and history artifacts:
    grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --output-dir ./dashboards --overwrite --include-history

  Export dashboards with a custom provisioning provider path:
    grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --output-dir ./dashboards --overwrite --provisioning-provider-name grafana-utils-prod --provisioning-provider-org-id 2 --provisioning-provider-path /srv/grafana/dashboards --provisioning-provider-disable-deletion --provisioning-provider-update-interval-seconds 60

  Export dashboards from one explicit org ID:
    grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --org-id 2 --output-dir ./dashboards --overwrite

  Export dashboards from the current org with an API token:
    export GRAFANA_API_TOKEN='your-token'
    grafana-util dashboard export --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-dir ./dashboards --overwrite"#;

pub(crate) const DASHBOARD_IMPORT_AFTER_HELP: &str = r#"Examples:

  Import one raw export directory into the current org:
    grafana-util dashboard import --url http://localhost:3000 --basic-user admin --basic-password admin --input-dir ./dashboards/raw --replace-existing

  Preview import actions without changing Grafana:
    grafana-util dashboard import --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --input-dir ./dashboards/raw --dry-run --table

  Interactively choose exported dashboards to restore/import:
    grafana-util dashboard import --url http://localhost:3000 --basic-user admin --basic-password admin --input-dir ./dashboards/raw --interactive --replace-existing"#;

pub(crate) const DASHBOARD_BROWSE_AFTER_HELP: &str = r#"What it does:
  Browse the dashboard tree in a terminal UI, with live-only actions for edit, raw JSON review/apply, history, and delete.

Examples:

  Browse the full dashboard tree from the current org:
    grafana-util dashboard browse --url http://localhost:3000 --token "$GRAFANA_API_TOKEN"

  Open the browser at one folder subtree:
    grafana-util dashboard browse --url http://localhost:3000 --basic-user admin --basic-password admin --path 'Platform / Infra'

  Browse a raw export tree from disk:
    grafana-util dashboard browse --input-dir ./dashboards/raw --path 'Platform / Infra'

  Browse one repo-backed workspace root from disk:
    grafana-util dashboard browse --workspace ./grafana-oac-repo --path 'Platform / Infra'

  Browse one explicit org:
    grafana-util dashboard browse --url http://localhost:3000 --basic-user admin --basic-password admin --org-id 2

  Browse all visible orgs with Basic auth:
    grafana-util dashboard browse --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs"#;

pub(crate) const DASHBOARD_DELETE_AFTER_HELP: &str = r#"Examples:

  Dry-run one dashboard delete by UID:
    grafana-util dashboard delete --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --uid cpu-main --dry-run --json

  Delete all dashboards under one folder subtree:
    grafana-util dashboard delete --url http://localhost:3000 --basic-user admin --basic-password admin --path 'Platform / Infra' --yes

  Prompt for a folder delete selector, preview the plan, and confirm in the terminal:
    grafana-util dashboard delete --url http://localhost:3000 --prompt"#;

pub(crate) const DASHBOARD_DIFF_AFTER_HELP: &str = r#"Examples:

  Compare one raw export directory against the current org:
    grafana-util dashboard diff --url http://localhost:3000 --basic-user admin --basic-password admin --input-dir ./dashboards/raw

  Compare a provisioning export root against the current org:
    grafana-util dashboard diff --url http://localhost:3000 --basic-user admin --basic-password admin --input-dir ./dashboards/provisioning --input-format provisioning

  Compare against one explicit org as structured JSON:
    grafana-util dashboard diff --url http://localhost:3000 --basic-user admin --basic-password admin --org-id 2 --input-dir ./dashboards/raw --json"#;

pub(crate) const DASHBOARD_PATCH_FILE_AFTER_HELP: &str = r#"Examples:

  Patch a raw export file in place:
    grafana-util dashboard patch --input ./dashboards/raw/cpu-main.json --name 'CPU Overview' --folder-uid infra --tag prod --tag sre

  Patch one draft file into a new output path:
    grafana-util dashboard patch --input ./drafts/cpu-main.json --output ./drafts/cpu-main-patched.json --uid cpu-main --message 'Add folder metadata before publish'

  Patch one dashboard from standard input into an explicit output file:
    jsonnet dashboards/cpu.jsonnet | grafana-util dashboard patch --input - --output ./drafts/cpu-main.json --folder-uid infra"#;

pub(crate) const DASHBOARD_REVIEW_AFTER_HELP: &str = r#"What it does:
  Review one local dashboard draft without touching Grafana and render the draft in text, YAML, or JSON form.

When to use:
  - Check a generated or edited draft before publish.
  - Confirm folder, tags, UID, panels, and datasource references in CI or local review.

Related commands:
  - dashboard get    Fetch a live dashboard into a local draft first.
  - dashboard patch  Adjust the local draft before review.
  - dashboard publish     Send the reviewed draft to Grafana.

Examples:

  Review one local dashboard file in text mode:
    grafana-util dashboard review --input ./drafts/cpu-main.json

  Review one local dashboard file as YAML:
    grafana-util dashboard review --input ./drafts/cpu-main.json --output-format yaml

  Review one generated dashboard from standard input:
    jsonnet dashboards/cpu.jsonnet | grafana-util dashboard review --input - --output-format json"#;

pub(crate) const DASHBOARD_PUBLISH_AFTER_HELP: &str = r#"What it does:
  Publish one local dashboard draft through the import pipeline, with dry-run support before any live write.

When to use:
  - Promote a reviewed draft back into Grafana.
  - Reuse the same import semantics for one-off dashboard edits or generated drafts.

Related commands:
  - dashboard review      Inspect the local draft before publish.
  - dashboard get    Start from the current live dashboard state.
  - dashboard clone  Prepare a new variant before publish.

Examples:

  Publish one draft file to the current Grafana org:
    grafana-util dashboard publish --url http://localhost:3000 --basic-user admin --basic-password admin --input ./drafts/cpu-main.json --folder-uid infra --message 'Promote CPU dashboard'

  Preview the same publish without writing to Grafana:
    grafana-util dashboard publish --url http://localhost:3000 --basic-user admin --basic-password admin --input ./drafts/cpu-main.json --dry-run --table

  Publish one generated dashboard from standard input:
    jsonnet dashboards/cpu.jsonnet | grafana-util dashboard publish --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --input - --replace-existing

  Watch one local draft file and rerun dry-run after each save:
    grafana-util dashboard publish --url http://localhost:3000 --basic-user admin --basic-password admin --input ./drafts/cpu-main.json --dry-run --watch"#;

pub(crate) const DASHBOARD_ANALYZE_AFTER_HELP: &str = r#"What it does:
  Analyze dashboards from live Grafana or a local export tree and render summary, governance, dependency, or queries-json outputs.

When to use:
  - Inspect a live environment before dependency, policy, or impact checks.
  - Reuse a local export tree in CI without calling Grafana again.

Related commands:
  - dashboard dependencies  Show which dashboards, variables, data sources, and alerts depend on each other.
  - dashboard policy        Check dashboard findings against a policy.
  - dashboard variables     List one dashboard's current variables only.

Examples:

  Analyze live Grafana and render governance JSON:
    grafana-util dashboard summary --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format governance-json

  Analyze a raw export tree without calling Grafana:
    grafana-util dashboard summary --input-dir ./dashboards/raw --input-format raw --output-format tree-table

  Analyze a repo-backed Git Sync dashboard tree from the repo root:
    grafana-util dashboard summary --input-dir ./grafana-oac-repo --input-format git-sync --output-format governance
"#;

pub(crate) const DASHBOARD_ANALYZE_EXPORT_AFTER_HELP: &str = r#"Examples:

  Render an operator-summary table from raw exports:
    grafana-util dashboard summary --input-dir ./dashboards/raw --input-format raw --output-format table

  Open the interactive analysis workbench over raw exports:
    grafana-util dashboard summary --input-dir ./dashboards/raw --input-format raw --interactive

  Render the machine-readable governance artifact from raw exports:
    grafana-util dashboard summary --input-dir ./dashboards/raw --input-format raw --output-format governance-json

  Render the queries-json artifact from raw exports:
    grafana-util dashboard summary --input-dir ./dashboards/raw --input-format raw --output-format queries-json

  Inspect a file-provisioning tree from the provisioning root:
    grafana-util dashboard summary --input-dir ./dashboards/provisioning --input-format provisioning --output-format tree-table"#;

pub(crate) const DASHBOARD_ANALYZE_LIVE_AFTER_HELP: &str = r#"Examples:

  Render governance JSON from live Grafana:
    grafana-util dashboard summary --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format governance-json

  Render the queries-json artifact from live Grafana:
    grafana-util dashboard summary --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format queries-json

  Open the interactive analysis workbench over live Grafana:
    grafana-util dashboard summary --url http://localhost:3000 --basic-user admin --basic-password admin --interactive"#;

pub(crate) const DASHBOARD_LIST_VARS_AFTER_HELP: &str = r#"Examples:

  List variables from a browser URL directly:
    grafana-util dashboard variables --dashboard-url 'https://grafana.example.com/d/cpu-main/cpu-overview?var-cluster=prod-a' --token "$GRAFANA_API_TOKEN" --output-format table

  List one dashboard UID with a vars-query fragment:
    grafana-util dashboard variables --url https://grafana.example.com --dashboard-uid cpu-main --vars-query 'var-cluster=prod-a&var-instance=node01' --token "$GRAFANA_API_TOKEN" --output-format json

  List variables from one local dashboard JSON file:
    grafana-util dashboard variables --input ./dashboards/raw/cpu-main.json --output-format yaml

  List variables from one local export tree:
    grafana-util dashboard variables --input-dir ./dashboards/raw --dashboard-uid cpu-main --output-format table"#;

pub(crate) const DASHBOARD_GOVERNANCE_GATE_AFTER_HELP: &str = r#"Examples:

  Check live Grafana directly with a JSON/YAML policy file:
    grafana-util dashboard policy --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --policy-source file --policy ./policy.yaml

  Check an export tree without calling Grafana:
    grafana-util dashboard policy --policy-source builtin --builtin-policy default --input-dir ./dashboards/raw --input-format raw

  Advanced reuse: recheck saved analysis artifacts and write normalized JSON:
    grafana-util dashboard policy --policy-source builtin --builtin-policy default --governance ./governance.json --queries ./queries.json --output-format json --json-output ./governance-check.json"#;

pub(crate) const DASHBOARD_TOPOLOGY_AFTER_HELP: &str = r#"Examples:

  Analyze live Grafana directly and render Mermaid:
    grafana-util dashboard dependencies --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format mermaid

  Analyze an export tree without calling Grafana:
    grafana-util dashboard dependencies --input-dir ./dashboards/raw --input-format raw --output-format text

  Advanced reuse: render Graphviz DOT from saved analysis artifacts:
    grafana-util dashboard dependencies --governance ./governance.json --queries ./queries.json --alert-contract ./alert-contract.json --output-format dot --output-file ./dashboard-dependencies.dot"#;

pub(crate) const DASHBOARD_IMPACT_AFTER_HELP: &str = r#"Examples:

  Check blast radius directly from live Grafana:
    grafana-util dashboard impact --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --datasource-uid prom-main --output-format text

  Check blast radius from an export tree:
    grafana-util dashboard impact --input-dir ./dashboards/raw --input-format raw --datasource-uid prom-main --output-format json

  Reuse saved artifacts and add alert contract context:
    grafana-util dashboard impact --governance ./governance.json --queries ./queries.json --datasource-uid prom-main --alert-contract ./alert-contract.json --output-format json"#;

pub(crate) const DASHBOARD_VALIDATE_EXPORT_AFTER_HELP: &str = r#"Examples:

  Validate a raw export and fail on migration or plugin issues:
    grafana-util dashboard validate-export --input-dir ./dashboards/raw --reject-custom-plugins --reject-legacy-properties --target-schema-version 39

  Validate a provisioning export root explicitly:
    grafana-util dashboard validate-export --input-dir ./dashboards/provisioning --input-format provisioning --reject-custom-plugins

  Validate a repo-backed Git Sync dashboard tree from the repo root:
    grafana-util dashboard validate-export --input-dir ./grafana-oac-repo --input-format git-sync --reject-custom-plugins

  Write the validation report as JSON:
    grafana-util dashboard validate-export --input-dir ./dashboards/raw --output-format json --output-file ./dashboard-validation.json"#;

pub(crate) const DASHBOARD_SCREENSHOT_AFTER_HELP: &str = r#"Examples:

  Capture a full dashboard from a browser URL and add an auto title/header block:
    grafana-util dashboard screenshot --dashboard-url 'https://grafana.example.com/d/cpu-main/cpu-overview?var-cluster=prod-a' --token "$GRAFANA_API_TOKEN" --output ./cpu-main.png --full-page --header-title --header-url --header-captured-at

  Capture a solo panel with a vars-query fragment and custom header note:
    grafana-util dashboard screenshot --url https://grafana.example.com --dashboard-uid rYdddlPWk --panel-id 20 --vars-query 'var-datasource=prom-main&var-job=node-exporter&var-node=host01:9100' --token "$GRAFANA_API_TOKEN" --output ./panel.png --header-title 'CPU Busy' --header-text 'Solo panel debug capture'"#;

pub(crate) const DASHBOARD_CLI_AFTER_HELP: &str = r#"Examples:

  Fetch one live dashboard into a local draft:
    grafana-util dashboard get --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --dashboard-uid cpu-main --output ./cpu-main.json

  Clone one live dashboard with a new UID and folder:
    grafana-util dashboard clone --url http://localhost:3000 --basic-user admin --basic-password admin --source-uid cpu-main --uid cpu-main-clone --folder-uid infra --output ./cpu-main-clone.json

  Export dashboards from local Grafana with Basic auth:
    grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --output-dir ./dashboards --overwrite

  Export dashboards across all visible orgs with Basic auth:
    grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --output-dir ./dashboards --overwrite

  List dashboards across all visible orgs with Basic auth:
    grafana-util dashboard list --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --json

  Export dashboards with an API token from the current org:
    export GRAFANA_API_TOKEN='your-token'
    grafana-util dashboard export --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-dir ./dashboards --overwrite

  Compare raw dashboard exports against local Grafana:
    grafana-util dashboard diff --url http://localhost:3000 --basic-user admin --basic-password admin --input-dir ./dashboards/raw

  Patch a local dashboard file before publishing:
    grafana-util dashboard patch --input ./dashboards/raw/cpu-main.json --name 'CPU Overview' --folder-uid infra --tag prod --tag sre

  Publish one local draft to Grafana:
    grafana-util dashboard publish --url http://localhost:3000 --basic-user admin --basic-password admin --input ./drafts/cpu-main.json --dry-run --table

  Capture a browser-rendered dashboard screenshot:
    grafana-util dashboard screenshot --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --dashboard-uid cpu-main --output ./cpu-main.png --from now-6h --to now"#;

pub(crate) const DASHBOARD_HISTORY_LIST_AFTER_HELP: &str = r#"Examples:

  List the last 20 live versions as a table:
    grafana-util dashboard history list --url http://localhost:3000 --basic-user admin --basic-password admin --dashboard-uid cpu-main --limit 20 --output-format table

  Review one saved history artifact without calling Grafana:
    grafana-util dashboard history list --input ./cpu-main.history.json --output-format yaml

  Scan one export tree created with --include-history:
    grafana-util dashboard history list --input-dir ./dashboards --dashboard-uid cpu-main --output-format json"#;

pub(crate) const DASHBOARD_HISTORY_RESTORE_AFTER_HELP: &str = r#"Examples:

  Preview a restore without changing Grafana:
    grafana-util dashboard history restore --url http://localhost:3000 --basic-user admin --basic-password admin --dashboard-uid cpu-main --version 17 --dry-run --output-format table

  Prompt for one recent version, preview it, and confirm the restore:
    grafana-util dashboard history restore --url http://localhost:3000 --basic-user admin --basic-password admin --dashboard-uid cpu-main --prompt

  Restore a historical version and record a new revision message:
    grafana-util dashboard history restore --url http://localhost:3000 --basic-user admin --basic-password admin --dashboard-uid cpu-main --version 17 --message 'Restore known good CPU dashboard after regression' --yes"#;

pub(crate) const DASHBOARD_HISTORY_DIFF_AFTER_HELP: &str = r#"Examples:

  Compare two live revisions from Grafana:
    grafana-util dashboard history diff --url http://localhost:3000 --basic-user admin --basic-password admin --base-dashboard-uid cpu-main --base-version 17 --new-dashboard-uid cpu-main --new-version 21

  Compare two versions from one local history artifact:
    grafana-util dashboard history diff --base-input ./cpu-main.history.json --base-version 17 --new-input ./cpu-main.history.json --new-version 21 --output-format json

  Compare two dated export roots for the same dashboard UID:
    grafana-util dashboard history diff --base-input-dir ./exports-2026-04-01 --base-dashboard-uid cpu-main --base-version 17 --new-input-dir ./exports-2026-04-07 --new-dashboard-uid cpu-main --new-version 21 --output-format json"#;

pub(crate) const DASHBOARD_HISTORY_EXPORT_AFTER_HELP: &str = r#"Examples:

  Export the last 20 revisions to a JSON artifact:
    grafana-util dashboard history export --url http://localhost:3000 --basic-user admin --basic-password admin --dashboard-uid cpu-main --output ./cpu-main.history.json

  Overwrite an existing history artifact and raise the export limit:
    grafana-util dashboard history export --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --dashboard-uid cpu-main --limit 50 --output ./cpu-main.history.json --overwrite"#;
