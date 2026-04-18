# dashboard policy

## Purpose
Evaluate governance policy directly against live Grafana or a local export tree, with saved review artifacts as an advanced reuse path.

## When to use
Use this when you want a policy pass or fail result before promotion. Prefer direct live or local review inputs for the common path; keep `governance-json` and `queries-json` for advanced reuse and CI pipelines.

## Before / After

- **Before**: teams can export dashboards and inspect them, but policy violations still need a human to spot them one by one.
- **After**: policy turns those artifacts into an explicit pass/fail decision with machine-readable findings for CI or review.

## Key flags
- `--policy-source`: choose `file` or `builtin`.
- `--policy`: policy file path when using file-based policy input.
- `--builtin-policy`: named built-in policy when using builtin policy input.
- `--url`: review live Grafana directly.
- `--input-dir`: review a local export tree directly.
- `--input-format`: choose `raw`, `provisioning`, or `git-sync` when reviewing local exports.
- `--governance`: path to dashboard summary governance JSON (`governance-json` artifact, advanced reuse).
- `--queries`: path to dashboard summary query-report JSON (`queries-json` artifact, advanced reuse).
- `--output-format`: render text or JSON.
- `--json-output`: optionally write the normalized result JSON.
- `--interactive`: open the interactive terminal browser over findings.

## Examples
```bash
# Evaluate governance policy against live Grafana directly.
grafana-util dashboard policy --url http://localhost:3000 --basic-user admin --basic-password admin --policy-source file --policy ./policy.yaml
```

```bash
# Evaluate governance policy against a local export tree directly.
grafana-util dashboard policy --input-dir ./dashboards/raw --input-format raw --policy-source builtin --builtin-policy default --output-format json --json-output ./governance-check.json
```

```bash
# Evaluate governance policy against a repo-backed Git Sync dashboard tree.
grafana-util dashboard policy --input-dir ./grafana-oac-repo --input-format git-sync --policy-source builtin --builtin-policy default --output-format json --json-output ./governance-check.json
```

```bash
# Advanced reuse: evaluate governance policy against reusable review artifacts.
grafana-util dashboard policy --policy-source builtin --builtin-policy default --governance ./governance.json --queries ./queries.json --output-format json --json-output ./governance-check.json
```

## What success looks like

- policy checks fail before promotion, not after a dashboard lands in the wrong environment
- text output is readable enough for manual review, while JSON output is stable enough for CI gates
- the same artifacts can be rechecked after a policy workspace without rerunning export or review from scratch

## Failure checks

- if the command fails immediately, confirm the policy source and whether the policy file path or builtin policy name is valid
- if the gate result seems incomplete, verify that `governance` and `queries` came from the same review run
- if automation reads the result, prefer `--output-format json` and validate the contract before treating a pass/fail as final

## Related commands
- [dashboard dependencies](./dashboard-dependencies.md)
- [dashboard summary](./dashboard-summary.md)
- [dashboard dependencies](./dashboard-dependencies.md)
