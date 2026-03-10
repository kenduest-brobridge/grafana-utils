# ai-status.md

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
