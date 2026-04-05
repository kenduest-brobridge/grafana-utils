# dashboard inspect-live

## Purpose
Analyze live Grafana dashboards via a temporary raw-export snapshot.

## When to use
Use this when you need the same inspection views as `inspect-export`, but sourced from live Grafana instead of a local export tree.

## Key flags
- `--page-size`: dashboard search page size.
- `--concurrency`: maximum parallel fetch workers.
- `--org-id`: inspect one explicit Grafana org.
- `--all-orgs`: inspect across visible orgs.
- `--report`, `--output-format`, `--output-file`, `--interactive`, `--no-header`: output controls.
- `--progress`: show fetch progress.

## Examples
```bash
# Purpose: Analyze live Grafana dashboards via a temporary raw-export snapshot.
grafana-util dashboard inspect-live --url http://localhost:3000 --basic-user admin --basic-password admin --output-format governance-json
```

```bash
# Purpose: Analyze live Grafana dashboards via a temporary raw-export snapshot.
grafana-util dashboard inspect-live --url http://localhost:3000 --basic-user admin --basic-password admin --interactive
```

## Related commands
- [dashboard inspect-export](./dashboard-inspect-export.md)
- [dashboard inspect-vars](./dashboard-inspect-vars.md)
- [dashboard governance-gate](./dashboard-governance-gate.md)
