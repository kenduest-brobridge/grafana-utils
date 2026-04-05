# dashboard governance-gate

## Purpose
Evaluate governance policy against dashboard inspect JSON artifacts.

## When to use
Use this when you already have `governance-json` and query-report artifacts and want a policy pass or fail result before promotion.

## Before / After

- **Before**: teams can export dashboards and inspect them, but policy violations still need a human to spot them one by one.
- **After**: governance-gate turns those artifacts into an explicit pass/fail decision with machine-readable findings for CI or review.

## Key flags
- `--policy-source`: choose `file` or `builtin`.
- `--policy`: policy file path when using file-based policy input.
- `--builtin-policy`: named built-in policy when using builtin policy input.
- `--governance`: path to dashboard inspect governance JSON.
- `--queries`: path to dashboard inspect query-report JSON.
- `--output-format`: render text or JSON.
- `--json-output`: optionally write the normalized result JSON.
- `--interactive`: open the interactive terminal browser over findings.

## Examples
```bash
# Purpose: Evaluate governance policy against dashboard inspect JSON artifacts.
grafana-util dashboard governance-gate --policy-source file --policy ./policy.yaml --governance ./governance.json --queries ./queries.json
```

```bash
# Purpose: Evaluate governance policy against dashboard inspect JSON artifacts.
grafana-util dashboard governance-gate --policy-source builtin --builtin-policy default --governance ./governance.json --queries ./queries.json --output-format json --json-output ./governance-check.json
```

## What success looks like

- policy checks fail before promotion, not after a dashboard lands in the wrong environment
- text output is readable enough for manual review, while JSON output is stable enough for CI gates
- the same artifacts can be rechecked after a policy change without rerunning export or inspect from scratch

## Failure checks

- if the command fails immediately, confirm the policy source and whether the policy file path or builtin policy name is valid
- if the gate result seems incomplete, verify that `governance` and `queries` came from the same inspect run
- if automation reads the result, prefer `--output-format json` and validate the contract before treating a pass/fail as final

## Related commands
- [dashboard inspect-export](./dashboard-inspect-export.md)
- [dashboard inspect-live](./dashboard-inspect-live.md)
- [dashboard topology](./dashboard-topology.md)
