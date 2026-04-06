# dashboard import

## Purpose
Import dashboard JSON files through the Grafana API.

## When to use
Use this when you have a local export tree or provisioning tree and need to push dashboards back into Grafana, either live or as a dry run. This command consumes `raw/` or `provisioning/` inputs; it does not consume the Grafana UI `prompt/` lane.

## Before / After
- **Before**: import is a blind replay step, and you find folder, org, or schema problems only after the live call.
- **After**: import becomes a controlled replay step that can be previewed first with `--dry-run`, then executed with clearer intent.

## Key flags
- `--input-dir`: source directory for raw or combined export input.
- `--input-format`: choose `raw` or `provisioning`.
- `--org-id`, `--use-export-org`, `--only-org-id`, `--create-missing-orgs`: control cross-org routing.
- `--import-folder-uid`: force a destination folder UID.
- `--ensure-folders`, `--replace-existing`, `--update-existing-only`: control import behavior.
- `--require-matching-folder-path`, `--require-matching-export-org`, `--strict-schema`, `--target-schema-version`: safety checks.
- `--import-message`: revision message stored in Grafana.
- `--interactive`, `--dry-run`, `--table`, `--json`, `--output-format`, `--output-columns`, `--no-header`, `--progress`, `--verbose`: preview and reporting controls.

## What success looks like
- dry-run shows the expected create/update actions before you touch the live server
- the destination org and folder routing are explicit enough to review
- the chosen input lane matches the replay goal: `raw` or `provisioning`, not `prompt`

## Failure checks
- if folder or org placement looks wrong, verify the routing flags before re-running live import
- if the replay looks too destructive, stop at `--dry-run` and inspect the export tree first
- if the schema check blocks replay, confirm whether the source tree needs normalization before import

## Examples
```bash
# Purpose: Import dashboard JSON files through the Grafana API.
grafana-util dashboard import --url http://localhost:3000 --basic-user admin --basic-password admin --input-dir ./dashboards/raw --replace-existing
```

```bash
# Purpose: Import dashboard JSON files through the Grafana API.
grafana-util dashboard import --url http://localhost:3000 --basic-user admin --basic-password admin --input-dir ./dashboards/raw --dry-run --table
```

## Related commands
- [dashboard export](./dashboard-export.md)
- [dashboard raw-to-prompt](./dashboard-raw-to-prompt.md)
- [dashboard diff](./dashboard-diff.md)
- [dashboard publish](./dashboard-publish.md)
