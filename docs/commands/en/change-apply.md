# `grafana-util change apply`

## Purpose

Turn a reviewed preview into a staged apply intent, or execute it live when you explicitly opt in.

## When to use

- Use this after preview and review are already complete.
- Choose the default staged form when you need approval evidence or a machine-readable apply intent.
- Add `--execute-live` only when you are ready to mutate Grafana for real.
- `--preview-file` is the preferred input, while `--plan-file` still works as a compatibility alias for older staged workflows.

## Before / After

- **Before**: preview exists, but the last mile between review and mutation is still a risky operator step.
- **After**: apply becomes either a clear staged intent artifact or a clear live execution result with approval attached.

## Key flags

- `--preview-file`: the reviewed preview artifact to apply.
- `--plan-file`: compatibility alias for older plan-based staged workflows.
- `--approve`: explicit acknowledgement that this step may proceed.
- `--execute-live`: switch from staged intent generation into real live execution.
- `--approval-reason`, `--apply-note`: carry human approval context into the output artifact.
- `--output-format`: render as `text` or `json`.

## Examples

```bash
# Purpose: Build a staged apply intent from a reviewed preview.
grafana-util change apply --preview-file ./change-preview.json --approve --output-format json
```

**Expected Output:**
```json
{
  "kind": "grafana-utils-sync-apply-intent",
  "approved": true,
  "reviewed": true,
  "operations": []
}
```

This confirms that apply built a staged intent document rather than executing live changes.

```bash
# Purpose: Execute the approved preview against live Grafana.
grafana-util change apply --preview-file ./change-preview.json --approve --execute-live --profile prod
```

**Expected Output:**
```text
SYNC APPLY:
- mode: live
- applied: 5
- failed: 0
```

## What success looks like

- reviewed lineage is preserved into the apply stage instead of being lost at the last step
- staged apply JSON is explicit enough for approval workflows, tickets, or handoff notes
- live apply output makes it obvious how many operations ran and whether anything failed

## Failure checks

- if apply refuses to proceed, confirm that the input preview is the reviewed artifact and that `--approve` is present
- if the staged intent looks right but live execution differs, compare the preview, optional preflight artifacts, and target environment before retrying
- if automation reads the output, distinguish staged `grafana-utils-sync-apply-intent` from live apply output before parsing fields

## Related commands

- [change](./change.md)
- [change preview](./change-preview.md)
- [change review](./change.md#review)
- [change preflight](./change.md#preflight)
- [status](./status.md)
