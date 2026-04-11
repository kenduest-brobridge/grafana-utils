# `grafana-util alert apply`

## Purpose

Apply a reviewed alert management plan.

## When to use

- Execute a plan that was reviewed outside the live Grafana connection.
- Require explicit acknowledgement before touching Grafana.

## Before / After

- **Before**: a reviewed alert plan still leaves one risky question open: what actually happens when someone presses go against live Grafana?
- **After**: `alert apply` turns that final step into an explicit command with approval, reproducible auth, and machine-readable output.

## Key flags

- `--plan-file` points to the reviewed plan document.
- `--approve` is required before execution is allowed.
- `--output-format` renders apply output as `text` or `json`.

## Examples

```bash
# Purpose: Apply a reviewed alert management plan.
grafana-util alert apply --plan-file ./alert-plan-reviewed.json --approve
```

```bash
# Purpose: Apply a reviewed alert management plan.
grafana-util alert apply --url http://localhost:3000 --basic-user admin --basic-password admin --plan-file ./alert-plan-reviewed.json --approve
```

## What success looks like

- a reviewed alert plan can be applied without hand-editing YAML or replaying a sequence of UI clicks
- the live apply step keeps approval explicit instead of hiding it in shell history
- JSON output is stable enough to feed into CI, workspace records, or a post-apply verification step

## Failure checks

- if apply refuses to run, confirm that the plan file is the reviewed artifact and that `--approve` is present
- if the live result differs from the expected plan, re-check credentials, org scope, and whether the reviewed plan matches the current target Grafana
- if automation reads the output, prefer `--output-format json` and validate the result shape before treating the apply as successful

## Related commands

- [alert](./alert.md)
- [alert plan](./alert-plan.md)
- [alert delete](./alert-delete.md)
