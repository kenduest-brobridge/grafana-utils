# Datasource Live Mutation Design History

Historical note:

- Live datasource mutation is now implemented in both the Python and Rust CLIs.
- Current operator-facing commands are `grafana-util datasource add`,
  `grafana-util datasource modify`, and `grafana-util datasource delete`.
- This file is kept as the original scaffold summary because older change-trace
  entries already reference it by name.

## Purpose

This note tracks the original isolated add/delete implementation scaffold for
live Grafana datasources before CLI wiring landed.

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

## Current Wiring Status

- Python CLI wiring now exists through `grafana_utils/datasource/parser.py`,
  `grafana_utils/datasource_cli.py`, and `grafana_utils/datasource/workflows.py`.
- Rust CLI parity now exists in `rust/src/datasource.rs`.
- User-facing usage belongs in `docs/user-guide.md` and `docs/user-guide-TW.md`,
  not in this historical scaffold note.

## Remaining Limits

- This historical note only captures the original add/delete scaffold.
- Current modify semantics, real parser wiring, and user-facing examples are
  documented elsewhere in the active CLI code and user guides.
