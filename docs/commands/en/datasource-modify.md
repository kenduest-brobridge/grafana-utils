# datasource modify

## Purpose
Modify one live Grafana datasource through the Grafana API.

## When to use
Use this when a datasource already exists and you need to update its URL, auth, JSON payload, or other live settings.

## Key flags
- `--uid`: datasource UID to modify.
- `--set-url`: replace the datasource URL.
- `--set-access`: replace the datasource access mode.
- `--set-default`: set or clear the default datasource flag.
- `--basic-auth`, `--basic-auth-user`, `--basic-auth-password`: update basic auth settings.
- `--user`, `--password`, `--with-credentials`, `--http-header`: update supported request settings.
- `--tls-skip-verify`, `--server-name`: update TLS-related settings.
- `--json-data`, `--secure-json-data`, `--secure-json-data-placeholders`, `--secret-values`: update structured fields and secrets.
- `--dry-run`, `--table`, `--json`, `--output-format`, `--no-header`: preview output controls.

## Examples
```bash
# Purpose: Modify one live Grafana datasource through the Grafana API.
grafana-util datasource modify --url http://localhost:3000 --basic-user admin --basic-password admin --uid prom-main --set-url http://prometheus-v2:9090 --dry-run --json
```

```bash
# Purpose: Modify one live Grafana datasource through the Grafana API.
grafana-util datasource modify --profile prod --uid prom-main --set-default true --dry-run --table
```

## Before / After

- **Before**: updating a live datasource usually meant editing a blob of JSON or clicking through multiple UI tabs.
- **After**: one command can preview the exact live update and keep the workspace reviewable before it lands.

## What success looks like

- the UID identifies the intended datasource before the mutation starts
- `--dry-run` shows the new URL, auth, or JSON fields you expect
- default flags and secret updates are visible before the live workspace

## Failure checks

- if the preview touches the wrong datasource, confirm the UID before retrying
- if auth or TLS changes are incomplete, compare the previewed payload against the current live settings first
- if a secret field does not resolve, check the placeholder mapping or the chosen profile defaults

## Related commands
- [datasource add](./datasource-add.md)
- [datasource list](./datasource-list.md)
- [datasource delete](./datasource-delete.md)
