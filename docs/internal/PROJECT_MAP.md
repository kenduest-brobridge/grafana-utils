# Project Map (AI Fast Path)

## Scope
This document is a fast-entry map for agents so they can avoid full repository rescans. It focuses on the highest-impact modules, call flow, and edit targets.

## Core Entry Points
- `grafana_utils/unified_cli.py`: primary Python dispatcher (`grafana-util ...`).
- `grafana_utils/__main__.py`: source-tree module entrypoint.
- `grafana_utils/dashboard_cli.py`: dashboard command implementation.
- `grafana_utils/alert_cli.py`: alert command implementation.
- `grafana_utils/access_cli.py`: access command implementation.
- `grafana_utils/http_transport.py`: shared HTTP transport abstraction and selection.
- `rust/src/bin/grafana-util.rs`: Rust binary entrypoint.
- `rust/src/access_org.rs`: Rust access org workflows.

## Execution Flow
1. Operator runs `grafana-util <group> <command> ...`.
2. `grafana_utils/unified_cli.py` selects module (`dashboard` / `alert` / `access` / `datasource` / `sync`).
3. Group CLI parses args and dispatches workflow functions.
4. Workflow uses `grafana_utils/clients/*_client.py`.
5. Clients route requests through `grafana_utils/http_transport.py`.
6. Tests assert parser UX, workflow behavior, and transport/client integration.

## Where To Edit
| Task | Primary files | Usually also touch |
|---|---|---|
| Dashboard behavior | `grafana_utils/dashboard_cli.py`, `grafana_utils/dashboards/*` | `tests/test_python_dashboard_cli.py`, `tests/test_python_dashboard_inspection_cli.py` |
| Alert behavior | `grafana_utils/alert_cli.py`, `grafana_utils/alerts/*` | `tests/test_python_alert_cli.py`, `tests/test_python_alert_sync_workbench.py` |
| Access behavior | `grafana_utils/access_cli.py`, `grafana_utils/access/parser.py`, `grafana_utils/access/workflows.py` | `tests/test_python_access_cli.py`, `tests/test_python_access_pending_cli_staging.py` |
| Unified routing/help | `grafana_utils/unified_cli.py`, `grafana_utils/__main__.py` | `tests/test_python_unified_cli.py`, related group parser/help tests |
| HTTP behavior | `grafana_utils/http_transport.py`, `grafana_utils/clients/*` | `tests/test_python_datasource_client.py`, any CLI tests touching transport flags |
| Rust access org | `rust/src/access_org.rs`, `rust/src/access.rs`, `rust/src/access_cli_defs.rs` | `rust/src/access_rust_tests.rs`, `cd rust && cargo test --quiet` |

## Minimal Read Order
1. `docs/context/PROJECT_MAP.md`
2. `docs/context/CHANGE_IMPACT.md`
3. `docs/context/FILE_INDEX.json`
4. Target module file(s)
5. Directly owned test file(s)

## Quick Commands
- Smallest relevant Python target first:
  - `poetry run python -m unittest -v tests/test_python_access_cli.py`
  - `poetry run python -m unittest -v tests/test_python_alert_cli.py`
  - `poetry run python -m unittest -v tests/test_python_dashboard_cli.py`
- Broader Python validation:
  - `poetry run python -m unittest -v`
- Rust validation:
  - `cd rust && cargo test --quiet`
- Full suite:
  - `make test`
