# TODO

This file tracks the active backlog only.

Completed items that were previously listed here now live in `docs/internal/todo-archive.md`.

## In Progress

- shared access TLS/auth parameter expansion is still incomplete for Python and Rust parity
- live validation coverage still needs follow-through for the newer destructive access commands
- Python packaging, docs, and syntax-floor tests now target Python 3.9+, but optional formatter/lint/static-check coverage still depends on tool availability in the active environment

## Next

- reduce Python/Rust inspect-export and inspect-live drift by keeping one stable summary/report schema, shared filters, and synchronized help/examples
- reduce repeated live Grafana lookups during dashboard import and dry-run paths so large imports do not multiply API round-trips per dashboard
- dashboard `prompt` export should surface the original datasource name in Grafana web-import prompts, not only the datasource type label
- dashboard `prompt` export should align `__requires` names and versions with Grafana external export where possible
- dashboard `prompt` export should add broader mixed-type and same-type datasource validation coverage beyond the current Prometheus/Loki cases
- add a broader import dependency preflight that checks datasource existence, plugin availability, and alert/contact references before mutating target Grafana
- add dashboard and folder ACL permission export/import support so user/team/service-account/role permissions can be reviewed and promoted alongside dashboard content
- extend dashboard offline inspection from counts and datasource usage into richer dependency analysis, including per-query extracted metrics/buckets/measurements where the datasource format is understood
- refactor query report extraction behind datasource-type-specific analyzers so Prometheus, Loki, Flux/Influx, SQL, and future datasource families can evolve independently without bloating one generic parser path
- extend query report extraction for Loki-style log queries so inspection can report stream selectors, label matchers, pipeline stages, filters, and range/aggregation functions instead of leaving Loki queries as empty `metrics`
- add report modes for datasource usage, orphaned datasource detection, and dashboard-to-datasource dependency summaries that can feed governance and cleanup work
- add an export package/bundle workflow that can snapshot dashboards, alerting resources, datasource inventory, and metadata as one portable migration artifact
- gradually replace ad hoc dashboard and alert datasource reference maps with typed structs where the shape is stable enough to justify it
- extract repeated dashboard and alert fallback strings into shared constants where they still appear in multiple places
- clean repo workflow noise by keeping local scratch files, temp exports, and ad hoc notes out of normal review/commit paths
- evaluate streaming or lower-memory dashboard listing/export paths only if large-instance validation shows the current full-materialization approach is a real bottleneck
- evaluate semantic alert diff normalization for equivalent values such as duration aliases after the current structural diff behavior is otherwise stable
- extend `grafana-util sync` beyond the current Grafana-aware plan fetch and limited live apply subset so alerts, richer preflight checks, and broader dependency validation can use the same review contract
- document `grafana-util sync` as one operator-facing workflow, including purpose, desired-state schema, staged artifact flow, review/preflight/apply semantics, and a minimal end-to-end demo
- fix the current Python `grafana-util sync` rough edges so the documented `plan -> review -> apply` path actually round-trips through `--plan-file` / `--output-file` without ad hoc manual artifact edits
- reconcile Python `sync` alert policy so `plan`, `assess-alerts`, `preflight`, and any future live-apply gate all agree on which alert cases are candidate, plan-only, or blocked
- stabilize Python `sync` availability/preflight contracts so datasource plugin checks, availability key names, and bundle-level blocking summaries match the documented schema and demo behavior
- formalize the desired-state and staged-artifact schema for `grafana-util sync`, including required `body`/`managedFields` semantics and what fields are intentionally excluded or plan-only
- decide whether Python `sync apply` remains intent-first or grows into a complete live mutation workflow, then align help text, output documents, and gating rules to that explicit contract
- extend datasource import placeholder secret handling from the current Python/Rust inline/file sidecars into a shared external-provider contract without weakening fail-closed behavior

### Roadmap Workbench: Inspection And Dependency Governance

- start in a separate development file first and avoid wiring the new paths into the current CLI until the contracts settle
- add dashboard-to-datasource dependency summaries that operators can review without custom scripts
- add blast-radius and orphan-detection reporting for dashboard and datasource governance
- add resource dependency graph export with at least JSON first, then evaluate DOT and SVG renderers on top of the same graph model
- keep datasource-type-specific query analyzers as the extension point for Prometheus, Loki, Flux/Influx, SQL, and later families
- add a static HTML inspection report only if it reuses the same canonical summary/report document

### Roadmap Workbench: Environment Promotion And Preflight Safety

- start in a separate development file first and avoid wiring promotion/preflight code into existing import flows until reviewable dry-run contracts are stable
- design a first-class `promote --from <env> --to <env>` workflow around the current export/import/diff contract
- expand import preflight to check datasources, plugins, alerts, contact points, library panels, and other common target prerequisites before mutation
- expand promotion planning to include dashboard/folder ACL permission prerequisites and explicit permission drift visibility
- support datasource and dashboard UID/name remap rules as explicit reviewable inputs
- keep dry-run and diff outputs trustworthy enough for team review before live mutation

### Roadmap Workbench: Declarative Sync And GitOps

- start from the new `grafana_utils.gitops_sync` contract and keep CLI wiring secondary until the plan/apply surface is reviewable
- the public Python CLI now exists as `grafana-util sync plan|review|apply`, and it can fetch live Grafana state for planning plus execute a limited live apply path for folders, dashboards, and datasources
- keep managed state explicitly scoped to dashboards, datasources, folders, and partial-alert ownership instead of implying full Grafana takeover
- require dry-run, review, and explicit apply acknowledgement before any future live mutation path is allowed
- keep alert sync in plan-only mode until partial ownership and safe mutation semantics are explicit enough to review
- decide how prune and unmanaged live resources are surfaced so Git-managed scope stays auditable in reviews
- write one canonical operator document for `sync` before expanding more staged subcommands so the workflow is understandable outside internal notes
- align Python and Rust staged artifact contracts only after the Python public flow, schema, and demo are stable enough to serve as the user-facing reference
- keep `sync` demos and docs based on real runnable fixtures so plan/review/preflight/apply examples do not drift from actual CLI behavior

### Roadmap Workbench: Secret Handling And Redaction

- start from the new `grafana_utils.datasource_secret_workbench` contract and keep provider integrations unwired until placeholder semantics settle
- datasource import now accepts placeholder sidecar mappings from CLI/file inputs in both Python and Rust, but the export bundle contract still intentionally excludes secrets
- use placeholder-based datasource secret references and reject opaque `secureJsonData` replay
- keep missing or empty secret resolution fail-closed rather than silently dropping or clearing secret-bearing fields
- align Python and Rust password/token/secret semantics before adding external secret providers

## Shared Access Parameters

Currently implemented:

- `--url`
- `--token`
- `--basic-user`
- `--basic-password`
- `--prompt-password`
- `--org-id`
- `--json`
- `--csv`
- `--table`

Still not implemented:

- `--insecure`
- `--ca-cert`

## Authentication Rules

Current implementation status:

- `user list --scope org`: token or Basic auth
- `user list --scope global`: Basic auth only
- `user list --with-teams`: Basic auth only
- `user add`: Basic auth only
- `user modify`: Basic auth only
- `user delete --scope global`: Basic auth only
- `user delete --scope org`: token or Basic auth
- `team list`: token or Basic auth
- `team add`: token or Basic auth
- `team modify`: token or Basic auth
- `team delete`: token or Basic auth
- `service-account list`: token or Basic auth
- `service-account add`: token or Basic auth
- `service-account token add`: token or Basic auth
- `service-account delete`: token or Basic auth
- `service-account token delete`: token or Basic auth

Rules to keep:

- if `--token` is provided, treat it as the primary authentication input unless the command explicitly requires Basic auth
- only require `--basic-user` and `--basic-password` for operations that truly need Basic auth
- reject mixed auth inputs unless the command has a specific, documented reason to support them
- keep prompted password support aligned with dashboard and alert auth behavior

## Priority Order

1. reduce Python/Rust inspect-export and inspect-live drift
2. reduce repeated dashboard import lookup calls on live Grafana
3. refactor query report extraction behind datasource-type-specific analyzers
4. add broader import dependency preflight for datasources/plugins/alert references
5. improve dashboard prompt export fidelity for datasource names and `__requires`
6. extend inspection into richer dependency analysis and datasource usage/orphan reports
7. typed datasource reference structs in the Rust dashboard and alert paths
8. clean repo workflow noise and local scratch artifacts
9. export package/bundle workflow
10. semantic alert diff normalization for equivalent values
