# datasource list

## Purpose
List datasource inventory from live Grafana or a local export bundle.

## When to use
Use this when you need a non-interactive inventory of datasources, either from live Grafana for the current org, one explicit org, or across all visible orgs, or from a local export bundle on disk.

## Key flags
- `--input-dir`: read a local datasource export bundle or provisioning tree.
- `--input-format`: choose `inventory` or `provisioning` when the local path can mean either source shape.
- `--org-id`: list one explicit Grafana org.
- `--all-orgs`: aggregate datasource inventory across visible orgs. Requires Basic auth.
- `--output-format`, `--text`, `--table`, `--csv`, `--json`, `--yaml`: output mode controls.
- `--output-columns`: choose the displayed columns.
- `--no-header`: suppress table headers.

## Examples
```bash
# Purpose: List live Grafana datasource inventory.
grafana-util datasource list --url http://localhost:3000 --basic-user admin --basic-password admin --output-format text
```

```bash
# Purpose: List live Grafana datasource inventory.
grafana-util datasource list --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --output-format yaml
```

```bash
# Purpose: List datasource inventory from a local export bundle.
grafana-util datasource list --input-dir ./datasources --json
```

## Before / After

- **Before**: datasource inventory was easy to read only after jumping between Grafana UI, export bundles, or ad hoc API calls.
- **After**: one inventory command can give you a reviewable snapshot in text, table, CSV, JSON, or YAML from live Grafana or a local bundle.

## What success looks like

- you can point the command at the org you care about or at a local bundle and get the inventory you expected
- table and CSV output are easy to hand to a script or review in a pull request
- all-org inventory only happens when you really want a cross-org read

## Failure checks

- if the inventory is empty, confirm the org scope and whether the credentials can see the target org
- if `--all-orgs` fails, fall back to Basic auth and check whether the token is limited to one org
- if a local bundle does not read correctly, confirm `--input-dir` and `--input-format`
- if column selection looks wrong, verify the output format and requested columns together

## Related commands
- [datasource browse](./datasource-browse.md)
- [datasource export](./datasource-export.md)
- [datasource diff](./datasource-diff.md)
