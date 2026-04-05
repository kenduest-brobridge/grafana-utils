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

## Before / After

- Before: edit the managed route tree by hand and keep the matcher shape in your head.
- After: write one staged route document that encodes the receiver and matcher set you want.

## What success looks like

- The staged route tree contains the receiver and matcher values you intended.
- A dry-run shows the route document you would write before touching files.
- The route is easy to compare against `preview-route` output.

## Failure checks

- Check that `--desired-dir` points at the correct staged tree before overwriting anything.
- Verify the receiver and matcher labels before trusting the generated route.
- If the dry-run does not match the intended route, stop and correct the matcher set first.

## Examples

```bash
# Purpose: Author or replace the tool-owned staged notification route.
grafana-util alert set-route --desired-dir ./alerts/desired --receiver pagerduty-primary --label team=platform --severity critical
```

```bash
# Purpose: Author or replace the tool-owned staged notification route.
grafana-util alert set-route --desired-dir ./alerts/desired --receiver pagerduty-primary --label team=platform --severity critical --dry-run
```

## Related commands

- [alert](./alert.md)
- [alert preview-route](./alert-preview-route.md)
- [alert add-contact-point](./alert-add-contact-point.md)
