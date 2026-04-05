# `grafana-util alert list-mute-timings`

## Purpose

List live Grafana mute timings.

## When to use

- Inspect mute timing inventory from one org or from all visible orgs.
- Render the list in text, table, CSV, JSON, or YAML form.

## Key flags

- `--org-id` lists mute timings from one Grafana org ID.
- `--all-orgs` aggregates inventory across visible orgs.
- `--output-format` controls the output format, including `text`, `table`, `csv`, `json`, and `yaml`.
- `--no-header` omits the header row.

## Notes

- Use `--profile` for repeatable single-org inventory.
- For `--all-orgs`, prefer admin-backed `--profile` or direct Basic auth because token scope can return a partial view.

## Before / After

- Before: browse Grafana pages manually to check which mute timings exist.
- After: list the inventory in one pass and compare it against the org or profile you need.

## What success looks like

- The mute timings you expect appear in the selected format.
- The output matches the org or profile scope you intended to query.
- The result is ready for a quick review or a CI check.

## Failure checks

- If the inventory looks partial, check token scope or use a broader profile.
- If `--all-orgs` omits entries, switch to admin-backed credentials.
- Make sure the org/profile matches the area you intended to inspect before treating an empty output as meaningful.

## Examples

```bash
# Purpose: List live Grafana mute timings.
grafana-util alert list-mute-timings --profile prod --output-format table
```

```bash
# Purpose: List live Grafana mute timings.
grafana-util alert list-mute-timings --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format json
```

```bash
# Purpose: List live Grafana mute timings.
grafana-util alert list-mute-timings --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --output-format yaml
```

## Related commands

- [alert](./alert.md)
- [alert list-rules](./alert-list-rules.md)
- [alert list-templates](./alert-list-templates.md)
