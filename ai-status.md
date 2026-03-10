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

## 2026-03-10 - Task: Add Grafana Alerting Utility
- State: Done
- Scope: `grafana-alert-utils.py`, `test_grafana_alert_utils.py`, `README.md`, `ai-status.md`, `ai-changes.md`
- Baseline: Alert rules are not supported. The workspace only has dashboard export/import tooling in `grafana-utils.py`.
- Current Update: Expanded the standalone alerting CLI so it now exports and imports four resource types under `alerts/raw/`: rules, contact points, mute timings, and notification policies. Import uses create by default, switches to update with `--replace-existing` for rules/contact points/mute timings, and always applies the notification policy tree with `PUT`. Validation now includes a live Docker-based Grafana 12.4.1 multi-resource round-trip: exported all four resource types, reset Grafana state, then re-imported them successfully with preserved rule UID, folder metadata, contact point UID, mute timing name, and policy references.
- Result: Grafana alerting backup/restore is now separated from `grafana-utils.py` and covers the core alerting resources needed for notifications. The tool rejects Grafana provisioning `/export` files for API import, documents the limitation, has dedicated unit tests, and has passed real Grafana container validation for all supported resource kinds.

## 2026-03-10 - Task: Export Grafana Dashboards
- State: Done
- Scope: `grafana-utils.py`, `test_dump_grafana_dashboards.py`
- Baseline: Workspace is empty and there is no existing Grafana export utility.
- Current Update: Added `--without-raw` and `--without-prompt` so operators can selectively suppress one export variant while keeping the dual-export default. The exporter now rejects disabling both at once.
- Result: The tool now supports both workflows: export both variants by default, or export only `raw/` or only `prompt/` when needed. API import still requires an explicit path and should point at `raw/`.
