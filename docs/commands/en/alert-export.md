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

## Before / After

- Before: capture alert rules, contact points, mute timings, templates, and policies by hand from Grafana.
- After: export one reproducible `raw/` bundle that you can review, diff, or feed into import.

## What success looks like

- The output directory contains the expected `raw/` resource files.
- The bundle covers the resource kinds you expected for that org or profile.
- You can hand the bundle to `alert diff` or `alert import` without editing its shape first.

## Failure checks

- Check that the connection flags point at the right Grafana instance and org scope.
- Use `--overwrite` only when you really intend to replace the existing export tree.
- If the bundle looks incomplete, verify token scope or switch to a profile with broader access.

## Examples

```bash
# Purpose: Export alerting resources into `raw/` JSON files.
grafana-util alert export --url http://localhost:3000 --basic-user admin --basic-password admin --output-dir ./alerts --overwrite
```

```bash
# Purpose: Export alerting resources into `raw/` JSON files.
grafana-util alert export --url http://localhost:3000 --basic-user admin --basic-password admin --output-dir ./alerts --flat
```

## Related commands

- [alert](./alert.md)
- [alert import](./alert-import.md)
- [alert plan](./alert-plan.md)
