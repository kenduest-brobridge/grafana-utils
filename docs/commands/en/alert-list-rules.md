# `grafana-util alert list-rules`

## Purpose

List live Grafana alert rules.

## When to use

- Inspect alert rule inventory from one org or from all visible orgs.
- Render the list in text, table, CSV, JSON, or YAML form.

## Key flags

- `--org-id` lists rules from one Grafana org ID.
- `--all-orgs` aggregates inventory across visible orgs.
- `--output-format` controls the output format, including `text`, `table`, `csv`, `json`, and `yaml`.
- `--no-header` omits the header row.

## Notes

- Use `--profile` for repeatable single-org inventory.
- For `--all-orgs`, prefer admin-backed `--profile` or direct Basic auth because token scope can return a partial view.

## Before / After

- Before: inspect alert rules through the UI and manually cross-check which org they belong to.
- After: collect one inventory output that is easier to compare, diff, or hand off to CI.

## What success looks like

- The alert rules you expect appear in the requested output format.
- The org or profile scope matches what you meant to inspect.
- The output is easy to feed into a review or audit step if needed.

## Failure checks

- If the list looks incomplete, verify token scope or switch to a broader profile.
- If `--all-orgs` does not show everything you expect, use admin-backed credentials.
- Check the org/profile context before assuming the inventory is empty.

## Examples

```bash
# Purpose: List live Grafana alert rules.
grafana-util alert list-rules --profile prod --output-format table
```

```bash
# Purpose: List live Grafana alert rules.
grafana-util alert list-rules --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format json
```

```bash
# Purpose: List live Grafana alert rules.
grafana-util alert list-rules --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --output-format yaml
```

## Related commands

- [alert](./alert.md)
- [alert list-contact-points](./alert-list-contact-points.md)
- [alert list-mute-timings](./alert-list-mute-timings.md)
