# `grafana-util alert diff`

## Purpose

Compare local alerting export files against live Grafana resources.

## When to use

- Check a raw export directory against Grafana before import or plan work.
- Render the diff as plain text or structured JSON.

## Key flags

- `--diff-dir` points to the raw export directory.
- `--json` renders the diff as structured JSON.
- `--dashboard-uid-map` and `--panel-id-map` repair linked alert rules during comparison.

## Examples

```bash
grafana-util alert diff --url http://localhost:3000 --diff-dir ./alerts/raw
grafana-util alert diff --url http://localhost:3000 --diff-dir ./alerts/raw --json
```

## Related commands

- [alert](./alert.md)
- [alert export](./alert-export.md)
- [alert import](./alert-import.md)
