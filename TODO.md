# TODO

This file tracks the active backlog only.

Completed items that were previously listed here now live in `docs/internal/todo-archive.md`.

## In Progress

- shared access TLS/auth parameter expansion is still incomplete for Python and Rust parity
- live validation coverage still needs follow-through for the newer destructive access commands
- baseline quality gate scripts now exist and are wired into `make` and CI, but optional Python formatter/lint/static-check coverage still depends on tool availability in the active environment
- Rust dashboard orchestration is now split across CLI definitions, export, import, inspect, list, shared live helpers, and dedicated help rendering, and the remaining cleanup is to keep shrinking the root `dashboard.rs` surface by moving the last typed export/report structs into dedicated modules

## Next

- continue splitting the Rust dashboard orchestration surface so `dashboard.rs` keeps only top-level entrypoints/re-exports while the remaining typed export/report structs move into dedicated modules
- reduce Python/Rust inspect-export and inspect-live drift by keeping one stable summary/report schema, shared filters, and synchronized help/examples
- reduce repeated live Grafana lookups during dashboard import and dry-run paths so large imports do not multiply API round-trips per dashboard
- dashboard `prompt` export should surface the original datasource name in Grafana web-import prompts, not only the datasource type label
- dashboard `prompt` export should align `__requires` names and versions with Grafana external export where possible
- dashboard `prompt` export should add broader mixed-type and same-type datasource validation coverage beyond the current Prometheus/Loki cases
- add a broader import dependency preflight that checks datasource existence, plugin availability, and alert/contact references before mutating target Grafana
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

1. continue splitting Rust dashboard live/import/report orchestration into smaller modules
2. reduce Python/Rust inspect-export and inspect-live drift
3. reduce repeated dashboard import lookup calls on live Grafana
4. refactor query report extraction behind datasource-type-specific analyzers
5. add broader import dependency preflight for datasources/plugins/alert references
6. improve dashboard prompt export fidelity for datasource names and `__requires`
7. extend inspection into richer dependency analysis and datasource usage/orphan reports
8. typed datasource reference structs in the Rust dashboard and alert paths
9. clean repo workflow noise and local scratch artifacts
10. export package/bundle workflow
11. semantic alert diff normalization for equivalent values
