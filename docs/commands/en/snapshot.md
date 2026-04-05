# `grafana-util snapshot`

## Root

Purpose: export and review Grafana snapshot inventory bundles.

When to use: when you want a local snapshot root that captures dashboard and datasource inventory for later inspection.

Description: open this page when you need an offline snapshot of Grafana inventory that can be reviewed later without talking to the server again. The `snapshot` namespace is useful for handoff, backup, incident review, or any workflow where you want one local artifact before moving into deeper analysis.

## Before / After

- **Before**: snapshot-style review usually means re-querying Grafana or opening a pile of dashboards and datasources one by one.
- **After**: export first, then review the local bundle as a repeatable artifact without touching the live server again.

## What success looks like

- you can hand off a snapshot root and another operator can inspect it without asking for live access
- export output is a durable artifact instead of a temporary UI session
- review output is clear enough to feed into a follow-up analysis or incident note

## Failure checks

- if a snapshot export looks empty, verify the auth profile or live connection before assuming the source system is blank
- if review output looks surprising, confirm that you are pointing at the intended snapshot directory
- if automation reads the output, keep the chosen `--output-format` explicit so the downstream parser knows the contract

Key flags: the root command is a namespace; the operational flags live on `export` and `review`. The shared root flag is `--color`.

Examples:

```bash
# Purpose: Export a local snapshot bundle from live Grafana.
grafana-util snapshot export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./snapshot
```

```bash
# Purpose: Review the exported snapshot bundle as JSON.
grafana-util snapshot review --input-dir ./snapshot --output-format json
```

Related commands: `grafana-util overview`, `grafana-util status staged`, `grafana-util change bundle`.

## `export`

Purpose: export dashboard and datasource inventory into a local snapshot bundle.

When to use: when you need a local snapshot root that can be reviewed without Grafana access.

Key flags: `--export-dir`, `--overwrite`, plus the shared Grafana connection and auth flags.

Examples:

```bash
# Purpose: export.
grafana-util snapshot export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./snapshot
```

```bash
# Purpose: export.
grafana-util snapshot export --profile prod --export-dir ./snapshot --overwrite
```

Related commands: `snapshot review`, `change bundle`, `overview`.

## `review`

Purpose: review a local snapshot inventory without touching Grafana.

When to use: when you want to inspect an exported snapshot root as table, csv, text, json, yaml, or interactive output.

Key flags: `--input-dir`, `--interactive`, `--output-format`.

Examples:

```bash
# Purpose: review an exported snapshot root as table, csv, text, json, yaml, or interactive output.
grafana-util snapshot review --input-dir ./snapshot --output-format table
```

```bash
# Purpose: review an exported snapshot root as table, csv, text, json, yaml, or interactive output.
grafana-util snapshot review --input-dir ./snapshot --interactive
```

Related commands: `snapshot export`, `overview`, `status staged`.
