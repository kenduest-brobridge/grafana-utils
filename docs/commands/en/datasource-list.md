# datasource list

## Purpose
List live Grafana datasource inventory.

## When to use
Use this when you need a non-interactive inventory of datasources, either for the current org, one explicit org, or across all visible orgs.

## Key flags
- `--org-id`: list one explicit Grafana org.
- `--all-orgs`: aggregate datasource inventory across visible orgs. Requires Basic auth.
- `--output-format`, `--text`, `--table`, `--csv`, `--json`, `--yaml`: output mode controls.
- `--output-columns`: choose the displayed columns.
- `--no-header`: suppress table headers.

## Examples
```bash
grafana-util datasource list --url http://localhost:3000 --basic-user admin --basic-password admin --output-format text
grafana-util datasource list --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --output-format yaml
```

## Related commands
- [datasource browse](./datasource-browse.md)
- [datasource export](./datasource-export.md)
- [datasource diff](./datasource-diff.md)
