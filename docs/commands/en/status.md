# `grafana-util status`

## Root

Purpose: read live and staged Grafana state through one canonical status surface.

When to use: when you want readiness, overview, snapshot, or direct live reads without moving into mutation work.

Description: `status` is the user-facing read-only entrypoint. Use `live` for current-state gating, `staged` for local artifact review, `overview` for project-wide summaries, `snapshot` for offline bundle capture and review, and `resource` for generic live reads when a richer domain workflow is not needed yet.

## Command groups

- Live Read-Only: `live`, `overview`, `resource`, `snapshot`
- Staged Review: `staged`, `overview`

Open this page when you know the task is read-only but you still need to decide whether the next step is a direct health check, a generic resource lookup, a project-wide overview, or a snapshot export for later review.

## Before / After

- **Before**: read-only Grafana checks are often split across one-off scripts, direct API calls, ad hoc exports, and workflow-specific commands.
- **After**: the `status` namespace keeps the common read surfaces together so operators can choose the right read path without immediately entering mutation workflows.

## What success looks like

- the next read path is obvious before you open a deeper command page
- live checks, staged review, and snapshot review share one predictable namespace
- generic resource reads stay available without forcing a full export or domain-specific flow

## Failure checks

- if a live command returns less data than expected, confirm the auth scope and org visibility first
- if a staged overview looks incomplete, verify the input directories and file paths before assuming the parser is wrong
- if downstream automation reads the output, set `--output-format` explicitly on the leaf command

### Key flags

- the root `status` command is a namespace; operational flags live on `live`, `staged`, `overview`, `snapshot`, and `resource`
- `--help-schema` is available on the `status` surface for schema-oriented help paths

### Examples

```bash
# inspect the read-only entrypoint before choosing a lane.
grafana-util status --help
```

```bash
# check live Grafana reachability and health.
grafana-util status live --profile prod --output-format yaml
```

```bash
# review staged desired-state artifacts before a preview or apply flow.
grafana-util status staged --desired-file ./desired.json --output-format json
```

```bash
# summarize one staged dashboard and alert tree.
grafana-util status overview --dashboard-export-dir ./dashboards/raw --alert-export-dir ./alerts --output-format table
```

```bash
# open the live overview path as an interactive workbench.
grafana-util status overview live --url http://localhost:3000 --basic-user admin --basic-password admin --output-format interactive
```

```bash
# describe the supported generic resource selectors.
grafana-util status resource describe dashboards --output-format json
```

```bash
# export a snapshot bundle for later offline review.
grafana-util status snapshot export --profile prod --output-dir ./snapshot
```

## Related commands

- `grafana-util export`
- `grafana-util workspace`
- `grafana-util config profile`

## `live`

Purpose: render a live readiness view from Grafana read surfaces.

## `staged`

Purpose: render a readiness gate from staged artifacts.

## `overview`

Purpose: summarize staged artifacts or open the live overview.

## `snapshot`

Purpose: review or export a snapshot-style artifact bundle.

## `resource`

Purpose: inspect supported live Grafana resources through a generic read-only query surface.
