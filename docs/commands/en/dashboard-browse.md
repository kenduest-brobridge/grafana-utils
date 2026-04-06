# dashboard browse

## Purpose
Open the live dashboard tree or a local export tree in an interactive terminal UI.

## When to use
Use this when you want to explore folders, select a dashboard, inspect the live tree before fetching, diffing, importing, or deleting, or review a local export tree without calling Grafana.

## Key flags
- `--path`: start at one folder subtree instead of the full tree.
- `--input-dir`: browse a local raw export root, all-orgs export root, or provisioning tree.
- `--input-format`: interpret the local export tree as `raw` or `provisioning`.
- `--org-id`: browse one explicit Grafana org.
- `--all-orgs`: aggregate browse results across visible orgs. Requires Basic auth.
- Shared live flags: `--url`, `--token`, `--basic-user`, `--basic-password`.

## Examples
```bash
# Purpose: Open the live dashboard tree in an interactive terminal UI.
grafana-util dashboard browse --url http://localhost:3000 --basic-user admin --basic-password admin
```

```bash
# Purpose: Open the live dashboard tree in an interactive terminal UI.
grafana-util dashboard browse --url http://localhost:3000 --basic-user admin --basic-password admin --path 'Platform / Infra'
```

```bash
# Purpose: Open a local raw export tree in an interactive terminal UI.
grafana-util dashboard browse --input-dir ./dashboards/raw --path 'Platform / Infra'
```

## Related commands
- [dashboard list](./dashboard-list.md)
- [dashboard fetch-live](./dashboard-fetch-live.md)
- [dashboard delete](./dashboard-delete.md)
