# `grafana-util alert list-templates`

## Purpose

List live Grafana notification templates.

## When to use

- Inspect template inventory from one org or from all visible orgs.
- Render the list in text, table, CSV, JSON, or YAML form.

## Key flags

- `--org-id` lists templates from one Grafana org ID.
- `--all-orgs` aggregates inventory across visible orgs.
- `--text`, `--table`, `--csv`, `--json`, `--yaml`, and `--output-format` control output.
- `--no-header` omits the header row.

## Notes

- Use `--profile` for repeatable single-org inventory.
- For `--all-orgs`, prefer admin-backed `--profile` or direct Basic auth because token scope can return a partial view.

## Examples

```bash
grafana-util alert list-templates --profile prod --table
grafana-util alert list-templates --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --json
grafana-util alert list-templates --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --output-format yaml
```

## Related commands

- [alert](./alert.md)
- [alert list-rules](./alert-list-rules.md)
- [alert list-contact-points](./alert-list-contact-points.md)
