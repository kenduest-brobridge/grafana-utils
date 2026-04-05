# datasource types

## Purpose
Show the built-in supported datasource type catalog.

## When to use
Use this when you need to see the canonical datasource type ids that the CLI normalizes and supports for create flows.

## Key flags
- `--output-format`: render the catalog as text, table, csv, json, or yaml.

## Examples
```bash
# Purpose: Show the built-in supported datasource type catalog.
grafana-util datasource types
```

```bash
# Purpose: Show the built-in supported datasource type catalog.
grafana-util datasource types --output-format yaml
```

## Before / After

- **Before**: you had to guess the canonical plugin type ids from UI labels, examples, or stale notes.
- **After**: one catalog lists the built-in datasource type ids the CLI normalizes for create flows.

## What success looks like

- you can pick the right type id before creating or modifying a datasource
- the list is short enough to scan quickly but explicit enough for automation

## Failure checks

- if a type is missing, confirm whether the plugin is actually supported by this repo version
- if the catalog does not match the Grafana you expect, check whether you are looking at an older binary

## Related commands
- [datasource add](./datasource-add.md)
- [datasource modify](./datasource-modify.md)
- [datasource list](./datasource-list.md)
