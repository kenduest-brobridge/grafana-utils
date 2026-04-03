# `grafana-util alert add-rule`

## Purpose

Author a staged alert rule from the higher-level authoring surface.

## When to use

- Create a new rule under a desired-state alert tree.
- Attach labels, annotations, severity, and threshold logic in one command.
- Generate a route for the rule unless you explicitly skip it.

## Key flags

- `--desired-dir` points to the staged alert tree.
- `--name`, `--folder`, and `--rule-group` define the rule placement.
- `--receiver` or `--no-route` controls route authoring.
- `--label`, `--annotation`, `--severity`, `--for`, `--expr`, `--threshold`, `--above`, and `--below` shape the rule.
- `--dry-run` previews the planned file output.

## Examples

```bash
grafana-util alert add-rule --desired-dir ./alerts/desired --name cpu-high --folder platform-alerts --rule-group cpu --receiver pagerduty-primary --severity critical --expr 'A' --threshold 80 --above --for 5m --label team=platform --annotation summary='CPU high'
grafana-util alert add-rule --desired-dir ./alerts/desired --name cpu-high --folder platform-alerts --rule-group cpu --receiver pagerduty-primary --dry-run
```

## Related commands

- [alert](./alert.md)
- [alert clone-rule](./alert-clone-rule.md)
- [alert new-rule](./alert-new-rule.md)
