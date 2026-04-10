# export

## Purpose
`grafana-util export` is the task-first surface for common backup and local inventory capture jobs.

## When to use
Use this namespace when the job is straightforward export or backup and you do not want to start from a domain-heavy tree.

## Description
`export` is intentionally narrow. It wraps the existing domain export flows without changing their underlying behavior. This keeps first-run usage simple while preserving the deeper `advanced` and compatibility trees for expert work.

## Subcommands

### Backup and artifact capture
- `export dashboard`: export dashboards into raw, prompt, and provisioning lanes.
- `export alert`: export alert resources into a local artifact tree.
- `export datasource`: export datasource inventory for review or restore.

### Access inventory capture
- `export access user`: export Grafana users.
- `export access org`: export Grafana org inventory.
- `export access team`: export Grafana teams.
- `export access service-account`: export service accounts.

## Examples
### Dashboard backup
```bash
grafana-util export dashboard --output-dir ./dashboards --overwrite
```

### Alert backup
```bash
grafana-util export alert --output-dir ./alerts --overwrite
```

### Datasource inventory
```bash
grafana-util export datasource --output-dir ./datasources
```

### Access inventory
```bash
grafana-util export access service-account --output-dir ./access-service-accounts
```

## Related commands

- [advanced](./advanced.md)
- [dashboard export](./dashboard-export.md)
- [alert export](./alert-export.md)
- [datasource export](./datasource-export.md)
- [access](./access.md)
