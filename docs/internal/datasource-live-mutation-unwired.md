# Datasource Live Mutation (Unwired)

## Purpose

This note tracks the isolated add/delete implementation scaffold for live
Grafana datasources before CLI wiring lands.

## Scope

- New Python helper module:
  - `grafana_utils/datasource/live_mutation.py`
- New unit tests:
  - `tests/test_python_datasource_live_mutation.py`

## Current Behavior

- `add_datasource(...)`
  - Validates an in-memory add spec.
  - Rejects unsupported fields.
  - Fails closed if an existing datasource already matches by UID or name.
  - Uses `POST /api/datasources` when executed live.
- `delete_datasource(...)`
  - Resolves one live datasource by UID or name.
  - Fails closed for missing or ambiguous targets.
  - Uses `DELETE /api/datasources/<id>` when executed live.
- Both paths support `dry_run=True` and return plan dictionaries instead of
  mutating Grafana.

## Not Yet Wired

- No argparse or unified CLI integration yet.
- No Rust parity yet.
- No update/modify semantics yet.
- No documentation exposed in user-facing guides yet.

## Future Wire Points

- `grafana_utils/datasource/parser.py`
- `grafana_utils/datasource_cli.py`
- `grafana_utils/datasource/workflows.py`
