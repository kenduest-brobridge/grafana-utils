# datasource delete

## Purpose
Delete one live Grafana datasource through the Grafana API.

## When to use
Use this when a datasource should be removed by UID or by name, either as a dry run or as an acknowledged live delete.

## Key flags
- `--uid`: datasource UID to delete.
- `--name`: datasource name to delete when UID is not available.
- `--yes`: acknowledge the live delete.
- `--dry-run`, `--table`, `--json`, `--output-format`, `--no-header`: preview output controls.

## Examples
```bash
# Purpose: Delete one live Grafana datasource through the Grafana API.
grafana-util datasource delete --url http://localhost:3000 --basic-user admin --basic-password admin --uid prom-main --dry-run --json
```

```bash
# Purpose: Delete one live Grafana datasource through the Grafana API.
grafana-util datasource delete --profile prod --uid prom-main --yes
```

## Before / After

- **Before**: deleting a datasource often depended on remembering the exact row in the UI or typing a name from memory.
- **After**: one command can target a UID or name, show a dry run, and make the final live delete explicit.

## What success looks like

- the target datasource is obvious before any live deletion
- `--dry-run` makes the action reviewable
- `--yes` is only used when the operator intentionally wants the live delete

## Failure checks

- if the command points at the wrong datasource, verify the UID or name before retrying
- if the dry-run is empty, check whether the org scope actually exposes the target datasource
- if the live delete is rejected, confirm whether the account has delete permission for that org

## Related commands
- [datasource browse](./datasource-browse.md)
- [datasource modify](./datasource-modify.md)
- [datasource list](./datasource-list.md)
