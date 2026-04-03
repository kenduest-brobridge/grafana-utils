# `grafana-util alert apply`

## Purpose

Apply a reviewed alert management plan.

## When to use

- Execute a plan that was reviewed outside the live Grafana connection.
- Require explicit acknowledgement before touching Grafana.

## Key flags

- `--plan-file` points to the reviewed plan document.
- `--approve` is required before execution is allowed.
- `--output` renders apply output as `text` or `json`.

## Examples

```bash
grafana-util alert apply --plan-file ./alert-plan-reviewed.json --approve
grafana-util alert apply --url http://localhost:3000 --basic-user admin --basic-password admin --plan-file ./alert-plan-reviewed.json --approve
```

## Related commands

- [alert](./alert.md)
- [alert plan](./alert-plan.md)
- [alert delete](./alert-delete.md)
