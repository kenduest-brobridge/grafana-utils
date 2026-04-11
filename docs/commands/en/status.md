# `grafana-util status`

## Root

Purpose: read live and staged Grafana state through one canonical status surface.

When to use: when you want readiness, overview, snapshot, or direct live reads without moving into mutation work.

Description: `status` is the user-facing read-only entrypoint. Use `live` for current-state gating, `staged` for artifact review, `overview` for project-wide summaries, `snapshot` for bundle-style review, and `resource` for direct live reads.

Examples:

```bash
grafana-util status live --profile prod --output-format yaml
grafana-util status staged --desired-file ./desired.json --output-format json
grafana-util status overview --dashboard-export-dir ./dashboards/raw --alert-export-dir ./alerts --output-format table
grafana-util status overview live --url http://localhost:3000 --basic-user admin --basic-password admin --output-format interactive
```

Related commands: `grafana-util export`, `grafana-util workspace`, `grafana-util config profile`.

## `live`

Purpose: render a live readiness view from Grafana read surfaces.

## `staged`

Purpose: render a readiness gate from staged artifacts.

## `overview`

Purpose: summarize staged artifacts or open the live overview.

## `snapshot`

Purpose: review or export a snapshot-style artifact bundle.

## `resource`

Purpose: inspect one live Grafana resource directly.
