# dashboard governance-gate

## Purpose
Evaluate governance policy against dashboard inspect JSON artifacts.

## When to use
Use this when you already have `governance-json` and query-report artifacts and want a policy pass or fail result before promotion.

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
grafana-util dashboard governance-gate --policy-source file --policy ./policy.yaml --governance ./governance.json --queries ./queries.json
grafana-util dashboard governance-gate --policy-source builtin --builtin-policy default --governance ./governance.json --queries ./queries.json --output-format json --json-output ./governance-check.json
```

## Related commands
- [dashboard inspect-export](./dashboard-inspect-export.md)
- [dashboard inspect-live](./dashboard-inspect-live.md)
- [dashboard topology](./dashboard-topology.md)

