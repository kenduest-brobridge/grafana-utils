# Sync Preflight Design History

Historical note:

- `grafana-util sync preflight` is now a wired command in both Python and Rust.
- The command can use local availability input or `--fetch-live` to gather
  availability hints from Grafana.
- This file is kept as the original scaffold summary for traceability.

## Purpose

This note tracks the original declarative sync preflight scaffold and its
implementation shape.

## Scope

- New Python helper module:
  - `grafana_utils/sync_preflight_workbench.py`
- New unit tests:
  - `tests/test_python_sync_preflight_workbench.py`

## Current Behavior

- `build_sync_preflight_document(...)`
  - Builds staged dependency and policy checks from desired sync specs plus
    explicit availability hints.
  - Covers datasource plugin availability, dashboard datasource references,
    and alert contact-point/live-apply blocking rules.
- `render_sync_preflight_text(...)`
  - Renders a deterministic text summary for the wired CLI surface.

## Wired Surface

- `grafana-util sync preflight`
  - Accepts local desired-state JSON plus optional `--availability-file`.
  - Also supports `--fetch-live` plus Grafana connection arguments in the
    current CLI implementation.
  - Supports `--fetch-live` to probe datasource UIDs, plugin IDs, and alert
    contact points from Grafana before rendering preflight output.

## Still Not Wired

- No live alert apply support; alerts stay explicitly blocked here.
