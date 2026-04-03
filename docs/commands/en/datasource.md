# datasource

## Purpose
`grafana-util datasource` is the namespace for catalog lookup, live browsing, export/import, diff, and live create/modify/delete workflows. The same namespace is also available as `grafana-util ds`.

## When to use
Use this namespace when you want to inspect supported datasource types, browse live inventory, export a datasource bundle, compare a local bundle with Grafana, or create and maintain live datasources.

## Key flags
- `--url`: Grafana base URL.
- `--token`, `--basic-user`, `--basic-password`: shared live Grafana credentials.
- `--profile`: load repo-local defaults from `grafana-util.yaml`.
- `--color`: control JSON color output for the namespace.

## Auth notes
- Prefer `--profile` for repeatable datasource inventory and change flows.
- Use direct Basic auth for org-spanning or admin-level mutation work.
- Token auth is acceptable for scoped reads and diffs when the token can see the target org.

## Examples
```bash
grafana-util datasource --help
grafana-util datasource types
grafana-util datasource browse --profile prod
grafana-util datasource browse --url http://localhost:3000 --basic-user admin --basic-password admin
```

## Related commands
- [datasource types](./datasource-types.md)
- [datasource list](./datasource-list.md)
- [datasource browse](./datasource-browse.md)
- [datasource inspect-export](./datasource-inspect-export.md)
- [datasource export](./datasource-export.md)
- [datasource import](./datasource-import.md)
- [datasource diff](./datasource-diff.md)
- [datasource add](./datasource-add.md)
- [datasource modify](./datasource-modify.md)
- [datasource delete](./datasource-delete.md)
