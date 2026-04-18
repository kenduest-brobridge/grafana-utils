# datasource plan

## Purpose
Build a review-first datasource reconcile plan from a local bundle against live Grafana.

## When to use
Use this when you want to understand what a datasource bundle would mean against a target Grafana before deciding whether to import, re-export, or prune remote-only datasources.

`datasource plan` does not mutate Grafana. It turns bundle-vs-live differences into operator actions such as `would-create`, `would-update`, `extra-remote`, `would-delete`, and `blocked-read-only`.

## Key flags
- `--input-dir`: local datasource bundle, workspace root, provisioning directory, or concrete provisioning YAML file.
- `--input-format`: choose `inventory` or `provisioning`.
- `--org-id`: plan against one explicit Grafana org.
- `--use-export-org`, `--only-org-id`, `--create-missing-orgs`: route an all-org datasource export back to matching destination orgs.
- `--prune`: show remote-only datasources as `would-delete` candidates. Without this flag they remain `extra-remote`.
- `--output-format`: choose `text`, `table`, or `json`.
- `--show-same`: include unchanged rows in text and table output.
- `--output-columns`, `--list-columns`, `--no-header`: tune table output.

## Examples
```bash
# Build a concise review plan for one datasource bundle.
grafana-util datasource plan --profile prod --input-dir ./datasources
```

```bash
# Render a table with action rows.
grafana-util datasource plan --profile prod --input-dir ./datasources --output-format table
```

```bash
# Include remote-only datasources as delete candidates.
grafana-util datasource plan --profile prod --input-dir ./datasources --prune --output-format json
```

```bash
# Plan an all-org export against matching destination orgs.
grafana-util datasource plan --url http://localhost:3000 --basic-user admin --basic-password admin --input-dir ./datasources --use-export-org --output-format table
```

## Before / After

- **Before**: `datasource diff` showed local-vs-live differences, and `datasource import --dry-run` previewed import records, but neither gave one full reconcile review surface.
- **After**: `datasource plan` shows create, update, remote-only, delete-candidate, and blocked actions in one review model.
- JSON output is structured for CI and future TUI review. Rows include stable `actionId`, status, target evidence, changed fields, and review hints.

## What success looks like

- create and update candidates are visible before import
- remote-only datasources are called out without deleting anything by default
- read-only or provisioned targets are blocked before an operator treats the plan as ready
- JSON output can be saved as review evidence or loaded by automation

## Failure checks

- if the plan points at the wrong org, confirm `--org-id` or `--use-export-org`
- if `--use-export-org` fails, confirm the input is a combined inventory export root and that credentials can enumerate orgs
- if delete candidates appear unexpectedly, rerun without `--prune` and inspect `extra-remote` rows first
- if secret-related rows are not conclusive, remember that Grafana live APIs do not return plaintext datasource secrets

## Related commands
- [datasource diff](./datasource-diff.md)
- [datasource import](./datasource-import.md)
- [datasource export](./datasource-export.md)
- [datasource delete](./datasource-delete.md)
