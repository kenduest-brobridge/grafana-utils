# Python Call Hierarchy Reference

This is a maintainer-only reference for the in-repo Python implementation. Use it when tracing parity bugs, test failures, or workflow ownership.

## Core entrypoints

- `python/grafana_utils/unified_cli.py`
- `python/grafana_utils/dashboard_cli.py`
- `python/grafana_utils/datasource_cli.py`
- `python/grafana_utils/alert_cli.py`
- `python/grafana_utils/access_cli.py`

## High-value workflow modules

- `python/grafana_utils/dashboards/export_workflow.py`
- `python/grafana_utils/dashboards/import_workflow.py`
- `python/grafana_utils/dashboards/inspection_workflow.py`
- `python/grafana_utils/datasource/workflows.py`
- `python/grafana_utils/access/workflows.py`
- `python/grafana_utils/alerts/provisioning.py`

## How to use this page

- Start at the domain facade for parser and dispatch behavior.
- Move into the matching workflow module for state changes and dry-run behavior.
- Use `rg` or local call graph tooling when you need a full refreshed call map; this page is intentionally a compact maintainer index, not a generated dump.
