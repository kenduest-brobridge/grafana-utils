# datasource inspect-export

## Purpose
Inspect a local masked recovery bundle without connecting to Grafana.

## When to use
Use this when you want to read datasource export artifacts from disk and review them with text, table, CSV, JSON, YAML, or interactive output.

## Key flags
- `--input-dir`: local directory containing the export artifacts.
- `--input-type`: select inventory or provisioning when the path could be interpreted either way.
- `--interactive`: open the local export inspection workbench.
- `--table`, `--csv`, `--text`, `--json`, `--yaml`, `--output-format`: output mode controls.

## Examples
```bash
grafana-util datasource inspect-export --input-dir ./datasources --table
grafana-util datasource inspect-export --input-dir ./datasources --json
```

## Related commands
- [datasource export](./datasource-export.md)
- [datasource import](./datasource-import.md)
- [datasource diff](./datasource-diff.md)

