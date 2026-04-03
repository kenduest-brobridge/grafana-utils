# dashboard browse

## Purpose
Open the live dashboard tree in an interactive terminal UI.

## When to use
Use this when you want to explore folders, select a dashboard, or inspect the live tree before fetching, diffing, importing, or deleting.

## Key flags
- `--path`: start at one folder subtree instead of the full tree.
- `--org-id`: browse one explicit Grafana org.
- `--all-orgs`: aggregate browse results across visible orgs. Requires Basic auth.
- Shared live flags: `--url`, `--token`, `--basic-user`, `--basic-password`.

## Examples
```bash
grafana-util dashboard browse --url http://localhost:3000 --basic-user admin --basic-password admin
grafana-util dashboard browse --url http://localhost:3000 --basic-user admin --basic-password admin --path 'Platform / Infra'
```

## Related commands
- [dashboard list](./dashboard-list.md)
- [dashboard get](./dashboard-get.md)
- [dashboard delete](./dashboard-delete.md)
