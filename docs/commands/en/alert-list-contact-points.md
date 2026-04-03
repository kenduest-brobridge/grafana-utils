# `grafana-util alert list-contact-points`

## Purpose

List live Grafana alert contact points.

## When to use

- Inspect notification endpoints configured in Grafana.
- Switch output between text, table, CSV, JSON, and YAML.

## Key flags

- `--org-id` lists contact points from one Grafana org ID.
- `--all-orgs` aggregates inventory across visible orgs.
- `--text`, `--table`, `--csv`, `--json`, `--yaml`, and `--output-format` control output.
- `--no-header` omits the header row.

## Notes

- Use `--profile` for repeatable single-org inventory.
- For `--all-orgs`, prefer admin-backed `--profile` or direct Basic auth because token scope can return a partial view.

## Examples

```bash
grafana-util alert list-contact-points --profile prod --table
grafana-util alert list-contact-points --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --json
grafana-util alert list-contact-points --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --output-format yaml
```

## Related commands

- [alert](./alert.md)
- [alert list-rules](./alert-list-rules.md)
- [alert list-templates](./alert-list-templates.md)
