# `grafana-util alert new-contact-point`

## Purpose

Create a low-level staged alert contact point scaffold.

## When to use

- Seed a new contact point file in a desired-state tree.
- Start from a scaffold before filling in receiver details.

## Key flags

- `--desired-dir` points to the staged alert tree.
- `--name` seeds the scaffold name.

## Examples

```bash
grafana-util alert new-contact-point --desired-dir ./alerts/desired --name pagerduty-primary
grafana-util alert add-contact-point --desired-dir ./alerts/desired --name pagerduty-primary
```

## Related commands

- [alert](./alert.md)
- [alert add-contact-point](./alert-add-contact-point.md)
- [alert set-route](./alert-set-route.md)
