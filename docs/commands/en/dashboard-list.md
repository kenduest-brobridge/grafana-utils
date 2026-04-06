# dashboard list

## Purpose
List dashboard summaries without writing export files.

## When to use
Use this when you want a non-interactive inventory view of live dashboards, optionally with resolved sources or structured output.

## Key flags
- `--page-size`: dashboard search page size.
- `--org-id`: list one explicit Grafana org.
- `--all-orgs`: aggregate results across visible orgs. Prefer Basic auth.
- `--show-sources`: include resolved datasource names in the list output. `--with-sources` remains accepted as a compatibility alias.
- `--output-columns`: choose the displayed columns. Selecting `sources` or `source_uids` also resolves datasource names.
- `--output-format`, `--json`, `--yaml`, `--csv`, `--table`, `--text`: output mode controls.
- `--no-header`: suppress table headers.

## Notes
- Use `--profile` for repeatable single-org inventory.
- For `--all-orgs`, prefer admin-backed `--profile` or direct Basic auth because token scope can hide other orgs.

## Examples
```bash
# Purpose: List dashboard summaries without writing export files.
grafana-util dashboard list --profile prod
```

```bash
# Purpose: List dashboard summaries without writing export files.
grafana-util dashboard list --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --json
```

```bash
# Purpose: List dashboard summaries without writing export files.
grafana-util dashboard list --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --json
```

```bash
# Purpose: List dashboard summaries and include resolved source names in a table.
grafana-util dashboard list --url http://localhost:3000 --basic-user admin --basic-password admin --show-sources --table
```

## Related commands
- [dashboard browse](./dashboard-browse.md)
- [dashboard export](./dashboard-export.md)
- [dashboard diff](./dashboard-diff.md)
