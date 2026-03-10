# ai-status.md

## 2026-03-10 - Task: Change Grafana Default Server URL
- State: Done
- Scope: `grafana-utils.py`, `grafana-alert-utils.py`, `test_dump_grafana_dashboards.py`, `test_grafana_alert_utils.py`, `README.md`, `ai-status.md`, `ai-changes.md`
- Baseline: Both utilities default to `https://10.21.104.120`, which assumes a specific remote server instead of a local Grafana instance.
- Current Update: Changed both CLI defaults to `http://127.0.0.1:3000`, added unit tests that assert the new parse-time default, and updated README command examples to match.
- Result: Operators now get a local Grafana default out of the box and can still override it with `--url` when targeting another instance.

## 2026-03-10 - Task: Make Grafana Utilities RHEL 8 Python Compatible
- State: Done
- Scope: `grafana-utils.py`, `grafana-alert-utils.py`, `ai-status.md`, `ai-changes.md`
- Baseline: Both utility scripts use `from __future__ import annotations`, PEP 585 built-in generics like `list[str]`, and PEP 604 unions like `str | None`, which Python 3.6 on RHEL 8 cannot parse.
- Current Update: Replaced those annotations with `typing` module equivalents such as `List[...]`, `Dict[...]`, `Optional[...]`, and `Tuple[...]`, and removed the unsupported future import so both scripts remain parseable on Python 3.6 without changing behavior.
- Result: The dashboard and alerting utilities now avoid Python 3.9+/3.10+ annotation syntax and are compatible with RHEL 8's default Python parser.

## 2026-03-10 - Task: Add Grafana Alert Rule Utility
- State: Done
- Scope: `grafana-alert-utils.py`, `test_grafana_alert_utils.py`, `README.md`, `ai-status.md`, `ai-changes.md`
- Baseline: Alert rules are not supported. The workspace only has dashboard export/import tooling in `grafana-utils.py`.
- Current Update: Added a standalone alert-rule CLI that exports one normalized JSON file per rule under `alerts/raw/` and re-imports the same format through Grafana's alerting provisioning API. Import uses create by default and switches to update when `--replace-existing` is set and the UID already exists. Validation now includes a live Docker-based Grafana 12.4.1 round-trip: created a folder and alert rule, exported via `grafana-alert-utils.py`, deleted the rule through Grafana API, then re-imported it successfully with the same UID and folder metadata.
- Result: Alert rules now have separate export/import support without expanding `grafana-utils.py`. The tool rejects Grafana provisioning `/export` files for API import, documents the limitation, has dedicated unit tests, and has passed one real Grafana container round-trip check.

## 2026-03-10 - Task: Export Grafana Dashboards
- State: Done
- Scope: `grafana-utils.py`, `test_dump_grafana_dashboards.py`
- Baseline: Workspace is empty and there is no existing Grafana export utility.
- Current Update: Added `--without-raw` and `--without-prompt` so operators can selectively suppress one export variant while keeping the dual-export default. The exporter now rejects disabling both at once.
- Result: The tool now supports both workflows: export both variants by default, or export only `raw/` or only `prompt/` when needed. API import still requires an explicit path and should point at `raw/`.
