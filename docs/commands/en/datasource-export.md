# datasource export

## Purpose
Export live Grafana datasource inventory as normalized JSON plus provisioning files.

## When to use
Use this when you need a local datasource bundle for later inspection, diff, or import.

## Key flags
- `--output-dir`: target directory for the export tree.
- `--org-id`: export one explicit Grafana org.
- `--all-orgs`: export each visible org into per-org subdirectories. Requires Basic auth.
- `--overwrite`: replace existing files.
- `--without-datasource-provisioning`: skip the provisioning variant.
- `--dry-run`: preview what would be written.

## Examples
```bash
# Purpose: Export live Grafana datasource inventory as normalized JSON plus provisioning files.
grafana-util datasource export --url http://localhost:3000 --basic-user admin --basic-password admin --output-dir ./datasources --overwrite
```

```bash
# Purpose: Export live Grafana datasource inventory as normalized JSON plus provisioning files.
grafana-util datasource export --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --output-dir ./datasources --overwrite
```

## Before / After

- **Before**: live datasource state was easy to lose because the export shape was not normalized or easy to reuse later.
- **After**: one export gives you a local bundle that is stable enough for inspection, diff, and import.

## What success looks like

- the export tree is complete enough to inspect later without Grafana
- normalized JSON and provisioning files stay aligned with the source inventory
- the bundle is ready for diff or import without extra hand cleanup

## Failure checks

- if the export tree is missing org data, confirm the org scope and whether the credentials can see it
- if `--all-orgs` fails, use Basic auth and verify that the account can see each target org
- if the bundle looks stale, verify the export directory and whether `--overwrite` was used intentionally

## Related commands
- [datasource list](./datasource-list.md)
- [datasource import](./datasource-import.md)
- [datasource diff](./datasource-diff.md)
