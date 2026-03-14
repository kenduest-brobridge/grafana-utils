# Grafana Utilities

Language / 語言: English | [繁體中文 README.zh-TW.md](README.zh-TW.md)

Export, back up, migrate, and re-import Grafana dashboards, datasource inventory, and alerting resources as JSON.

This repository provides one primary unified CLI in two implementations:

- `grafana-util`: unified dashboard, datasource, alerting, and access-management CLI
- packaged Python implementation under [`grafana_utils/`](grafana_utils/)
- Rust implementation under [`rust/`](rust/)

The repository is useful when you need to:

- back up dashboards or alerting resources from Grafana
- move Grafana content between environments
- keep Grafana JSON under version control
- prepare dashboard files either for API import or Grafana web UI import with datasource prompts

Compatibility:

- supported on RHEL 8 and later
- Python entrypoints now target Python 3.9+ syntax and runtime support

## Contents

- [Overview](#overview)
- [Choose Python or Rust](#choose-python-or-rust)
- [Quick Start](#quick-start)
- [Dashboard Utility](#dashboard-utility)
- [Datasource Utility](#datasource-utility)
- [Alerting Utility](#alerting-utility)
- [Access Utility](#access-utility)
- [Build and Install](#build-and-install)
- [Authentication and TLS](#authentication-and-tls)
- [Output Directory Layout](#output-directory-layout)
- [Validation](#validation)
- [Documentation](#documentation)

## Overview

The repo now uses one primary command name with explicit areas underneath it.

- `grafana-util dashboard export ...`
- `grafana-util dashboard list ...`
- `grafana-util datasource list ...`
- `grafana-util datasource export ...`
- `grafana-util datasource import ...`
- `grafana-util datasource diff ...`
- `grafana-util dashboard inspect-live ...`
- `grafana-util dashboard import ...`
- `grafana-util dashboard diff ...`
- `grafana-util alert export ...`
- `grafana-util alert import ...`
- `grafana-util alert diff ...`
- `grafana-util alert list-rules ...`
- `grafana-util alert list-contact-points ...`
- `grafana-util alert list-mute-timings ...`
- `grafana-util alert list-templates ...`
- `grafana-util access user list ...`
- `grafana-util access user add ...`
- `grafana-util access user modify ...`
- `grafana-util access user delete ...`
- `grafana-util access team list ...`
- `grafana-util access team add ...`
- `grafana-util access team modify ...`
- `grafana-util access team delete ...`
- `grafana-util access group ...`
- `grafana-util access service-account ...`

Maintainer architecture overviews:

- Python architecture: [`docs/overview-python.md`](docs/overview-python.md)
- Rust architecture: [`rust/overview-rust.md`](rust/overview-rust.md)

Compatibility notes:

- old dashboard direct forms such as `grafana-util export-dashboard ...` and `grafana-util list-dashboard ...` still work
- alert direct forms such as `grafana-util export-alert ...` and `grafana-util list-alert-rules ...` still work
- Rust still keeps `grafana-access-utils ...` as a compatibility binary, but Python now uses only `grafana-util access ...`

The most important distinction in this repo is dashboard export format:

- `dashboards/raw/` is for Grafana API re-import
- `dashboards/prompt/` is for Grafana web UI import with datasource mapping prompts

## Choose Python or Rust

Use the path that matches how you want to operate the repo.

| Option | When to use it | Commands |
| --- | --- | --- |
| Installed Python package | Best default for normal usage | `grafana-util dashboard ...`, `grafana-util datasource ...`, `grafana-util alert ...`, `grafana-util access ...` |
| Python from git checkout | Best when editing or testing the repo directly | `python3 python/grafana-util.py dashboard ...`, `python3 python/grafana-util.py datasource ...`, `python3 python/grafana-util.py alert ...`, `python3 python/grafana-util.py access ...` |
| Rust from git checkout | Best when validating or developing the Rust implementation | `cargo run --bin grafana-util -- dashboard ...`, `cargo run --bin grafana-util -- alert ...`, `cargo run --bin grafana-util -- access ...` |

Notes:

- the Python package is the normal install path from this repository
- the Rust binaries are built from [`rust/`](rust/) and are not installed by `python3 -m pip install .`
- both implementations use the same command names and the same operator concepts

## Quick Start

Dashboard export, writing both `raw/` and `prompt/` variants:

```bash
python3 python/grafana-util.py dashboard export \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --export-dir ./dashboards \
  --overwrite
```

Dashboard export from one explicit Grafana org:

```bash
python3 python/grafana-util.py dashboard export \
  --url http://localhost:3000 \
  --basic-user "$GRAFANA_USERNAME" \
  --basic-password "$GRAFANA_PASSWORD" \
  --org-id 2 \
  --export-dir ./dashboards \
  --overwrite
```

Dashboard export across every visible Grafana org:

```bash
python3 python/grafana-util.py dashboard export \
  --url http://localhost:3000 \
  --basic-user "$GRAFANA_USERNAME" \
  --basic-password "$GRAFANA_PASSWORD" \
  --all-orgs \
  --export-dir ./dashboards \
  --overwrite
```

Datasource inventory export from the current Grafana org:

```bash
python3 python/grafana-util.py datasource export \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --export-dir ./datasources \
  --overwrite
```

Datasource inventory dry-run import into one explicit Grafana org:

```bash
python3 python/grafana-util.py datasource import \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --org-id 7 \
  --import-dir ./datasources \
  --replace-existing \
  --require-matching-export-org \
  --dry-run \
  --table
```

Inspect one raw export directory and summarize its structure:

```bash
python3 python/grafana-util.py dashboard inspect-export \
  --import-dir ./dashboards/raw
```

Inspect the same raw export directory as JSON:

```bash
python3 python/grafana-util.py dashboard inspect-export \
  --import-dir ./dashboards/raw \
  --json
```

Inspect the same raw export directory as tables:

```bash
python3 python/grafana-util.py dashboard inspect-export \
  --import-dir ./dashboards/raw \
  --table
```

Inspect the same raw export directory as a full per-query report:

```bash
python3 python/grafana-util.py dashboard inspect-export \
  --import-dir ./dashboards/raw \
  --report
```

Inspect the same query report as CSV and explicitly include datasource UIDs:

```bash
python3 python/grafana-util.py dashboard inspect-export \
  --import-dir ./dashboards/raw \
  --report csv \
  --report-columns dashboard_uid,panel_id,datasource_uid,datasource,query
```

Inspect the same query report tree by dashboard and panel:

```bash
python3 python/grafana-util.py dashboard inspect-export \
  --import-dir ./dashboards/raw \
  --report tree \
  --report-filter-panel-id 7
```

Inspect the same query report as per-dashboard tables:

```bash
python3 python/grafana-util.py dashboard inspect-export \
  --import-dir ./dashboards/raw \
  --report tree-table \
  --report-columns panel_id,panel_title,datasource,query
```

Inspect live Grafana dashboards with the same report contract:

```bash
python3 python/grafana-util.py dashboard inspect-live \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --report json \
  --report-filter-panel-id 7
```

Dashboard list, including resolved datasource names per dashboard:

```bash
python3 python/grafana-util.py dashboard list \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --with-sources \
  --table
```

List dashboards from one explicit Grafana org:

```bash
python3 python/grafana-util.py dashboard list \
  --url http://localhost:3000 \
  --basic-user "$GRAFANA_USERNAME" \
  --basic-password "$GRAFANA_PASSWORD" \
  --org-id 2 \
  --json
```

List dashboards across every visible Grafana org:

```bash
python3 python/grafana-util.py dashboard list \
  --url http://localhost:3000 \
  --basic-user "$GRAFANA_USERNAME" \
  --basic-password "$GRAFANA_PASSWORD" \
  --all-orgs \
  --json
```

List live dashboards without writing export files:

```bash
python3 python/grafana-util.py dashboard list \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin
```

List live dashboards as a table with folder tree path:

```bash
python3 python/grafana-util.py dashboard list \
  --table \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin
```

List live dashboards as CSV:

```bash
python3 python/grafana-util.py dashboard list \
  --csv \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin
```

List live dashboards as JSON:

```bash
python3 python/grafana-util.py dashboard list \
  --json \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin
```

List live Grafana data sources as a table:

```bash
python3 python/grafana-util.py dashboard list-data-sources \
  --table \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin
```

Dashboard API import from the raw export:

```bash
python3 python/grafana-util.py dashboard import \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --import-dir ./dashboards/raw \
  --replace-existing
```

Dashboard diff against the current Grafana state:

```bash
python3 python/grafana-util.py dashboard diff \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --import-dir ./dashboards/raw
```

Alerting export:

```bash
python3 python/grafana-util.py alert export \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --output-dir ./alerts \
  --overwrite
```

Alerting import:

```bash
python3 python/grafana-util.py alert import \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --import-dir ./alerts/raw \
  --replace-existing
```

Alerting diff against the current Grafana state:

```bash
python3 python/grafana-util.py alert diff \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --diff-dir ./alerts/raw
```

Access-management user listing, org scope with token auth:

```bash
python3 python/grafana-util.py access user list \
  --url http://127.0.0.1:3000 \
  --scope org \
  --token "$GRAFANA_API_TOKEN" \
  --table
```

Access-management user listing, global scope with Basic auth:

```bash
python3 python/grafana-util.py access user list \
  --url http://127.0.0.1:3000 \
  --scope global \
  --basic-user "$GRAFANA_USERNAME" \
  --basic-password "$GRAFANA_PASSWORD" \
  --json
```

Access-management user creation, global scope with Basic auth:

```bash
python3 python/grafana-util.py access user add \
  --url http://127.0.0.1:3000 \
  --basic-user "$GRAFANA_USERNAME" \
  --basic-password "$GRAFANA_PASSWORD" \
  --login automation-user \
  --email automation-user@example.com \
  --name "Automation User" \
  --password temporary-password \
  --json
```

Access-management user update, global scope with Basic auth:

```bash
python3 python/grafana-util.py access user modify \
  --url http://127.0.0.1:3000 \
  --basic-user "$GRAFANA_USERNAME" \
  --basic-password "$GRAFANA_PASSWORD" \
  --login automation-user \
  --set-email automation-user+ops@example.com \
  --set-name "Automation User Ops" \
  --set-org-role Editor \
  --set-grafana-admin true \
  --json
```

Access-management user delete, global scope with Basic auth:

```bash
python3 python/grafana-util.py access user delete \
  --url http://127.0.0.1:3000 \
  --basic-user "$GRAFANA_USERNAME" \
  --basic-password "$GRAFANA_PASSWORD" \
  --login automation-user \
  --scope global \
  --yes \
  --json
```

Access-management team listing, org scope with token auth:

```bash
python3 python/grafana-util.py access team list \
  --url http://127.0.0.1:3000 \
  --token "$GRAFANA_API_TOKEN" \
  --table
```

Access-management team membership change, org scope with token auth:

```bash
python3 python/grafana-util.py access team modify \
  --url http://127.0.0.1:3000 \
  --token "$GRAFANA_API_TOKEN" \
  --name Ops \
  --add-member alice@example.com \
  --add-admin bob@example.com \
  --json
```

Access-management team creation, org scope with token auth:

```bash
python3 python/grafana-util.py access team add \
  --url http://127.0.0.1:3000 \
  --token "$GRAFANA_API_TOKEN" \
  --name platform-operators \
  --email platform-operators@example.com \
  --member alice@example.com \
  --admin bob@example.com
```

Access-management team deletion, org scope with token auth:

```bash
python3 python/grafana-util.py access group delete \
  --url http://127.0.0.1:3000 \
  --token "$GRAFANA_API_TOKEN" \
  --name platform-operators \
  --yes \
  --json
```

Access-management service-account listing, org scope with token auth:

```bash
python3 python/grafana-util.py access service-account list \
  --url http://127.0.0.1:3000 \
  --token "$GRAFANA_API_TOKEN" \
  --table
```

Access-management service-account creation:

```bash
python3 python/grafana-util.py access service-account add \
  --url http://127.0.0.1:3000 \
  --token "$GRAFANA_API_TOKEN" \
  --name automation-bot \
  --role Editor \
  --json
```

Access-management service-account token creation:

```bash
python3 python/grafana-util.py access service-account token add \
  --url http://127.0.0.1:3000 \
  --token "$GRAFANA_API_TOKEN" \
  --name automation-bot \
  --token-name automation-bot-short-lived \
  --seconds-to-live 3600 \
  --json
```

Rust access-management user listing from the repo checkout:

```bash
cd rust && cargo run --quiet --bin grafana-util -- access user list \
  --url http://127.0.0.1:3000 \
  --scope org \
  --token "$GRAFANA_API_TOKEN" \
  --table
```

Rust access-management user creation from the repo checkout:

```bash
cd rust && cargo run --quiet --bin grafana-util -- access user add \
  --url http://127.0.0.1:3000 \
  --basic-user "$GRAFANA_USERNAME" \
  --basic-password "$GRAFANA_PASSWORD" \
  --login automation-user \
  --email automation-user@example.com \
  --name "Automation User" \
  --password temporary-password \
  --json
```

## Dashboard Utility

`grafana-util dashboard` has explicit subcommands:

- `list`
- `export`
- `import`
- `diff`

### Export Variants

One dashboard export run writes two variants by default:

- `dashboards/raw/`
- `dashboards/prompt/`

Use these flags when you want only one side:

- `--without-dashboard-raw`
- `--without-dashboard-prompt`

Use `raw/` when you want:

- the same dashboard `uid`
- minimal transformation
- API re-import through `grafana-util dashboard import`

Use `prompt/` when you want:

- Grafana web UI import
- datasource mapping prompts
- Grafana-style `__inputs`

### Important Export Options

| Option | Purpose |
| --- | --- |
| `--url` | Grafana base URL. Default: `http://localhost:3000` |
| `--export-dir` | Root export directory. Default: `dashboards/` |
| `--page-size` | Dashboard search page size. Default: `500` |
| `--org-id ORG_ID` | For `dashboard list`, `dashboard export`, or `dashboard import`, switch the whole command to one explicit Grafana org ID; requires Basic auth |
| `--all-orgs` | For `dashboard list` or `dashboard export`, enumerate visible Grafana orgs and aggregate list output or export each org; requires Basic auth |
| `--with-sources` | For `dashboard list` table or CSV output, fetch each dashboard payload and include datasource names used by that dashboard; JSON already includes datasource names and best-effort datasource UIDs by default |
| `--no-header` | For `dashboard list`, `dashboard list-data-sources`, `dashboard import --dry-run --table`, or `dashboard inspect-export --table`, omit the table header row |
| `--output-format <mode>` | New cross-command output selector for list and dry-run summary commands. Current commands still keep their old defaults, but you can now use one flag such as `table`, `csv`, `json`, or `text` where supported instead of remembering whether a command needs `--table`, `--csv`, or `--json` |
| `--progress` | For `dashboard export` or `dashboard import`, print concise per-dashboard `current/total` progress lines while the command runs |
| `-v, --verbose` | For `dashboard export` or `dashboard import`, print detailed per-item output including variants, paths, and import results; overrides `--progress` |
| `dashboard import --dry-run --table` | Render dry-run import predictions as a table showing `uid`, destination state, action, destination folder path, and file |
| `inspect-export --json` | Analyze a raw export directory and emit machine-readable structure summary including folder paths, panels, queries, datasource usage, datasource inventory, orphaned datasources, and mixed dashboards |
| `inspect-export --table` | Analyze a raw export directory and render multi-section tables for summary, folder paths, datasource usage, datasource inventory, orphaned datasources, and mixed dashboards |
| `inspect-export --output-format <mode>` | Alternative single-flag selector for inspect output using `text`, `table`, `json`, `report-table`, `report-csv`, `report-json`, `report-tree`, `report-tree-table`, `governance`, or `governance-json` |
| `inspect-export --report[=table|json|tree|tree-table]` | Emit one full per-query inspection report; default `table` output stays flat row-per-query, `tree` renders the same records as a dashboard -> panel -> query tree, and `tree-table` renders per-dashboard grouped tables |
| `inspect-live --json|--table|--report[=table|csv|json|tree|tree-table]` | Inspect live Grafana dashboards by materializing a temporary raw-style snapshot and then rendering the same summary/report outputs as `inspect-export` |
| `inspect-export --help-full` / `inspect-live --help-full` | Show the normal inspect help plus a short extended examples section for report modes, filters, and `--report-columns` |
| `inspect-export --report-columns ...` | With `--report` table, csv, or tree-table output, limit the query report to selected columns such as `dashboard_uid,panel_title,datasource,metrics,query` or add optional fields such as `datasource_uid` |
| `inspect-export --report-filter-datasource ...` | With `--report`, include only rows whose datasource label exactly matches the requested value |
| `inspect-export --report-filter-panel-id ...` | With `--report`, include only rows whose panel id exactly matches the requested value |
| `--update-existing-only` | For `dashboard import`, update only dashboards whose UID already exists in Grafana and skip missing dashboards instead of creating them |
| `--ensure-folders` | For `dashboard import`, read `raw/folders.json` and create any missing destination folder chain before importing dashboards |
| `--require-matching-export-org` | For `dashboard import`, verify that the raw export's recorded `orgId` matches the target Grafana org for this run before dry-run or live import |
| `dashboard list-data-sources --table|--csv|--json` | List live Grafana data sources in human-readable or machine-readable output |
| `--flat` | Do not create per-folder subdirectories |
| `--overwrite` | Replace existing exported files |
| `--without-dashboard-raw` | Skip the `raw/` export variant |
| `--without-dashboard-prompt` | Skip the `prompt/` export variant |
| `--dry-run` | Preview export output without writing files |
| `--verify-ssl` | Enable TLS certificate verification |

Dashboard auth scope rules:

- `--token` works inside the token's current Grafana org context
- `--token` may be used for current-org `dashboard list`, `dashboard export`, and `dashboard import`
- `--token` does not unlock explicit org switching in dashboard commands
- `dashboard list --org-id`, `dashboard export --org-id`, `dashboard export --all-orgs`, and `dashboard import --org-id` all require Basic auth

For dashboard listing:

- default `dashboard list` output is a table showing `uid`, `name`, `folder`, `folderUid`, resolved folder tree path, `org`, and `orgId`
- `dashboard list --no-header` omits the table header row
- `dashboard list --org-id <ID>` reads dashboards from that explicit org instead of the current auth context and requires Basic auth
- `dashboard list --all-orgs` aggregates dashboards across every visible org and requires Basic auth
- plain `dashboard list --token ...` stays in the token's current org context
- `dashboard list --json` includes datasource names and a best-effort `sourceUids` array by default
- `dashboard list --with-sources` remains useful for table or CSV output, where datasource expansion stays opt-in to keep the default list view compact
- `dashboard list --with-sources --csv` adds `sources` plus a `sourceUids` column with best-effort datasource UIDs
- `dashboard list --with-sources` is slower than plain `dashboard list` because it fetches each dashboard payload and the datasource catalog

For dashboard export:

- `dashboard export --org-id <ID>` exports dashboards from that explicit org instead of the current auth context and requires Basic auth
- `dashboard export --all-orgs` exports dashboards from every visible org and requires Basic auth
- `dashboard export --all-orgs` writes per-org trees such as `org_2_Org_Two/raw/...` and `org_2_Org_Two/prompt/...` to avoid cross-org file collisions
- plain `dashboard export --token ...` exports only from the token's current org context
- `dashboard export` stays quiet by default except for the final summary
- `dashboard export --progress` prints one concise progress line per exported dashboard, such as `Exporting dashboard 3/10: cpu-main`
- `dashboard export -v` prints detailed per-variant output such as `Exported raw    cpu-main -> dashboards/raw/Infra/CPU__cpu-main.json`
- `dashboard export -v --progress` uses verbose output and suppresses the concise progress form
- `dashboard import` stays quiet by default except for the final summary
- `dashboard import --progress` prints one concise progress line per imported dashboard, such as `Importing dashboard 2/7: cpu-main`
- `dashboard import -v` prints detailed per-file import results, including dry-run actions or returned status values
- `dashboard import -v --progress` uses verbose output and suppresses the concise progress form
- `dashboard import --dry-run --table` prints a final table with `uid`, `destination`, `action`, `folder_path`, and `file`
- `dashboard import --dry-run --table --output-columns uid,source_folder_path,destination_folder_path,reason,file` trims the dry-run table to the columns you care about, which is useful when reviewing folder-path mismatches
- `dashboard import --dry-run --table --no-header` omits the dry-run table header row
- `dashboard import --org-id <ID>` imports the whole run into that explicit destination org instead of the current auth context and requires Basic auth
- plain `dashboard import --token ...` imports into the token's current org context
- `dashboard import --require-matching-export-org` fails early when the raw export's recorded `orgId` does not match the resolved target org for this run
- `dashboard import --update-existing-only` updates only existing dashboard UIDs, skips missing dashboards, and implies `--replace-existing`
- `dashboard import` now prints an `Import mode: ...` line up front so you can see whether the run is `create-only`, `create-or-update`, or `update-or-skip-missing`
- `dashboard inspect-export` analyzes a raw export directory offline and summarizes dashboard count, folder paths, panels, queries, datasource usage, datasource inventory, orphaned datasources, and mixed-datasource dashboards
- `dashboard inspect-export --json` emits the same analysis as one JSON document for scripts or CI checks
- `dashboard inspect-export --table` renders the same analysis as multiple tables for summary, folder paths, datasource usage, datasource inventory, orphaned datasources, and mixed dashboards
- `dashboard inspect-export --output-format text|table|json|report-table|report-csv|report-json|report-tree|report-tree-table|governance|governance-json` is the newer single-flag alternative when you do not want to remember whether a mode currently lives under `--json`, `--table`, or `--report`
- `dashboard inspect-export --report` emits one row per query target with dashboard uid/title, folder path, panel id/title/type, datasource, query field, extracted metrics/measurements/buckets, and the raw query text
- `dashboard inspect-export --report json` emits the same per-query inspection model as one machine-readable JSON document, including `datasourceUid` when the raw export carries a concrete datasource uid
- `dashboard inspect-export --report tree` keeps the same underlying query records but renders them as a dashboard -> panel -> query tree when you want to read one dashboard at a time instead of scanning a wide flat table
- `dashboard inspect-export --report tree-table` keeps the same dashboard-first grouping but renders each dashboard section as a compact table, which is easier to scan when you still want columns
- `dashboard inspect-export --report-columns dashboard_uid,panel_title,datasource,metrics,query` trims the table report down to the columns you care about most
- `dashboard inspect-export --report-columns dashboard_uid,panel_id,datasource_uid,datasource,query` opts `datasource_uid` into table or csv output without widening the default report
- `dashboard inspect-export --report-filter-datasource <label>` narrows table or JSON report output to one datasource label, which is useful when checking migration leftovers or datasource retirement impact
- `dashboard inspect-export --report-filter-panel-id <id>` narrows table or JSON report output to one panel id when one dashboard contains many panels and you only want one panel's queries
- `--output-format` cannot be combined with `--json`, `--table`, or `--report`
- `dashboard inspect-live` reuses the same summary/report flags as `dashboard inspect-export`, but sources the dashboards, folders, and datasource inventory directly from Grafana instead of a pre-existing raw export directory
- `dashboard inspect-export --table --no-header` suppresses each section's header row when you need compact copy/paste output

For datasource listing:

- `dashboard list-data-sources` defaults to a table showing `uid`, `name`, `type`, `url`, and `isDefault`
- `dashboard list-data-sources --no-header` omits the table header row
- `dashboard list-data-sources --csv` emits `uid,name,type,url,isDefault`
- `dashboard list-data-sources --json` emits an array of datasource objects

## Datasource Utility

`grafana-util datasource` currently provides:

- `list`
- `export`
- `import`
- `diff`

For datasource inventory:

- `datasource list` defaults to a table showing `uid`, `name`, `type`, `url`, and `isDefault`
- `datasource list --output-format table|csv|json` is the new single-flag alternative to `--table`, `--csv`, or `--json`
- `datasource list --no-header` omits the table header row
- `datasource list --csv` emits `uid,name,type,url,isDefault`
- `datasource list --json` emits an array of datasource objects
- `datasource export` writes `datasources.json`, `index.json`, and `export-metadata.json` into the chosen export directory
- `datasource export` normalizes each record to `uid`, `name`, `type`, `access`, `url`, `isDefault`, `org`, and `orgId`
- `datasource export --dry-run` prints the target files without writing them
- `datasource export --overwrite` replaces existing export files in the target directory
- `datasource import` reads the normalized datasource export root back into Grafana
- `datasource import` rejects datasource records that carry extra fields outside the normalized contract, such as `id`, `jsonData`, `secureJsonData`, or passwords
- `datasource import --dry-run` predicts create/update/skip/block actions without calling Grafana write APIs
- `datasource import --org-id <ID>` switches the whole import run to one explicit destination org and requires Basic auth
- `datasource import --require-matching-export-org` fails when the export's recorded `orgId` does not match the resolved target org for the run
- `datasource import --replace-existing` updates existing destination datasources that match by `uid` and otherwise by exact `name`
- `datasource import --replace-existing` still blocks same-name updates when the live datasource `uid` differs from the exported `uid`, so one datasource identity cannot silently overwrite another by name alone
- `datasource import --update-existing-only` skips missing datasources and only updates matched destination datasources
- `datasource import --dry-run --output-format text|table|json` is the new single-flag alternative to the old dry-run output flags
- `datasource import --dry-run --table` renders predicted datasource import actions as a compact table and `--no-header` suppresses that header row
- `datasource import --dry-run --table --output-columns uid,action,org_id,file` trims the dry-run table to the selected review columns without changing the default table when the flag is omitted
- `datasource import --dry-run --json` renders one machine-readable JSON document with the active mode, per-datasource actions, and summary counts
- `datasource diff` compares one exported datasource inventory root against the current live Grafana datasource inventory
- `datasource diff --diff-dir <DIR>` reads `datasources.json`, `index.json`, and `export-metadata.json` from that export root, rejects extra datasource fields outside the normalized contract, prints per-datasource diff status, and exits with status `1` when differences are found

### Raw Export

Raw export preserves the Grafana dashboard identity as much as possible:

- preserves dashboard `uid`
- preserves dashboard `title`
- sets numeric dashboard `id` to `null`
- keeps datasource references unchanged

If you only want the prompt variant:

```bash
python3 python/grafana-util.py dashboard export \
  --export-dir ./dashboards \
  --without-dashboard-raw
```

### Prompt Export

Prompt export rewrites the dashboard into a shape Grafana web import understands:

- creates non-empty `__inputs`
- keeps `__elements`
- rewrites datasource references into import placeholders
- may normalize panel datasource refs to `{"uid":"$datasource"}` when the dashboard uses one datasource type

Important notes:

- mixed-datasource dashboards keep explicit `DS_...` placeholders
- untyped datasource variables that cannot be converted safely are preserved as-is
- prompt JSON is for Grafana web UI import, not API import

If you only want the raw variant:

```bash
python3 python/grafana-util.py dashboard export \
  --export-dir ./dashboards \
  --without-dashboard-prompt
```

### Dashboard Import

Dashboard import reads normal dashboard JSON through the Grafana API.

Example:

```bash
python3 python/grafana-util.py dashboard import \
  --url http://127.0.0.1:3000 \
  --import-dir ./dashboards/raw \
  --replace-existing
```

Import into one explicit Grafana org:

```bash
python3 python/grafana-util.py dashboard import \
  --url http://127.0.0.1:3000 \
  --basic-user "$GRAFANA_USERNAME" \
  --basic-password "$GRAFANA_PASSWORD" \
  --org-id 2 \
  --import-dir ./dashboards/raw \
  --replace-existing
```

Dry-run import as a table:

```bash
python3 python/grafana-util.py dashboard import \
  --url http://127.0.0.1:3000 \
  --import-dir ./dashboards/raw \
  --dry-run \
  --table
```

Dry-run import with only the folder mismatch review columns:

```bash
python3 python/grafana-util.py dashboard import \
  --url http://127.0.0.1:3000 \
  --import-dir ./dashboards/raw \
  --dry-run \
  --output-format table \
  --output-columns uid,source_folder_path,destination_folder_path,reason,file
```

Update only dashboards that already exist in Grafana:

```bash
python3 python/grafana-util.py dashboard import \
  --url http://127.0.0.1:3000 \
  --import-dir ./dashboards/raw \
  --update-existing-only
```

Ensure exported folder UIDs exist before importing dashboards:

```bash
python3 python/grafana-util.py dashboard import \
  --url http://127.0.0.1:3000 \
  --import-dir ./dashboards/raw \
  --ensure-folders \
  --replace-existing
```

Only update existing dashboards when the raw source folder path matches the current Grafana folder path:

```bash
python3 python/grafana-util.py dashboard import \
  --url http://127.0.0.1:3000 \
  --import-dir ./dashboards/raw \
  --replace-existing \
  --require-matching-folder-path
```

Fail early when the raw export org does not match the import target org:

```bash
python3 python/grafana-util.py dashboard import \
  --url http://127.0.0.1:3000 \
  --token "$GRAFANA_API_TOKEN" \
  --import-dir ./dashboards/raw \
  --dry-run \
  --require-matching-export-org
```

Important rules:

- point `--import-dir` at `dashboards/raw/`, not the combined `dashboards/` root
- do not feed `prompt/` files into API import
- files containing `__inputs` should be imported through Grafana web UI
- `--import-folder-uid` overrides the target folder for all imported dashboards
- `--ensure-folders` uses `raw/folders.json` to create any missing destination folders before dashboard import; do not combine it with `--import-folder-uid`
- `--import-message` sets the dashboard version-history message
- `--dry-run` shows whether each dashboard would create or update without calling Grafana import APIs
- `--dry-run --ensure-folders` also checks the exported folder inventory against destination Grafana and reports which folders are missing, matching, or mismatched before any real import run
- `--dry-run --table` renders the same predictions as a summary table, including each dashboard's destination folder path, and `--no-header` suppresses that table's header row
- `--dry-run --json` renders one machine-readable JSON document with the active import mode, folder inventory checks, per-dashboard actions, destination folder paths, and summary counts
- `--org-id <ID>` switches the whole import run to one explicit destination Grafana org and requires Basic auth; the raw export's recorded `orgId` is not used automatically for routing
- plain `--token` import still works, but only in the token's current org context
- `--require-matching-export-org` compares the raw export's recorded `orgId` from `index.json`, `folders.json`, or `datasources.json` against the resolved target org for this run
- `--require-matching-export-org` is useful when a token already points at one org and you want a guard against accidentally replaying a raw export from another org
- `--require-matching-export-org` fails when the raw export does not carry one stable `orgId`, because the safety check cannot prove the import target is correct
- `--update-existing-only` changes import mode from `create-or-update` to `update-or-skip-missing`, keyed by dashboard `uid`
- when updating an existing dashboard by `uid`, import preserves the destination Grafana folder by default unless you explicitly pass `--import-folder-uid`
- `--require-matching-folder-path` only affects updates to existing dashboards; missing dashboards still follow the active create or skip mode
- `--require-matching-folder-path` compares the raw source folder path against the current destination Grafana folder path and skips updates that drifted to a different folder path
- `--require-matching-folder-path` adds `source_folder_path` and `destination_folder_path` details to dry-run table and JSON output when relevant
- do not combine `--require-matching-folder-path` with `--import-folder-uid`
- `diff` compares local raw files with the live Grafana dashboard payload and returns exit code `1` when differences are found

Dashboard export also writes small versioned manifest files named `export-metadata.json` at the root and per-variant directories. The raw export additionally writes `raw/folders.json` with folder `uid`, `title`, `parentUid`, `path`, `org`, and `orgId` records plus `raw/datasources.json` with datasource `uid`, `name`, `type`, `access`, `url`, `isDefault`, `org`, and `orgId` records so later offline inspection can compare usage against the exported Grafana datasource inventory.

## Alerting Utility

`grafana-util alert` handles Grafana alerting resources separately from dashboards.

Supported resources:

- alert rules
- contact points
- mute timings
- notification policies
- notification message templates

Read-only alert listing:

- `grafana-util alert list-rules`
- `grafana-util alert list-contact-points`
- `grafana-util alert list-mute-timings`
- `grafana-util alert list-templates`

Direct alert aliases:

- `grafana-util export-alert`
- `grafana-util import-alert`
- `grafana-util diff-alert`
- `grafana-util list-alert-rules`
- `grafana-util list-alert-contact-points`
- `grafana-util list-alert-mute-timings`
- `grafana-util list-alert-templates`

### Alerting Export

Example:

```bash
python3 python/grafana-util.py alert export \
  --url http://localhost:3000 \
  --output-dir ./alerts \
  --overwrite
```

Use `--flat` if you want a flatter directory layout:

```bash
python3 python/grafana-util.py alert export --output-dir ./alerts --flat
```

### Alerting Import

Example:

```bash
python3 python/grafana-util.py alert import \
  --url http://localhost:3000 \
  --import-dir ./alerts/raw \
  --replace-existing
```

Import with linked dashboard or panel remapping:

```bash
python3 python/grafana-util.py alert import \
  --url https://grafana.example.com \
  --import-dir ./alerts/raw \
  --replace-existing \
  --dashboard-uid-map ./dashboard-map.json \
  --panel-id-map ./panel-map.json
```

Alerting diff:

```bash
python3 python/grafana-util.py alert diff \
  --url https://grafana.example.com \
  --diff-dir ./alerts/raw \
  --dashboard-uid-map ./dashboard-map.json \
  --panel-id-map ./panel-map.json
```

Example `dashboard-map.json`:

```json
{
  "old-dashboard-uid": "new-dashboard-uid"
}
```

Example `panel-map.json`:

```json
{
  "old-dashboard-uid": {
    "7": "19"
  }
}
```

### Alerting Import Rules

- `--replace-existing` updates existing rules by `uid`
- `--replace-existing` updates existing contact points by `uid`
- `--replace-existing` updates existing mute timings by `name`
- notification policies are always applied with `PUT`
- notification templates are applied with `PUT`, and with `--replace-existing` the tool reads the current template version first
- without `--replace-existing`, rule/contact-point/mute-timing import uses create and Grafana rejects conflicts
- without `--replace-existing`, template import fails if the template name already exists
- import expects files exported by this tool
- do not point `--import-dir` at the combined `alerts/` root
- `--dry-run` predicts whether each file would create, update, or fail without changing Grafana
- `--diff-dir` compares local exported files with live alerting resources and returns exit code `1` when differences are found

Important limitation:

- Grafana official alert provisioning `/export` output is not a supported import format for this tool
- this tool only guarantees round-trip import for files exported by `grafana-util alert export`

Why this happens:

- Grafana's alert provisioning `export` payload is meant for provisioning-style representation, not direct HTTP API round-trip import
- Grafana's create/update APIs expect a different request shape than the `/export` response shape
- because of that mismatch, this tool uses its own export format for backup and restore workflows

For linked alert rules:

- use `--dashboard-uid-map` and `--panel-id-map` when dashboard or panel identities changed
- maintainer details about fallback matching and repair behavior are in [`DEVELOPER.md`](DEVELOPER.md)

## Access Utility

`grafana-util access ...` is the primary access-management CLI track.

Compatibility shims:

- Python source-tree and package usage now use only `grafana-util access ...`

Current implementation scope:

- Python implementation
- Rust implementation
- `user list`
- `user add`
- `user modify`
- `user delete`
- `team list`
- `team modify`
- `team add`
- `team delete`
- `service-account list`
- `service-account add`
- `service-account token add`
- `service-account delete`
- `service-account token delete`
- `group` compatibility alias for `team`

Initial auth model:

- `user list --scope org` may use token auth or Basic auth
- `user list --scope global` requires Basic auth because Grafana global user APIs are Basic-auth-first admin workflows
- `user add` requires Basic auth because Grafana user-creation is a server-admin workflow
- `user modify` requires Basic auth because it uses global and admin user-management APIs
- `user delete --scope global` requires Basic auth because it uses the global admin delete API
- `user delete --scope org` may use token auth or Basic auth
- `team list` is org-scoped and may use token auth or Basic auth
- `team modify` is org-scoped and may use token auth or Basic auth
- `team add` is org-scoped and may use token auth or Basic auth
- service-account commands are org-scoped and may use token auth or Basic auth

Output modes:

- compact text by default
- `--table`
- `--csv`
- `--json`

## Build and Install

### Python Package

Install into the current Python environment:

```bash
python3 -m pip install .
```

Install into a user-local environment:

```bash
python3 -m pip install --user .
```

Optional HTTP/2 dependencies on Python 3.9+:

```bash
python3 -m pip install '.[http2]'
```

### Makefile Shortcuts

The repo root includes a [`Makefile`](Makefile):

- `make help`
- `make build-python`
- `make build-rust`
- `make build-rust-macos-arm64`
- `make build-rust-linux-amd64`
- `make build-rust-linux-amd64-zig`
- `make seed-grafana-sample-data`
- `make destroy-grafana-sample-data`
- `make reset-grafana-all-data`
- `make build`
- `make test-python`
- `make test-rust`
- `make fmt-rust-check`
- `make lint-rust`
- `make quality`
- `make quality-python`
- `make quality-rust`
- `make test-rust-live`
- `make test-access-live`
- `make test`

Artifact locations:

- `make build-python` writes the wheel into `dist/`
- `make build-rust` writes release binaries into `rust/target/release/`
- `make build-rust-macos-arm64` writes native Apple Silicon Rust binaries into `dist/macos-arm64/`
- `make build-rust-linux-amd64` writes Linux `amd64` Rust binaries into `dist/linux-amd64/`
- `make build-rust-linux-amd64-zig` writes Linux `amd64` Rust binaries into `dist/linux-amd64/` without Docker

Basic quality gates:

- `make quality` runs the repo's script-backed baseline automated checks across Python and Rust
- `make quality-python` runs `scripts/check-python-quality.sh`
- `make quality-rust` runs `scripts/check-rust-quality.sh`
- the Python quality script always runs bytecode compilation plus `unittest`, and it only runs optional tools such as `ruff`, `mypy`, and `black --check` when they are installed
- `make fmt-rust-check` runs `cargo fmt --check`
- `make lint-rust` runs `cargo clippy --all-targets -- -D warnings`

### Rust Build and Run

Build Rust release binaries:

```bash
make build-rust
```

Build native macOS Apple Silicon Rust release binaries into a platform output directory:

```bash
make build-rust-macos-arm64
```

Build Linux `amd64` Rust release binaries from macOS or another non-Linux host with Docker:

```bash
make build-rust-linux-amd64
```

Build Linux `amd64` Rust release binaries from macOS without Docker, using `zig`:

```bash
make build-rust-linux-amd64-zig
```

Run the Rust dashboard CLI from the repo and export dashboards from local Grafana:

```bash
cd rust
cargo run --bin grafana-util -- dashboard export \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --export-dir ./dashboards
```

List dashboards from local Grafana with the Rust CLI:

```bash
cd rust
cargo run --bin grafana-util -- dashboard list \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --table
```

Compare raw dashboard exports with local Grafana using the Rust CLI:

```bash
cd rust
cargo run --bin grafana-util -- dashboard diff \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --import-dir ./dashboards/raw
```

Show Rust dashboard CLI help:

```bash
cd rust
cargo run --bin grafana-util -- dashboard -h
```

Linux `amd64` build notes:

- `make build-rust-macos-arm64` is the explicit native Apple Silicon release path and copies binaries into `dist/macos-arm64/`
- `make build-rust-linux-amd64` uses Docker with the official Rust image
- `make build-rust-linux-amd64-zig` uses local `zig`, `cargo-zigbuild`, and a `rustup` target instead of Docker
- output binaries are written to `dist/linux-amd64/`
- current output name is `dist/linux-amd64/grafana-util`
- this path is intended for macOS hosts that need Linux release artifacts without installing a local cross-linker

Run the Rust alerting CLI from the repo:

```bash
cd rust
cargo run --bin grafana-util -- alert -h
```

Run the Docker-backed Rust live smoke test:

```bash
make test-rust-live
```

Run the Docker-backed Python access live smoke test:

```bash
make test-access-live
```

Seed reusable developer sample data into a running local Grafana:

```bash
make seed-grafana-sample-data
```

Destroy the same developer sample data from a running local Grafana:

```bash
make destroy-grafana-sample-data
```

Dangerous developer reset for a disposable local Grafana:

```bash
make reset-grafana-all-data
```

Notes:

- requires Docker plus local access to the Docker daemon
- defaults to `grafana/grafana:12.4.1` and can be overridden with `GRAFANA_IMAGE=...`
- uses a random localhost port by default; set `GRAFANA_PORT=43000` if you want a fixed port
- starts a temporary Grafana container, seeds one dashboard, one datasource, and one contact point
- validates Rust dashboard export/import/diff/dry-run and Rust alerting export/import/diff/dry-run

Python access live smoke test notes:

- `make test-access-live` runs `scripts/test-python-access-live-grafana.sh`
- the script defaults to `grafana/grafana:12.4.1` and binds Grafana to a random localhost port unless `GRAFANA_PORT` is set explicitly
- it bootstraps an API token, then validates `user add`, `user modify`, `user delete`, `team add`, `team modify`, `team list`, `service-account add`, `service-account token add`, and `service-account list`
- useful overrides: `GRAFANA_IMAGE`, `GRAFANA_PORT`, `GRAFANA_USER`, `GRAFANA_PASSWORD`, `PYTHON_BIN`

Developer sample-data seed notes:

- `make seed-grafana-sample-data` runs `scripts/seed-grafana-sample-data.sh`
- `make destroy-grafana-sample-data` runs `scripts/seed-grafana-sample-data.sh --destroy`
- `make reset-grafana-all-data` runs `scripts/seed-grafana-sample-data.sh --reset-all-data --yes`
- defaults to `http://localhost:3000` with `admin/admin`
- seeds idempotent sample orgs, datasources, folders, and dashboards for manual CLI testing
- destroy mode removes only the known sample dashboards, folders, datasources, and extra sample orgs
- reset-all-data mode is for disposable developer Grafana instances and deletes repo-relevant test data such as extra orgs, dashboards, folders, datasources, teams, service accounts, alert rules, and non-admin users
- useful overrides: `GRAFANA_URL`, `GRAFANA_USER`, `GRAFANA_PASSWORD`

## Authentication and TLS

Authentication methods:

- API token with `--token` or legacy `--api-token`
- Prompted API token with `--prompt-token`
- Basic auth with `--basic-user` and `--basic-password`
- Prompted Basic auth with `--basic-user` and `--prompt-password`

Auth note:

- prefer either token auth or Basic auth for one command, not both
- `--prompt-token` hides the API token input instead of putting it in shell history or process arguments
- the CLIs reject partial Basic auth input such as only `--basic-user` without `--basic-password` or `--prompt-password`
- `--prompt-password` hides the password input instead of putting it in shell history or process arguments
- `GRAFANA_API_TOKEN`, `GRAFANA_USERNAME`, and `GRAFANA_PASSWORD` still work as environment fallbacks
- for `grafana-util access user list`, org-scoped listing can use token auth or Basic auth
- for `grafana-util access user list --scope global`, Basic auth is required
- for `grafana-util access user add`, Basic auth is required
- for `grafana-util access user modify`, Basic auth is required
- for `grafana-util access user delete --scope global`, Basic auth is required
- for `grafana-util access user delete --scope org`, token auth or Basic auth can be used
- for `grafana-util access team list`, token auth or Basic auth can be used
- for `grafana-util access team add`, token auth or Basic auth can be used
- for `grafana-util access team modify`, token auth or Basic auth can be used
- for `grafana-util access team delete`, token auth or Basic auth can be used
- for `grafana-util access service-account ...`, token auth or Basic auth can be used

Username/password example:

```bash
export GRAFANA_USERNAME='admin'
export GRAFANA_PASSWORD='admin'
python3 python/grafana-util.py dashboard export \
  --url http://localhost:3000 \
  --basic-user "$GRAFANA_USERNAME" \
  --basic-password "$GRAFANA_PASSWORD" \
  --export-dir ./dashboards
```

Prompted password example:

```bash
python3 python/grafana-util.py dashboard export \
  --url http://localhost:3000 \
  --basic-user admin \
  --prompt-password \
  --export-dir ./dashboards
```

Prompted token example:

```bash
python3 python/grafana-util.py dashboard export \
  --url http://localhost:3000 \
  --prompt-token \
  --export-dir ./dashboards
```

TLS note:

- SSL verification is disabled by default
- add `--verify-ssl` when you want strict certificate verification

Example:

```bash
python3 python/grafana-util.py dashboard export --verify-ssl
```

## Output Directory Layout

Dashboard export layout:

```text
dashboards/
  index.json
  export-metadata.json
  raw/
    export-metadata.json
    folders.json
    index.json
    ...
  prompt/
    export-metadata.json
    index.json
    ...
```

Alerting export layout:

```text
alerts/
  index.json
  raw/
    rules/
    contact-points/
    mute-timings/
    policies/
    templates/
```

## Validation

Common validation commands:

```bash
make test
python3 -m unittest -v
python3 python/grafana-util.py -h
python3 python/grafana-util.py dashboard -h
python3 python/grafana-util.py dashboard export -h
python3 python/grafana-util.py dashboard list -h
python3 python/grafana-util.py dashboard import -h
python3 python/grafana-util.py alert -h
python3 python/grafana-util.py access -h
python3 python/grafana-util.py access user list -h
python3 python/grafana-util.py access user add -h
python3 python/grafana-util.py access user modify -h
python3 python/grafana-util.py access user delete -h
python3 python/grafana-util.py access team list -h
python3 python/grafana-util.py access team add -h
python3 python/grafana-util.py access team modify -h
python3 python/grafana-util.py access team delete -h
python3 python/grafana-util.py access group delete -h
python3 python/grafana-util.py access service-account list -h
python3 python/grafana-util.py access service-account add -h
python3 python/grafana-util.py access service-account delete -h
python3 python/grafana-util.py access service-account token add -h
python3 python/grafana-util.py access service-account token delete -h
cd rust && cargo test
cd rust && cargo run --quiet --bin grafana-util -- -h
cd rust && cargo run --quiet --bin grafana-util -- dashboard -h
cd rust && cargo run --quiet --bin grafana-util -- dashboard export -h
cd rust && cargo run --quiet --bin grafana-util -- dashboard list -h
cd rust && cargo run --quiet --bin grafana-util -- dashboard import -h
cd rust && cargo run --quiet --bin grafana-util -- alert -h
cd rust && cargo run --quiet --bin grafana-util -- access -h
cd rust && cargo run --quiet --bin grafana-util -- access user list -h
cd rust && cargo run --quiet --bin grafana-util -- access user add -h
cd rust && cargo run --quiet --bin grafana-util -- access user modify -h
cd rust && cargo run --quiet --bin grafana-util -- access user delete -h
cd rust && cargo run --quiet --bin grafana-util -- access team list -h
cd rust && cargo run --quiet --bin grafana-util -- access team add -h
cd rust && cargo run --quiet --bin grafana-util -- access team modify -h
cd rust && cargo run --quiet --bin grafana-util -- access team delete -h
cd rust && cargo run --quiet --bin grafana-util -- access group delete -h
cd rust && cargo run --quiet --bin grafana-util -- access service-account list -h
cd rust && cargo run --quiet --bin grafana-util -- access service-account add -h
cd rust && cargo run --quiet --bin grafana-util -- access service-account delete -h
cd rust && cargo run --quiet --bin grafana-util -- access service-account token add -h
cd rust && cargo run --quiet --bin grafana-util -- access service-account token delete -h
make test-rust-live
make test-access-live
```

## Documentation

- English README: [`README.md`](README.md)
- Traditional Chinese README: [`README.zh-TW.md`](README.zh-TW.md)
- recent change history: [`CHANGELOG.md`](CHANGELOG.md)
- maintainer and implementation notes: [`DEVELOPER.md`](DEVELOPER.md)
- architecture overviews: [`Python`](docs/overview-python.md), [`Rust`](rust/overview-rust.md)
