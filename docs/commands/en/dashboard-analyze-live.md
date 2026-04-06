# dashboard analyze-live

## Purpose
Analyze live Grafana dashboards through the canonical `dashboard analyze` command.

## When to use
Use this when you need the same analysis views as the local export-tree flow, but sourced from live Grafana instead of a local export tree. Prefer `dashboard analyze --url ...` in new docs and scripts.

## Key flags
- `--page-size`: dashboard search page size.
- `--concurrency`: maximum parallel fetch workers.
- `--org-id`: analyze one explicit Grafana org.
- `--all-orgs`: analyze across visible orgs.
- `--output-format`, `--output-file`, `--interactive`, `--no-header`: output controls.
- `--progress`: show fetch progress.

## Examples
```bash
# Purpose: Analyze live Grafana dashboards through the canonical dashboard analyze command.
grafana-util dashboard analyze --url http://localhost:3000 --basic-user admin --basic-password admin --output-format governance
```

```bash
# Purpose: Analyze live Grafana dashboards through the canonical dashboard analyze command.
grafana-util dashboard analyze --url http://localhost:3000 --basic-user admin --basic-password admin --interactive
```

## Related commands
- [dashboard analyze (local)](./dashboard-analyze-export.md)
- [dashboard list-vars](./dashboard-list-vars.md)
- [dashboard governance-gate](./dashboard-governance-gate.md)
