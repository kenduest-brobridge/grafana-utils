# Datasource Secret Handling (Unwired)

## Purpose

This note tracks the isolated placeholder-based datasource secret scaffold
before any CLI, bundle, or provider wiring lands.

## Scope

- New Python helper module:
  - `grafana_utils/datasource_secret_workbench.py`
- New unit tests:
  - `tests/test_python_datasource_secret_workbench.py`

## Current Behavior

- `collect_secret_placeholders(...)`
  - Accepts only `${secret:...}` placeholder strings inside
    `secureJsonDataPlaceholders`.
  - Rejects raw secret values or non-string objects so secret-bearing payloads
    cannot be replayed opaquely.
- `resolve_secret_placeholders(...)`
  - Resolves placeholders only from an explicit in-memory mapping.
  - Fails closed when a placeholder is missing or resolves to an empty value.
- `build_datasource_secret_plan(...)`
  - Produces one review-required plan object with resolved `secureJsonData`.
  - Keeps provider behavior explicit as `inline-placeholder-map`.

## Not Yet Wired

- No argparse or unified CLI integration yet.
- No datasource import/live-mutation workflow integration yet.
- No external secret provider support yet.
- No Rust parity implementation yet; parity remains a later wire-up concern.

## Future Wire Points

- `grafana_utils/datasource/parser.py`
- `grafana_utils/datasource/workflows.py`
- `rust/src/datasource.rs`
