# `grafana-util alert new-template`

## Purpose

Create a low-level staged alert template scaffold.

## When to use

- Seed a new notification template file in a desired-state tree.
- Start from a scaffold before adding template content.

## Key flags

- `--desired-dir` points to the staged alert tree.
- `--name` seeds the scaffold name.

## Examples

```bash
grafana-util alert new-template --desired-dir ./alerts/desired --name sev1-notification
```

## Related commands

- [alert](./alert.md)
- [alert list-templates](./alert-list-templates.md)
