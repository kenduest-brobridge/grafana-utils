# dashboard import

## Purpose
Import dashboard JSON files through the Grafana API.

## When to use
Use this when you have a local export tree or provisioning tree and need to push dashboards back into Grafana, either live or as a dry run. This command consumes `raw/` or `provisioning/` inputs; it does not consume the Grafana UI `prompt/` lane.

## Key flags
- `--import-dir`: source directory for raw or combined export input.
- `--input-format`: choose `raw` or `provisioning`.
- `--org-id`, `--use-export-org`, `--only-org-id`, `--create-missing-orgs`: control cross-org routing.
- `--import-folder-uid`: force a destination folder UID.
- `--ensure-folders`, `--replace-existing`, `--update-existing-only`: control import behavior.
- `--require-matching-folder-path`, `--require-matching-export-org`, `--strict-schema`, `--target-schema-version`: safety checks.
- `--import-message`: revision message stored in Grafana.
- `--interactive`, `--dry-run`, `--table`, `--json`, `--output-format`, `--output-columns`, `--no-header`, `--progress`, `--verbose`: preview and reporting controls.

## Examples
```bash
grafana-util dashboard import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards/raw --replace-existing
grafana-util dashboard import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards/raw --dry-run --table
```

## Related commands
- [dashboard export](./dashboard-export.md)
- [dashboard raw-to-prompt](./dashboard-raw-to-prompt.md)
- [dashboard diff](./dashboard-diff.md)
- [dashboard publish](./dashboard-publish.md)
