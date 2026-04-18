# `grafana-util dashboard plan`

## Purpose
Build a review-first dashboard reconcile plan from a local dashboard export tree against live Grafana.

## When to use
Use this when you want to understand what a dashboard export would mean against a target Grafana before deciding whether to import, re-export, prune remote-only dashboards, or inspect dependency warnings.

`dashboard plan` does not mutate Grafana. It turns local-vs-live dashboard differences into operator actions such as `same`, `would-create`, `would-update`, `extra-remote`, `would-delete`, and `blocked-target`.
It remains a read-only review surface. `--use-export-org` maps a combined multi-org export root back to the matching target org IDs, and it requires Basic auth so the command can resolve live org routing.

## Key flags
- `--input-dir`: local dashboard export root or dashboard variant directory.
- `--input-type`: choose `raw` or `source`. Use `source` for prompt-lane exports.
- `--org-id`: plan against one explicit Grafana org.
- `--use-export-org`: route a combined multi-org export root by exported org IDs instead of a single explicit target org.
- `--only-org-id`: limit the plan to one or more exported source org IDs.
- `--create-missing-orgs`: keep missing destination orgs as review-only `would-create` entries in the plan.
- `--prune`: show remote-only dashboards as `would-delete` candidates. Without this flag they remain `extra-remote`.
- `--output-format`: choose `text`, `table`, or `json`.
- `--show-same`: include unchanged rows in text and table output.
- `--output-columns`, `--list-columns`, `--no-header`: tune table output.

## Examples
```bash
# Build a concise review plan for a raw dashboard export.
grafana-util dashboard plan --profile prod --input-dir ./dashboards/raw
```

```bash
# Render a table with selected review columns.
grafana-util dashboard plan --profile prod --input-dir ./dashboards/raw --output-format table --output-columns actionId,dashboardTitle,folderPath,status
```

```bash
# Review a prompt/source export tree.
grafana-util dashboard plan --profile prod --input-dir ./dashboards/prompt --input-type source --output-format json
```

```bash
# Plan a combined all-org export root against matching target org IDs.
grafana-util dashboard plan --profile prod --input-dir ./dashboards --use-export-org --output-format json
```

```bash
# Limit the plan to one exported source org.
grafana-util dashboard plan --profile prod --input-dir ./dashboards --use-export-org --only-org-id 42 --output-format table
```

```bash
# Keep missing destination orgs as review-only would-create entries.
grafana-util dashboard plan --profile prod --input-dir ./dashboards --use-export-org --only-org-id 42 --create-missing-orgs --output-format json
```

```bash
# Include remote-only dashboards as delete candidates.
grafana-util dashboard plan --profile prod --input-dir ./dashboards/raw --prune --output-format json
```

## Before / After

- **Before**: dashboard import and diff flows could show pieces of local-vs-live state, but there was no one dashboard-specific reconcile review document.
- **After**: `dashboard plan` shows create, update, remote-only, delete-candidate, blocked, and warning rows in one review model.
- JSON output is structured for CI and future TUI review. Rows include stable `actionId`, status, changed fields, target evidence, dependency hints, and review hints.
- Multi-org export roots are resolved from exported org IDs, not from file-system folder names alone.

## What success looks like

- create and update candidates are visible before import
- remote-only dashboards are called out without deleting anything by default
- provisioned or managed targets are blocked before an operator treats the plan as ready
- unresolved datasource references and folder issues are surfaced as review hints
- missing destination orgs can stay as review-only `would-create` entries when `--create-missing-orgs` is set

## Failure checks

- if the plan points at the wrong org, confirm `--org-id` or the selected profile
- if `--use-export-org` is needed, make sure the export root is a combined multi-org export and that Basic auth is configured
- if `--only-org-id` is set, confirm the exported source org IDs are present in the export metadata
- if delete candidates appear unexpectedly, rerun without `--prune` and inspect `extra-remote` rows first
- if dependency hints appear, confirm the target Grafana has the expected datasource inventory and folder structure

## Related commands
- [dashboard export](./dashboard-export.md)
- [dashboard import](./dashboard-import.md)
- [dashboard diff](./dashboard-diff.md)
- [dashboard dependencies](./dashboard-dependencies.md)
