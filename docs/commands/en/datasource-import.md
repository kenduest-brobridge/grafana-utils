# datasource import

## Purpose
Import datasource inventory through the Grafana API.

## When to use
Use this when you have a local datasource bundle or provisioning tree and want to push it into Grafana, either live or as a dry run.

## Key flags
- `--input-dir`: source path for inventory or provisioning input.
- `--input-format`: choose `inventory` or `provisioning`.
- `--org-id`, `--use-export-org`, `--only-org-id`, `--create-missing-orgs`: control cross-org routing.
- `--replace-existing`, `--update-existing-only`, `--require-matching-export-org`: import safety and reconciliation controls.
- `--secret-values`: resolve placeholder secrets during import.
- `--secret-values-file`: resolve placeholder secrets from a JSON file during import.
- `--dry-run`, `--table`, `--json`, `--output-format`, `--no-header`, `--output-columns`, `--list-columns`, `--progress`, `--verbose`: preview and reporting controls. Use `--output-columns all` for the full dry-run table.

## Examples
```bash
# Purpose: Import datasource inventory through the Grafana API.
grafana-util datasource import --url http://localhost:3000 --basic-user admin --basic-password admin --input-dir ./datasources --dry-run --table
```

```bash
# Purpose: Import datasource inventory through the Grafana API.
grafana-util datasource import --url http://localhost:3000 --basic-user admin --basic-password admin --input-dir ./datasources --use-export-org --only-org-id 2 --create-missing-orgs --dry-run --json
```

## Before / After

- **Before**: importing datasource bundles usually meant manually reconciling files, orgs, and secrets before touching Grafana.
- **After**: one import command can preview the plan, reconcile org routing, and then push the bundle with the right guardrails.

## What success looks like

- the import preview shows which orgs and datasources will change
- provisioning and inventory inputs both route correctly
- secrets are resolved before the live import, not after the damage is done

## Failure checks

- if the import touches the wrong org, verify the routing flags before trying again
- if the plan is incomplete, confirm the `--input-format` and whether the bundle is inventory or provisioning
- if secrets stay unresolved, check the placeholder names in `secureJsonDataPlaceholders` and the keys provided through `--secret-values` or `--secret-values-file`

## Related commands
- [datasource list](./datasource-list.md)
- [datasource export](./datasource-export.md)
- [datasource diff](./datasource-diff.md)
