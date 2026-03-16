# Alert Sync Design History

Historical note:

- Alert sync assessment is now wired through `grafana-util sync assess-alerts`
  in both Python and Rust.
- Live alert create/update/delete support is also wired through the current
  sync apply flow when the sync operation carries a complete alert rule payload.
- This file remains as the original isolated scaffold summary.

## Purpose

This note tracks the original isolated alert-sync ownership and policy scaffold
from before the sync CLI and live alert apply paths landed.

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

## Current Wiring Status

- Assessment wiring now exists through `grafana_utils/sync_cli.py` and
  `rust/src/sync.rs`.
- The workbench still intentionally classifies some alert specs as staged
  `plan-only` when they do not contain the full Grafana provisioning payload
  required for safe live apply.
- No Grafana alert API probing yet.
