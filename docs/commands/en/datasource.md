# datasource

## Purpose
`grafana-util datasource` is the namespace for catalog lookup, live browsing, export/import, diff, and live create/modify/delete workflows. The same namespace is also available as `grafana-util ds`.

## When to use
Use this namespace when you want to inspect supported datasource types, browse live inventory, export a datasource bundle, compare a local bundle with Grafana, or create and maintain live datasources.

## Description
Open this page when your task is about the full data source lifecycle rather than one single mutation. The `datasource` namespace groups the work operators usually do together: checking supported types, reading live inventory, exporting and diffing bundles, and repairing or updating live Grafana data source objects.

This page is especially useful when you need to decide whether the next step is inventory, export/import, diff, or a live add/modify/delete action.

## Key flags
- `--url`: Grafana base URL.
- `--token`, `--basic-user`, `--basic-password`: shared live Grafana credentials.
- `--profile`: load repo-local defaults from `grafana-util.yaml`.
- `--color`: control JSON color output for the namespace.

## Auth notes
- Prefer `--profile` for repeatable datasource inventory and change flows.
- Use direct Basic auth for org-spanning or admin-level mutation work.
- Token auth is acceptable for scoped reads and diffs when the token can see the target org.

## Before / After

- **Before**: data source work is split across Grafana UI edits, API calls, and one-off shell snippets that are hard to review later.
- **After**: the same lifecycle is grouped into one namespace, so browse, export, diff, and mutation flows can share the same auth and review habits.

## What success looks like

- you can tell whether the next step is inventory, export/import, diff, or live mutation before touching a production data source
- repeatable auth and profile settings keep the same commands usable across daily operations and CI
- export and diff flows give you a safer path than editing a live data source first and asking questions later

## Failure checks

- if browse or list output looks incomplete, confirm whether the token or profile can actually see the target org
- if export or diff results look stale, verify that you are pointing at the correct Grafana and not at an older local bundle
- if a live mutation fails, compare the intended input with the current live data source before retrying the same command

## Examples
```bash
# Purpose: Show the datasource namespace help and its subcommands.
grafana-util datasource --help
```

```bash
# Purpose: Show the built-in datasource type catalog.
grafana-util datasource types
```

```bash
# Purpose: Browse live datasources from a saved profile.
grafana-util datasource browse --profile prod
```

```bash
# Purpose: Browse one org with explicit credentials.
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
