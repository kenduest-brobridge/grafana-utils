# Alert Sync (Unwired)

## Purpose

This note tracks the isolated alert-sync ownership and policy scaffold before
any alert sync CLI or live mutation path lands.

## Scope

- New Python helper module:
  - `grafana_utils/alert_sync_workbench.py`
- New unit tests:
  - `tests/test_python_alert_sync_workbench.py`

## Current Behavior

- `assess_alert_sync_specs(...)`
  - Classifies alert sync specs as `candidate`, `plan-only`, or `blocked`.
  - Requires explicit `managedFields` and rejects unsupported ownership fields.
  - Keeps alerts with contact-point or annotation ownership in staged plan-only
    mode.
- `render_alert_sync_assessment_text(...)`
  - Renders a deterministic text summary for later wiring.

## Not Yet Wired

- No CLI integration yet.
- No live alert mutation support yet.
- No Grafana alert API probing yet.
