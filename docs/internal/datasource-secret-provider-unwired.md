# Datasource Secret Provider Contract (Unwired)

## Purpose

This note tracks the isolated provider-reference contract for datasource
secrets before any external provider integration lands.

## Scope

- New Python helper module:
  - `grafana_utils/datasource_secret_provider_workbench.py`
- New unit tests:
  - `python/tests/test_python_datasource_secret_provider_workbench.py`

## Current Behavior

- `collect_provider_references(...)`
  - Accepts only `${provider:NAME:PATH}` references and rejects opaque
    secret replay.
- `build_provider_plan(...)`
  - Produces one review-required plan that records provider names and secret
    paths without resolving any secret values.
- `summarize_provider_plan(...)`
  - Returns a redacted review summary for later CLI or workflow wiring.

## Not Yet Wired

- No provider-specific fetch logic yet.
- No CLI flags or runtime integration yet.
- No Rust parity yet.
