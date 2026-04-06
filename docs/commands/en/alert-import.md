# `grafana-util alert import`

## Purpose

Import alerting resource JSON files through the Grafana API.

## When to use

- Recreate an exported alert bundle in Grafana.
- Update existing alert resources with `--replace-existing`.
- Preview import actions before making changes.

## Key flags

- `--input-dir` points at the `raw/` export directory.
- `--replace-existing` updates resources with matching identities.
- `--dry-run` previews the import.
- `--json` renders dry-run output as structured JSON.
- `--dashboard-uid-map` and `--panel-id-map` repair linked alert rules during import.

## Before / After

- Before: recreate alert resources manually or paste JSON back into Grafana one object at a time.
- After: import one export bundle and let the command handle the resource set as a unit.

## What success looks like

- Dry-run output matches the resources you intended to import.
- Live import restores the expected rules, contact points, mute timings, templates, and policies.
- Dashboard-linked alert rules still point at the right dashboards or panels after mapping.

## Failure checks

- Make sure `--input-dir` points at the `raw/` directory, not the parent export folder.
- If dashboard-linked rules move, supply the dashboard and panel mapping flags before importing.
- Use `--replace-existing` only when you expect to overwrite matching live resources.

## Examples

```bash
# Purpose: Import alerting resource JSON files through the Grafana API.
grafana-util alert import --url http://localhost:3000 --input-dir ./alerts/raw --replace-existing
```

```bash
# Purpose: Import alerting resource JSON files through the Grafana API.
grafana-util alert import --url http://localhost:3000 --input-dir ./alerts/raw --replace-existing --dry-run --json
```

## Related commands

- [alert](./alert.md)
- [alert export](./alert-export.md)
- [alert diff](./alert-diff.md)
