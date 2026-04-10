# advanced

## Purpose
`grafana-util advanced` is the expert-facing namespace for domain-specific workflows.

## When to use
Use this namespace once you already know the subsystem you need, such as dashboard import, alert authoring, datasource diff, or access administration.

## Description
The `advanced` tree preserves the full domain depth of `grafana-util` without making new users learn every lane on the first screen. It is the preferred canonical home for domain-heavy workflows, while older top-level roots remain available as compatibility paths.

## Subcommands

### Dashboard workflows
- `advanced dashboard live ...`: browse, inspect history, and fetch live dashboards.
- `advanced dashboard draft ...`: review, patch, serve, and publish local drafts.
- `advanced dashboard sync ...`: export, import, diff, and convert migration artifacts.
- `advanced dashboard analyze ...`: summary, topology, impact, and governance checks.
- `advanced dashboard capture ...`: browser-rendered screenshots and PDFs.

### Alert workflows
- `advanced alert live ...`: list live alert inventory and delete one live resource.
- `advanced alert migrate ...`: export, import, and diff alert artifacts.
- `advanced alert author ...`: initialize and author desired-state alert resources.
- `advanced alert scaffold ...`: seed low-level alert files directly.
- `advanced alert change ...`: plan and apply staged alert changes.

### Datasource and access workflows
- `advanced datasource ...`: list, browse, export, import, and diff datasources.
- `advanced access ...`: user, team, org, and service-account administration.

## Examples
### Dashboard import
```bash
grafana-util advanced dashboard sync import --input-dir ./dashboards/raw --dry-run --table
```

### Alert route preview
```bash
grafana-util advanced alert author route preview --desired-dir ./alerts/desired --label team=sre --severity critical
```

### Datasource diff
```bash
grafana-util advanced datasource diff --diff-dir ./datasources --input-format inventory
```

### Access diff
```bash
grafana-util advanced access user diff --diff-dir ./access-users --scope global
```

## Related commands

- [export](./export.md)
- [change](./change.md)
- [dashboard](./dashboard.md)
- [alert](./alert.md)
- [datasource](./datasource.md)
- [access](./access.md)
