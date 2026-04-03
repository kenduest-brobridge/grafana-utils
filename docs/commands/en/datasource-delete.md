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
grafana-util datasource delete --url http://localhost:3000 --basic-user admin --basic-password admin --uid prom-main --dry-run --json
grafana-util datasource delete --profile prod --uid prom-main --yes
```

## Related commands
- [datasource browse](./datasource-browse.md)
- [datasource modify](./datasource-modify.md)
- [datasource list](./datasource-list.md)
