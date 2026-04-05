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
# Purpose: Inspect a local masked recovery bundle without connecting to Grafana.
grafana-util datasource inspect-export --input-dir ./datasources --table
```

```bash
# Purpose: Inspect a local masked recovery bundle without connecting to Grafana.
grafana-util datasource inspect-export --input-dir ./datasources --json
```

## Before / After

- **Before**: reading a datasource bundle meant opening raw files and guessing which part belonged to inventory or provisioning.
- **After**: one local inspection command can mask secrets and show the bundle in text, table, CSV, JSON, YAML, or interactive form.

## What success looks like

- you can review a local export bundle without touching Grafana
- masked secrets stay masked while the structure stays readable
- the output format is good enough for both manual review and scripts

## Failure checks

- if the bundle does not open, confirm the input directory and whether it is inventory or provisioning
- if the masked fields look wrong, verify whether the export source actually contains the data you expect
- if interactive mode is unavailable, fall back to a text or JSON output first

## Related commands
- [datasource export](./datasource-export.md)
- [datasource import](./datasource-import.md)
- [datasource diff](./datasource-diff.md)
