# `grafana-util alert add-rule`

## Purpose

Author a staged alert rule from the higher-level authoring surface.

## When to use

- Create a new rule under a desired-state alert tree.
- Attach labels, annotations, severity, and threshold logic in one command.
- Generate a route for the rule unless you explicitly skip it.

## Before / After

- **Before**: adding a rule often means hand-assembling YAML, then separately fixing routing or metadata gaps.
- **After**: one CLI step can create the rule, wire its route, and leave a staged artifact that is easier to review later.

## What success looks like

- the rule name, folder, rule group, thresholds, and route are created together
- the staged alert tree is readable enough for another operator to review
- `--dry-run` shows the shape of the files before you commit them

## Failure checks

- if rule creation fails, confirm that `--desired-dir` points at the correct staged alert tree
- if the route is missing, check whether `--no-route` was used or whether `--receiver` was omitted
- if you plan to hand the output to a later apply step, inspect the `--dry-run` result or the generated files first

## Key flags

- `--desired-dir` points to the staged alert tree.
- `--name`, `--folder`, and `--rule-group` define the rule placement.
- `--receiver` or `--no-route` controls route authoring.
- `--label`, `--annotation`, `--severity`, `--for`, `--expr`, `--threshold`, `--above`, and `--below` shape the rule.
- `--dry-run` previews the planned file output.

## Examples

```bash
# Purpose: Author a staged alert rule from the higher-level authoring surface.
grafana-util alert add-rule --desired-dir ./alerts/desired --name cpu-high --folder platform-alerts --rule-group cpu --receiver pagerduty-primary --severity critical --expr 'A' --threshold 80 --above --for 5m --label team=platform --annotation summary='CPU high'
```

```bash
# Purpose: Author a staged alert rule from the higher-level authoring surface.
grafana-util alert add-rule --desired-dir ./alerts/desired --name cpu-high --folder platform-alerts --rule-group cpu --receiver pagerduty-primary --dry-run
```

## Related commands

- [alert](./alert.md)
- [alert clone-rule](./alert-clone-rule.md)
- [alert new-rule](./alert-new-rule.md)
