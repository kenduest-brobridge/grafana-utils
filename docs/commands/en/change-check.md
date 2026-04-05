# `grafana-util change check`

## Purpose

Check whether the staged change package looks structurally safe to continue.

## When to use

- Run this after `change inspect` when you need one readiness gate before preview.
- Use this in CI when you want a fast staged readiness decision without building a full plan yet.
- Prefer this over `status staged` when you want to stay in the task-first `change` lane.

## Before / After

- **Before**: you know what the package contains, but not whether the staged inputs are coherent enough to continue.
- **After**: you have a readiness-style result with blockers and warnings that can stop the workflow early.

## Key flags

- `--workspace`: auto-discover the staged package from common repo-local inputs.
- `--availability-file`: merge staged availability hints into the check.
- `--target-inventory`, `--mapping-file`: deepen bundle or promotion-oriented checks when those artifacts exist.
- `--fetch-live`: merge live target checks into the staged readiness decision.
- `--output-format`: render as `text` or `json`.

## Examples

```bash
# Purpose: Check the discovered staged package.
grafana-util change check --workspace . --output-format json
```

**Expected Output:**
```json
{
  "status": "ready",
  "blockers": [],
  "warnings": []
}
```

```bash
# Purpose: Check the staged package with live and availability context.
grafana-util change check --workspace . --fetch-live --availability-file ./availability.json
```

**Expected Output:**
```text
PREFLIGHT CHECK:
- dashboards: valid
- datasources: valid
- result: 0 blockers
```

## What success looks like

- the result clearly distinguishes hard blockers from softer warnings
- another operator or CI job can stop safely without reverse-engineering the inputs
- live-backed checks line up with the target environment you meant to inspect

## Failure checks

- if blockers appear unexpectedly, verify that staged files and availability hints come from the same environment
- if a live-backed check looks wrong, re-check credentials, org scope, and target URL before trusting the result
- if automation reads the JSON, inspect `status`, `blockers`, and `warnings` only after validating the result shape

## Related commands

- [change](./change.md)
- [change inspect](./change-inspect.md)
- [change preview](./change-preview.md)
- [status](./status.md)
