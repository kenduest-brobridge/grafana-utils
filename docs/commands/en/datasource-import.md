# datasource import

## Purpose
Import datasource inventory through the Grafana API.

## When to use
Use this when you have a local datasource bundle or provisioning tree and want to push it into Grafana, either live or as a dry run.

## Key flags
- `--import-dir`: source path for inventory or provisioning input.
- `--input-format`: choose `inventory` or `provisioning`.
- `--org-id`, `--use-export-org`, `--only-org-id`, `--create-missing-orgs`: control cross-org routing.
- `--replace-existing`, `--update-existing-only`, `--require-matching-export-org`: import safety and reconciliation controls.
- `--secret-values`: resolve placeholder secrets during import.
- `--dry-run`, `--table`, `--json`, `--output-format`, `--no-header`, `--output-columns`, `--progress`, `--verbose`: preview and reporting controls.

## Examples
```bash
grafana-util datasource import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./datasources --dry-run --table
grafana-util datasource import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./datasources --use-export-org --only-org-id 2 --create-missing-orgs --dry-run --json
```

## Related commands
- [datasource export](./datasource-export.md)
- [datasource diff](./datasource-diff.md)
- [datasource inspect-export](./datasource-inspect-export.md)
