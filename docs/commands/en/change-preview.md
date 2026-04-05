# `grafana-util change preview`

## Purpose

Preview what would change from discovered or explicit staged inputs.

## When to use

- Use this when you want the actionable review artifact for the current staged package.
- This is the task-first replacement for the common `plan` step.
- Choose this when the operator question is “what will change?” rather than “which lower-level builder should I run?”

## Before / After

- **Before**: you know the package exists and passes basic checks, but the actual creates, updates, and deletes are still implicit.
- **After**: you have one staged preview contract that can move into review and eventually into apply.

## Key flags

- `--workspace`: auto-discover the staged package from common repo-local inputs.
- `--desired-file`: preview one explicit desired change file.
- `--source-bundle`, `--target-inventory`, `--mapping-file`, `--availability-file`: switch into bundle or promotion-aware preview paths.
- `--live-file`: compare against one saved live-state document.
- `--fetch-live`: query live Grafana instead of relying on `--live-file`.
- `--allow-prune`: allow delete operations to appear in the preview.
- `--trace-id`: stamp the preview with explicit review lineage.
- `--output-format`, `--output-file`: render or persist the preview artifact.

## Examples

```bash
# Purpose: Preview the current staged package against live Grafana.
grafana-util change preview --workspace . --fetch-live --profile prod
```

**Expected Output:**
```text
SYNC PLAN:
- create: 1
- update: 4
- delete: 0
- blocked alerts: 0
```

```bash
# Purpose: Preview one explicit desired/live pair as JSON.
grafana-util change preview --desired-file ./desired.json --live-file ./live.json --output-format json
```

**Expected Output:**
```json
{
  "kind": "grafana-utils-sync-plan",
  "reviewed": false,
  "summary": {
    "would_create": 1,
    "would_update": 4,
    "would_delete": 0
  },
  "operations": []
}
```

This is the normal task-first preview contract. `reviewed: false` means the preview exists, but it has not been approved for apply yet.

## What success looks like

- the summary counts match the operator’s expectation of creates, updates, and deletes
- the preview can be handed to another person as the review artifact
- the output is explicit enough to compare against later apply behavior

## Failure checks

- if preview fails from `--workspace`, retry once with explicit staged input flags before assuming the package is bad
- if live-backed output looks different from expectation, verify auth, org scope, and the target Grafana before approving it
- if preview emits bundle or promotion preflight kinds instead of a sync plan, confirm which staged inputs you provided

## Related commands

- [change](./change.md)
- [change check](./change-check.md)
- [change apply](./change-apply.md)
- [change advanced](./change.md#advanced)
