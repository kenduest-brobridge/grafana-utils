# `grafana-util alert add-contact-point`

## Purpose

Author a staged alert contact point from the higher-level authoring surface.

## When to use

- Create a new contact point under a desired-state alert tree.
- Preview the generated file before writing it.

## Key flags

- `--desired-dir` points to the staged alert tree.
- `--name` sets the contact point name.
- `--dry-run` previews the planned output.

## Examples

```bash
grafana-util alert add-contact-point --desired-dir ./alerts/desired --name pagerduty-primary
grafana-util alert add-contact-point --desired-dir ./alerts/desired --name pagerduty-primary --dry-run
```

## Related commands

- [alert](./alert.md)
- [alert set-route](./alert-set-route.md)
- [alert new-contact-point](./alert-new-contact-point.md)
