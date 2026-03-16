# TODO

This file tracks the active backlog only.

Completed items that were previously listed here now live in `docs/internal/todo-archive.md`.

## In Progress

- Python packaging, docs, and syntax-floor tests now target Python 3.9+, but optional formatter/lint/static-check coverage still depends on tool availability in the active environment

## Next

- reduce repeated live Grafana lookups during dashboard import and dry-run paths so large imports do not multiply API round-trips per dashboard
- dashboard `prompt` export should surface the original datasource name in Grafana web-import prompts, not only the datasource type label
- dashboard `prompt` export should align `__requires` names and versions with Grafana external export where possible
- dashboard `prompt` export should add broader mixed-type and same-type datasource validation coverage beyond the current Prometheus/Loki cases
- add a broader import dependency preflight that checks datasource existence, plugin availability, and alert/contact references before mutating target Grafana
- extend dashboard offline inspection from counts and datasource usage into richer dependency analysis, including per-query extracted metrics/buckets/measurements where the datasource format is understood
- refactor query report extraction behind datasource-type-specific analyzers so Prometheus, Loki, Flux/Influx, SQL, and future datasource families can evolve independently without bloating one generic parser path
- extend query report extraction for Loki-style log queries so inspection can report stream selectors, label matchers, pipeline stages, filters, and range/aggregation functions instead of leaving Loki queries as empty `metrics`
- add report modes for datasource usage, orphaned datasource detection, and dashboard-to-datasource dependency summaries that can feed governance and cleanup work
- extend the Rust export package/bundle workflow beyond normalized alert-rule specs so contact points, mute timings, policies, and templates can also participate in top-level sync/preflight contracts where that is safe
- gradually replace ad hoc dashboard and alert datasource reference maps with typed structs where the shape is stable enough to justify it
- extract repeated dashboard and alert fallback strings into shared constants where they still appear in multiple places
- clean repo workflow noise by keeping local scratch files, temp exports, and ad hoc notes out of normal review/commit paths
- evaluate streaming or lower-memory dashboard listing/export paths only if large-instance validation shows the current full-materialization approach is a real bottleneck
- evaluate semantic alert diff normalization for equivalent values such as duration aliases after the current structural diff behavior is otherwise stable

## Shared Access Parameters

Currently implemented:

- `--url`
- `--token`
- `--basic-user`
- `--basic-password`
- `--prompt-password`
- `--insecure`
- `--ca-cert`
- `--org-id`
- `--json`
- `--csv`
- `--table`

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

1. reduce repeated dashboard import lookup calls on live Grafana
2. refactor query report extraction behind datasource-type-specific analyzers
3. add broader import dependency preflight for datasources/plugins/alert references
4. improve dashboard prompt export fidelity for datasource names and `__requires`
5. extend inspection into richer dependency analysis and datasource usage/orphan reports
7. typed datasource reference structs in the Rust dashboard and alert paths
8. clean repo workflow noise and local scratch artifacts
9. extend Rust bundle normalization beyond alert-rule specs
10. semantic alert diff normalization for equivalent values
