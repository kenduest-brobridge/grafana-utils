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
# Purpose: Open a live datasource browser against Grafana with in-place modify and delete actions.
grafana-util datasource browse --url http://localhost:3000 --basic-user admin --basic-password admin
```

```bash
# Purpose: Open a live datasource browser against Grafana with in-place modify and delete actions.
grafana-util datasource browse --url http://localhost:3000 --basic-user admin --basic-password admin --org-id 2
```

## Before / After

- **Before**: you had to bounce between inventory pages, edit dialogs, and delete prompts to inspect a datasource.
- **After**: one browser view keeps the live inventory in front of you and puts edit/delete actions next to the rows you are reviewing.

## What success looks like

- you can inspect the live list without losing context
- edit and delete actions stay close to the rows they affect
- org-scoped browsing is obvious before you change anything

## Failure checks

- if the browser opens with missing rows, verify the org scope and the credentials used for the view
- if edit or delete actions are missing, confirm that the account actually has permission to mutate datasources
- if the org switch looks wrong, check whether `--all-orgs` or `--org-id` is being used intentionally

## Related commands
- [datasource list](./datasource-list.md)
- [datasource modify](./datasource-modify.md)
- [datasource delete](./datasource-delete.md)
