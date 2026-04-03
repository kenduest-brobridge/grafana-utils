# Change Impact Matrix

Use this matrix to avoid repo-wide rescans. Start from the changed module, then follow linked files/tests/docs.

## Smallest Test First
1. Run the narrowest `unittest` module that owns the changed behavior.
2. Run adjacent module tests when shared parser/transport/client paths are touched.
3. Run broad Python suite (`poetry run python -m unittest -v`) when changes cross command groups.
4. Run Rust suite (`cd rust && cargo test --quiet`) for Rust module changes.
5. Run `make test` only when Python+Rust behavior both changed.

## Matrix
| If you change... | Then inspect/touch... | Smallest test first | Broader follow-up |
|---|---|---|---|
| `grafana_utils/unified_cli.py` | `grafana_utils/__main__.py`, group CLI modules (`dashboard_cli.py`, `alert_cli.py`, `access_cli.py`, `datasource_cli.py`, `sync_cli.py`) | `poetry run python -m unittest -v tests/test_python_unified_cli.py` | Group-specific tests + `poetry run python -m unittest -v` |
| `grafana_utils/dashboard_cli.py` | `grafana_utils/dashboards/*`, `grafana_utils/clients/dashboard_client.py`, `grafana_utils/http_transport.py` | `poetry run python -m unittest -v tests/test_python_dashboard_cli.py` | `tests/test_python_dashboard_inspection_cli.py`, `tests/test_python_dashboard_integration_flow.py`, full Python suite |
| `grafana_utils/alert_cli.py` | `grafana_utils/alerts/*`, `grafana_utils/clients/alert_client.py`, `grafana_utils/http_transport.py` | `poetry run python -m unittest -v tests/test_python_alert_cli.py` | `tests/test_python_alert_sync_workbench.py`, full Python suite |
| `grafana_utils/access_cli.py` | `grafana_utils/access/parser.py`, `grafana_utils/access/workflows.py`, `grafana_utils/clients/access_client.py`, `grafana_utils/http_transport.py` | `poetry run python -m unittest -v tests/test_python_access_cli.py` | `tests/test_python_access_pending_cli_staging.py`, `tests/test_python_unified_cli.py`, full Python suite |
| `grafana_utils/access/parser.py` | `grafana_utils/access_cli.py`, CLI help docs in `README.md` or `docs/user-guide*.md` if UX changed | `poetry run python -m unittest -v tests/test_python_access_cli.py` | `tests/test_python_unified_cli.py`, full Python suite |
| `grafana_utils/access/workflows.py` | `grafana_utils/access_cli.py`, `grafana_utils/access/models.py`, `grafana_utils/clients/access_client.py` | `poetry run python -m unittest -v tests/test_python_access_cli.py` | Relevant integration-like tests + full Python suite |
| `grafana_utils/http_transport.py` | `grafana_utils/clients/*_client.py`, all CLI modules that expose `--http-transport` | `poetry run python -m unittest -v tests/test_python_datasource_client.py` | `tests/test_python_access_cli.py`, `tests/test_python_alert_cli.py`, `tests/test_python_dashboard_cli.py`, full Python suite |
| `rust/src/access_org.rs` | `rust/src/access.rs`, `rust/src/access_cli_defs.rs`, `rust/src/access_render.rs` | `cd rust && cargo test --quiet access` | `cd rust && cargo test --quiet` |
| `rust/src/cli.rs` or `rust/src/bin/grafana-util.rs` | Rust command modules and CLI def files | `cd rust && cargo test --quiet cli` | `cd rust && cargo test --quiet` |
| `pyproject.toml` / entrypoints | `grafana_utils/unified_cli.py`, `grafana_utils/__main__.py`, packaging tests | `poetry run python -m unittest -v tests/test_python_packaging.py` | full Python suite + install smoke checks |
| `Makefile` test/build targets | scripts and documented commands in README/docs | run changed target directly | `make test` |

## Docs To Update When Behavior Changes
- Operator-facing command/flag/output changes: `README.md`, `README.zh-TW.md`, `docs/user-guide.md`, `docs/user-guide-TW.md`.
- Internal mapping or maintenance behavior: `docs/DEVELOPER.md`.
- Meaningful feature or architecture changes: `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`.

## Rebuild Context Index
- After meaningful module/test mapping changes, run:
  - `scripts/update_context_index.sh`
