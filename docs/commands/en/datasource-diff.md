# datasource diff

## Purpose
Compare datasource inventory from a local bundle against live Grafana and print an operator-summary diff report.

## When to use
Use this when you want a concise live-versus-local difference report before import.

## Key flags
- `--diff-dir`: local datasource bundle to compare.
- `--input-format`: choose `inventory` or `provisioning`.

## Examples
```bash
# Purpose: Compare datasource inventory from a local bundle against live Grafana and print an operator-summary diff report.
grafana-util datasource diff --url http://localhost:3000 --basic-user admin --basic-password admin --diff-dir ./datasources --input-format inventory
```

```bash
# Purpose: Compare datasource inventory from a local bundle against live Grafana and print an operator-summary diff report.
grafana-util datasource diff --profile prod --diff-dir ./datasources/provisioning --input-format provisioning
```

## Before / After

- **Before**: you had to inspect local and live datasource JSON by hand to find drift.
- **After**: one diff command shows what changed between the bundle and Grafana before you import anything.

## What success looks like

- you can explain the change set before import
- inventory and provisioning inputs both produce a readable summary
- the output makes it obvious whether the bundle or the live side changed

## Failure checks

- if the diff is unexpectedly empty, verify the bundle path and `--input-format`
- if the live side looks wrong, confirm the target Grafana and org scope before trusting the report
- if the diff is noisy, make sure you are comparing the intended inventory bundle rather than an older provisioning tree

## Related commands
- [datasource list](./datasource-list.md)
- [datasource export](./datasource-export.md)
- [datasource import](./datasource-import.md)
