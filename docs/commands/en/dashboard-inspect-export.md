# dashboard inspect-export

## Purpose
Analyze dashboard export directories with operator-summary and report-contract views.

## When to use
Use this when you want to read a local export tree, inspect its structure, or render governance and dependency reports without contacting Grafana.

## Key flags
- `--import-dir`: dashboard export root to inspect.
- `--input-format`: choose `raw` or `provisioning`.
- `--input-type`: select `raw` or `source` when the export root has multiple dashboard variants.
- `--report`: render table, csv, json, tree, tree-table, dependency, dependency-json, governance, or governance-json views.
- `--output-format`: single-flag output selector.
- `--interactive`: open the shared inspect workbench.
- `--output-file`: write the result to disk.
- `--no-header`: suppress table-like headers.

## Examples
```bash
grafana-util dashboard inspect-export --import-dir ./dashboards/raw --input-format raw --table
grafana-util dashboard inspect-export --import-dir ./dashboards/provisioning --input-format provisioning --report governance-json
```

## Related commands
- [dashboard export](./dashboard-export.md)
- [dashboard diff](./dashboard-diff.md)
- [dashboard governance-gate](./dashboard-governance-gate.md)

