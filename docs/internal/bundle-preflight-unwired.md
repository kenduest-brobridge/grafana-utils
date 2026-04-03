# Bundle Preflight

## Purpose

This note tracks the staged bundle-level preflight scaffold and its current
wiring status.

## Scope

- New Python helper module:
  - `grafana_utils/bundle_preflight_workbench.py`
- New unit tests:
  - `tests/test_python_bundle_preflight_workbench.py`

## Current Behavior

- `build_bundle_preflight_document(...)`
  - Combines staged promotion plan/preflight, sync preflight, and alert sync
    assessment into one reviewable bundle-level document.
  - Also aggregates datasource secret placeholder availability and external
    secret-provider reference availability into explicit blocking summaries.
- `render_bundle_preflight_text(...)`
  - Renders a concise aggregate summary for the wired CLI surface.

## Wired Surface

- `grafana-util sync bundle-preflight`
  - Accepts local source-bundle and target-inventory JSON documents.
  - Accepts provider and secret availability via `--availability-file`.
  - Supports `--fetch-live` for Grafana-backed datasource/plugin/contact-point
    availability hints, while provider and placeholder availability remain
    explicit file-driven inputs.

## Still Not Wired

- No external provider fetch or secret resolution.
