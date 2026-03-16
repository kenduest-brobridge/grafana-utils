# GitOps Sync Design History

Historical note:

- The sync surface is now wired in both Python and Rust as
  `grafana-util sync ...`.
- Current implemented commands include `summary`, `plan`, `review`, `apply`,
  `preflight`, `assess-alerts`, `bundle-preflight`, and `bundle`.
- Live Grafana fetch/apply support now exists for the supported resource kinds.
- This file is kept as the original planning scaffold because historical change
  entries already point at it.

## Purpose

This note tracks the original isolated declarative sync planning scaffold from
before CLI wiring and live Grafana mutation paths landed.

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

## Current Wiring Status

- Python wiring now lives in `grafana_utils/sync_cli.py` and the unified CLI.
- Rust parity now lives in `rust/src/sync.rs`.
- Review-gated live apply and live fetch are implemented for the current
  supported sync resource kinds.
- Secret-provider integration still remains separate follow-up work.
