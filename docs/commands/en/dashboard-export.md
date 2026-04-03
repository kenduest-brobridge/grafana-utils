# dashboard export

## Purpose
Export dashboards to `raw/`, `prompt/`, and `provisioning/` files.

## When to use
Use this when you need a local export tree for later import, review, diff, or file-provisioning workflows. The `prompt/` lane is for Grafana UI import, not dashboard API import; use `dashboard raw-to-prompt` when you need to convert ordinary or raw dashboard JSON into prompt JSON.

## Key flags
- `--export-dir`: target directory for the export tree.
- `--org-id`: export one explicit Grafana org.
- `--all-orgs`: export each visible org into per-org subdirectories. Prefer Basic auth.
- `--flat`: write files directly into each export variant directory.
- `--overwrite`: replace existing export files.
- `--without-dashboard-raw`, `--without-dashboard-prompt`, `--without-dashboard-provisioning`: skip a variant.
- `--provisioning-provider-name`, `--provisioning-provider-org-id`, `--provisioning-provider-path`: customize the generated provisioning provider file.
- `--provisioning-provider-disable-deletion`, `--provisioning-provider-allow-ui-updates`, `--provisioning-provider-update-interval-seconds`: tune provisioning behavior.
- `--dry-run`: preview what would be written.

## Notes
- Use `--profile` for normal single-org export flows.
- For `--all-orgs`, prefer admin-backed `--profile` or direct Basic auth because token visibility may not cover every org you expect.

## Examples
```bash
grafana-util dashboard export --profile prod --export-dir ./dashboards --overwrite
grafana-util dashboard export --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --export-dir ./dashboards --overwrite
grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --export-dir ./dashboards --overwrite
```

## Related commands
- [dashboard inspect-export](./dashboard-inspect-export.md)
- [dashboard import](./dashboard-import.md)
- [dashboard diff](./dashboard-diff.md)
- [dashboard raw-to-prompt](./dashboard-raw-to-prompt.md)
