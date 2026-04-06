# dashboard diff

## Purpose
Compare local dashboard files against live Grafana dashboards.

## When to use
Use this when you want to see what would change before importing or publishing a dashboard bundle.

## Key flags
- `--input-dir`: compare this export directory against Grafana.
- `--input-format`: choose `raw` or `provisioning`.
- `--import-folder-uid`: override the destination folder UID for the comparison.
- `--context-lines`: unified diff context.

## Examples
```bash
# Purpose: Compare local dashboard files against live Grafana dashboards.
grafana-util dashboard diff --url http://localhost:3000 --basic-user admin --basic-password admin --input-dir ./dashboards/raw
```

```bash
# Purpose: Compare local dashboard files against live Grafana dashboards.
grafana-util dashboard diff --url http://localhost:3000 --basic-user admin --basic-password admin --org-id 2 --input-dir ./dashboards/raw --json
```

## Related commands
- [dashboard export](./dashboard-export.md)
- [dashboard import](./dashboard-import.md)
- [dashboard analyze (local)](./dashboard-analyze-export.md)
