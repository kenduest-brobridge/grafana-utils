# `grafana-util alert export`

## Purpose

Export alerting resources into `raw/` JSON files.

## When to use

- Capture alert rules, contact points, mute timings, templates, and policies from Grafana.
- Build a local bundle before review or import.

## Key flags

- `--output-dir` writes the export bundle, defaulting to `alerts`.
- `--flat` writes resource files directly into their resource directories.
- `--overwrite` replaces existing export files.
- Uses the shared connection flags from `grafana-util alert`.

## Examples

```bash
grafana-util alert export --url http://localhost:3000 --basic-user admin --basic-password admin --output-dir ./alerts --overwrite
grafana-util alert export --url http://localhost:3000 --basic-user admin --basic-password admin --output-dir ./alerts --flat
```

## Related commands

- [alert](./alert.md)
- [alert import](./alert-import.md)
- [alert plan](./alert-plan.md)
