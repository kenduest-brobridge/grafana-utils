# `grafana-util alert set-route`

## Purpose

Author or replace the tool-owned staged notification route.

## When to use

- Replace the managed route with a new receiver and matcher set.
- Re-run the command to fully replace the managed route instead of merging fields.

## Key flags

- `--desired-dir` points to the staged alert tree.
- `--receiver` sets the route receiver.
- `--label` adds route matchers in `key=value` form.
- `--severity` adds a convenience severity matcher.
- `--dry-run` renders the managed route document without writing files.

## Examples

```bash
grafana-util alert set-route --desired-dir ./alerts/desired --receiver pagerduty-primary --label team=platform --severity critical
grafana-util alert set-route --desired-dir ./alerts/desired --receiver pagerduty-primary --label team=platform --severity critical --dry-run
```

## Related commands

- [alert](./alert.md)
- [alert preview-route](./alert-preview-route.md)
- [alert add-contact-point](./alert-add-contact-point.md)
