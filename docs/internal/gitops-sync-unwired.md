# GitOps Sync (Unwired)

## Purpose

This note tracks the isolated declarative sync planning scaffold before any
CLI wiring or live Grafana mutation path lands.

## Scope

- New Python helper module:
  - `grafana_utils/gitops_sync.py`
- New unit tests:
  - `tests/test_python_gitops_sync.py`

## Current Behavior

- `build_sync_plan(...)`
  - Normalizes desired and live resource specs for `dashboard`, `datasource`,
    `folder`, and partial `alert` resources.
  - Produces reviewable operations with `would-create`, `would-update`,
    `would-delete`, `noop`, or `unmanaged` actions.
  - Fails closed on duplicate identities and on alert specs that do not
    declare explicit `managedFields`.
- `mark_plan_reviewed(...)`
  - Keeps live-apply preparation behind one explicit review token step.
- `build_apply_intent(...)`
  - Returns dry-run intent documents freely.
  - Refuses live apply intent until the plan is both reviewed and explicitly
    approved.

## Not Yet Wired

- No argparse or unified CLI integration yet.
- No live Grafana client or filesystem/Git integration yet.
- No external secret-provider integration yet.
- No Rust parity yet.

## Future Wire Points

- `grafana_utils/unified_cli.py`
- `grafana_utils/dashboard_cli.py`
- `grafana_utils/datasource_cli.py`
- `grafana_utils/alert_cli.py`
