# `grafana-util alert plan`

## Purpose

Build a staged alert management plan from desired alert resources.

## When to use

- Review the changes needed to align Grafana with a desired-state alert tree.
- Prune live-only alert resources from the plan when needed.
- Repair linked rules with dashboard or panel remapping during planning.

## Key flags

- `--desired-dir` points to the staged alert desired-state tree.
- `--prune` marks live-only resources as delete candidates.
- `--dashboard-uid-map` and `--panel-id-map` repair linked alert rules.
- `--output` renders the plan as `text` or `json`.

## Examples

```bash
grafana-util alert plan --desired-dir ./alerts/desired
grafana-util alert plan --desired-dir ./alerts/desired --prune --dashboard-uid-map ./dashboard-map.json --panel-id-map ./panel-map.json --output json
```

## Related commands

- [alert](./alert.md)
- [alert apply](./alert-apply.md)
- [alert delete](./alert-delete.md)
