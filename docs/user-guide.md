Grafana Utilities User Guide
============================

This guide documents the maintained Rust command surface used by the repository. Use `grafana-util ...` as the primary command shape throughout this manual.

Contents
--------

- [1) Before You Start](#before-you-start)
- [2) Global Options](#global-options)
- [3) Dashboard Commands](#dashboard-commands)
- [4) Alert Commands](#alert-commands)
- [5) Datasource Commands](#datasource-commands)
- [6) Access Commands](#access-commands)
- [7) Shared Output Rules, `change`, `overview`, and `status`](#shared-output-rules)
- [8) Common Operator Scenarios](#common-operator-scenarios)
- [9) Minimal SOP Commands](#minimal-sop-commands)
- [10) Output and Org Control Matrix](#output-and-org-control-matrix)

Quick jump sections:

- [dashboard](#dashboard-commands): `browse/export/list/import/delete/diff/inspect-export/inspect-live/screenshot`
- [alert](#alert-commands): `plan/apply/delete/init/new-rule/new-contact-point/new-template/export/import/diff/list-rules/list-contact-points/list-mute-timings/list-templates`
- [datasource](#datasource-commands): `browse/list/export/import/diff/add`
- [access](#access-commands): `org/user/team/service-account`
- [change](#shared-output-rules): `summary/bundle/bundle-preflight/plan/review/apply/assess-alerts`
- [overview](#shared-output-rules)
- [status](#shared-output-rules)

<a id="before-you-start"></a>
1) Before You Start
-------------------

Confirm the CLI surface first so the flags in the document match your local checkout:

```bash
grafana-util -h
grafana-util dashboard -h
grafana-util alert -h
grafana-util datasource -h
grafana-util access -h
grafana-util change -h
grafana-util overview -h
grafana-util status -h
```

Installed entrypoints:

```text
grafana-util <domain> <command> [options]
```

CLI notes:

- `grafana-util` is the primary unified CLI.
- Use the namespaced `grafana-util <domain> <command>` layout throughout this guide.
- `dashboard list-data-sources` remains available under the dashboard command surface, but new datasource inventory workflows should prefer `datasource list`.
- `overview` is the human project entrypoint, `status` is the canonical status contract, and `change` owns the staged change workflow.

### 1.1 How To Use This Manual

- Read `README.md` first when you need project positioning, supported areas, and quick examples for GitHub browsing.
- Use this guide when you need command-level behavior, option differences, authentication rules, real output shapes, or dry-run interpretation.
- If you are new to the tool, the fastest path is: command domains -> support depth -> output surfaces -> the specific command section you need.
- If you are operating an established workflow, jump directly to the domain section, then use section 9 and section 10 as the shortest repeatable reference.

### 1.2 Output And Interaction Surfaces

The same Grafana workflow may be available through more than one surface depending on whether you are exploring manually or automating.

| Surface | Best for | Representative commands | Notes |
| --- | --- | --- | --- |
| Interactive TUI | Guided review, browsing, in-terminal workflows | `dashboard browse`, `dashboard inspect-export --interactive`, `dashboard inspect-live --interactive`, `datasource browse`, `overview --output interactive`, `status ... --output interactive` | Only selected workflows provide TUI support; interactive output requires the TUI-capable build |
| Plain text | Human-readable summaries and default dry-run previews | `change`, `overview`, `status`, many dry-run summaries | Best for operator review in terminal logs |
| JSON | CI, scripting, stable machine-readable handoff | import dry-runs, change documents, staged/live status contracts | Prefer this when another tool will parse the result |
| Table / CSV / report outputs | Inventory listing, diff review, dashboard analysis | list commands, `dashboard inspect-*`, review tables | Usually the best fit for audits and spreadsheets |

### 1.3 Support Depth By Area

Use this table before reading per-command details if you want to know how mature each area is and what kind of workflow it is meant to support.

| Area | Support depth | Primary workflows | Main surfaces | Notes |
| --- | --- | --- | --- | --- |
| `dashboard` | Deepest and broadest | browse, list, export, import, diff, delete, inspect, dependency analysis, permission export, screenshot/PDF | text, table/csv/json, report modes, interactive TUI | The most feature-complete analysis and migration surface |
| `datasource` | Deep and mature | browse, list, export, import, diff, add, modify, delete, org-aware replay | text, table/csv/json, interactive browse | Covers both live mutation and file-based replay |
| `alert` | Mature management and migration surface | plan, apply, delete, init, scaffold, list, export, import, diff for alerting resources | text/json, table/csv/json | Review-first alert management lane plus the older inventory and replay lane |
| `access` | Mature inventory and replay surface | list, add, modify, delete, export, import, diff for org/user/team/service-account flows | table/csv/json | Strongest for access-state inventory, rebuild, and review |
| `change` | Advanced staged workflow | summary, bundle, preflight, plan, review, apply intent, audit, promotion-preflight | text/json | Review-first project change workflow, not blind direct mutation |
| `overview` | Human project entrypoint | staged/live project snapshots, cross-domain summaries, handoff views | text/json/interactive | Start here when an operator wants one whole-project picture |
| `status` | Canonical status contract | staged/live readiness, cross-domain summaries, machine-readable handoff | text/json/interactive | Use when you need the stable cross-domain readiness contract |

<a id="global-options"></a>
2) Global Options
-----------------

Default URLs:

- `dashboard` and `datasource` default to `http://localhost:3000`
- `alert` and `access` default to `http://127.0.0.1:3000`

| Option | Purpose | Typical use |
| --- | --- | --- |
| `--url` | Grafana base URL | Any live Grafana operation |
| `--token`, `--api-token` | API token auth | Scripts and non-interactive workflows |
| `--basic-user` | Basic auth username | Org switching, admin workflows, access management |
| `--basic-password` | Basic auth password | Used with `--basic-user` |
| `--prompt-token` | Prompt for token without echo | Safer interactive usage |
| `--prompt-password` | Prompt for password without echo | Safer interactive usage |
| `--timeout` | HTTP timeout in seconds | Slow APIs or unstable networks |
| `--verify-ssl` | Enable TLS certificate verification | Production TLS environments |

### 2.1 How To Read Example Output

- `Example command` shows a practical invocation shape.
- `Example output` shows the expected format, not a guarantee that your own UIDs, names, counts, or folders will match exactly.
- When a section includes a `Live note`, the command shape and output excerpt were checked against a local Docker Grafana `12.4.1` service.
- For this revision, the validated sample set came from `scripts/seed-grafana-sample-data.sh` plus extra seeded alerting resources and a service account/token so live output covers multi-org, nested folders, alerting, and access surfaces.
- Table output is best for operators.
- JSON output is best for scripts, CI, or when you need stable machine-readable fields.
- Common `ACTION` values:
  - `create`: the target does not exist yet.
  - `update`: the target already exists and would be modified.
  - `no-change`: source and destination already match.
  - `would-*`: a dry-run prediction only.
- Common dry-run destination or status hints:
  - `missing`: no live match was found yet.
  - `exists` / `existing`: a live match is already present.
  - `exists-uid`: the live match was found by UID.
  - `exists-name`: the live match was found by name.
  - `missing-org`: the routed destination org does not exist.
  - `would-create-org`: `--dry-run` detected that `--create-missing-orgs` would create the routed destination org.
- In diff output:
  - `-` is usually the live or current value.
  - `+` is usually the exported or expected value.

### Command Domains

Use this routing table when you know the task but not yet the exact command.

| Goal | Start here | Common commands |
| --- | --- | --- |
| Dashboard inventory and analysis | `dashboard` | `browse`, `list`, `export`, `import`, `diff`, `delete`, `inspect-export`, `inspect-live`, `inspect-vars`, `screenshot` |
| Alerting management, inventory, and migration | `alert` | `plan`, `apply`, `delete`, `init`, `new-rule`, `new-contact-point`, `new-template`, `list-rules`, `list-contact-points`, `list-mute-timings`, `list-templates`, `export`, `import`, `diff` |
| Datasource inventory and replay | `datasource` | `browse`, `list`, `export`, `import`, `diff`, `add`, `modify`, `delete` |
| Access management for orgs | `access org` | `list`, `add`, `modify`, `delete`, `export`, `import` |
| Access management for users | `access user` | `list`, `add`, `modify`, `delete`, `export`, `import`, `diff` |
| Access management for teams | `access team` | `list`, `add`, `modify`, `delete`, `export`, `import`, `diff` |
| Access management for service accounts | `access service-account` | `list`, `add`, `delete`, `export`, `import`, `diff`, `token add`, `token delete` |
| Staged change and promotion workflows | `change` | `summary`, `bundle`, `bundle-preflight`, `preflight`, `assess-alerts`, `plan`, `review`, `apply`, `audit`, `promotion-preflight` |
| Project-wide staged or live reads | `overview`, `status` | `overview`, `overview live`, `status staged`, `status live` |

### Command capability summary

Use this table first when you need to confirm whether a resource supports inventory, file export/import, or drift comparison before reading the per-command sections.

| Resource | List | Export | Import | Diff | Inspect | Add | Modify | Delete | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Dashboards | Yes | Yes | Yes | Yes | Yes | No | No | No | Inventory, backup, and cross-environment migration |
| Datasources | Yes | Yes | Yes | Yes | No | No | No | No | Drift review and migration checkpoints |
| Alert rules & alerting resources | Yes | Yes | Yes | Yes | No | No | No | No | management lane: `plan/apply/delete/init/new-*`; migration lane: `export/import/diff` |
| Organizations | Yes | Yes | Yes | No | No | Yes | Yes | Yes | Org inventory plus membership replay on import |
| Users | Yes | Yes | Yes | Yes | No | Yes | Yes | Yes | User inventory, migration, and drift comparison |
| Teams | Yes | Yes | Yes | Yes | No | Yes | Yes | Yes | Team inventory, migration, and drift comparison |
| Service accounts | Yes | Yes | Yes | Yes | No | Yes | Yes | Yes | Service account lifecycle, snapshot replay, and drift review |
| Service account tokens | Yes | No | No | No | No | Yes | No | Yes | token add/delete workflows |

Project-level surfaces:

| Surface | Inputs | Live reads | Output modes | Main use |
| --- | --- | --- | --- | --- |
| `change` | desired JSON, bundle files, lock files, availability/mapping metadata | Optional, command-dependent | text/json | staged review, preflight, plan, review, apply intent |
| `overview` | staged exports plus optional change/promotion inputs | `overview live` only | text/json/interactive | one operator-facing staged or live project snapshot |
| `status` | staged exports or live Grafana | Yes | text/json/interactive | canonical project-wide staged/live readiness surface |

Authentication exclusivity rules:

1. `--token` / `--api-token` cannot be combined with `--basic-user` / `--basic-password`.
2. `--token` / `--api-token` cannot be combined with `--prompt-token`.
3. `--basic-password` cannot be combined with `--prompt-password`.
4. `--prompt-password` requires `--basic-user`.

<a id="dashboard-commands"></a>
3) Dashboard Commands
---------------------

### 3.1 `dashboard export`

Purpose: export live dashboards into `raw/` and `prompt/` variants.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--export-dir` | Export root directory | Default `dashboards`; contains `raw/` and `prompt/` |
| `--page-size` | Pagination size | Increase for large estates |
| `--org-id` | Export from one explicit org | API token is not supported here; use Grafana username/password login |
| `--all-orgs` | Export from all visible orgs | Best for central backups |
| `--flat` | Flatten folder paths | Useful for simpler git diffs |
| `--overwrite` | Replace existing files | Typical for repeatable exports |
| `--without-dashboard-raw` | Skip `raw/` | Use only if API restore is not needed |
| `--without-dashboard-prompt` | Skip `prompt/` | Use only if UI import is not needed |
| `--dry-run` | Preview files without writing them | Validate scope and paths first |
| `--progress` | Print concise progress lines | Large exports |
| `-v`, `--verbose` | Print detailed per-item output | Troubleshooting export behavior |

Example command:
```bash
grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./dashboards --overwrite --progress
```

Example output:
```text
Exporting dashboard 1/7: mixed-query-smoke
Exporting dashboard 2/7: smoke-prom-only
Exporting dashboard 3/7: query-smoke
Exporting dashboard 4/7: smoke-main
Exporting dashboard 5/7: subfolder-chain-smoke
Exporting dashboard 6/7: subfolder-main
Exporting dashboard 7/7: two-prom-query-smoke
```

How to read it:
- `--progress` prints one concise line per dashboard; use `--verbose` when you need per-file output paths.
- `raw` is the API-friendly reversible export.
- `prompt` is the UI-import-friendly variant.
- `raw/permissions.json` captures dashboard and folder permission metadata for backup and review.
- With `--all-orgs`, the export root `export-metadata.json` includes `orgCount` plus an `orgs[]` summary for every exported org.
- For replay-heavy dry-run examples in this guide, `--flat` is the most repeatable export shape because the resulting `raw/` tree can be pointed at `dashboard import` directly.

### 3.2 `dashboard list`

Purpose: list live dashboards without writing files.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--page-size` | Results per page | Increase for large estates |
| `--org-id` | Restrict to one org | Explicit org selection |
| `--all-orgs` | Aggregate visible orgs | Cross-org inventory |
| `--with-sources` | Add datasource names in table/csv | Useful for dependency checks |
| `--table` | Table output | Best for operators |
| `--csv` | CSV output | Best for spreadsheets |
| `--json` | JSON output | Best for automation |
| `--output-format table\|csv\|json` | Single output selector | Replaces the legacy trio |
| `--no-header` | Hide table header row | Cleaner scripting |

Example command:
```bash
grafana-util dashboard list --url http://localhost:3000 --basic-user admin --basic-password admin --with-sources --table
```

Example output:
```text
UID              TITLE            FOLDER   TAGS        DATASOURCES
cpu-main         CPU Overview     Infra    ops,linux   prometheus-main
mem-main         Memory Overview  Infra    ops,linux   prometheus-main
latency-main     API Latency      Apps     api,prod    loki-prod
```

How to read it:
- `UID` is the most stable identity for follow-up automation.
- `FOLDER` is the fastest way to see placement.
- `DATASOURCES` is the main reason to enable `--with-sources`.

Example command (JSON):
```bash
grafana-util dashboard list --url http://localhost:3000 --token <TOKEN> --json
```

```json
[
  {
    "uid": "cpu-main",
    "title": "CPU Overview",
    "folder": "Infra",
    "tags": ["ops", "linux"]
  }
]
```

### 3.3 `dashboard list-data-sources`

Purpose: use the dashboard-scoped datasource inventory path when you want datasource output while already working from the dashboard surface. New runbooks should prefer `datasource list`.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--table` | Table output | Human inspection |
| `--csv` | CSV output | Spreadsheet workflows |
| `--json` | JSON output | Automation |
| `--output-format table\|csv\|json` | Unified output selector | Replaces the legacy trio |
| `--no-header` | Hide table header | Cleaner scripting |

Example command:
```bash
grafana-util datasource list --url http://localhost:3000 --basic-user admin --basic-password admin --table
```

Example output:
```text
UID                NAME               TYPE         IS_DEFAULT
prom-main          prometheus-main    prometheus   true
loki-prod          loki-prod          loki         false
tempo-prod         tempo-prod         tempo        false
```

Preferred path:
- Use section `5.1 datasource list` for new automation, saved examples, and operator documentation.

### 3.4 `dashboard import`

Purpose: import dashboards from a `raw/` export into live Grafana.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--import-dir` | Input `raw/` directory or multi-org export root | Use `raw/` for normal import; use the combined export root with `--use-export-org` |
| `--org-id` | Target org | Org-specific import |
| `--use-export-org` | Route each exported org back into Grafana | Import a combined `--all-orgs` export root |
| `--only-org-id` | Restrict `--use-export-org` to selected source orgs | Repeat the flag to import multiple orgs |
| `--create-missing-orgs` | Create missing destination orgs before routed import | Only for `--use-export-org`; with `--dry-run` it reports `would-create-org` without creating anything |
| `--import-folder-uid` | Force destination folder uid | Controlled placement |
| `--ensure-folders` | Create missing folders | Helpful for first-time restore |
| `--replace-existing` | Overwrite matching dashboards | Standard restore mode |
| `--update-existing-only` | Update only existing dashboards | Safe partial reconcile |
| `--require-matching-folder-path` | Refuse mismatched folder paths | Prevent wrong placement |
| `--require-matching-export-org` | Enforce exported org match | Safer cross-org restore |
| `--import-message` | Dashboard version message | Audit trail |
| `--dry-run` | Preview only | Always recommended first |
| `--table` | Dry-run table output | Best operator summary |
| `--json` | Dry-run JSON output | Best for automation |
| `--output-format text\|table\|json` | Dry-run output mode | Unified selector |
| `--output-columns` | Column whitelist | Tailored dry-run tables |
| `--no-header` | Hide table header | Cleaner scripting |
| `--progress` | Show import progress | Large restores |
| `-v`, `--verbose` | Detailed import logs | Troubleshooting |

Example command:
```bash
grafana-util dashboard import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards-flat/raw --replace-existing --dry-run --table
```

Current note:
- `dashboard import` currently ignores the exported `raw/permissions.json` bundle. The permission bundle is backed up by default for later review or future restore flows, but the current import path still restores dashboard content, folder placement, and related raw inventory only.

Example output:
```text
Import mode: create-or-update
UID                    DESTINATION  ACTION  FOLDER_PATH                    FILE
---------------------  -----------  ------  -----------------------------  ------------------------------------------------------------
mixed-query-smoke      exists       update  General                        ./dashboards-flat/raw/Mixed_Query_Dashboard__mixed-query-smoke.json
smoke-main             exists       update  General                        ./dashboards-flat/raw/Smoke_Dashboard__smoke-main.json
subfolder-chain-smoke  exists       update  Platform / Team / Apps / Prod  ./dashboards-flat/raw/Subfolder_Chain_Dashboard__subfolder-chain-smoke.json

Dry-run checked 7 dashboard(s) from ./dashboards-flat/raw
```

Live note:
- The dry-run table above was validated against a local Grafana `12.4.1` container by first exporting dashboards with `dashboard export --flat`.

How to read it:
- `ACTION=update` means the dashboard already exists and would be changed.
- `ACTION=create` means the dashboard is not present yet.
- `DESTINATION` describes the live target state, not the local directory.
- `DESTINATION=missing` means dry-run found no live dashboard with that UID yet.
- Routed multi-org dry-runs can also report `missing-org` or `would-create-org` before any per-dashboard action rows are applied.

### 3.4a `dashboard delete`

Purpose: delete live dashboards by explicit UID or by one folder-path subtree.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--uid` | Delete one dashboard by stable identity | Best for automation |
| `--path` | Delete dashboards under one folder-path subtree | Good for cleanup by hierarchy |
| `--delete-folders` | Also delete matched folder resources | Only with `--path`; folders are removed after dashboards |
| `--interactive` | Guided selector + preview + confirmation | Best for manual maintenance |
| `--yes` | Confirm live delete | Required for non-interactive live deletes |
| `--dry-run` | Preview only | Always recommended first |
| `--table` | Dry-run table output | Human review |
| `--json` | Dry-run JSON output | Automation |
| `--output-format text\|table\|json` | Dry-run output mode | Unified selector |
| `--org-id` | Target one explicit org | Safer than cross-org delete |

Example command:
```bash
grafana-util dashboard delete --url http://localhost:3000 --basic-user admin --basic-password admin --path "Platform / Infra" --dry-run --table
```

Current note:
- `--path` matches the resolved Grafana folder path tree and deletes dashboards recursively from that subtree.
- Without `--delete-folders`, folder resources remain in Grafana even when all matched dashboards are removed.

### 3.5 `dashboard diff`

Purpose: compare local exported dashboards against live Grafana.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--import-dir` | `raw/` directory to compare | Read-only comparison |
| `--import-folder-uid` | Override folder uid assumption | Useful when folder mapping differs |
| `--context-lines` | Diff context size | Increase when JSON changes are large |

Example command:
```bash
grafana-util dashboard diff --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards/raw
```

Example output:
```text
Dashboard diff found 1 differing item(s).

--- live/cpu-main
+++ export/cpu-main
@@
-  "title": "CPU Overview"
+  "title": "CPU Overview v2"
```

How to read it:
- Start with the summary count.
- `-` is the current live value.
- `+` is the exported expected value.

### 3.6 `dashboard inspect-export`

Purpose: analyze exported dashboards offline without calling Grafana.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--import-dir` | One org `raw/` directory or a combined multi-org export root | Offline analysis only |
| `--json` | JSON output | Script-friendly |
| `--table` | Table output | Operator-friendly |
| `--report` | Shortcut report mode | Empty `--report` means flat table; explicit values include `csv`, `json`, `tree`, `tree-table`, `dependency`, `dependency-json`, `governance`, and `governance-json` |
| `--output-format ...` | Select report family explicitly | Use `text`, `table`, `json`, `report-table`, `report-csv`, `report-json`, `report-tree`, `report-tree-table`, `dependency`, `dependency-json`, `report-dependency`, `report-dependency-json`, `governance`, or `governance-json` |
| `--report-columns` | Column whitelist | Only valid with `report-table`, `report-csv`, `report-tree-table`, or the equivalent `--report` modes |
| `--report-filter-datasource` | Filter by datasource | Exact match on datasource label, uid, type, or normalized family |
| `--report-filter-panel-id` | Filter by panel id | Report-only filter for single-panel troubleshooting |
| `--help-full` | Show richer examples | Useful for report discovery |
| `--no-header` | Hide table header | Cleaner scripting |

Example command:
```bash
grafana-util dashboard inspect-export --import-dir ./dashboards/raw --output-format report-table
```

Combined multi-org export root:
```bash
grafana-util dashboard inspect-export --import-dir ./dashboards --output-format report-tree-table
```

Inspect datasource-level org, database, bucket, and index-pattern fields:
```bash
grafana-util dashboard inspect-export --import-dir ./dashboards/raw --report csv \
  --report-columns datasource_name,datasource_org,datasource_org_id,datasource_database,datasource_bucket,datasource_index_pattern,query
```

Inspect metrics, functions, and bucket extraction:
```bash
grafana-util dashboard inspect-export --import-dir ./dashboards/raw --report csv \
  --report-columns panel_id,ref_id,datasource_name,metrics,functions,buckets,query
```

Inspect folder identity and source path details:
```bash
grafana-util dashboard inspect-export --import-dir ./dashboards/raw --report csv \
  --report-columns dashboard_uid,folder_path,folder_uid,parent_folder_uid,file
```

Example output:
```text
UID           TITLE             PANEL_COUNT   DATASOURCES
cpu-main      CPU Overview      6             prometheus-main
mem-main      Memory Overview   4             prometheus-main
latency-main  API Latency       8             loki-prod
```

### 3.7 `dashboard inspect-live`

Purpose: run the same report logic directly against live dashboards.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--page-size` | Live pagination size | Lower it if the server is slow |
| `--org-id` | Restrict to one org | Explicit org inspection |
| `--all-orgs` | Aggregate visible orgs | Cross-org inspection |
| `--json` / `--table` / `--report` / `--output-format` | Same meaning as `inspect-export` | Includes `dependency` / `dependency-json` and governance modes |
| `--help-full` | Show report details | Useful during report design |
| `--no-header` | Hide table header | Cleaner scripting |

Example command:
```bash
grafana-util dashboard inspect-live --url http://localhost:3000 --basic-user admin --basic-password admin --output-format governance-json
```

Example output:
```json
{
  "kind": "grafana-utils-dashboard-governance",
  "summary": {
    "dashboardCount": 1,
    "mixedDashboardCount": 0
  },
  "dashboardDependencies": [
    {
      "dashboardUid": "cpu-main",
      "dashboardTitle": "CPU Overview",
      "datasources": ["prom-main"],
      "datasourceFamilies": ["prometheus"],
      "pluginIds": ["timeseries"]
    }
  ]
}
```

Notes:
- `--report-columns` is only valid with flat or grouped table-style report modes; it is rejected for summary JSON, dependency contracts, and governance output.
- `--report-filter-datasource` matches datasource label, uid, type, or normalized family exactly.
- `--report-filter-panel-id` is a report-only filter.
- `dependency` / `dependency-json` outputs a machine-readable contract document: top-level count fields (`queryCount`, `datasourceCount`, `dashboardCount`) plus `queries` and `datasourceUsage`.

### 3.8 `dashboard inspect-vars`

Purpose: inspect live dashboard templating variables before replaying browser-like state into `dashboard screenshot`.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--dashboard-uid` | Target one dashboard by UID | Best for API-driven inspection |
| `--dashboard-url` | Reuse a full browser dashboard URL | Auto-derives UID and current query state |
| `--vars-query` | Overlay `${__all_variables}`-style query fragments | Useful when you only have `var-*` output |
| `--org-id` | Inspect in one explicit org | Adds `X-Grafana-Org-Id` |
| `--output-format` | Choose table, csv, or json | JSON is best for scripting |
| `--no-header` | Hide table/CSV headers | Cleaner shell pipelines |

Example command:
```bash
grafana-util dashboard inspect-vars --url https://192.168.1.112:3000 --dashboard-uid rYdddlPWk --vars-query 'var-datasource=bMcTJFtVz&var-job=node-exporter&var-node=192.168.1.112:9100&var-diskdevices=%5Ba-z%5D%2B%7Cnvme%5B0-9%5D%2Bn%5B0-9%5D%2B%7Cmmcblk%5B0-9%5D%2B&refresh=1m&showCategory=Panel%20links&timezone=browser' --basic-user admin --basic-password admin --output-format table
```

Example output:
```text
NAME         TYPE        LABEL       CURRENT                                DATASOURCE     OPTIONS
datasource   datasource  Datasource  bMcTJFtVz
job          query       Job         node-exporter                          ${datasource}
node         query       Host        192.168.1.112:9100                     ${datasource}
diskdevices  custom                  [a-z]+|nvme[0-9]+n[0-9]+|mmcblk[0-9]+                 [a-z]+|nvme[0-9]+n[0-9]+|mmcblk[0-9]+
```

### 3.9 `dashboard screenshot`

Purpose: open one Grafana dashboard in headless Chromium and capture PNG, JPEG, or PDF output with browser-like state replay.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--dashboard-uid` / `--dashboard-url` | Choose dashboard target | URL mode preserves browser query state directly |
| `--panel-id` | Capture one solo panel | Uses the Grafana `d-solo` route |
| `--width`, `--height` | Control browser viewport size | Useful for wide dashboards or panel crops |
| `--device-scale-factor` | Increase raster density without changing CSS viewport size | Use `2` for sharper PNG/JPEG output |
| `--vars-query` | Replay `var-*` plus compatible query keys | Supports `refresh`, `showCategory`, `timezone`, and `${__all_variables}`-style fragments |
| `--print-capture-url` | Print the final resolved URL | Best for troubleshooting capture state |
| `--full-page` | Stitch a tall dashboard image | Browser-style long screenshot |
| `--full-page-output` | Keep one stitched image or emit segmented files | `tiles` writes `part-0001.*` etc.; `manifest` also writes `manifest.json` with title/dashboard/panel metadata |
| `--browser-path` | Pin the Chrome/Chromium binary | Useful on workstations with multiple browsers |
| `--header-title`, `--header-url`, `--header-captured-at`, `--header-text` | Add a dark header block above PNG/JPEG output | Header is composed after capture, so it does not disturb Grafana layout |

Example command:
```bash
grafana-util dashboard screenshot --url https://192.168.1.112:3000 --dashboard-uid rYdddlPWk --panel-id 20 --vars-query 'var-datasource=bMcTJFtVz&var-job=node-exporter&var-node=192.168.1.112:9100&var-diskdevices=%5Ba-z%5D%2B%7Cnvme%5B0-9%5D%2Bn%5B0-9%5D%2B%7Cmmcblk%5B0-9%5D%2B&refresh=1m&showCategory=Panel%20links&timezone=browser' --basic-user admin --basic-password admin --output /tmp/node-exporter-full-panel-20-header.png --header-title --header-url --header-captured-at --header-text 'Solo panel debug capture' --print-capture-url --browser-path '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' --wait-ms 20000
```

Example output:
```text
Capture URL: https://192.168.1.112:3000/d-solo/rYdddlPWk/node-exporter-full?refresh=1m&showCategory=Panel+links&timezone=browser&panelId=20&viewPanel=20&theme=dark&kiosk=tv&var-datasource=bMcTJFtVz&var-job=node-exporter&var-node=192.168.1.112%3A9100&var-diskdevices=%5Ba-z%5D%2B%7Cnvme%5B0-9%5D%2Bn%5B0-9%5D%2B%7Cmmcblk%5B0-9%5D%2B
```

Validated output file:
```text
/tmp/node-exporter-full-panel-20-header-v2.png
```

<a id="alert-commands"></a>
4) Alert Commands
-----------------

Alert now has three operator-facing layers:
- Authoring layer: `init`, `add-rule`, `clone-rule`, `add-contact-point`, `set-route`, `preview-route`, and the lower-level `new-*` scaffolds.
- Review/apply layer: `plan`, `apply`, and the explicit delete preview surface.
- Migration layer: `export`, `import`, `diff`, plus the live `list-*` inventory commands.

Keep these layers separate:
- The authoring commands only write or preview desired-state files. They do not call live Grafana mutation APIs.
- `plan` and `apply` are the only normal live-mutation path for the authoring lane.
- `export/import/diff` stays on the older `raw/` replay lane and should not be mixed into the desired-state authoring workflow.

Authoring boundaries that matter in practice:
- `add-rule` is intentionally limited to simple threshold or classic-condition style authoring.
- For complex rules, use `clone-rule` from an existing desired rule, then edit the desired file by hand before `plan`.
- `set-route` owns one tool-managed route. Re-running it overwrites that same route instead of merging field-by-field.
- `preview-route` previews desired labels against the managed route contract. It is not a full Grafana routing simulator.
- `--folder` records the desired folder identity only. It is not a live folder resolve/create workflow.
- `--dry-run` on the authoring commands renders the desired document without writing files.

### 4.1 Authoring Layer

Purpose: build or edit desired alert files under one managed desired tree before any live review/apply step.

Start with the desired tree:

```bash
grafana-util alert init --desired-dir ./alerts/desired
```

That tree is intentionally separate from the migration-oriented `alerts/raw` export tree.

Authoring command map:

| Command | Use it for | Important boundary |
| --- | --- | --- |
| `alert add-contact-point` | Create a simple contact-point desired document quickly | Writes desired files only |
| `alert add-rule` | Create a simple threshold/classic-condition rule | Not for complex multi-query authoring |
| `alert clone-rule` | Start from an existing desired rule and hand-edit the clone | Best path for richer rule bodies |
| `alert set-route` | Author the tool-owned managed route document | Re-run overwrites the same managed route |
| `alert preview-route` | Preview desired labels against the managed route input shape | Preview only; not a Grafana routing simulation |
| `alert new-rule`, `new-contact-point`, `new-template` | Low-level starter scaffolds when the higher-level authoring surface is not enough | Leaves more of the document for manual editing |

Common authoring commands:

```bash
grafana-util alert add-contact-point --desired-dir ./alerts/desired --name pagerduty-primary
grafana-util alert add-rule --desired-dir ./alerts/desired --name cpu-high --folder platform-alerts --rule-group cpu --receiver pagerduty-primary --label team=platform --severity critical --expr A --threshold 80 --above --for 5m
grafana-util alert preview-route --desired-dir ./alerts/desired --label team=platform --severity critical
```

Validated authoring output excerpt from `preview-route`:

```json
{
  "input": {
    "labels": {
      "team": "platform"
    },
    "severity": "critical"
  },
  "matches": []
}
```

`matches: []` here is expected. `preview-route` shows the desired-state preview contract, not whether Grafana would route a live alert instance exactly the same way.

Managed route overwrite example:

```bash
grafana-util alert set-route --desired-dir ./alerts/desired --receiver pagerduty-primary --label team=platform --severity critical
grafana-util alert set-route --desired-dir ./alerts/desired --receiver pagerduty-primary --label team=infra --severity critical
```

On the second run, the managed route changes from `team=platform` to `team=infra`. It does not preserve the old matcher and merge the new one on top.

### 4.2 Review And Apply Layer

#### `alert plan`

Purpose: compare desired alert YAML or JSON files against live Grafana and build a reviewable plan.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--desired-dir` | Desired alert directory | Reads managed alert files instead of `raw/` exports |
| `--prune` | Turn live-only resources into delete rows | Required before the plan can propose delete actions |
| `--dashboard-uid-map` | Dashboard UID map | Preserve linked alert-rule references across environments |
| `--panel-id-map` | Panel id map | Preserve linked panel references across environments |
| `--output text\|json` | Plan rendering mode | Use `json` for CI or reviewed handoff artifacts |

Example command:
```bash
grafana-util alert plan --url http://localhost:3000 --basic-user admin --basic-password admin --desired-dir ./alerts/desired --prune --output json
```

How to read it:
- `create` means the desired resource is missing in live Grafana.
- `update` means the live resource exists but differs.
- `noop` means desired and live already match.
- `delete` appears only when `--prune` is enabled.
- `blocked` means the plan found something actionable but refused to treat it as live-safe, for example a live-only resource without `--prune`.

#### `alert apply`

Purpose: execute a reviewed alert plan file back to Grafana.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--plan-file` | Reviewed alert plan document | `apply` does not read desired files directly |
| `--approve` | Explicit approval gate | Required before live execution |
| `--output text\|json` | Apply result rendering | `json` is better for audit capture |

Example command:
```bash
grafana-util alert apply --url http://localhost:3000 --basic-user admin --basic-password admin --plan-file ./alert-plan-reviewed.json --approve --output json
```

How to read it:
- `appliedCount` tells you how many plan rows actually executed.
- `results[]` records each executed resource action.
- `apply` skips `noop` and `blocked` rows; it only executes `create`, `update`, and `delete`.

#### `alert delete`

Purpose: preview one explicit alert resource delete request by kind and identity.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--kind` | Resource kind | `rule`, `contact-point`, `mute-timing`, `template`, `policy-tree` |
| `--identity` | Explicit resource identity | UID or name, depending on the resource kind |
| `--allow-policy-reset` | Allow notification policy reset | Required before policy-tree delete is treated as executable |
| `--output text\|json` | Preview rendering | JSON is easier to hand off or inspect programmatically |

Example command:
```bash
grafana-util alert delete --kind policy-tree --identity default --allow-policy-reset --output json
```

How to read it:
- This is a preview surface, not a blind delete shortcut.
- `policy-tree` is special because Grafana models it as a reset path, not a normal delete.
- If `--allow-policy-reset` is omitted, the preview row is marked `blocked`.

### 4.3 Operator Workflows

#### Simple add path (`add-contact-point -> add-rule -> preview-route -> plan -> apply`)

Validated locally on March 30, 2026 against Docker Grafana `12.4.1` at `http://127.0.0.1:43111`.

1. Build the desired tree.

```bash
grafana-util alert init --desired-dir ./alerts/desired
grafana-util alert add-contact-point --desired-dir ./alerts/desired --name pagerduty-primary
grafana-util alert add-rule --desired-dir ./alerts/desired --name cpu-high --folder platform-alerts --rule-group cpu --receiver pagerduty-primary --label team=platform --severity critical --expr A --threshold 80 --above --for 5m
grafana-util alert preview-route --desired-dir ./alerts/desired --label team=platform --severity critical
```

2. Review the live plan.

Validated command:

```bash
grafana-util alert plan --url http://127.0.0.1:43111 --basic-user admin --basic-password admin --desired-dir ./alerts/desired --output json
```

Validated summary excerpt:

```json
{
  "summary": {
    "blocked": 1,
    "create": 2,
    "delete": 0,
    "noop": 0,
    "processed": 4,
    "update": 1
  }
}
```

In that run:
- `create` covered the new contact point and alert rule.
- `update` covered the managed notification policy document because Grafana started from the default empty policy tree.
- `blocked` represented the live default policy tree row when `--prune` was not enabled.

3. Apply the reviewed plan.

Validated command:

```bash
grafana-util alert apply --url http://127.0.0.1:43111 --basic-user admin --basic-password admin --plan-file ./alert-plan-reviewed.json --approve --output json
```

Validated result excerpt:

```json
{
  "appliedCount": 3,
  "results": [
    {
      "action": "create",
      "identity": "pagerduty-primary",
      "kind": "grafana-contact-point"
    },
    {
      "action": "update",
      "identity": "pagerduty-primary",
      "kind": "grafana-notification-policies"
    },
    {
      "action": "create",
      "identity": "cpu-high",
      "kind": "grafana-alert-rule"
    }
  ]
}
```

4. Practical limit from the same validation: a follow-up `plan --prune` did not return to all `noop` rows. Grafana normalizes some live payload fields, so the current authoring documents can still re-plan as `update` on contact-point, policy, and rule resources after apply. Do not read the authoring lane as a byte-for-byte round-trip guarantee.

#### Complex path (`clone-rule -> edit desired file -> plan -> apply`)

Use this when `add-rule` is too small for the rule you want to express.

```bash
grafana-util alert clone-rule --desired-dir ./alerts/desired --source cpu-high --name cpu-high-staging --folder staging-alerts --rule-group cpu --receiver slack-platform
# edit ./alerts/desired/rules/cpu-high-staging.yaml or .json by hand
grafana-util alert plan --url http://localhost:3000 --basic-user admin --basic-password admin --desired-dir ./alerts/desired --output json
grafana-util alert apply --url http://localhost:3000 --basic-user admin --basic-password admin --plan-file ./alert-plan-reviewed.json --approve --output json
```

Recommended operator pattern:
- Clone from a known-good desired rule or exported rule first.
- Hand-edit the cloned rule body for richer queries, expressions, annotations, recording semantics, or linked dashboard metadata.
- Review the resulting live diff with `plan` before `apply`.

#### Delete path (`remove desired file -> plan --prune -> apply`)

Validated locally on March 30, 2026 against the same Docker Grafana `12.4.1` fixture.

1. Remove the rule file from desired state.

```bash
rm ./alerts/desired/rules/cpu-high.yaml
```

2. Rebuild the plan with prune enabled.

Validated command:

```bash
grafana-util alert plan --url http://127.0.0.1:43111 --basic-user admin --basic-password admin --desired-dir ./alerts/desired --prune --output json
```

Validated summary excerpt:

```json
{
  "summary": {
    "blocked": 0,
    "create": 0,
    "delete": 1,
    "noop": 0,
    "processed": 3,
    "update": 2
  }
}
```

Validated delete row excerpt:

```json
{
  "action": "delete",
  "identity": "cpu-high",
  "kind": "grafana-alert-rule",
  "reason": "missing-from-desired-state"
}
```

The same validation still carried two `update` rows because of the live normalization differences described above. The key prune signal is the `delete` row created for the missing desired rule.

3. Apply the reviewed prune plan.

Validated result excerpt:

```json
{
  "appliedCount": 3,
  "results": [
    {
      "action": "update",
      "identity": "pagerduty-primary",
      "kind": "grafana-contact-point"
    },
    {
      "action": "update",
      "identity": "pagerduty-primary",
      "kind": "grafana-notification-policies"
    },
    {
      "action": "delete",
      "identity": "cpu-high",
      "kind": "grafana-alert-rule"
    }
  ]
}
```

### 4.4 Migration Layer

#### `alert export`

Purpose: export alerting resources into `raw/` JSON files.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--output-dir` | Export root directory | Default `alerts` |
| `--flat` | Flatten subdirectories | Easier diffing in some repos |
| `--overwrite` | Replace existing files | Standard repeatable export mode |

Example command:
```bash
grafana-util alert export --url http://localhost:3000 --basic-user admin --basic-password admin --output-dir ./alerts --overwrite
```

Example output:
```text
Exported alert rule cpu-main -> /tmp/alert-export-after-apply/raw/rules/general/example-rules/CPU_Main__cpu-main.json
Exported contact point contact-point-uid -> /tmp/alert-export-after-apply/raw/contact-points/example-contact-point/example-contact-point__contact-point-uid.json
Exported notification policies empty -> /tmp/alert-export-after-apply/raw/policies/notification-policies.json
Exported template example-template -> /tmp/alert-export-after-apply/raw/templates/example-template/example-template.json
Exported 1 alert rules, 1 contact points, 0 mute timings, 1 notification policy documents, 1 templates. Root index: /tmp/alert-export-after-apply/index.json
```

#### `alert import` (legacy `import-alert`)

Purpose: import alerting resources from a `raw/` directory.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--import-dir` | Alert `raw/` directory | Must point to `raw/` |
| `--replace-existing` | Update existing resources | Standard restore mode |
| `--dry-run` | Preview only | Best first pass |
| `--json` | Structured dry-run preview | Best for automation |
| `--dashboard-uid-map` | Dashboard UID map | Fix linked alert references |
| `--panel-id-map` | Panel id map | Fix linked panel references |

Example command:
```bash
grafana-util alert import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./alerts/raw --replace-existing --dry-run --json
```

Example output:
```json
{
  "summary": {
    "processed": 4,
    "wouldCreate": 0,
    "wouldUpdate": 4,
    "wouldFailExisting": 0
  },
  "rows": [
    {
      "path": "/tmp/alert-export-after-apply/raw/contact-points/example-contact-point/example-contact-point__contact-point-uid.json",
      "kind": "grafana-contact-point",
      "identity": "contact-point-uid",
      "action": "would-update"
    },
    {
      "path": "/tmp/alert-export-after-apply/raw/policies/notification-policies.json",
      "kind": "grafana-notification-policies",
      "identity": "empty",
      "action": "would-update"
    },
    {
      "path": "/tmp/alert-export-after-apply/raw/rules/general/example-rules/CPU_Main__cpu-main.json",
      "kind": "grafana-alert-rule",
      "identity": "cpu-main",
      "action": "would-update"
    },
    {
      "path": "/tmp/alert-export-after-apply/raw/templates/example-template/example-template.json",
      "kind": "grafana-notification-template",
      "identity": "example-template",
      "action": "would-update"
    }
  ]
}
```

How to read it:
- `summary` is the fastest safety check before replaying a bundle.
- `would-*` values are dry-run predictions.
- `kind` tells you which resource family would change.

Migration example:

```bash
grafana-util alert export --url http://localhost:3000 --basic-user admin --basic-password admin --output-dir ./alerts --overwrite
grafana-util alert import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./alerts/raw --replace-existing --dry-run --json
```

Use this split as the default mental model:
- `add-rule/clone-rule/add-contact-point/set-route/preview-route/init/new-*` is the desired-state authoring lane.
- `plan/apply` is the review-first live mutation lane.
- `export/import/diff` is the migration and replay lane.

### 4.5 `alert diff` (legacy `diff-alert`)

Purpose: compare local alert exports against live Grafana.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--diff-dir` | Raw alert directory | Read-only comparison |
| `--json` | Structured diff output | Best for automation |
| `--dashboard-uid-map` | Dashboard mapping | Stable cross-environment compare |
| `--panel-id-map` | Panel mapping | Stable cross-environment compare |

Example command:
```bash
grafana-util alert diff --url http://localhost:3000 --basic-user admin --basic-password admin --diff-dir ./alerts/raw --json
```

Example output:
```json
{
  "summary": {
    "checked": 2,
    "same": 1,
    "different": 1,
    "missingRemote": 0
  },
  "rows": [
    {
      "path": "alerts/raw/contact-points/Smoke_Webhook/Smoke_Webhook__smoke-webhook.json",
      "kind": "grafana-contact-point",
      "identity": "smoke-webhook",
      "action": "different"
    },
    {
      "path": "alerts/raw/policies/notification-policies.json",
      "kind": "grafana-notification-policies",
      "identity": "grafana-default-email",
      "action": "same"
    }
  ]
}
```

### 4.9 `alert list-rules`
### 4.10 `alert list-contact-points`
### 4.11 `alert list-mute-timings`
### 4.12 `alert list-templates`

Purpose: list live alerting resources.

Common output options:

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--org-id` | Restrict to one org | Explicit org selection with Basic auth |
| `--all-orgs` | Aggregate visible orgs | Cross-org inventory with Basic auth |
| `--table` | Table output | Operators |
| `--csv` | CSV output | Spreadsheet export |
| `--json` | JSON output | Automation |
| `--output-format table\|csv\|json` | Unified output selector | Replaces the legacy trio |
| `--no-header` | Hide table header | Cleaner scripting |

Example command:
```bash
grafana-util alert list-rules --url http://localhost:3000 --basic-user admin --basic-password admin --table
```

Example output:
```text
UID                 TITLE              FOLDER        CONDITION
cpu-high            CPU High           linux-hosts   A > 80
memory-pressure     Memory Pressure    linux-hosts   B > 90
api-latency         API Latency        apps-prod     C > 500
```

`alert list-contact-points` example output:
```text
UID               NAME             TYPE      DESTINATION
oncall-webhook    Oncall Webhook   webhook   http://alert.example.com/hook
slack-primary     Slack Primary    slack     #ops-alerts
```

`alert list-mute-timings` example output:
```text
NAME                 INTERVALS
maintenance-window   mon-fri 01:00-02:00
release-freeze       sat-sun 00:00-23:59
```

`alert list-templates` example output:
```text
NAME               PREVIEW
default_message    Alert: {{ .CommonLabels.alertname }}
ops_summary        [{{ .Status }}] {{ .CommonLabels.severity }}
```

Cross-org note:
- `--org-id` and `--all-orgs` are Basic-auth-only for alert list commands because Grafana org switching requires a server-admin-style org scope change.

<a id="datasource-commands"></a>
5) Datasource Commands
----------------------

### 5.1 `datasource list`

Purpose: list live datasource inventory.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--table` | Table output | Operators |
| `--csv` | CSV output | Spreadsheet export |
| `--json` | JSON output | Automation |
| `--output-format table\|csv\|json` | Unified output selector | Replaces the legacy trio |
| `--no-header` | Hide table header | Cleaner scripting |

Example command:
```bash
grafana-util datasource list --url http://localhost:3000 --token <TOKEN> --table
```

Example output:
```text
UID                NAME               TYPE         URL
prom-main          prometheus-main    prometheus   http://prometheus:9090
loki-prod          loki-prod          loki         http://loki:3100
tempo-prod         tempo-prod         tempo        http://tempo:3200
```

Cross-org note:
- `--org-id` and `--all-orgs` are Basic-auth-only because datasource list must switch org context through Grafana admin APIs.

### 5.2 `datasource export`

Purpose: export datasource inventory as normalized JSON.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--export-dir` | Export directory | Default `datasources` |
| `--org-id` | Export from one explicit org | Basic-auth only explicit org export |
| `--all-orgs` | Export from all visible orgs | Writes one `org_<id>_<name>/` subtree per org |
| `--overwrite` | Replace existing export files | Repeatable export runs |
| `--dry-run` | Preview only | Validate destination first |

Example command:
```bash
grafana-util datasource export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./datasources --overwrite
```

Example output:
```text
Exported datasource inventory -> datasources/datasources.json
Exported metadata            -> datasources/export-metadata.json
Datasource export completed: 3 item(s)
```

Live note:
- The command shape above is exercised against a real Grafana `12.4.1` Docker server in the Rust live smoke flow.

### 5.3 `datasource import`

Purpose: import datasource inventory into live Grafana.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--import-dir` | Export root with `datasources.json` or combined export root | Use the combined root with `--use-export-org` |
| `--org-id` | Target org | Explicit org restore |
| `--use-export-org` | Route each exported org back into Grafana | Import a combined `--all-orgs` export root |
| `--only-org-id` | Restrict `--use-export-org` to selected source orgs | Repeat the flag to import multiple orgs |
| `--create-missing-orgs` | Create missing destination orgs before routed import | Only for `--use-export-org`; with `--dry-run` it reports `would-create-org` without creating anything |
| `--require-matching-export-org` | Enforce export org match | Safer multi-org restore |
| `--replace-existing` | Update existing datasources | Standard restore mode |
| `--update-existing-only` | Only touch existing datasources | Safer reconcile mode |
| `--dry-run` | Preview only | Recommended first |
| `--table` | Dry-run table output | Operator summary |
| `--json` | Dry-run JSON output | Automation |
| `--output-format text\|table\|json` | Dry-run output selector | Unified selector |
| `--output-columns` | Column whitelist | Tailored dry-run views |
| `--no-header` | Hide table header | Cleaner scripting |
| `--progress` | Show import progress | Large imports |
| `-v`, `--verbose` | Detailed logs | Troubleshooting |

Example command:
```bash
grafana-util datasource import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./datasources --replace-existing --dry-run --table
```

Example output:
```text
UID         NAME               TYPE         ACTION   DESTINATION
prom-main   prometheus-main    prometheus   update   existing
loki-prod   loki-prod          loki         create   missing
```

Live note:
- Real Docker-backed runs also validate routed datasource replay with `--use-export-org`, repeated `--only-org-id`, and `--create-missing-orgs`; in routed dry-run JSON the org preview reports `exists`, `missing-org`, or `would-create-org` before per-datasource actions.

How to read it:
- `UID` and `NAME` both matter, but automation should prefer `UID`.
- `TYPE` helps catch name collisions with wrong datasource types.
- `DESTINATION=missing` means the datasource is absent and the dry-run would create it.
- `DESTINATION=exists`, `exists-uid`, or `exists-name` tells you how the importer matched the live datasource before deciding whether it would update or skip it.

### 5.4 `datasource diff`

Purpose: compare exported datasource inventory with live Grafana.

| Option | Purpose |
| --- | --- |
| `--diff-dir` | Datasource export root directory |

Example command:
```bash
grafana-util datasource diff --url http://localhost:3000 --basic-user admin --basic-password admin --diff-dir ./datasources
```

Example output:
```text
Datasource diff found 1 differing item(s).

uid=loki-prod
- url=http://loki:3100
+ url=http://loki-prod:3100
```

### 5.5 `datasource add`

Purpose: create one live datasource directly in Grafana without using a local export bundle.

Note:
- `datasource add`, `datasource modify`, and `datasource delete` are part of the maintained `grafana-util` command surface.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--name` | Datasource name | Required |
| `--type` | Datasource plugin type id | Required |
| `--uid` | Stable datasource uid | Recommended |
| `--access` | Datasource access mode | Common values: `proxy`, `direct` |
| `--datasource-url` | Datasource target URL | Common HTTP datasource setup |
| `--default` | Mark as default datasource | Optional |
| `--basic-auth` | Enable upstream HTTP Basic auth | Common for protected Prometheus/Loki endpoints |
| `--basic-auth-user` | Basic auth username | Used with `--basic-auth-password` |
| `--basic-auth-password` | Basic auth password | Stored in `secureJsonData` |
| `--user` | Datasource user/login field | Common for Elasticsearch, SQL, InfluxDB |
| `--password` | Datasource password field | Stored in `secureJsonData` |
| `--with-credentials` | Set `withCredentials=true` | Browser credential forwarding for supported types |
| `--http-header NAME=VALUE` | Add one custom HTTP header | Repeat for multiple headers |
| `--tls-skip-verify` | Set `jsonData.tlsSkipVerify=true` | Relax TLS verification when needed |
| `--server-name` | Set `jsonData.serverName` | TLS/SNI override |
| `--json-data` | Inline `jsonData` JSON object | Advanced plugin-specific settings |
| `--secure-json-data` | Inline `secureJsonData` JSON object | Advanced secret-bearing settings |
| `--dry-run` | Preview only | Recommended first |
| `--table` / `--json` | Dry-run output mode | Operator or automation view |

Notes:
- Common type values include `prometheus`, `loki`, `elasticsearch`, `influxdb`, `graphite`, `postgres`, `mysql`, `mssql`, `tempo`, and `cloudwatch`.
- Dedicated auth/header flags are merged into the datasource payload. If the same key is already present in `--json-data` or `--secure-json-data`, the command fails closed instead of silently overwriting it.

Example: Prometheus with basic auth
```bash
grafana-util datasource add \
  --url http://localhost:3000 \
  --token <TOKEN> \
  --uid prom-main \
  --name prometheus-main \
  --type prometheus \
  --access proxy \
  --datasource-url http://prometheus:9090 \
  --basic-auth \
  --basic-auth-user metrics-user \
  --basic-auth-password metrics-pass \
  --dry-run --table
```

Example output:
```text
INDEX  NAME               TYPE         ACTION  DETAIL
1      prometheus-main    prometheus   create  would create datasource uid=prom-main
```

Example: Loki with tenant header
```bash
grafana-util datasource add \
  --url http://localhost:3000 \
  --token <TOKEN> \
  --uid loki-main \
  --name loki-main \
  --type loki \
  --access proxy \
  --datasource-url http://loki:3100 \
  --http-header X-Scope-OrgID=tenant-a \
  --dry-run --json
```

Example output:
```json
[
  {
    "name": "loki-main",
    "type": "loki",
    "action": "create",
    "detail": "would create datasource uid=loki-main"
  }
]
```

Example: InfluxDB with extra plugin settings
```bash
grafana-util datasource add \
  --url http://localhost:3000 \
  --token <TOKEN> \
  --uid influx-main \
  --name influx-main \
  --type influxdb \
  --access proxy \
  --datasource-url http://influxdb:8086 \
  --user influx-user \
  --password influx-pass \
  --json-data '{"version":"Flux","organization":"main-org","defaultBucket":"metrics"}' \
  --dry-run --table
```

Example output:
```text
INDEX  NAME          TYPE       ACTION  DETAIL
1      influx-main   influxdb   create  would create datasource uid=influx-main
```

Live note:
- The datasource mutation surface is validated in the Docker Grafana `12.4.1` live smoke path, including dry-run preview and persisted secret-field behavior on live add/modify flows.

<a id="access-commands"></a>
6) Access Commands
------------------

Use `access team` as the canonical team command path in this guide.

### 6.1 `access user list`

Purpose: list users in org or global scope.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--scope` | `org` or `global` | Select listing scope |
| `--query` | Fuzzy match on login/email/name | Broad discovery |
| `--login` | Exact login match | Precise lookup |
| `--email` | Exact email match | Precise lookup |
| `--org-role` | Filter by org role | Permission audit |
| `--grafana-admin` | Filter by server admin status | Admin audit |
| `--with-teams` | Include team membership | Team visibility |
| `--page`, `--per-page` | Pagination | Large user sets |
| `--table`, `--csv`, `--json` | Output mode | Human vs automation |
| `--output-format table\|csv\|json` | Unified output selector | Replaces the legacy trio |

Example command:
```bash
grafana-util access user list --url http://localhost:3000 --basic-user admin --basic-password admin --scope global --table
```

Example output:
```text
ID   LOGIN      EMAIL                NAME             ORG_ROLE   GRAFANA_ADMIN
1    admin      admin@example.com    Grafana Admin    Admin      true
7    svc-ci     ci@example.com       CI Service       Editor     false
9    alice      alice@example.com    Alice Chen       Viewer     false
```

How to read it:
- `ORG_ROLE` is org-local, not full server-admin authority.
- `GRAFANA_ADMIN=true` should normally be rare.

### 6.2 `access user add`

Purpose: create a user.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--login` | Login name | Required |
| `--email` | Email | Required |
| `--name` | Display name | Required |
| `--password` | Initial password | One password input option |
| `--password-file` | Read initial password from file | Safer non-interactive usage |
| `--prompt-user-password` | Prompt for initial password | Safer interactive usage |
| `--org-role` | Initial org role | Default role assignment |
| `--grafana-admin` | Server admin flag | Use sparingly |
| `--json` | JSON output | Automation |

Example command:
```bash
grafana-util access user add --url http://localhost:3000 --basic-user admin --basic-password admin --login bob --email bob@example.com --name "Bob Lin" --password '<SECRET>' --org-role Editor --json
```

Safer alternatives:
- Use exactly one of `--password`, `--password-file`, or `--prompt-user-password`.
- `--password-file` trims one trailing newline, which fits secret files created by common shell tools.

Example with a password file:
```bash
grafana-util access user add --url http://localhost:3000 --basic-user admin --basic-password admin --login bob --email bob@example.com --name "Bob Lin" --password-file ./secrets/bob-password.txt --org-role Editor --json
```

Example output:
```json
{
  "id": 12,
  "login": "bob",
  "email": "bob@example.com",
  "name": "Bob Lin",
  "orgRole": "Editor",
  "grafanaAdmin": false
}
```

### 6.3 `access user modify`

Purpose: update an existing user.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--user-id` / `--login` / `--email` | User locator | Choose one |
| `--set-login` | Change login | Rename account |
| `--set-email` | Change email | Contact update |
| `--set-name` | Change display name | Identity cleanup |
| `--set-password` | Reset password | One password input option |
| `--set-password-file` | Read new password from file | Safer non-interactive rotation |
| `--prompt-set-password` | Prompt for new password | Safer interactive rotation |
| `--set-org-role` | Change org role | Permission changes |
| `--set-grafana-admin` | Change server admin status | Permission changes |
| `--json` | JSON output | Automation |

Example command:
```bash
grafana-util access user modify --url http://localhost:3000 --basic-user admin --basic-password admin --login alice --set-email alice@example.com --set-org-role Editor --json
```

Safer alternatives:
- Use at most one of `--set-password`, `--set-password-file`, or `--prompt-set-password`.

Example with an interactive password prompt:
```bash
grafana-util access user modify --url http://localhost:3000 --basic-user admin --basic-password admin --login alice --prompt-set-password --set-org-role Editor --json
```

Example output:
```json
{
  "id": 9,
  "login": "alice",
  "result": "updated",
  "changes": ["set-org-role", "set-email"]
}
```

### 6.4 `access user delete`

Purpose: delete a user.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--user-id` / `--login` / `--email` | User locator | Choose one |
| `--scope` | `org` or `global` | Deletion scope |
| `--yes` | Skip confirmation | Typical for automation |
| `--json` | JSON output | Automation |

Example command:
```bash
grafana-util access user delete --url http://localhost:3000 --basic-user admin --basic-password admin --login temp-user --scope global --yes --json
```

Example output:
```json
{
  "id": 14,
  "login": "temp-user",
  "scope": "global",
  "result": "deleted"
}
```

### 6.5 `access user export`

Purpose: export users and role/team membership snapshots for migration.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--export-dir` | Directory to write `users.json` and `export-metadata.json` | Default is `access-users` |
| `--overwrite` | Replace existing output files | Controlled by automation |
| `--dry-run` | Show planned outputs only | Useful for folder and permission checks |
| `--scope` | `org` or `global` | Choose target identity scope |
| `--with-teams` | Include team memberships in each user record | Enable for migration replay |

Example command:
```bash
grafana-util access user export --url http://localhost:3000 --token <TOKEN> --export-dir ./access-users --scope org --with-teams
```

Example output:
```text
Exported users from http://localhost:3000 -> /tmp/access-users/users.json and /tmp/access-users/export-metadata.json
```

### 6.6 `access user import`

Purpose: import users from exported snapshot files.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--import-dir` | Directory that contains `users.json` and `export-metadata.json` | Must match export layout |
| `--scope` | `org` or `global` | Resolve duplicate matching rules |
| `--replace-existing` | Update existing user records | Required for repeat reconcile runs |
| `--dry-run` | Plan actions only, no API mutation | Safer first pass |
| `--yes` | Skip confirmation for destructive membership removals | Required when team removals are detected |
| `--table`, `--json`, `--output-format table/json` | Dry-run output mode selector | Available only with `--dry-run`; mutually exclusive |

Example command:
```bash
grafana-util access user import --url http://localhost:3000 --token <TOKEN> --import-dir ./access-users --replace-existing --dry-run --output-format table
```

Example output:
```text
INDEX  IDENTITY        ACTION        DETAIL
1      alice@example.com skip          existing and --replace-existing was not set.
2      bob@example.com   create        would create user
3      carol@example.com update-admin  would update grafanaAdmin -> true

Import summary: processed=3 created=1 updated=1 skipped=1 source=./access-users
```

For JSON dry-run:
```json
[
  {"index":"2","identity":"bob@example.com","action":"create","detail":"would create user"}
]
```

### `access user diff`

Purpose: compare an exported users snapshot with live users.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--diff-dir` | Directory containing `users.json` and `export-metadata.json` | Default is `access-users` |
| `--scope` | `org` or `global` | Compare under the same identity scope |

Example command:
```bash
grafana-util access user diff --url http://localhost:3000 --token <TOKEN> --diff-dir ./access-users --scope org
```

Example output:
```text
Diff checked 2 user(s).
alice@example.com  UPDATE  change role from Viewer to Editor
bob@example.com    DELETE  user not present in snapshot
```

### `access team diff`

Purpose: compare an exported teams snapshot with live teams and memberships.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--diff-dir` | Directory containing `teams.json` and `export-metadata.json` | Default is `access-teams` |

Example command:
```bash
grafana-util access team diff --url http://localhost:3000 --token <TOKEN> --diff-dir ./access-teams
```

Example output:
```text
Diff checked 1 team(s).
Ops               UPDATE   add-member alice@example.com
SRE               DELETE   team absent from snapshot
```

### 6.7 `access team list`

Purpose: list teams.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--query` | Fuzzy team search | Discovery |
| `--name` | Exact team name | Precise lookup |
| `--with-members` | Include members | Team audits |
| `--page`, `--per-page` | Pagination | Large orgs |
| `--table`, `--csv`, `--json` | Output mode | Human vs automation |
| `--output-format table\|csv\|json` | Unified output selector | Replaces the legacy trio |

Example command:
```bash
grafana-util access team list --url http://localhost:3000 --token <TOKEN> --with-members --table
```

Example output:
```text
ID   NAME        EMAIL              MEMBERS   ADMINS
3    sre-team    sre@example.com    5         2
7    app-team    app@example.com    8         1
```

### 6.8 `access team add`

Purpose: create a team.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--name` | Team name | Required |
| `--email` | Team email | Optional metadata |
| `--member` | Initial member | Repeatable |
| `--admin` | Initial admin | Repeatable |
| `--json` | JSON output | Automation |

Example command:
```bash
grafana-util access team add --url http://localhost:3000 --token <TOKEN> --name platform-team --email platform@example.com --member alice --member bob --admin alice --json
```

Example output:
```json
{
  "teamId": 15,
  "name": "platform-team",
  "membersAdded": 2,
  "adminsAdded": 1
}
```

### 6.9 `access team modify`

Purpose: adjust team members and admins.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--team-id` / `--name` | Team locator | Choose one |
| `--add-member` / `--remove-member` | Member changes | Repeatable |
| `--add-admin` / `--remove-admin` | Admin changes | Repeatable |
| `--json` | JSON output | Automation |

Example command:
```bash
grafana-util access team modify --url http://localhost:3000 --token <TOKEN> --name platform-team --add-member carol --remove-member bob --remove-admin alice --json
```

Example output:
```json
{
  "teamId": 15,
  "name": "platform-team",
  "membersAdded": 1,
  "membersRemoved": 1,
  "adminsRemoved": 1
}
```

### 6.10 `access team delete`

Purpose: delete a team.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--team-id` / `--name` | Team locator | Choose one |
| `--yes` | Skip confirmation | Typical for automation |
| `--json` | JSON output | Automation |

Example command:
```bash
grafana-util access team delete --url http://localhost:3000 --token <TOKEN> --name platform-team --yes --json
```

Example output:
```json
{
  "teamId": 15,
  "name": "platform-team",
  "result": "deleted"
}
```

### 6.11 `access team export`

Purpose: export teams and member/admin membership snapshots for migration.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--export-dir` | Directory to write `teams.json` and `export-metadata.json` | Default is `access-teams` |
| `--overwrite` | Replace existing output files | Controlled by automation |
| `--dry-run` | Show planned outputs only | Useful for folder and permission checks |
| `--with-members` | Include members/admins in each team record | Required for membership replay |

Example command:
```bash
grafana-util access team export --url http://localhost:3000 --token <TOKEN> --export-dir ./access-teams --with-members
```

Example output:
```text
Exported teams from http://localhost:3000 -> /tmp/access-teams/teams.json and /tmp/access-teams/export-metadata.json
```

### 6.12 `access team import`

Purpose: import teams and synchronize memberships from exported snapshots.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--import-dir` | Directory that contains `teams.json` and `export-metadata.json` | Must match export layout |
| `--replace-existing` | Update existing teams rather than skip | Required for cross-instance replay |
| `--dry-run` | Plan actions only, no API mutation | Recommended before replay |
| `--yes` | Skip confirmation for destructive removals | Required when members would be removed |
| `--table`, `--json`, `--output-format table/json` | Dry-run output mode selector | Available only with `--dry-run`; mutually exclusive |

Example command:
```bash
grafana-util access team import --url http://localhost:3000 --token <TOKEN> --import-dir ./access-teams --replace-existing --dry-run --output-format table
```

Example output:
```text
INDEX  IDENTITY         ACTION       DETAIL
1      platform-team    skip         existing and --replace-existing was not set.
2      sre-team         create       would create team
3      edge-team        add-member   would add team member alice@example.com
4      edge-team        remove-member would remove team member bob@example.com

Import summary: processed=4 created=1 updated=1 skipped=1 source=./access-teams
```

### 6.13 `access service-account list`

Purpose: list service accounts.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--query` | Fuzzy name search | Discovery |
| `--page`, `--per-page` | Pagination | Large estates |
| `--table`, `--csv`, `--json` | Output mode | Human vs automation |
| `--output-format table\|csv\|json` | Unified output selector | Replaces the legacy trio |

Example command:
```bash
grafana-util access service-account list --url http://localhost:3000 --token <TOKEN> --table
```

Example output:
```text
ID   NAME          ROLE     DISABLED
2    ci-bot        Editor   false
5    backup-bot    Viewer   true
```

### 6.14 `access service-account add`

Purpose: create a service account.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--name` | Service account name | Required |
| `--role` | `Viewer\|Editor\|Admin\|None` | Default `Viewer` |
| `--disabled` | Disabled flag | Textual boolean in Rust CLI |
| `--json` | JSON output | Automation |

Example command:
```bash
grafana-util access service-account add --url http://localhost:3000 --token <TOKEN> --name deploy-bot --role Editor --json
```

Example output:
```json
{
  "id": 21,
  "name": "deploy-bot",
  "role": "Editor",
  "disabled": false
}
```

### 6.15 `access service-account export`

Purpose: export service-account snapshots for backup, reconciliation, or cross-environment review.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--export-dir` | Directory that receives `service-accounts.json` and `export-metadata.json` | Default `access-service-accounts` |
| `--overwrite` | Replace existing snapshot files | Repeatable backup jobs |
| `--dry-run` | Preview output paths without writing files | Check target path first |

Example command:
```bash
grafana-util access service-account export --url http://localhost:3000 --token <TOKEN> --export-dir ./access-service-accounts --overwrite
```

Example output:
```text
Exported 3 service-account(s) from http://localhost:3000 -> access-service-accounts/service-accounts.json and access-service-accounts/export-metadata.json
```

Live note:
- This snapshot flow is covered by `make test-access-live` against Grafana `12.4.1`, including export, diff, dry-run import, live replay, delete, and token lifecycle commands.

### 6.16 `access service-account import`

Purpose: replay service-account snapshot files into Grafana.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--import-dir` | Directory containing `service-accounts.json` and `export-metadata.json` | Must match export layout |
| `--replace-existing` | Create missing service accounts and update existing ones | Required for replay |
| `--dry-run` | Preview create/update/skip decisions without writing | Recommended first pass |
| `--table`, `--json`, `--output-format text\|table\|json` | Dry-run output mode | Summary vs machine-readable review |

Example command:
```bash
grafana-util access service-account import --url http://localhost:3000 --token <TOKEN> --import-dir ./access-service-accounts --replace-existing --dry-run --output-format table
```

Example output:
```text
INDEX  IDENTITY     ACTION  DETAIL
1      deploy-bot   update  would update fields=role,disabled
2      report-bot   create  would create service account

Import summary: processed=2 created=1 updated=1 skipped=0 source=./access-service-accounts
```

Live note:
- The live smoke rewrites an exported snapshot, confirms the dry-run update preview, then replays the same file into Grafana to verify the live update path.

### 6.17 `access service-account diff`

Purpose: compare service-account snapshot files with live Grafana state.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--diff-dir` | Directory containing `service-accounts.json` and `export-metadata.json` | Default `access-service-accounts` |

Example command:
```bash
grafana-util access service-account diff --url http://localhost:3000 --token <TOKEN> --diff-dir ./access-service-accounts
```

Example output:
```text
Diff different service-account deploy-bot fields=role
Diff missing-live service-account report-bot
Diff extra-live service-account old-bot
Diff checked 3 service-account(s); 3 difference(s) found.
```

### 6.18 `access service-account delete`

Purpose: delete a service account.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--service-account-id` / `--name` | Locator | Choose one |
| `--yes` | Skip confirmation | Typical for automation |
| `--json` | JSON output | Automation |

Example command:
```bash
grafana-util access service-account delete --url http://localhost:3000 --token <TOKEN> --name deploy-bot --yes --json
```

Example output:
```json
{
  "id": 21,
  "name": "deploy-bot",
  "result": "deleted"
}
```

### 6.19 `access service-account token add`

Purpose: create a service-account token.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--service-account-id` / `--name` | Owner locator | Choose one |
| `--token-name` | Token name | Required |
| `--seconds-to-live` | Token TTL in seconds | Optional expiry |
| `--json` | JSON output | Automation |

Example command:
```bash
grafana-util access service-account token add --url http://localhost:3000 --token <TOKEN> --name deploy-bot --token-name ci-token --seconds-to-live 86400 --json
```

Example output:
```json
{
  "serviceAccountId": 21,
  "tokenId": 34,
  "tokenName": "ci-token",
  "secondsToLive": 86400,
  "key": "glsa_xxxxxxxxx"
}
```

### 6.20 `access service-account token delete`

Purpose: delete a service-account token.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--service-account-id` / `--name` | Owner locator | Choose one |
| `--token-id` / `--token-name` | Token locator | Choose one |
| `--yes` | Skip confirmation | Typical for automation |
| `--json` | JSON output | Automation |

Example command:
```bash
grafana-util access service-account token delete --url http://localhost:3000 --token <TOKEN> --name deploy-bot --token-name ci-token --yes --json
```

Example output:
```json
{
  "serviceAccountId": 21,
  "tokenName": "ci-token",
  "result": "deleted"
}
```

<a id="shared-output-rules"></a>
7) Shared Output Rules
----------------------

| Rule | Explanation |
| --- | --- |
| Output flags are mutually exclusive | Most commands do not allow `--table`, `--csv`, `--json`, and `--output-format` together |
| Prefer dry-run first | Especially for import-like workflows |
| Org control is explicit | `--org-id` and `--all-orgs` should be used deliberately |
| Top-level names are distinct by role | Use `overview` for human entry, `status` for the canonical readiness contract, and `change` for staged change workflows |
| Prefer canonical team commands | Use `access team` throughout this guide |

### 7.1 Change, overview, and status surfaces

- `change` is the staged change lane: build desired summaries, source bundles, preflight checks, review/apply intent documents, and alert-change assessment.
- `overview` is the human project entrypoint: use it for operator-facing staged or live snapshots when you want one readable project home.
- `status` is the canonical readiness surface: use `staged` for exported artifacts and `live` for current Grafana reads.

### 7.2 `change summary`

Purpose: normalize a desired resource list into one stable staged summary document.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--desired-file` | Input desired-resource JSON | Required |
| `--output text\|json` | Render summary document | Use JSON for later plan/review stages |

Example command:
```bash
grafana-util change summary --desired-file ./desired.json --output json
```

Example output:
```json
{
  "kind": "grafana-utils-sync-summary",
  "summary": {
    "resourceCount": 4,
    "dashboardCount": 1,
    "datasourceCount": 1,
    "folderCount": 1,
    "alertCount": 1
  }
}
```

### 7.3 `change bundle` and `change bundle-preflight`

Purpose: turn exported dashboard/alert/datasource artifacts into one source bundle, then evaluate what blocks or stays plan-only before apply.

| Command | Key flags | Main use |
| --- | --- | --- |
| `change bundle` | `--dashboard-export-dir`, `--alert-export-dir`, `--datasource-export-file`, `--output-file` | package staged exports into one portable source bundle |
| `change bundle-preflight` | `--source-bundle`, `--target-inventory`, `--availability-file` | review blocking plugin, datasource, alert-artifact, and secret/provider checks |

Example command:
```bash
grafana-util change bundle \
  --dashboard-export-dir ./dashboards/raw \
  --alert-export-dir ./alerts/raw \
  --datasource-export-file ./datasources/datasources.json \
  --output-file ./change-source-bundle.json \
  --output json
```

Example output excerpt:
```json
{
  "kind": "grafana-utils-sync-source-bundle",
  "summary": {
    "dashboardCount": 7,
    "datasourceCount": 3,
    "folderCount": 5,
    "alertRuleCount": 1,
    "contactPointCount": 1,
    "muteTimingCount": 1,
    "policyCount": 1,
    "templateCount": 1
  }
}
```

Example command:
```bash
grafana-util change bundle-preflight \
  --source-bundle ./change-source-bundle.json \
  --target-inventory ./target-inventory.json \
  --output json
```

Example output excerpt:
```json
{
  "kind": "grafana-utils-sync-bundle-preflight",
  "summary": {
    "resourceCount": 20,
    "syncBlockingCount": 8,
    "alertArtifactCount": 4,
    "alertArtifactPlanOnlyCount": 1,
    "alertArtifactBlockedCount": 3
  }
}
```

How to read it:
- `change bundle` is the packaging step.
- `change bundle-preflight` is the review step that surfaces what is ready, plan-only, or blocked before any live apply path is considered.

### 7.4 `change plan`, `change review`, `change apply`, and `change assess-alerts`

Purpose: convert staged desired resources into a reviewable plan, stamp that plan reviewed, then emit a gated apply intent. Use `assess-alerts` when you need only the alert-change review signal.

| Command | Key flags | Main use |
| --- | --- | --- |
| `change plan` | `--desired-file`, `--live-file` or `--fetch-live`, `--allow-prune`, `--output json` | build the staged plan document |
| `change review` | `--plan-file`, `--review-note`, `--reviewed-by` | mark a plan reviewed without applying it |
| `change apply` | `--plan-file`, `--approve`, `--execute-live` | emit a local apply intent, or execute live when explicitly enabled |
| `change assess-alerts` | `--alerts-file`, `--output json` | isolate alert candidate/plan-only/blocked classification |

Example command:
```bash
grafana-util change plan --desired-file ./desired-plan.json --live-file ./live.json --output json
```

Example output excerpt:
```json
{
  "kind": "grafana-utils-sync-plan",
  "summary": {
    "would_create": 3,
    "would_update": 0,
    "would_delete": 0,
    "noop": 0
  },
  "reviewRequired": true
}
```

Example command:
```bash
grafana-util change review --plan-file ./change-plan.json --review-note "docs-reviewed" --reviewed-by docs-user --output json
grafana-util change apply --plan-file ./change-plan-reviewed.json --approve --output json
```

Example output excerpt:
```json
{
  "kind": "grafana-utils-sync-apply-intent",
  "approved": true,
  "reviewed": true,
  "summary": {
    "would_create": 3,
    "would_update": 0,
    "would_delete": 0,
    "noop": 0
  }
}
```

Example command:
```bash
grafana-util change assess-alerts --alerts-file ./alerts-only.json --output json
```

Example output excerpt:
```json
{
  "kind": "grafana-utils-alert-sync-plan",
  "summary": {
    "alertCount": 1,
    "candidateCount": 0,
    "planOnlyCount": 1,
    "blockedCount": 0
  }
}
```

### 7.5 `overview`

Purpose: summarize staged exports and staged change inputs into one project-wide operator snapshot.

| Option | Purpose | Difference / scenario |
| --- | --- | --- |
| `--dashboard-export-dir` | Staged dashboard export root | Usually one `raw/` directory |
| `--datasource-export-dir` | Staged datasource export directory | Usually the org export directory containing `datasources.json` |
| `--alert-export-dir` | Staged alert export directory | Point at the export root, not just `raw/` |
| `--access-*-export-dir` | Staged access bundles | Add only the bundles you want summarized |
| `--desired-file` | Optional change summary input | Adds staged change rows |
| `--source-bundle`, `--target-inventory`, `--mapping-file` | Optional bundle/promotion context | Broadens the project-level staged picture |
| `--output text\|json\|interactive` | Render format | `interactive` requires the TUI-capable build |

Example command:
```bash
grafana-util overview \
  --dashboard-export-dir ./dashboards/raw \
  --datasource-export-dir ./datasources \
  --alert-export-dir ./alerts \
  --access-user-export-dir ./access-users \
  --access-team-export-dir ./access-teams \
  --access-org-export-dir ./access-orgs \
  --access-service-account-export-dir ./access-service-accounts \
  --desired-file ./desired.json \
  --output text
```

Example output:
```text
Project overview
Status: blocked domains=6 present=5 blocked=1 blockers=3 warnings=0 freshness=current oldestAge=222s
Artifacts: 8 total, 1 dashboard export, 1 datasource export, 1 alert export, 1 access user export, 1 access team export, 1 access org export, 1 access service-account export, 1 change summary, 0 bundle preflight, 0 promotion preflight
Domain status:
- dashboard status=blocked reason=blocked-by-blockers primary=10 blockers=3 warnings=0 freshness=current next=resolve orphaned datasources, then mixed dashboards
- datasource status=ready reason=ready primary=3 blockers=0 warnings=0 freshness=current
- alert status=ready reason=ready primary=1 blockers=0 warnings=0 freshness=current next=re-run alert export after alerting changes
- access status=ready reason=ready primary=13 blockers=0 warnings=0 freshness=current next=re-run access export after membership changes
- change status=ready reason=ready primary=4 blockers=0 warnings=0 freshness=current next=re-run change summary after staged changes
```

### 7.6 `status staged` and `status live`

Purpose: render the canonical project-wide readiness contract from either staged exports or current Grafana state.

| Command | Key flags | Main use |
| --- | --- | --- |
| `status staged` | staged export dirs plus optional desired/bundle inputs | machine-readable staged readiness |
| `status live` | `--url`, auth, optional staged context files | machine-readable current Grafana status |
| `overview live` | same live auth flags | human-facing live project read that routes through the shared `status live` path |

Example command:
```bash
grafana-util status staged \
  --dashboard-export-dir ./dashboards/raw \
  --datasource-export-dir ./datasources \
  --alert-export-dir ./alerts \
  --access-user-export-dir ./access-users \
  --access-team-export-dir ./access-teams \
  --access-org-export-dir ./access-orgs \
  --access-service-account-export-dir ./access-service-accounts \
  --desired-file ./desired.json \
  --output json
```

Example output excerpt:
```json
{
  "scope": "staged-only",
  "overall": {
    "status": "blocked",
    "domainCount": 6,
    "blockedCount": 1,
    "blockerCount": 3
  }
}
```

Example command:
```bash
grafana-util status live --url http://localhost:3000 --basic-user admin --basic-password admin --output json
```

Example output excerpt:
```json
{
  "scope": "live",
  "overall": {
    "status": "blocked",
    "domainCount": 6,
    "blockedCount": 1,
    "blockerCount": 1,
    "warningCount": 21
  }
}
```

<a id="common-operator-scenarios"></a>
8) Common Operator Scenarios
----------------------------

### 8.1 Cross-environment dashboard migration

1. `grafana-util dashboard export --all-orgs --overwrite --flat --export-dir ./dashboards`
2. `grafana-util dashboard import --dry-run --replace-existing --table --import-dir ./dashboards/raw`
3. Remove `--dry-run` after reviewing the output.

### 8.2 Audit only

1. Use `dashboard diff`, `datasource diff`, or `alert diff`.
2. Use `dashboard inspect-export` or `dashboard inspect-live` for structural analysis.
3. Prefer JSON output when another system will parse the results.

### 8.3 Access cleanup

1. Start with `access user list --scope global --table`.
2. Use `access user modify` for role changes.
3. Use `access team modify` for membership changes.
4. Use `access service-account` and token commands for automation identities.
5. Validate any snapshot migration with `access user diff` and `access team diff` before import.

### 8.4 Dashboard governance gate in CI

1. Export dashboards into a raw tree, or reuse the raw export committed in the repo.
2. Generate the governance and flat query reports:

```bash
grafana-util dashboard inspect-export --import-dir ./dashboards/raw --report governance-json > governance.json
grafana-util dashboard inspect-export --import-dir ./dashboards/raw --report json > queries.json
```

3. Evaluate a team policy file against those reports:

```bash
./scripts/check_dashboard_governance.py \
  --policy examples/dashboard-governance-policy.json \
  --governance governance.json \
  --queries queries.json \
  --json-output governance-check.json
```

4. Use `--import-dir ./dashboards/raw` only as a fallback when you are feeding older governance artifacts that do not yet carry dashboard dependency facts in `governance.json`.

5. Review `governance-check.json` in CI artifacts when the gate fails. The governance-json-first checker can block on:
   - datasource family or uid allowlists
   - unknown datasource identity
   - mixed-datasource dashboards
   - panel plugin allowlists
   - library panel allowlists
   - disallowed dashboard folder prefixes for routing boundaries
   - undefined datasource variables referenced by dashboard panels
   - query count thresholds
   - query or dashboard complexity thresholds
   - SQL `select *`
   - missing SQL Grafana time filters
   - broad Loki selectors or regexes

<a id="minimal-sop-commands"></a>
9) Minimal SOP Commands
-----------------------

```bash
grafana-util dashboard export --url <URL> --basic-user <USER> --basic-password <PASS> --export-dir <DIR> [--overwrite] [--all-orgs] [--flat]
grafana-util dashboard list --url <URL> --basic-user <USER> --basic-password <PASS> [--table|--csv|--json]
grafana-util dashboard import --url <URL> --basic-user <USER> --basic-password <PASS> --import-dir <DIR>/raw --replace-existing [--dry-run]
grafana-util dashboard delete --url <URL> --basic-user <USER> --basic-password <PASS> [--org-id <ORG_ID>] (--uid <UID>|--path <FOLDER_PATH>) [--delete-folders] [--dry-run|--yes]
grafana-util dashboard diff --url <URL> --basic-user <USER> --basic-password <PASS> --import-dir <DIR>/raw

grafana-util alert export --url <URL> --basic-user <USER> --basic-password <PASS> --output-dir <DIR> [--overwrite]
grafana-util alert import --url <URL> --basic-user <USER> --basic-password <PASS> --import-dir <DIR>/raw --replace-existing [--dry-run] [--json]
grafana-util alert diff --url <URL> --basic-user <USER> --basic-password <PASS> --diff-dir <DIR>/raw [--json]
grafana-util alert list-rules --url <URL> --basic-user <USER> --basic-password <PASS> [--org-id <ORG_ID>|--all-orgs] [--table|--csv|--json]

grafana-util datasource list --url <URL> --token <TOKEN> [--table|--csv|--json]
grafana-util datasource list --url <URL> --basic-user <USER> --basic-password <PASS> [--org-id <ORG_ID>|--all-orgs] [--table|--csv|--json]
grafana-util datasource add --url <URL> --token <TOKEN> --name <NAME> --type <TYPE> [--uid <UID>] [--access proxy|direct] [--datasource-url <URL>] [--basic-auth] [--basic-auth-user <USER>] [--basic-auth-password <PASS>] [--user <USER>] [--password <PASS>] [--with-credentials] [--http-header NAME=VALUE] [--tls-skip-verify] [--server-name <NAME>] [--json-data <JSON>] [--secure-json-data <JSON>] [--dry-run] [--output-format text|table|json]
grafana-util datasource export --url <URL> --basic-user <USER> --basic-password <PASS> --export-dir <DIR> [--overwrite] [--org-id <ORG_ID>|--all-orgs]
grafana-util datasource import --url <URL> --basic-user <USER> --basic-password <PASS> --import-dir <DIR> --replace-existing [--org-id <ORG_ID>] [--use-export-org [--only-org-id <ORG_ID>]... [--create-missing-orgs]] [--dry-run]
grafana-util datasource diff --url <URL> --basic-user <USER> --basic-password <PASS> --diff-dir <DIR>

grafana-util access user list --url <URL> --basic-user <USER> --basic-password <PASS> --scope global --table
grafana-util access team list --url <URL> --token <TOKEN> --table
grafana-util access user export --url <URL> --token <TOKEN> --export-dir ./access-users
grafana-util access team export --url <URL> --token <TOKEN> --export-dir ./access-teams
grafana-util access user import --url <URL> --token <TOKEN> --import-dir ./access-users --replace-existing --dry-run --output-format table
grafana-util access team import --url <URL> --token <TOKEN> --import-dir ./access-teams --replace-existing --dry-run --output-format table
grafana-util access user diff --url <URL> --token <TOKEN> --diff-dir ./access-users
grafana-util access team diff --url <URL> --token <TOKEN> --diff-dir ./access-teams
grafana-util access service-account export --url <URL> --token <TOKEN> --export-dir ./access-service-accounts [--overwrite]
grafana-util access service-account import --url <URL> --token <TOKEN> --import-dir ./access-service-accounts --replace-existing [--dry-run] [--output-format text|table|json]
grafana-util access service-account diff --url <URL> --token <TOKEN> --diff-dir ./access-service-accounts
grafana-util access service-account list --url <URL> --token <TOKEN> --table

grafana-util change summary --desired-file ./desired.json --output json
grafana-util change bundle --dashboard-export-dir ./dashboards/raw --alert-export-dir ./alerts/raw --datasource-export-file ./datasources/datasources.json --output-file ./change-source-bundle.json --output json
grafana-util change bundle-preflight --source-bundle ./change-source-bundle.json --target-inventory ./target-inventory.json --output json
grafana-util change plan --desired-file ./desired.json --live-file ./live.json --output json
grafana-util change review --plan-file ./change-plan.json --review-note "peer-reviewed" --reviewed-by ops-user --output json
grafana-util change apply --plan-file ./change-plan-reviewed.json --approve --output json
grafana-util change assess-alerts --alerts-file ./alerts-only.json --output json

grafana-util overview --dashboard-export-dir ./dashboards/raw --datasource-export-dir ./datasources --alert-export-dir ./alerts --output text
grafana-util overview live --url <URL> --basic-user <USER> --basic-password <PASS> --output json
grafana-util status staged --dashboard-export-dir ./dashboards/raw --datasource-export-dir ./datasources --alert-export-dir ./alerts --output json
grafana-util status live --url <URL> --basic-user <USER> --basic-password <PASS> --output json
```

<a id="output-and-org-control-matrix"></a>
10) Output and Org Control Matrix
---------------------------------

| Command | `--output-format` values | Notes |
| --- | --- | --- |
| `dashboard list` | `table/csv/json` | Replaces legacy output flags |
| `dashboard import` | `text/table/json` | Dry-run focused |
| `dashboard delete` | `text/table/json` | Dry-run focused |
| `alert list-*` | `table/csv/json` | Shared across list commands |
| `datasource list` | `table/csv/json` | Shared list pattern |
| `datasource add` | `text/table/json` | Dry-run capable |
| `datasource import` | `text/table/json` | Dry-run supports single-org previews plus routed org-summary preview |
| `access list` commands | `table/csv/json` | Shared list pattern |
| `access user import` | `text/table/json` | Dry-run table/json/ text summary |
| `access team import` | `text/table/json` | Dry-run table/json/text summary |
| `access user diff` | text | Summary output |
| `access team diff` | text | Summary output |
| `access service-account import` | `text/table/json` | Dry-run table/json/text summary |
| `access service-account diff` | text | Summary output |
| `change summary` | `text/json` | Desired-resource summary |
| `change bundle` | `text/json` | Source bundle document |
| `change bundle-preflight` | `text/json` | Bundle review document |
| `change plan` | `text/json` | Reviewable change plan |
| `change review` | `text/json` | Reviewed plan stamp |
| `change apply` | `text/json` | Apply intent or live execution summary |
| `change assess-alerts` | `text/json` | Alert-change classification |
| `overview` | `text/json/interactive` | Project-wide staged overview |
| `status staged/live` | `text/json/interactive` | Canonical project-wide readiness |

Common dry-run status hints:
- `missing`: no live target exists yet.
- `exists`, `exists-uid`, `exists-name`, `existing`: a live target was matched before the CLI decided whether it would update or skip it.
- `missing-org`, `would-create-org`: routed multi-org restore needs org creation or operator review first.

| Command | `--org-id` | `--all-orgs` |
| --- | --- | --- |
| `dashboard list` | Yes | Yes |
| `dashboard export` | Yes | Yes |
| `dashboard import` | Yes | No |
| `dashboard delete` | Yes | No |
| `datasource list` | Yes | Yes |
| `datasource export` | Yes | Yes |
| `datasource import` | Yes | No |
| `alert list-*` | Yes | Yes |
| `alert export/import/diff` | No | No |
| `alert plan/apply/delete` | No | No |
| `access` commands | No | No |
