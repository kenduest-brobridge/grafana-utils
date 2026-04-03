# datasource add

## Purpose
Create one live Grafana datasource through the Grafana API.

## When to use
Use this when you want to create a new datasource directly, or dry-run the create step before applying it.

## Key flags
- `--uid`: stable datasource identity.
- `--name`: datasource name.
- `--type`: Grafana datasource plugin type id.
- `--datasource-url`: datasource target URL.
- `--access`: proxy or direct access mode.
- `--default`: mark as the default datasource.
- `--preset-profile` and `--apply-supported-defaults`: scaffold supported defaults.
- `--json-data`, `--secure-json-data`, `--secure-json-data-placeholders`, `--secret-values`: configure custom fields and secrets.
- `--dry-run`, `--table`, `--json`, `--output-format`, `--no-header`: preview output controls.

## Examples
```bash
grafana-util datasource add --url http://localhost:3000 --basic-user admin --basic-password admin --name prometheus-main --type prometheus --datasource-url http://prometheus:9090 --dry-run --table
grafana-util datasource add --profile prod --name tempo-main --type tempo --datasource-url http://tempo:3200 --preset-profile full --dry-run --json
```

## Related commands
- [datasource types](./datasource-types.md)
- [datasource modify](./datasource-modify.md)
- [datasource list](./datasource-list.md)
