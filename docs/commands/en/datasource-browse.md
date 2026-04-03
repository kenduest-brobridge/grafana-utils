# datasource browse

## Purpose
Open a live datasource browser against Grafana with in-place modify and delete actions.

## When to use
Use this when you want an interactive inventory view for inspecting, editing, or deleting live datasources.

## Key flags
- `--org-id`: browse one explicit Grafana org.
- `--all-orgs`: aggregate datasource browsing across visible orgs. Requires Basic auth.
- Shared live flags: `--url`, `--token`, `--basic-user`, `--basic-password`.

## Examples
```bash
grafana-util datasource browse --url http://localhost:3000 --basic-user admin --basic-password admin
grafana-util datasource browse --url http://localhost:3000 --basic-user admin --basic-password admin --org-id 2
```

## Related commands
- [datasource list](./datasource-list.md)
- [datasource modify](./datasource-modify.md)
- [datasource delete](./datasource-delete.md)
