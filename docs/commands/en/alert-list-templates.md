# `grafana-util alert list-templates`

## Purpose

List live Grafana notification templates.

## When to use

- Inspect template inventory from one org or from all visible orgs.
- Render the list in text, table, CSV, JSON, or YAML form.

## Key flags

- `--org-id` lists templates from one Grafana org ID.
- `--all-orgs` aggregates inventory across visible orgs.
- `--output-format` controls the output format, including `text`, `table`, `csv`, `json`, and `yaml`.
- `--no-header` omits the header row.

## Notes

- Use `--profile` for repeatable single-org inventory.
- For `--all-orgs`, prefer admin-backed `--profile` or direct Basic auth because token scope can return a partial view.

## Before / After

- Before: open template pages one by one to see what notification templates exist.
- After: list the inventory once and compare it against the org or profile you care about.

## What success looks like

- The templates you expect are present in the chosen format.
- The output scope matches the org or profile you intended to inspect.
- The format is ready for human review or scripting if needed.

## Failure checks

- If the list is unexpectedly small, verify token scope or switch to a broader profile.
- If `--all-orgs` misses entries, try admin-backed credentials instead of a narrow token.
- Confirm the org/profile before interpreting an empty response as a real absence.

## Examples

```bash
# Purpose: List live Grafana notification templates.
grafana-util alert list-templates --profile prod --output-format table
```

```bash
# Purpose: List live Grafana notification templates.
grafana-util alert list-templates --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format json
```

```bash
# Purpose: List live Grafana notification templates.
grafana-util alert list-templates --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --output-format yaml
```

## Related commands

- [alert](./alert.md)
- [alert list-rules](./alert-list-rules.md)
- [alert list-contact-points](./alert-list-contact-points.md)
