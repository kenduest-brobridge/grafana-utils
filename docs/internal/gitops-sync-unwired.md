# GitOps Sync

## Purpose

This note tracks the declarative sync planning surface and the remaining gaps
between the shipped staged CLI and a broader Git-managed workflow.

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

## Wired Surface

- `grafana-util sync plan`
  - Builds one review-required sync plan from local desired/live JSON files.
  - Can also fetch live inventory directly from Grafana instead of `--live-file`.
- `grafana-util sync review`
  - Marks a staged plan reviewed and preserves the review token in the emitted
    JSON document.
- `grafana-util sync apply`
  - Builds a gated apply-intent document from a reviewed plan and can bridge to
    limited live execution only after explicit approval.
- `grafana-util sync preflight`
  - Builds staged dependency and policy checks from desired-state inputs plus
    availability hints or optional live fetches.
- `grafana-util sync bundle-preflight`
  - Aggregates staged promotion/sync checks plus datasource secret/provider
    availability into one reviewable bundle gate.
- `grafana-util sync assess-alerts`
  - Renders the staged alert-ownership assessment contract directly from local
    alert specs.

## Still Not Wired

- No end-to-end Git workspace automation; operators still assemble and review
  the staged JSON artifacts explicitly.
- No broad always-on reconcile loop or controller model.
- No external secret-provider resolution in the sync CLI itself.
- No full Python/Rust parity for the same staged sync contract surface.

## Future Wire Points

- `grafana_utils/gitops_sync.py`
- `grafana_utils/sync_cli.py`
- `grafana_utils/sync_preflight_workbench.py`
- `grafana_utils/bundle_preflight.py`
