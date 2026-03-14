# TODO

## Status

### Done

- unified primary CLI is now `grafana-utils`
- Python source-tree wrapper is now `python/grafana-utils.py`
- Python `grafana-access-utils` shim was removed
- Python and Rust both support access-management commands through `grafana-utils access ...`
- implemented access `user list`
- implemented access `user add`
- implemented access `user modify`
- implemented access `user delete`
- implemented access `team list`
- implemented access `team add`
- implemented access `team modify`
- implemented access `team delete`
- implemented access `service-account list`
- implemented access `service-account add`
- implemented access `service-account delete`
- implemented access `service-account token add`
- implemented access `service-account token delete`
- implemented access `group` alias for `team`
- added unit tests and Docker-backed live validation for the implemented access workflows
- dashboard CLI also includes `list-data-sources` in both Python and Rust, but that is outside the remaining access-management scope tracked below
- Rust dashboard internals were split into:
  - `dashboard_cli_defs.rs`
  - `dashboard_list.rs`
  - `dashboard_export.rs`
  - `dashboard_prompt.rs`
- Rust dashboard export metadata and index documents now use typed internal structs without changing JSON output shape
- Rust `access.rs` internals were split into:
  - `access_cli_defs.rs`
  - `access_user.rs`
  - `access_team.rs`
  - `access_service_account.rs`
  - `access_render.rs`
- Rust alert internals were split further across:
  - `alert_cli_defs.rs`
  - `alert_client.rs`
  - `alert_list.rs`
  - `alert.rs` orchestration helpers

### In Progress

- access-management CLI exists in both Python and Rust, and the remaining open access work is now limited to shared TLS/auth parameter expansion plus live validation coverage for the newer destructive commands
- service-account support now includes delete and token-delete, but broader auth/TLS option parity and live validation coverage still need follow-through
- baseline quality gate scripts now exist and are wired into `make` and CI, but optional Python formatter/lint/static-check coverage still depends on tool availability in the active environment
- Python dashboard orchestration is now split across dedicated export/import/inspection/diff runtime helpers, and the remaining cleanup is limited to trimming compatibility wrappers from `dashboard_cli.py`

### Next

- split the oversized Rust dashboard orchestration paths further so live/export/import/inspect flows do not keep accreting in one module
- trim the remaining Python dashboard CLI compatibility wrappers now that export/import/inspection/diff runtime wiring has moved into dedicated helper modules
- consolidate duplicated Python auth resolution logic across dashboard, alert, and access CLIs into shared helpers to reduce behavior drift
- reduce Python/Rust inspect-export and inspect-live drift by keeping one stable summary/report schema, shared filters, and synchronized help/examples
- reduce repeated live Grafana lookups during dashboard import and dry-run paths so large imports do not multiply API round-trips per dashboard
- add datasource `diff` workflows so inventory replay has a first-class compare path alongside `list`, `export`, and `import`
- tighten the datasource import/export contract further around server-managed fields, secure settings, and Python/Rust payload normalization alignment
- add cross-language datasource fixtures that cover Prometheus, Loki, InfluxDB, and mixed auth/secret-handling cases so Python and Rust stay behaviorally aligned
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

## Remaining Access Work

Current implementation status:

- `user list`: done
- `user add`: done
- `user modify`: done
- `user delete`: done
- `team list`: done
- `team add`: done
- `team modify`: done
- `team delete`: done
- `service-account list`: done
- `service-account add`: done
- `service-account token add`: done
- `service-account delete`: done
- `service-account token delete`: done
- `group` alias: done

Recommended user-facing command shape:

```text
grafana-utils access user list
grafana-utils access user add
grafana-utils access user modify
grafana-utils access user delete

grafana-utils access team list
grafana-utils access team add
grafana-utils access team modify
grafana-utils access team delete

grafana-utils access group list
grafana-utils access group add
grafana-utils access group modify
grafana-utils access group delete

grafana-utils access service-account list
grafana-utils access service-account add
grafana-utils access service-account delete
grafana-utils access service-account token add
grafana-utils access service-account token delete
```

Notes:

- `group` should remain a compatibility alias for `team`
- Rust may still keep `grafana-access-utils` as a compatibility binary, but the primary command model is `grafana-utils access ...`
- Python should not reintroduce a separate `grafana-access-utils` wrapper or console script

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

1. split Rust dashboard live/export/import/inspect orchestration into smaller modules
2. trim the remaining Python dashboard CLI compatibility wrappers
3. refactor query report extraction behind datasource-type-specific analyzers
4. add datasource `diff` workflows plus a tighter stable import/export contract
5. add broader import dependency preflight for datasources/plugins/alert references
6. reduce repeated dashboard import lookup calls on live Grafana
7. extend inspection into richer dependency analysis and datasource usage/orphan reports
8. consolidate duplicated Python auth resolution logic across dashboard, alert, and access CLIs into shared helpers
9. typed datasource reference structs in the Rust dashboard and alert paths
10. clean repo workflow noise and local scratch artifacts
11. export package/bundle workflow
12. semantic alert diff normalization for equivalent values
