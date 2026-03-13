# Developer Notes

This document is for maintainers. Keep `README.md` GitHub-facing and task-oriented; put implementation detail, internal tradeoffs, and maintenance notes here.

Commit message default for this repo:

- first line: short imperative title
- blank line
- then 2-4 flat `- ...` detail bullets
- keep the bullets concrete about code, tests, docs, or migration impact

## Repository Scope

- `grafana_utils/dashboard_cli.py`: packaged dashboard export/import utility
- `grafana_utils/datasource_cli.py`: packaged datasource inventory list/export utility
- `grafana_utils/dashboards/export_workflow.py`: Python dashboard export orchestration helper that keeps the CLI-facing export workflow out of `dashboard_cli.py`
- `grafana_utils/dashboards/export_inventory.py`: Python dashboard raw-export discovery, inventory loading, and export metadata validation helpers shared by diff/import/inspect paths
- `grafana_utils/dashboards/inspection_report.py`: Python dashboard inspection report model, column/mode constants, query-row normalization, and flat/grouped report renderers shared by `inspect-export` and `inspect-live`
- `grafana_utils/dashboards/inspection_summary.py`: Python dashboard inspection summary builder and summary/table renderers for offline and live inspection paths
- `grafana_utils/dashboards/listing.py`: Python dashboard live list/datasource-list renderers plus datasource/source-enrichment helpers shared by the stable CLI facade
- `grafana_utils/dashboards/inspection_workflow.py`: Python dashboard inspect-live and inspect-export orchestration helper that reuses the existing render/analysis functions through dependency injection
- `grafana_utils/dashboards/import_workflow.py`: Python dashboard import orchestration helper for dry-run, ensure-folder, and live import flows
- `grafana_utils/alert_cli.py`: packaged alerting resource export/import utility
- `grafana_utils/access_cli.py`: packaged access-management utility, currently covering `user list`, `user add`, `user modify`, `user delete`, `team list`, `team add`, `team modify`, and initial service-account commands
- `rust/src/access.rs`: Rust access-management orchestration entrypoint and shared request helpers
- `rust/src/access_cli_defs.rs`: Rust access CLI arg definitions and auth/client builders
- `rust/src/access_render.rs`: Rust access table/CSV/JSON renderers and row normalization helpers
- `rust/src/access_user.rs`: Rust access user list/add/modify/delete flows
- `rust/src/access_team.rs`: Rust access team list/add/modify flows
- `rust/src/access_service_account.rs`: Rust access service-account list/add/token-add flows
- `rust/src/alert.rs`: Rust alert orchestration entrypoint plus shared alert import/export/diff helpers
- `rust/src/alert_cli_defs.rs`: Rust alert CLI arg definitions and auth-context builders
- `rust/src/alert_client.rs`: Rust Grafana alert provisioning HTTP client wrapper and shared response parsers
- `rust/src/alert_list.rs`: Rust alert list rendering and list-command orchestration
- `rust/src/dashboard.rs`: Rust dashboard orchestration entrypoint and shared dashboard helpers that are still used across import, diff, and prompt-export flows
- `rust/src/dashboard_cli_defs.rs`: Rust dashboard CLI arg definitions and auth/client builders
- `rust/src/dashboard_files.rs`: Rust dashboard raw-export file discovery, inventory loading, and export metadata validation helpers shared by diff/import/inspect paths
- `rust/src/dashboard_list.rs`: Rust dashboard and datasource list rendering plus multi-org list orchestration
- `rust/src/dashboard_export.rs`: Rust dashboard export pathing and multi-org export orchestration
- `rust/src/dashboard_prompt.rs`: Rust dashboard prompt-export datasource resolution and template-rewrite logic
- `grafana_utils/http_transport.py`: shared HTTP transport adapters and transport selection
- `grafana_utils/unified_cli.py`: unified Python entrypoint that dispatches dashboard, datasource, alert, and access workflows
- `python/grafana-utils.py`: thin source-tree wrapper for the packaged unified CLI
- `rust/src/cli.rs`: unified Rust entrypoint that dispatches dashboard, alert, and access workflows
- `rust/src/bin/grafana-access-utils.rs`: thin Rust compatibility binary for the access-management CLI
- `pyproject.toml`: build metadata, dependencies, and console-script entrypoints
- `tests/test_python_dashboard_cli.py`: dashboard Python unit tests
- `tests/test_python_dashboard_inspection_cli.py`: focused Python inspection summary/report tests kept separate from the broader dashboard CLI suite
- `tests/test_python_alert_cli.py`: alerting Python unit tests
- `tests/test_python_packaging.py`: Python package metadata and console-script tests
- `Makefile`: shared developer shortcuts for Python wheel builds, Rust release builds, and test runs
- `.github/workflows/ci.yml`: baseline CI gates for Python tests plus Rust tests/format/lint checks
- `scripts/build-rust-macos-arm64.sh`: native Apple Silicon Rust release build helper that copies binaries into `dist/macos-arm64/`
- `scripts/build-rust-linux-amd64.sh`: Docker-based Linux `amd64` Rust build helper for macOS or other non-Linux hosts
- `scripts/build-rust-linux-amd64-zig.sh`: non-Docker Linux `amd64` Rust build helper using local `zig` and `cargo-zigbuild`
- `scripts/seed-grafana-sample-data.sh`: idempotent developer seed helper for sample orgs, datasources, folders, and dashboards in a running Grafana
- `scripts/test-rust-live-grafana.sh`: Docker-backed Grafana smoke test for the Rust CLIs

## Python Baseline

- Both Python entrypoints are kept parseable on Python 3.6 syntax for RHEL 8 compatibility.
- Avoid Python 3.9+ built-in generics such as `list[str]`.
- Avoid Python 3.10 union syntax such as `str | None`.
- Prefer `typing.List`, `typing.Dict`, `typing.Optional`, `typing.Set`, and `typing.Tuple`.

## Dashboard Utility

### CLI shape

- Mode selection is explicit.
- Installed Python console script is `grafana-utils`.
- Rust still keeps `grafana-access-utils` as a compatibility binary.
- Alert workflows no longer ship a separate `grafana-alert-utils` entrypoint; use `grafana-utils alert ...`.
- `grafana-utils` is now the primary entrypoint for dashboard, datasource, alert, and access workflows.
- Use `python3 python/grafana-utils.py dashboard list ...` to inspect live dashboard summaries.
- Use `python3 python/grafana-utils.py dashboard list-data-sources ...` to inspect live Grafana data sources.
- Use `python3 python/grafana-utils.py datasource list ...` for the first-class datasource inventory CLI.
- Use `python3 python/grafana-utils.py dashboard inspect-live ...` to inspect live Grafana dashboards through the same summary/report renderers used for raw exports.
- Use `python3 python/grafana-utils.py dashboard export ...` for export.
- Use `python3 python/grafana-utils.py dashboard import ...` for import.
- Use `python3 python/grafana-utils.py dashboard diff ...` for live-vs-local comparison.
- Use `python3 python/grafana-utils.py access ...` or `cargo run --bin grafana-utils -- access ...` for Grafana access-management workflows.
- `grafana-utils access user list ...` inspects Grafana users.
- `grafana-utils access user add ...` creates Grafana users through the server-admin API.
- `grafana-utils access user modify ...` updates Grafana users through the global and admin user APIs.
- `grafana-utils access user delete ...` removes Grafana users from the org or globally with explicit confirmation.
- `grafana-utils access team list ...` inspects Grafana teams.
- `grafana-utils access team add ...` creates an org-scoped Grafana team with optional initial members and admins.
- `grafana-utils access team modify ...` changes Grafana team membership and admin assignments.
- `grafana-utils access service-account ...` handles org-scoped service-account operations.
- The export subcommand intentionally uses `--export-dir` instead of `--output-dir` to avoid mixing export terminology with import behavior.
- `export-dashboard --org-id <ID>` rebuilds the dashboard client with `X-Grafana-Org-Id` and is Basic-auth-only because org switching is a server-admin-style workflow rather than a token-bound current-org workflow.
- `export-dashboard --all-orgs` lists `/api/orgs`, rebuilds one scoped export client per org, and exports each org into an `org_<id>_<name>/` subtree to avoid cross-org file collisions on disk.
- Multi-org export still writes aggregate root-level `raw/index.json` and `prompt/index.json` files under the chosen export root so the top-level manifest points at one coherent variant index.
- Top-level dashboard help and `export-dashboard -h` now include both a local Basic-auth example and a token example so operators can see both auth styles directly from the CLI.
- The `list-dashboard` subcommand is read-only and now defaults to table output with `UID`, `NAME`, `FOLDER`, `FOLDER_UID`, `FOLDER_PATH`, `ORG`, and `ORG_ID` columns.
- `list-dashboard --no-header` keeps the table rows but suppresses the header line for shell pipelines or snapshot-style output.
- `list-dashboard --csv` emits header `uid,name,folder,folderUid,path,org,orgId` with CSV escaping.
- `list-dashboard --json` emits an array of objects with keys `uid`, `name`, `folder`, `folderUid`, `path`, `org`, `orgId`, `sources`, and `sourceUids`.
- `list-dashboard` fetches the current org from `GET /api/org` once and attaches that `org` and `orgId` metadata to every listed dashboard summary.
- `list-dashboard --org-id <ID>` rebuilds the client with `X-Grafana-Org-Id` and is Basic-auth-only because the CLI needs a server-admin-style org switch rather than a token-bound current org context.
- `list-dashboard --all-orgs` lists `/api/orgs`, rebuilds one scoped client per org, and aggregates the combined dashboard list output. This is also Basic-auth-only.
- `list-dashboard --json` now fetches each dashboard payload plus the datasource catalog by default so machine-readable output includes `sources` and `sourceUids` without an extra flag.
- `list-dashboard --with-sources` still controls datasource expansion for table and CSV output, because those compact human-readable formats would otherwise become wider and slower by default.
- `list-dashboard --with-sources --csv` appends both `sources` and `sourceUids` so spreadsheet or script consumers can correlate dashboards back to concrete datasource UIDs when Grafana exposed them.
- `export-dashboard` and `import-dashboard` stay quiet by default except for summary output and explicit warnings/errors.
- `export-dashboard --progress` and `import-dashboard --progress` turn on concise per-dashboard `current/total` progress lines.
- `export-dashboard -v` and `import-dashboard -v` turn on detailed per-item output and intentionally suppress the concise `--progress` form when both flags are present.
- Folder tree path is resolved from `GET /api/folders/{uid}` using the folder `parents[]` chain when `folderUid` is present.
- `list-data-sources` is read-only and now defaults to a table with `UID`, `NAME`, `TYPE`, `URL`, and `IS_DEFAULT`.
- `list-data-sources --no-header` suppresses the table header line while keeping the same column layout.
- `list-data-sources --csv` emits header `uid,name,type,url,isDefault`.
- `list-data-sources --json` emits an array of objects with keys `uid`, `name`, `type`, `url`, and `isDefault`.
- `datasource list` mirrors the same human/CSV/JSON output contract as `dashboard list-data-sources` so the standalone datasource surface stays familiar.
- `datasource export` writes one normalized current-org datasource inventory rooted at `datasources.json`, `index.json`, and `export-metadata.json`, and each exported record carries `uid`, `name`, `type`, `access`, `url`, `isDefault`, `org`, and `orgId`.
- The Rust alert implementation is intentionally split by responsibility: `alert_cli_defs.rs` owns clap/auth normalization, `alert_client.rs` owns the Grafana alert provisioning client plus shared response parsing helpers, `alert_list.rs` owns list rendering and list-command dispatch, and `alert.rs` keeps the remaining import/export/diff orchestration plus shared alert document helpers.
- The Python dashboard implementation is intentionally split by responsibility: `dashboard_cli.py` stays as the stable CLI facade and shared helper host, `grafana_utils/dashboards/listing.py` owns live dashboard/datasource listing plus datasource/source-enrichment helpers, `grafana_utils/dashboards/export_inventory.py` owns raw-export discovery plus inventory/manifest validation helpers, `grafana_utils/dashboards/inspection_summary.py` owns the inspection summary document plus summary/table renderers, `grafana_utils/dashboards/inspection_report.py` owns the explicit per-query report model plus flat/grouped renderers, and `grafana_utils/dashboards/export_workflow.py`, `grafana_utils/dashboards/inspection_workflow.py`, and `grafana_utils/dashboards/import_workflow.py` own the high-level orchestration bodies for export, inspect-live/inspect-export, and import respectively.
- The Rust dashboard implementation follows the same boundary at a crate-module level: `dashboard.rs` stays as the public facade, `dashboard_files.rs` owns raw-export discovery plus inventory/manifest validation helpers, `dashboard_inspect_report.rs` owns the query-report contract and grouped renderers, and the import/inspect orchestration stays in the dedicated dashboard submodules.
- The Rust dashboard implementation is intentionally split by responsibility: `dashboard_cli_defs.rs` owns clap/auth/client setup, `dashboard_list.rs` owns list/datasource renderers and org-aware list orchestration, `dashboard_export.rs` owns export pathing and multi-org export orchestration, `dashboard_prompt.rs` owns datasource resolution plus prompt-export template rewrites, and `dashboard.rs` keeps the remaining shared helpers, import, diff, and top-level orchestration flows.
- The Rust access implementation is intentionally split by responsibility: `access_cli_defs.rs` owns clap/auth/client setup, `access_render.rs` owns output formatting and row normalization, `access_user.rs` owns user flows, `access_team.rs` owns team flows, `access_service_account.rs` owns service-account flows, and `access.rs` keeps shared request wrappers plus top-level dispatch.

### Packaging layout

- The installable package lives under `grafana_utils/`.
- `python/` keeps only thin wrappers so the repo can still be used without installation.
- `pyproject.toml` exposes `grafana-utils` as the Python console script.
- Base installation depends on `requests`.
- Optional extra `.[http2]` adds `httpx[http2]` for Python 3.8+ environments.

### Quality gates

- `make quality` is the baseline local gate and currently runs Python unit tests, Rust unit tests, `cargo fmt --check`, and `cargo clippy --all-targets -- -D warnings`.
- `.github/workflows/ci.yml` mirrors that baseline split into a Python test job and a Rust quality job so CLI and module changes hit the same minimum checks before merge.
- The repo does not yet enforce a dedicated Python formatter or type checker; the current baseline is intentionally limited to checks that already pass reliably in the existing toolchain.

### Rust cross-build notes

- `make build-rust-macos-arm64` runs `scripts/build-rust-macos-arm64.sh`.
- That script is the explicit native release path for Apple Silicon Macs and copies binaries into `dist/macos-arm64/`.
- `make build-rust-linux-amd64` runs `scripts/build-rust-linux-amd64.sh`.
- The script uses Docker plus the official Rust image to build `x86_64-unknown-linux-gnu` binaries from macOS.
- `make build-rust-linux-amd64-zig` runs `scripts/build-rust-linux-amd64-zig.sh`.
- The zig path expects local `zig`, `cargo-zigbuild`, and a rustup-managed `x86_64-unknown-linux-gnu` target.
- Output is copied into `dist/linux-amd64/` as `grafana-utils`.
- This is the preferred Linux `amd64` build path on macOS because it avoids managing a local Linux cross-linker toolchain.

### Export variants

Dashboard export writes two variants by default:

- `raw/`: API-safe dashboard JSON intended for later `import`
- `prompt/`: Grafana web-import JSON with datasource `__inputs`

Current export suppression flags:

- `--without-dashboard-raw`
- `--without-dashboard-prompt`

The two variants serve different consumers and should not be treated as interchangeable.

Dashboard export also writes versioned `export-metadata.json` files at:

- the combined export root
- `raw/`
- `prompt/`

Those manifests use `schemaVersion` and `variant` markers so `import` and `diff` can reject directories that are not the expected raw export layout.

The Python and Rust dashboard CLIs also have `inspect-export` for offline raw-export analysis. The summary path reads the raw `export-metadata.json`, `index.json`, `folders.json`, `datasources.json`, and dashboard files, then summarizes dashboard count, folder paths, panel/query totals, datasource usage, datasource inventory, orphaned datasources, and mixed-datasource dashboards. `inspect-export --json` emits the same summary as one machine-readable document, while `inspect-export --table` renders the summary as separate summary, folder-path, datasource-usage, datasource-inventory, orphaned-datasource, and mixed-dashboard tables.

The Python CLI also has `inspect-live`, which accepts the normal live dashboard auth/common args, materializes a temporary raw-export-like directory from live dashboard payloads plus current folder and datasource inventories, and then reuses the same summary/report inspection pipeline as `inspect-export`. This keeps the operator-facing output contract aligned while avoiding a second inspection implementation.

`inspect-export` and `inspect-live` also expose `--help-full` on both the Python and Rust CLIs. Normal `-h/--help` stays concise, while `--help-full` prints that same subcommand help followed by a short examples block focused on report modes such as `table`, `tree`, `tree-table`, plus datasource/panel filters and `--report-columns`.

`inspect-export --report` takes the same raw export input but emits one per-query record instead of the higher-level summary. Each record carries dashboard uid/title, folder path, panel id/title/type, target `refId`, resolved datasource label, a best-effort `datasourceUid`, the query field chosen from the target payload (`expr`, `query`, `rawSql`, and similar), the raw query text, and heuristic extraction fields such as `metrics`, `measurements`, and `buckets`. `--report` defaults to flat table output, `--report json` emits the same flat record model as JSON for downstream analysis, `--report tree` renders the same filtered record set as a dashboard -> panel -> query tree for operator reading, and `--report tree-table` renders dashboard-first sections with one compact query table per dashboard. Flux and SQL-family extraction remain heuristic and conservative: Flux currently uses `metrics` for pipeline/source function names plus `measurements`/`buckets` for `_measurement` and `bucket` references, while SQL-family queries currently use `measurements` for table/source references and `metrics` for coarse query-shape hints because the shared report contract does not yet expose dedicated table or shape fields.

`--report-columns` affects flat `table`/`csv` output and the grouped `tree-table` output, and uses stable column ids such as `dashboard_uid`, `panel_title`, `datasource`, `metrics`, and `query`. Optional columns such as `datasource_uid` stay out of the default table/CSV layout so the common report shape remains stable, but callers can opt them in explicitly. `--report-filter-datasource` applies before flat or grouped rendering and keeps only rows whose datasource label exactly matches the requested value. `--report-filter-panel-id` applies at the same stage and keeps only rows whose `panelId` exactly matches the requested value, which is useful when one dashboard expands into many panel/query rows.

### Raw export intent

- Keep dashboard JSON close to Grafana's API payload.
- Preserve `uid`.
- Clear numeric `id`.
- Keep datasource references unchanged.
- Best input for `python3 python/grafana-utils.py import-dashboard`.

### Prompt export intent

- Transform datasource references into Grafana web-import placeholders.
- Populate `__inputs`, `__requires`, and `__elements` in the shape Grafana expects.
- Intended for Grafana UI import, not for API re-import.

### Prompt export datasource pipeline

The prompt export rewrite flow is intentionally multi-stage:

1. Fetch datasource catalog from Grafana.
2. Index datasources by both `uid` and `name`.
3. Walk the dashboard tree and collect every `datasource` reference.
4. Normalize each datasource reference into a stable key.
5. Build one generated input mapping per unique datasource reference.
6. Rewrite matching dashboard refs to `${DS_*}` placeholders.
7. If every datasource resolves to the same plugin type, add Grafana's shared `$datasource` variable and collapse panel-level refs to it.

This is why prompt export needs live datasource metadata while raw export does not.

### Dashboard import constraints

- Import expects raw dashboard JSON, not prompt JSON.
- Files containing `__inputs` should be imported through Grafana web UI.
- Import can override folder destination with `--import-folder-uid`.
- Raw export writes `raw/folders.json` plus `raw/export-metadata.json::foldersFile` so later imports can reconstruct folder `uid`, `title`, `parentUid`, `path`, `org`, and `orgId` inventory.
- Raw export also writes `raw/datasources.json` plus `raw/export-metadata.json::datasourcesFile` so offline inspection can reconcile datasource `uid`, `name`, `type`, `access`, `url`, `isDefault`, `org`, and `orgId` inventory against dashboard usage.
- Import `--ensure-folders` reads that raw folder inventory, creates missing parent folders through Grafana's folder API, and rejects the run when the inventory manifest is missing.
- Import `--dry-run --ensure-folders` inspects the destination folder inventory first and reports missing versus mismatched exported folders so operators can catch path or parent drift before running a real import.
- Import can set the dashboard version-history message with `--import-message`.
- Import `--dry-run` predicts `would-create`, `would-update`, or `would-fail-existing` by checking the live Grafana UID first.
- Import `--dry-run --table` renders those predictions as `UID`, `DESTINATION`, `ACTION`, `FOLDER_PATH`, and `FILE`, and `--no-header` can suppress the header row only in that mode.
- Import `--dry-run --json` renders one JSON document with `mode`, `folders`, `dashboards`, and `summary`, and suppresses the normal human-readable progress/summary lines so scripts can parse it safely.
- Import `--update-existing-only` switches the workflow to `update-or-skip-missing` by dashboard `uid`, implies overwrite-on-existing behavior, and never creates missing dashboards.
- When import updates an existing dashboard by `uid`, it preserves the destination Grafana folder by default; only an explicit `--import-folder-uid` overrides that folder placement.
- `inspect-export` is a local raw-export analysis workflow; it does not call Grafana APIs and instead reads `raw/export-metadata.json`, `raw/folders.json`, `raw/datasources.json`, and dashboard JSON files to summarize folder paths, panels, queries, datasource references, datasource inventory, orphaned datasources, and mixed-datasource dashboards.
- `inspect-live` is the live-data adapter for the same inspection workflow; it calls the live dashboard, folder, and datasource APIs, writes a temporary raw-style layout, and then hands off to the existing inspection renderers.
- `inspect-export --report` walks the same local dashboard JSON but emits one per-target query record so operators can inspect datasource usage plus query text and extracted metric-like names without contacting Grafana.
- Report extraction should stay decomposed by datasource/query family over time. Shared traversal and row rendering can remain generic, but Prometheus, Loki, Flux/Influx, SQL, and future datasource-specific parsing should be pluggable so one datasource's parser growth does not complicate the others.
- `inspect-export --table --no-header` suppresses the header row for each rendered section table when operators need compact terminal output.
- Import now prints an `Import mode: ...` line before processing files so operators can confirm the active create/update/skip strategy immediately.
- `diff` compares normalized local raw payloads against live Grafana dashboard wrappers and prints a unified diff when they differ.

## Alerting Utility

### Supported resource kinds

`grafana-utils alert` currently supports:

- alert rules
- contact points
- mute timings
- notification policies
- notification message templates
- preferred command forms:
  - `grafana-utils alert export ...`
  - `grafana-utils alert import ...`
  - `grafana-utils alert diff ...`
  - `grafana-utils alert list-rules ...`
  - `grafana-utils alert list-contact-points ...`
  - `grafana-utils alert list-mute-timings ...`
  - `grafana-utils alert list-templates ...`
- legacy direct aliases also exist:
  - `grafana-utils export-alert ...`
  - `grafana-utils import-alert ...`
  - `grafana-utils diff-alert ...`
  - `grafana-utils list-alert-rules ...`
  - `grafana-utils list-alert-contact-points ...`
  - `grafana-utils list-alert-mute-timings ...`
  - `grafana-utils list-alert-templates ...`

The alerting export root is `alerts/raw/`, with one subdirectory per resource kind.

Default layout:

- `alerts/raw/rules/<folderUID>/<ruleGroup>/<title>__<uid>.json`
- `alerts/raw/contact-points/<name>/<name>__<uid>.json`
- `alerts/raw/mute-timings/<name>/<name>.json`
- `alerts/raw/policies/notification-policies.json`
- `alerts/raw/templates/<name>/<name>.json`

Alerting export documents and the root `index.json` carry both:

- `apiVersion`: the older tool document version marker kept for compatibility
- `schemaVersion`: the current export schema marker used by newer import and diff flows

### Import behavior by resource kind

- rules: create by default, update by `uid` when `--replace-existing` is set
- contact points: create by default, update by `uid` when `--replace-existing` is set
- mute timings: create by default, update by `name` when `--replace-existing` is set
- notification policies: always applied as one policy tree with `PUT`
- notification templates: applied with `PUT`; when `--replace-existing` is set, fetch the current template version first and send it back with the update payload
- import `--dry-run` predicts `would-create`, `would-update`, or `would-fail-existing` without mutating Grafana
- `--diff-dir` compares normalized import payloads with live provisioning resources and prints a unified diff when they differ

Template handling notes:

- Grafana template identity is the template `name`
- template list may return JSON `null`; treat that as an empty list
- template updates should strip `name` from the request body because the API path already carries the name
- without `--replace-existing`, importing an existing template should fail fast instead of silently updating it

### Alerting import shape and rejection rules

- Import accepts the tool-owned document format emitted by `grafana-utils alert export`
- Import accepts both current tool documents with `schemaVersion` and older tool documents that only carry `apiVersion`
- `detect_document_kind(...)` also accepts plain resource-shaped JSON for rules/contact points/mute timings/policies/templates
- Grafana official provisioning `/export` payloads are intentionally rejected for API import
- Round-trip import is only guaranteed for the tool-owned export format emitted by `grafana-utils alert export`
- Reject the combined `alerts/` export root on import; require callers to point at `alerts/raw/`

### Dashboard-linked alert rules

Alert rules may contain `__dashboardUid__` and `__panelId__` in annotations.

Export behavior:

- preserve the original linkage fields
- export extra linked-dashboard metadata used for import-time repair
- when the source dashboard still exists during export, enrich metadata with:
  - `dashboardTitle`
  - `folderTitle`
  - `folderUid`
  - `dashboardSlug`
  - `panelTitle`
  - `panelType`

Import behavior:

1. try the original `__dashboardUid__`
2. if `--dashboard-uid-map` is present, apply that mapping first
3. if `--panel-id-map` is present, rewrite `__panelId__` using the mapped source dashboard UID plus source panel ID
4. if the target Grafana has the mapped or original dashboard UID, stop there
5. otherwise fall back to exported dashboard metadata
6. search target dashboards by exported title, then narrow by folder title and slug
7. rewrite `__dashboardUid__` only when that fallback search resolves to exactly one dashboard

Current limitation:

- automatic fallback only rewrites `__dashboardUid__`
- `__panelId__` is preserved unless `--panel-id-map` is supplied
- panel matching is intentionally explicit; there is no heuristic panel-title-based rewrite

### Mapping file formats

Dashboard UID map:

```json
{
  "old-dashboard-uid": "new-dashboard-uid"
}
```

Panel ID map:

```json
{
  "old-dashboard-uid": {
    "7": "19"
  }
}
```

Notes:

- both mapping loaders coerce keys and values to strings
- panel maps are keyed by source dashboard UID, then source panel ID
- explicit maps take precedence over fallback dashboard metadata matching

### Live validation notes

- Primary automated coverage lives in `tests/test_python_alert_cli.py`
- Container-based validation was done against Grafana `12.4.1`
- Verified round-trip coverage includes:
  - rules
  - contact points
  - mute timings
  - notification policies
  - notification templates
  - dashboard-linked rules with repaired `__dashboardUid__`

## Grafana API Endpoints Used

This section lists the Grafana HTTP API paths used by this project. It is intended as a maintainer map of what each endpoint means to Grafana and how the Python and Rust implementations use it.

### Dashboard and shared lookup APIs

| Method | Endpoint | Grafana meaning | Project usage |
| --- | --- | --- | --- |
| `GET` | `/api/search` | Search Grafana objects. In this project it is always called with `type=dash-db` plus pagination params. | List dashboards for export and search dashboards by title when repairing linked alert-rule dashboard references. |
| `GET` | `/api/dashboards/uid/{uid}` | Fetch one dashboard plus Grafana `meta` fields by dashboard UID. | Export a dashboard by UID, and inspect dashboard metadata during alert-rule linked-dashboard repair. |
| `POST` | `/api/dashboards/db` | Create or update a dashboard from the standard dashboard import payload. Grafana expects a wrapped payload such as `{dashboard, folderUid, overwrite, message}`. | Import dashboards from the tool's raw dashboard files. |
| `GET` | `/api/folders/{uid}` | Fetch one Grafana folder plus its parent chain metadata. | Resolve folder tree paths during export and detect whether a folder UID already exists before `--ensure-folders` import runs. |
| `POST` | `/api/folders` | Create one Grafana folder, optionally nested under `parentUid`. | Recreate missing folder chains from `raw/folders.json` when operators opt into `--ensure-folders`. |
| `GET` | `/api/datasources` | List datasource definitions known to Grafana. | Build the datasource catalog used by dashboard prompt export so datasource references can be rewritten into Grafana import placeholders. |

Notes:

- Normal dashboard placement still flows through `folderUid` inside the dashboard import payload. The dedicated folder API is only used when `--ensure-folders` explicitly asks the tool to recreate missing destination folders first.
- The alerting utility reuses `/api/search` and `/api/dashboards/uid/{uid}` only for linked-dashboard metadata lookup and repair, not for dashboard export/import.

### Alerting provisioning APIs

| Method | Endpoint | Grafana meaning | Project usage |
| --- | --- | --- | --- |
| `GET` | `/api/v1/provisioning/alert-rules` | List all provisioned alert rules. | Export alert rules. |
| `GET` | `/api/v1/provisioning/alert-rules/{uid}` | Fetch one alert rule by UID. | Check whether a rule already exists before update/replace flows. |
| `POST` | `/api/v1/provisioning/alert-rules` | Create a new alert rule from a provisioning-style rule payload. | Import a rule when not replacing an existing one. |
| `PUT` | `/api/v1/provisioning/alert-rules/{uid}` | Replace an existing alert rule by UID. | Import a rule when `--replace-existing` is set. |
| `GET` | `/api/v1/provisioning/contact-points` | List provisioned contact points. | Export contact points and detect existing identities before updates. |
| `POST` | `/api/v1/provisioning/contact-points` | Create a new contact point. | Import a contact point when not replacing an existing one. |
| `PUT` | `/api/v1/provisioning/contact-points/{uid}` | Replace an existing contact point by UID. | Import a contact point when `--replace-existing` is set. |
| `GET` | `/api/v1/provisioning/mute-timings` | List provisioned mute timings. | Export mute timings and detect existing identities before updates. |
| `POST` | `/api/v1/provisioning/mute-timings` | Create a new mute timing. | Import a mute timing when not replacing an existing one. |
| `PUT` | `/api/v1/provisioning/mute-timings/{name}` | Replace an existing mute timing by name. | Import a mute timing when `--replace-existing` is set. |
| `GET` | `/api/v1/provisioning/policies` | Fetch the notification policy tree. Grafana models policies as one tree, not as many independent objects. | Export the policy tree. |
| `PUT` | `/api/v1/provisioning/policies` | Replace the notification policy tree. | Import the policy tree. The tool always uses `PUT` because this resource is tree-shaped. |
| `GET` | `/api/v1/provisioning/templates` | List notification templates. Grafana may return JSON `null` when none exist. | Export templates and detect existing template names. |
| `GET` | `/api/v1/provisioning/templates/{name}` | Fetch one notification template by name. | Read the current template version before a replace/update. |
| `PUT` | `/api/v1/provisioning/templates/{name}` | Replace a notification template by name. | Import or update a template. The request body intentionally omits `name` because the API path already carries the identity. |

Alerting import format notes:

- The tool accepts its own tool-owned export documents, not Grafana's official provisioning `/export` documents.
- The create/update payload shapes for these APIs are not the same as Grafana's `/export` response shape, which is why the project normalizes resources into its own round-trip format first.

## Access Utility

### Current scope

Primary access entrypoints are `python/grafana-utils.py access ...` and `cargo run --bin grafana-utils -- access ...`.

Rust still keeps a compatibility shim via `cargo run --bin grafana-access-utils -- ...` for the same command surface:

- `user list`
- `user add`
- `user modify`
- `user delete`
- `team list`
- `team modify`
- `team add`
- `service-account list`
- `service-account add`
- `service-account token add`

Not implemented yet:

- `team delete`
- any `group` alias commands

Current team creation command shape:

```bash
python3 python/grafana-utils.py access team add \
  --url http://localhost:3000 \
  --token "$GRAFANA_API_TOKEN" \
  --name platform-operators \
  --email platform-operators@example.com \
  --member alice@example.com \
  --admin bob@example.com
```

### Auth constraints

- `user list --scope org` may use token auth or Basic auth
- `user list --scope global` requires Basic auth and should be treated as a Grafana server-admin workflow
- `user add` requires Basic auth and should be treated as a Grafana server-admin workflow
- `user modify` requires Basic auth and should be treated as a Grafana server-admin workflow
- `user delete --scope global` requires Basic auth and should be treated as a Grafana server-admin workflow
- `user delete --scope org` may use token auth or Basic auth
- `team list` is org-scoped and may use token auth or Basic auth
- `team modify` is org-scoped and may use token auth or Basic auth
- `team add` is org-scoped and may use token auth or Basic auth
- service-account commands are org-scoped and may use token auth or Basic auth
- do not silently fall back from a token-only global request into a weaker behavior; fail early with a clear error instead

### Expected output modes

- compact text by default
- `--table`
- `--csv`
- `--json`

## Validation

Common checks:

```bash
make help
make build-python
make build-rust
make test
make test-rust-live
make test-access-live
python3 -m pip install --no-deps --target /tmp/grafana-utils-install .
python3 -m unittest tests.test_python_dashboard_cli
python3 -m unittest tests.test_python_alert_cli
python3 -m unittest tests.test_python_access_cli
python3 -m unittest tests.test_python_packaging
python3 -m unittest -v
```

Rust live smoke test notes:

- `make test-rust-live` runs `scripts/test-rust-live-grafana.sh`
- the script defaults to `grafana/grafana:12.4.1` and binds Grafana to a random localhost port unless `GRAFANA_PORT` is set explicitly
- the script seeds one Prometheus datasource, one dashboard, and one webhook contact point
- dashboard coverage: export, prompt export datasource rewrite, diff same, diff drifted, dry-run export, dry-run import, delete-and-import restore
- alerting coverage: export, diff same, diff changed, dry-run import, update import
- useful overrides: `GRAFANA_IMAGE`, `GRAFANA_PORT`, `GRAFANA_USER`, `GRAFANA_PASSWORD`, `CARGO_BIN`

Python access live smoke test notes:

- `make test-access-live` runs `scripts/test-python-access-live-grafana.sh`
- the script defaults to `grafana/grafana:12.4.1` and binds Grafana to a random localhost port unless `GRAFANA_PORT` is set explicitly
- user coverage: add, modify, global delete, org delete, global list, org list
- team coverage: add, list, modify
- service-account coverage: add, token add, list
- useful overrides: `GRAFANA_IMAGE`, `GRAFANA_PORT`, `GRAFANA_USER`, `GRAFANA_PASSWORD`, `PYTHON_BIN`

Developer sample-data seed notes:

- `make seed-grafana-sample-data` runs `scripts/seed-grafana-sample-data.sh`
- `make destroy-grafana-sample-data` runs `scripts/seed-grafana-sample-data.sh --destroy`
- `make reset-grafana-all-data` runs `scripts/seed-grafana-sample-data.sh --reset-all-data --yes`
- defaults to `http://localhost:3000` with `admin/admin`
- the script is idempotent and reuses existing orgs, datasources, and folders when possible
- destroy mode removes only the known sample resources; it does not wipe arbitrary Grafana content
- reset-all-data mode is intentionally destructive and is only for disposable local Grafana instances used during developer testing
- current seeded layout covers:
  - org `1` with `Smoke Prometheus`, `Smoke Loki`, `Platform`, `Platform / Infra`, and dashboards `smoke-main`, `smoke-prom-only`, `query-smoke`, `subfolder-main`
  - org `2` `Org Two` with dashboard `org-two-main`
  - org `3` `QA Org` with dashboard `qa-overview`
  - org `4` `Audit Org` with dashboard `audit-home`
- useful overrides: `GRAFANA_URL`, `GRAFANA_USER`, `GRAFANA_PASSWORD`

Useful CLI help checks:

```bash
grafana-utils -h
grafana-utils dashboard -h
grafana-utils dashboard list -h
grafana-utils dashboard list-data-sources -h
grafana-utils dashboard export -h
grafana-utils dashboard import -h
grafana-utils dashboard diff -h
grafana-utils alert -h
grafana-utils access -h
grafana-utils access user list -h
grafana-utils access user add -h
grafana-utils access user modify -h
grafana-utils access user delete -h
grafana-utils access team list -h
grafana-utils access team add -h
grafana-utils access team modify -h
grafana-utils access service-account list -h
grafana-utils access service-account add -h
grafana-utils access service-account token add -h
grafana-utils alert -h
cargo run --quiet --manifest-path rust/Cargo.toml --bin grafana-utils -- -h
cargo run --quiet --manifest-path rust/Cargo.toml --bin grafana-utils -- dashboard -h
cargo run --quiet --manifest-path rust/Cargo.toml --bin grafana-utils -- dashboard list -h
cargo run --quiet --manifest-path rust/Cargo.toml --bin grafana-utils -- dashboard list-data-sources -h
cargo run --quiet --manifest-path rust/Cargo.toml --bin grafana-utils -- dashboard export -h
cargo run --quiet --manifest-path rust/Cargo.toml --bin grafana-utils -- dashboard import -h
cargo run --quiet --manifest-path rust/Cargo.toml --bin grafana-utils -- dashboard diff -h
cargo run --quiet --manifest-path rust/Cargo.toml --bin grafana-utils -- alert -h
cargo run --quiet --manifest-path rust/Cargo.toml --bin grafana-utils -- access -h
cargo run --quiet --manifest-path rust/Cargo.toml --bin grafana-utils -- access user list -h
cargo run --quiet --manifest-path rust/Cargo.toml --bin grafana-utils -- access user add -h
cargo run --quiet --manifest-path rust/Cargo.toml --bin grafana-utils -- access user modify -h
cargo run --quiet --manifest-path rust/Cargo.toml --bin grafana-utils -- access user delete -h
cargo run --quiet --manifest-path rust/Cargo.toml --bin grafana-utils -- access team list -h
cargo run --quiet --manifest-path rust/Cargo.toml --bin grafana-utils -- access team add -h
cargo run --quiet --manifest-path rust/Cargo.toml --bin grafana-utils -- access team modify -h
cargo run --quiet --manifest-path rust/Cargo.toml --bin grafana-utils -- access service-account list -h
cargo run --quiet --manifest-path rust/Cargo.toml --bin grafana-utils -- access service-account add -h
cargo run --quiet --manifest-path rust/Cargo.toml --bin grafana-utils -- access service-account token add -h
python3 python/grafana-utils.py -h
python3 python/grafana-utils.py dashboard -h
python3 python/grafana-utils.py dashboard list -h
python3 python/grafana-utils.py dashboard list-data-sources -h
python3 python/grafana-utils.py dashboard export -h
python3 python/grafana-utils.py dashboard import -h
python3 python/grafana-utils.py dashboard diff -h
python3 python/grafana-utils.py alert -h
python3 python/grafana-utils.py access -h
python3 python/grafana-utils.py access user list -h
python3 python/grafana-utils.py access user add -h
python3 python/grafana-utils.py access user modify -h
python3 python/grafana-utils.py access user delete -h
python3 python/grafana-utils.py access team list -h
python3 python/grafana-utils.py access team add -h
python3 python/grafana-utils.py access team modify -h
python3 python/grafana-utils.py access service-account list -h
python3 python/grafana-utils.py access service-account add -h
python3 python/grafana-utils.py access service-account token add -h
python3 python/grafana-utils.py alert -h
python3 python/grafana-utils.py access -h
```

## Documentation split

- `README.md`: public usage and high-level behavior
- `DEVELOPER.md`: maintenance notes, internal architecture, compatibility rules, and implementation tradeoffs
- `docs/internal/ai-status.md` / `docs/internal/ai-changes.md`: internal working notes only; do not treat them as public GitHub-facing documentation

## Auth Notes

- Shared CLI auth now supports `--prompt-password` for Basic auth without echo.
- Reject `--prompt-password` when `--token` is also set.
- Reject `--prompt-password` when `--basic-password` is also set.
- Require `--basic-user` with `--prompt-password`.

## GitHub metadata updates

When updating GitHub repository description or topics for this project, use `gh api` against the REST endpoints instead of relying on `gh repo view` GraphQL lookups alone.

Known repositories:

- public: `kenduest/grafana-utils`
- private mirror: `kenduest-brobridge/grafana-utils`

Recommended sequence:

1. Check current auth:

```bash
gh auth status
```

2. Switch to the account that owns the target repo if needed:

```bash
gh auth switch -u kenduest
gh auth switch -u kenduest-brobridge
```

3. Update description with REST:

```bash
gh api repos/<owner>/grafana-utils -X PATCH \
  -f description='Python and Rust CLI tools for exporting, backing up, migrating, and re-importing Grafana dashboards and alerting resources.'
```

4. Update topics with REST:

```bash
gh api repos/<owner>/grafana-utils/topics -X PUT \
  -H 'Accept: application/vnd.github+json' \
  -f 'names[]=grafana' \
  -f 'names[]=dashboards' \
  -f 'names[]=alerting' \
  -f 'names[]=backup' \
  -f 'names[]=migration' \
  -f 'names[]=cli' \
  -f 'names[]=python' \
  -f 'names[]=rust'
```

Things to remember:

- `gh repo view <owner>/<repo>` may fail to resolve a private repo depending on the active account and GraphQL visibility, even when `gh api repos/<owner>/<repo>` works
- in `zsh`, quote each `names[]=...` argument or the shell will treat it as a glob and fail before the API call
- if one repo update returns `404`, check the active `gh` account before assuming the repo path is wrong

Documentation policy:

- keep `README.md` suitable for GitHub readers
- keep environment-specific validation logs, migration notes, and maintainer-only tradeoffs in `DEVELOPER.md`
- avoid relying on `docs/internal/ai-status.md` and `docs/internal/ai-changes.md` for public project documentation
- if user-facing release history is needed, prefer a curated `CHANGELOG.md`
