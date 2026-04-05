# `grafana-util alert list-contact-points`

## Purpose

List live Grafana alert contact points.

## When to use

- Inspect notification endpoints configured in Grafana.
- Switch output between text, table, CSV, JSON, and YAML.

## Key flags

- `--org-id` lists contact points from one Grafana org ID.
- `--all-orgs` aggregates inventory across visible orgs.
- `--output-format` controls the output format, including `text`, `table`, `csv`, `json`, and `yaml`.
- `--no-header` omits the header row.

## Notes

- Use `--profile` for repeatable single-org inventory.
- For `--all-orgs`, prefer admin-backed `--profile` or direct Basic auth because token scope can return a partial view.

## Before / After

- Before: click through Grafana to check which contact points exist and who owns them.
- After: read one inventory output and compare it against the org or profile you intended to inspect.

## What success looks like

- The contact points you expect are visible in the chosen output format.
- The inventory matches the org or profile scope you intended to query.
- The output format matches the consumers you want to feed, whether human or CI.

## Failure checks

- If the list looks partial, check whether the token or profile only sees a subset of orgs.
- If `--all-orgs` returns less than expected, switch to an admin-backed profile or Basic auth.
- Confirm the org/profile before treating an empty list as a true absence.

## Examples

```bash
# Purpose: List live Grafana alert contact points.
grafana-util alert list-contact-points --profile prod --output-format table
```

```bash
# Purpose: List live Grafana alert contact points.
grafana-util alert list-contact-points --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format json
```

```bash
# Purpose: List live Grafana alert contact points.
grafana-util alert list-contact-points --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --output-format yaml
```

## Related commands

- [alert](./alert.md)
- [alert list-rules](./alert-list-rules.md)
- [alert list-templates](./alert-list-templates.md)
