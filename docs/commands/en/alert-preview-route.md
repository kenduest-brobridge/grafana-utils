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

## Examples

```bash
grafana-util alert preview-route --desired-dir ./alerts/desired --label team=platform --severity critical
```

## Related commands

- [alert](./alert.md)
- [alert set-route](./alert-set-route.md)
