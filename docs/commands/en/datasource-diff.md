# datasource diff

## Purpose
Compare datasource inventory from a local bundle against live Grafana and print an operator-summary diff report.

## When to use
Use this when you want a concise live-versus-local difference report before import.

## Key flags
- `--diff-dir`: local datasource bundle to compare.
- `--input-format`: choose `inventory` or `provisioning`.

## Examples
```bash
grafana-util datasource diff --url http://localhost:3000 --basic-user admin --basic-password admin --diff-dir ./datasources --input-format inventory
grafana-util datasource diff --profile prod --diff-dir ./datasources/provisioning --input-format provisioning
```

## Related commands
- [datasource export](./datasource-export.md)
- [datasource import](./datasource-import.md)
- [datasource inspect-export](./datasource-inspect-export.md)
