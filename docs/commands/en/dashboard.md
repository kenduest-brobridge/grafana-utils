# dashboard

## Purpose
`grafana-util dashboard` is the namespace for dashboard workflows. It keeps dashboard commands flat for fast terminal use, but groups the help and docs by task: browse and inspect, export and import, review and diff, edit and publish, and capture. The same namespace is also available as `grafana-util db`.

## When to use
Use this namespace when you need to inspect dashboards, pull one live dashboard into a local draft, compare local files with Grafana, normalize dashboard JSON for UI upload, or publish a prepared dashboard back to Grafana.

## Description
Open this page first when the work is about the full dashboard workflow rather than one isolated flag. The `dashboard` namespace brings together the tasks that usually travel together in real operator work: inventory reads, export and backup, review before apply, live summary, dependency checks, policy checks, and reproducible screenshots.

If you are an SRE, Grafana operator, or responder, this page should help you decide which dashboard path to open next. If you already know the exact action, jump from here into the matching subcommand page for the concrete flags and examples.

## Command groups

The command path stays flat, such as `grafana-util dashboard list`, so experienced operators do not need extra namespace depth. Use the groups below to choose the right command faster.

- Browse & Inspect: find, read, or inspect dashboards before deciding what to do. Commands: `browse`, `list`, `get`, `variables`, `history`.
- Export & Import: move dashboard artifacts between Grafana, raw JSON, prompt JSON, or provisioning files. Commands: `export`, `import`, `convert raw-to-prompt`.
- Review & Diff: collect proof, compare state, plan local-vs-live reconciliation, review dependencies, review blast radius, or run governance checks. Commands: `plan`, `diff`, `review`, `summary`, `dependencies`, `impact`, `policy`.
- Edit & Publish: create or change one local draft, then publish or delete deliberately. Commands: `get`, `clone`, `patch`, `serve`, `edit-live`, `publish`, `delete`.
- Operate & Capture: capture visual evidence for reports, incidents, or handoff. Commands: `screenshot`.

For single-dashboard authoring, the local draft path is:
- `dashboard get` or `dashboard clone` to start from one live dashboard
- `dashboard serve` to keep one or more drafts open in a local preview browser while you edit
- `dashboard review` to verify one draft
- `dashboard patch` to rewrite local metadata
- `dashboard edit-live` to fetch one live dashboard into an editor with a safe local-draft default and a review-aware apply gate
- `dashboard publish` to replay that draft back through the import pipeline

`review`, `patch`, and `publish` also accept `--input -` for one wrapped or bare dashboard JSON document from standard input. Use that when an external generator already writes the dashboard JSON to stdout. `patch --input -` requires `--output`, and `publish --watch` is the local-file variant for repeated save-and-preview loops and does not support `--input -`.

Choose this page when the task is dashboard work but you are still deciding whether the next step is to summarize, review, normalize, or capture.

## Before / After

- **Before**: dashboard work is often split across UI browsing, one-off exports, local JSON edits, and ad hoc screenshot or review steps.
- **After**: the `dashboard` namespace keeps browse, summary, review, normalize, and capture in one place, with grouped help so you can pick the lane first and then jump to the matching flat subcommand.

## What success looks like

- you can tell whether the task is summarize, review, normalize, or capture before opening a subcommand
- inventory reads, export/import flows, and review surfaces share the same auth and bundle conventions
- screenshot and dependency-review paths stay available when you need proof instead of only a final JSON blob

## Failure checks

- if browse output looks incomplete, verify the profile, auth flags, and target org before retrying
- if dependency output looks stale, confirm you are pointing at the current Grafana and not at an older local export
- if a result is going into automation, set `--output-format` explicitly so the downstream step knows the contract

## Key flags

- `--url`: Grafana base URL.
- `--token`, `--basic-user`, `--basic-password`: shared live Grafana credentials.
- `--profile`: load repo-local defaults from `grafana-util.yaml`.
- `--color`: control JSON color output for the namespace.

## Auth notes

- Prefer `--profile` for repeatable daily work and CI.
- Use direct Basic auth for bootstrap or admin-heavy flows.
- Token auth can be enough for scoped reads, but cross-org workflows such as `--all-orgs` are safer with admin-backed `--profile` or Basic auth.
- `dashboard convert raw-to-prompt` is usually offline, but it can optionally use `--profile` or live auth flags to look up datasource inventory while repairing prompt files.

## Examples

```bash
# Inspect the dashboard namespace before choosing a lane.
grafana-util dashboard --help
```

```bash
# Browse dashboards from a saved profile.
grafana-util dashboard browse --profile prod
```

```bash
# Browse a local Grafana instance with explicit credentials.
grafana-util dashboard browse --url http://localhost:3000 --basic-user admin --basic-password admin
```

```bash
# Convert a legacy dashboard export into prompt-friendly JSON.
grafana-util dashboard convert raw-to-prompt --input-file ./legacy/cpu-main.json --profile prod --org-id 2
```

```bash
# Review one generated dashboard from standard input before mutation.
jsonnet dashboards/cpu.jsonnet | grafana-util dashboard review --input - --output-format json
```

```bash
# Watch one local draft file and rerun publish dry-run after each save.
grafana-util dashboard publish --url http://localhost:3000 --basic-user admin --basic-password admin --input ./drafts/cpu-main.json --dry-run --watch
```

```bash
# Open one local dashboard draft in the local preview server.
grafana-util dashboard serve --input ./drafts/cpu-main.json --port 18080 --open-browser
```

```bash
# Pull one live dashboard into an external editor and keep the result as a local draft by default.
grafana-util dashboard edit-live --profile prod --dashboard-uid cpu-main --output ./drafts/cpu-main.edited.json
```

```bash
# Summarize live dashboard governance data for downstream review.
grafana-util dashboard summary --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format governance
```

```bash
# Open the interactive summary workbench for a live dashboard.
grafana-util dashboard summary --url http://localhost:3000 --basic-user admin --basic-password admin --interactive
```

## Related commands

### Browse & Inspect

- [dashboard browse](./dashboard-browse.md)
- [dashboard list](./dashboard-list.md)
- [dashboard get](./dashboard-get.md)
- [dashboard variables](./dashboard-variables.md)
- [dashboard history](./dashboard-history.md)

### Export & Import

- [dashboard export](./dashboard-export.md)
- [dashboard import](./dashboard-import.md)
- [dashboard convert raw-to-prompt](./dashboard-convert-raw-to-prompt.md)
- [dashboard convert export-layout](./dashboard-convert-export-layout.md)

### Review & Diff

- [dashboard diff](./dashboard-diff.md)
- [dashboard plan](./dashboard-plan.md)
- [dashboard review](./dashboard-review.md)
- [dashboard summary](./dashboard-summary.md)
- [dashboard dependencies](./dashboard-dependencies.md)
- [dashboard policy](./dashboard-policy.md)
- [dashboard impact](./dashboard-impact.md)

### Edit & Publish

- [dashboard get](./dashboard-get.md)
- [dashboard clone](./dashboard-clone.md)
- [dashboard patch](./dashboard-patch.md)
- [dashboard serve](./dashboard-serve.md)
- [dashboard edit-live](./dashboard-edit-live.md)
- [dashboard publish](./dashboard-publish.md)
- [dashboard delete](./dashboard-delete.md)

### Operate & Capture

- [dashboard screenshot](./dashboard-screenshot.md)
