# `grafana-util alert preview-route`

## Purpose

Preview the managed route inputs without changing runtime behavior.

## When to use

- Inspect the matcher set you intend to feed into `set-route`.
- Validate route inputs before writing the managed route document.

## Key flags

- `--desired-dir` points to the staged alert tree.
- `--label` adds preview matchers in `key=value` form.
- `--severity` adds a convenience severity matcher value.

## Before / After

- Before: guess whether a matcher set will behave the way you expect when it reaches `set-route`.
- After: preview the route inputs first so the intended receiver and labels are visible before writing files.

## What success looks like

- The preview output matches the matcher set you plan to hand to `set-route`.
- The route shape is clear enough to review without editing the staged tree first.

## Failure checks

- Check the labels and severity value if the preview does not match your intended route.
- Make sure `--desired-dir` points at the staged tree you expect.

## Examples

```bash
# Purpose: Preview the managed route inputs without changing runtime behavior.
grafana-util alert preview-route --desired-dir ./alerts/desired --label team=platform --severity critical
```

## Related commands

- [alert](./alert.md)
- [alert set-route](./alert-set-route.md)
