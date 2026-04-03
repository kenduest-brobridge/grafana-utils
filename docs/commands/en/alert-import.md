# `grafana-util alert import`

## Purpose

Import alerting resource JSON files through the Grafana API.

## When to use

- Recreate an exported alert bundle in Grafana.
- Update existing alert resources with `--replace-existing`.
- Preview import actions before making changes.

## Key flags

- `--import-dir` points at the `raw/` export directory.
- `--replace-existing` updates resources with matching identities.
- `--dry-run` previews the import.
- `--json` renders dry-run output as structured JSON.
- `--dashboard-uid-map` and `--panel-id-map` repair linked alert rules during import.

## Examples

```bash
grafana-util alert import --url http://localhost:3000 --import-dir ./alerts/raw --replace-existing
grafana-util alert import --url http://localhost:3000 --import-dir ./alerts/raw --replace-existing --dry-run --json
```

## Related commands

- [alert](./alert.md)
- [alert export](./alert-export.md)
- [alert diff](./alert-diff.md)
