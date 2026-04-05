# `grafana-util alert plan`

## Purpose

Build a staged alert management plan from desired alert resources.

## When to use

- Review the changes needed to align Grafana with a desired-state alert tree.
- Prune live-only alert resources from the plan when needed.
- Repair linked rules with dashboard or panel remapping during planning.

## Before / After

- **Before**: alert changes are hard to reason about until you attempt apply.
- **After**: one plan shows create/update/delete intent and linked-rule repair choices before live mutation.

## Key flags

- `--desired-dir` points to the staged alert desired-state tree.
- `--prune` marks live-only resources as delete candidates.
- `--dashboard-uid-map` and `--panel-id-map` repair linked alert rules.
- `--output-format` renders the plan as `text` or `json`.

## What success looks like

- you can review alert changes before apply
- linked dashboard or panel remapping is visible in the plan step
- delete candidates are explicit instead of implicit

## Failure checks

- if the plan is missing expected rules, check the desired tree first
- if linked rules still look broken, verify the dashboard and panel mapping files
- if prune feels too destructive, remove `--prune` and compare again before apply

## Examples

```bash
# Purpose: Build a staged alert management plan from desired alert resources.
grafana-util alert plan --desired-dir ./alerts/desired
```

```bash
# Purpose: Build a staged alert management plan from desired alert resources.
grafana-util alert plan --desired-dir ./alerts/desired --prune --dashboard-uid-map ./dashboard-map.json --panel-id-map ./panel-map.json --output-format json
```

## Related commands

- [alert](./alert.md)
- [alert apply](./alert-apply.md)
- [alert delete](./alert-delete.md)
