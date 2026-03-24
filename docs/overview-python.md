# Grafana Utils Python Architecture for Maintainers

This page is maintainer-only context for the in-repo Python implementation. It is not part of the user-facing distribution story.

## Purpose

The Python tree remains in the repository for:

- behavior comparison against the Rust CLI
- regression coverage during refactors
- source-tree workflow validation when debugging parity issues

## Entry points

- `python/grafana_utils/__main__.py`: source-tree module entrypoint for `python3 -m grafana_utils`
- `python/grafana_utils/unified_cli.py`: top-level namespaced dispatcher
- `python/grafana_utils/dashboard_cli.py`: dashboard facade
- `python/grafana_utils/datasource_cli.py`: datasource facade
- `python/grafana_utils/alert_cli.py`: alert facade
- `python/grafana_utils/access_cli.py`: access facade

## Module boundaries

- `python/grafana_utils/dashboards/`: export/import/diff/inspect helpers, renderers, and dashboard support code
- `python/grafana_utils/datasource/`: parser and datasource workflow helpers
- `python/grafana_utils/access/`: parser, models, and access workflows
- `python/grafana_utils/alerts/`: alerting provisioning helpers
- `python/grafana_utils/http_transport.py`: transport abstraction shared across Python flows

## Validation commands

- `PYTHONPATH=python python3 -m unittest -v`
- `PYTHONPATH=python python3 -m unittest -v python/tests/test_python_dashboard_cli.py`
- `PYTHONPATH=python python3 -m unittest -v python/tests/test_python_datasource_cli.py`
- `PYTHONPATH=python python3 -m unittest -v python/tests/test_python_alert_cli.py`
- `PYTHONPATH=python python3 -m unittest -v python/tests/test_python_access_cli.py`

## Maintainer guidance

- Keep Python notes and examples in maintainer docs, not README or user guides.
- When Rust behavior changes, decide whether Python should match, intentionally diverge, or only remain as historical validation coverage.
- If Python coverage is still used in CI or local quality gates, keep the commands here current.
