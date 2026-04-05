# `grafana-util alert clone-rule`

## Purpose

Clone an existing staged alert rule into a new authoring target.

## When to use

- Reuse an existing rule as a starting point for a variant.
- Override folder, rule group, receiver, or route behavior while cloning.

## Key flags

- `--desired-dir` points to the staged alert tree.
- `--source` identifies the rule to clone.
- `--name` sets the new rule name.
- `--folder`, `--rule-group`, `--receiver`, and `--no-route` adjust the clone target.
- `--dry-run` previews the cloned output.

## Examples

```bash
# Purpose: Clone an existing staged alert rule into a new authoring target.
grafana-util alert clone-rule --desired-dir ./alerts/desired --source cpu-high --name cpu-high-staging --folder staging-alerts --rule-group cpu --receiver slack-platform
```

```bash
# Purpose: Clone an existing staged alert rule into a new authoring target.
grafana-util alert clone-rule --desired-dir ./alerts/desired --source cpu-high --name cpu-high-staging --dry-run
```

## Related commands

- [alert](./alert.md)
- [alert add-rule](./alert-add-rule.md)
- [alert new-rule](./alert-new-rule.md)
