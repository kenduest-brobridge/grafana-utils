# `grafana-util alert new-rule`

## Purpose

Create a low-level staged alert rule scaffold.

## When to use

- Seed a new rule file in a desired-state tree.
- Start from a simple scaffold before filling in rule details.

## Key flags

- `--desired-dir` points to the staged alert tree.
- `--name` seeds the scaffold name.

## Examples

```bash
grafana-util alert new-rule --desired-dir ./alerts/desired --name cpu-main
grafana-util alert add-rule --desired-dir ./alerts/desired --name cpu-main --folder platform-alerts --rule-group cpu --receiver pagerduty-primary
```

## Related commands

- [alert](./alert.md)
- [alert add-rule](./alert-add-rule.md)
- [alert clone-rule](./alert-clone-rule.md)
