# ai-status.md

## 2026-03-10 - Task: Export Grafana Dashboards
- State: Done
- Scope: `grafana-utils.py`, `test_dump_grafana_dashboards.py`
- Baseline: Workspace is empty and there is no existing Grafana export utility.
- Current Update: Added `--without-raw` and `--without-prompt` so operators can selectively suppress one export variant while keeping the dual-export default. The exporter now rejects disabling both at once.
- Result: The tool now supports both workflows: export both variants by default, or export only `raw/` or only `prompt/` when needed. API import still requires an explicit path and should point at `raw/`.
