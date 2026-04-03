# Dashboard Handbook

This page covers `grafana-util dashboard` as an operator workflow for inventory, export/import, drift review, and dashboard analysis. It is intentionally broader than a command reference: the goal is to explain how the dashboard area fits into staged change, live review, and Grafana file provisioning, and how to read the outputs that drive those decisions.

## What this area is for

Use the dashboard area when you need to understand what exists in Grafana, move dashboards between environments, validate what was exported, or compare a staged dashboard tree against live Grafana before making changes. It is the strongest part of the tool for estate-level dashboard governance.

This area is not trying to replace Grafana's dashboard editor. Treat it as the migration, audit, and operator-safe review surface.

Typical reasons to use it:

- inventory dashboards and their folder placement across one org or many orgs
- export dashboards into reviewable files
- inspect queries, folder structure, and datasource dependencies offline
- compare local dashboard files against live Grafana before applying changes
- import dashboards back into Grafana with a dry-run first
- delete dashboards deliberately by UID or folder subtree
- publish or patch a local dashboard file through the same import pipeline used by the CLI

Do not use this area when you only want to tweak a panel interactively in Grafana, or when the problem is limited to datasource configuration. In those cases, stay in Grafana's editor or use the datasource handbook instead.

## Workflow Boundaries

Dashboard export intentionally produces three different lanes because each lane serves a different operator workflow.

- `raw/` is the canonical Grafana Utilities replay and import lane. Use this when you want a reversible export that can be fed back into `dashboard import`.
- `prompt/` is the Grafana UI import lane. Use this when you need a file shape that lines up with the dashboard import experience in Grafana itself.
- `provisioning/` is the Grafana file-provisioning lane. Use this when Grafana should read dashboards from disk through provisioning rather than through API replay.

These lanes are not interchangeable. Pick the lane that matches the workflow you are performing and keep it consistent through review, diff, and import.

The export tree also includes permission metadata in `raw/permissions.json`. That bundle is useful for backup and review, but the current import path does not restore permissions from it. Import restores dashboard content, folder placement, and the related raw inventory.

Local authoring commands such as `dashboard get`, `dashboard clone-live`, `dashboard patch-file`, and `dashboard publish` keep the wrapped Grafana document shape so the file can later pass through the same import pipeline.

## Staged vs Live

Think of the dashboard workflow in two halves:

- staged work is the local export tree, validation, offline inspection, and dry-run import review
- live work is the Grafana-backed inventory, live diff, import, delete, and live inspection path

Use staged commands when you want to understand the shape of the data before touching Grafana. Use live commands when you are ready to compare, mutate, or verify the live instance.

Practical rule:

- start with `dashboard list` or `dashboard browse` when you need to discover what exists
- export to `raw/`, `prompt/`, or `provisioning/` when you need a reusable file tree
- use `dashboard inspect-export` and `dashboard validate-export` to review staged files offline
- use `dashboard diff` to compare staged files against live Grafana
- use `dashboard import --dry-run` before any live import
- use `dashboard delete` only when the target is explicit and reviewed

## Reading Live Inventory

Start here when you want a fast picture of the estate before touching files or imports. `dashboard list` shows what exists right now, where each dashboard lives, and which org owns it.

```bash
grafana-util dashboard list \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --table
```

Validated live output excerpt:

```text
UID                      NAME                                      FOLDER  FOLDER_UID      FOLDER_PATH  ORG        ORG_ID
-----------------------  ----------------------------------------  ------  --------------  -----------  ---------  ------
rYdddlPWl                Node Exporter Full for Host               Demo    ffhrmit0usjk0b  Demo         Main Org.  1
spring-jmx-node-unified  Spring JMX + Node Unified Dashboard (VM)  Demo    ffhrmit0usjk0b  Demo         Main Org.  1
spring-jmx-new           Spring JMX Unified Dashboard (VM)         Demo    ffhrmit0usjk0b  Demo         Main Org.  1

Listed 3 dashboard(s).
```

Read the table as a live inventory, not as a change set:

- `UID` is the stable identity for automation, export, diff, and delete.
- `NAME` is the human-facing dashboard title.
- `FOLDER` and `FOLDER_PATH` tell you where the dashboard is organized in Grafana.
- `ORG` and `ORG_ID` tell you which Grafana org owns the object.

If the folder path or org is surprising, stop and inspect before exporting or importing. Most operator mistakes in this area come from targeting the right UID in the wrong org or folder.

## Key Commands

The dashboard area is centered on a small set of commands that cover the main operator lifecycle.

| Command | Best use |
| --- | --- |
| `dashboard list` | Live inventory with UID, title, folder, and optional datasource context |
| `dashboard export` | Export live dashboards into `raw/`, `prompt/`, and `provisioning/` trees |
| `dashboard import` | Replay or provision dashboards into live Grafana, usually after a dry-run |
| `dashboard diff` | Compare local dashboard files against live Grafana |
| `dashboard inspect-export` | Analyze an export tree offline, including dependency and governance views |
| `dashboard inspect-live` | Inspect live dashboard structure without exporting first |
| `dashboard validate-export` | Check export compatibility before syncing or importing |
| `dashboard delete` | Remove live dashboards by UID or folder subtree |
| `dashboard publish` | Push one local dashboard file through the same import pipeline |
| `dashboard clone-live` | Copy a live dashboard into a local file for editing |
| `dashboard patch-file` | Apply focused edits to a dashboard file while keeping the file shape usable |

The strongest operational pattern is:

1. discover with `list` or `browse`
2. export the relevant dashboards
3. inspect and validate the staged files
4. diff against live Grafana
5. import with `--dry-run`
6. remove or replace live dashboards only after the dry-run matches your intent

## Docker Grafana Validated Examples

The examples below match the live-smoke path used in the main guide and are validated against a local Docker Grafana `12.4.1` instance seeded with `scripts/seed-grafana-sample-data.sh`.

### Export progress

Use `--progress` when you want one concise line per dashboard during a repeatable export.

```bash
grafana-util dashboard export \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --export-dir ./dashboards \
  --overwrite \
  --progress
```

Example output excerpt:

```text
Exporting dashboard 1/7: mixed-query-smoke
Exporting dashboard 2/7: smoke-prom-only
Exporting dashboard 3/7: query-smoke
Exporting dashboard 4/7: smoke-main
Exporting dashboard 5/7: subfolder-chain-smoke
Exporting dashboard 6/7: subfolder-main
Exporting dashboard 7/7: two-prom-query-smoke
```

Read this output as an export progress log, not as a diff. The export names are the dashboards being written into the local tree.

### Dry-run import preview

Use a dry-run import to confirm the destination before any live mutation.

```bash
grafana-util dashboard import \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --import-dir ./dashboards-flat/raw \
  --replace-existing \
  --dry-run \
  --table
```

Example output excerpt:

```text
Import mode: create-or-update
UID                    DESTINATION  ACTION  FOLDER_PATH                    FILE
---------------------  -----------  ------  -----------------------------  ------------------------------------------------------------
mixed-query-smoke      exists       update  General                        ./dashboards-flat/raw/Mixed_Query_Dashboard__mixed-query-smoke.json
smoke-main             exists       update  General                        ./dashboards-flat/raw/Smoke_Dashboard__smoke-main.json
subfolder-chain-smoke  exists       update  Platform / Team / Apps / Prod  ./dashboards-flat/raw/Subfolder_Chain_Dashboard__subfolder-chain-smoke.json

Dry-run checked 7 dashboard(s) from ./dashboards-flat/raw
```

How to read it:

- `ACTION=create` means Grafana does not currently have that dashboard and the import would create it.
- `ACTION=update` means the dashboard already exists and the import would replace the matching live object.
- `DESTINATION=missing` means the dry-run found no live dashboard with that identity.
- `DESTINATION=exists` means a matching live dashboard was found.

### Provisioning-oriented comparison

Use the provisioning lane explicitly when Grafana should read the dashboard tree from disk.

```bash
grafana-util dashboard diff \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --import-dir ./dashboards/provisioning \
  --input-format provisioning
```

Example output excerpt:

```text
Dashboard diff found 1 differing item(s).

--- live/cpu-main
+++ export/cpu-main
@@
-  "title": "CPU Overview"
+  "title": "CPU Overview v2"
```

This is the right shape when you want to verify a provisioning tree instead of a raw replay tree.

## Output Excerpts

The dashboard command family uses a few recurring output cues:

- `UID` is the stable identity for automation.
- `FOLDER_PATH` or `FOLDER` tells you where the dashboard will land.
- `ACTION=create` and `ACTION=update` are the primary import decisions.
- `raw/` should be treated as the canonical replay source.
- `prompt/` should be treated as the UI import source.
- `provisioning/` should be treated as the file-provisioning source.

If you are reviewing a large estate, prefer table or report output for humans and JSON for automation. If you are moving a dashboard tree between environments, always review the export tree before removing `--dry-run`. When the output is a table, read the row status first, then the folder path, then the file or UID column that tells you what will actually be touched.
