# datasource export

## Purpose
Export live Grafana datasource inventory as normalized JSON plus provisioning files.

## When to use
Use this when you need a local datasource bundle for later inspection, diff, or import.

## Key flags
- `--export-dir`: target directory for the export tree.
- `--org-id`: export one explicit Grafana org.
- `--all-orgs`: export each visible org into per-org subdirectories. Requires Basic auth.
- `--overwrite`: replace existing files.
- `--without-datasource-provisioning`: skip the provisioning variant.
- `--dry-run`: preview what would be written.

## Examples
```bash
grafana-util datasource export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./datasources --overwrite
grafana-util datasource export --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --export-dir ./datasources --overwrite
```

## Related commands
- [datasource inspect-export](./datasource-inspect-export.md)
- [datasource import](./datasource-import.md)
- [datasource diff](./datasource-diff.md)

