# dashboard

## Purpose
`grafana-util dashboard` is the namespace for live dashboard workflows, local draft handling, export/import review, inspection, topology, and screenshots. The same namespace is also available as `grafana-util db`.

## When to use
Use this namespace when you need to browse live dashboards, fetch or clone a live dashboard into a local JSON draft, compare local files with Grafana, inspect export or live metadata, convert raw dashboards into prompt JSON, or publish a prepared dashboard back to Grafana.

## Description
Open this page first when the work is about the full dashboard workflow rather than one isolated flag. The `dashboard` namespace brings together the tasks that usually travel together in real operator work: inventory reads, export and backup, migration between environments, staged review before apply, live inspection, topology checks, and reproducible screenshots.

If you are an SRE, Grafana operator, or responder, this page should help you decide which dashboard path to open next. If you already know the exact action, jump from here into the matching subcommand page for the concrete flags and examples.

## Key flags
- `--url`: Grafana base URL.
- `--token`, `--basic-user`, `--basic-password`: shared live Grafana credentials.
- `--profile`: load repo-local defaults from `grafana-util.yaml`.
- `--color`: control JSON color output for the namespace.

## Auth notes
- Prefer `--profile` for repeatable daily work and CI.
- Use direct Basic auth for bootstrap or admin-heavy flows.
- Token auth can be enough for scoped reads, but cross-org workflows such as `--all-orgs` are safer with admin-backed `--profile` or Basic auth.
- `dashboard raw-to-prompt` is usually offline, but it can optionally use `--profile` or live auth flags to look up datasource inventory while repairing prompt files.

## Examples
```bash
# Purpose: `grafana-util dashboard` is the namespace for live dashboard workflows, local draft handling, export/import review, inspection, topology, and screenshots. The same namespace is also available as `grafana-util db`.
grafana-util dashboard --help
```

```bash
# Purpose: `grafana-util dashboard` is the namespace for live dashboard workflows, local draft handling, export/import review, inspection, topology, and screenshots. The same namespace is also available as `grafana-util db`.
grafana-util dashboard browse --profile prod
```

```bash
# Purpose: `grafana-util dashboard` is the namespace for live dashboard workflows, local draft handling, export/import review, inspection, topology, and screenshots. The same namespace is also available as `grafana-util db`.
grafana-util dashboard browse --url http://localhost:3000 --basic-user admin --basic-password admin
```

```bash
# Purpose: `grafana-util dashboard` is the namespace for live dashboard workflows, local draft handling, export/import review, inspection, topology, and screenshots. The same namespace is also available as `grafana-util db`.
grafana-util dashboard raw-to-prompt --input-file ./legacy/cpu-main.json --profile prod --org-id 2
```

```bash
# Purpose: `grafana-util dashboard` is the namespace for live dashboard workflows, local draft handling, export/import review, inspection, topology, and screenshots. The same namespace is also available as `grafana-util db`.
grafana-util dashboard inspect-live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format governance-json
```

```bash
# Purpose: `grafana-util dashboard` is the namespace for live dashboard workflows, local draft handling, export/import review, inspection, topology, and screenshots. The same namespace is also available as `grafana-util db`.
grafana-util dashboard inspect-live --url http://localhost:3000 --basic-user admin --basic-password admin --interactive
```

## Related commands
- [dashboard browse](./dashboard-browse.md)
- [dashboard get](./dashboard-get.md)
- [dashboard clone-live](./dashboard-clone-live.md)
- [dashboard list](./dashboard-list.md)
- [dashboard export](./dashboard-export.md)
- [dashboard import](./dashboard-import.md)
- [dashboard raw-to-prompt](./dashboard-raw-to-prompt.md)
- [dashboard patch-file](./dashboard-patch-file.md)
- [dashboard review](./dashboard-review.md)
- [dashboard publish](./dashboard-publish.md)
- [dashboard delete](./dashboard-delete.md)
- [dashboard diff](./dashboard-diff.md)
- [dashboard inspect-export](./dashboard-inspect-export.md)
- [dashboard inspect-live](./dashboard-inspect-live.md)
- [dashboard inspect-vars](./dashboard-inspect-vars.md)
- [dashboard governance-gate](./dashboard-governance-gate.md)
- [dashboard topology](./dashboard-topology.md)
- [dashboard screenshot](./dashboard-screenshot.md)
