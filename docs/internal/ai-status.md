# ai-status.md

## 2026-03-10 - Task: Extend Grafana Alerting Resource Coverage
- State: Done
- Scope: `grafana-alert-utils.py`, `tests/test_grafana_alert_utils.py`, `README.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `grafana-alert-utils.py` already exports and imports alert rules, contact points, mute timings, and notification policies, and it can repair linked alert-rule dashboard UIDs by matching exported dashboard metadata. It does not yet cover notification templates, manual dashboard UID maps, or panel ID maps.
- Current Update: Added notification template export/import support, including version-aware template updates on `--replace-existing` and empty-list handling when Grafana returns `null`. Added `--dashboard-uid-map` and `--panel-id-map` so linked alert rules can be remapped explicitly during import before the existing metadata fallback logic runs. Exported linked-dashboard metadata now also captures panel title and panel type when available, and the README now documents the new alerting resource scope and mapping-file usage.
- Result: The standalone alert CLI now covers templates in addition to the existing alerting resources, supports operator-provided dashboard and panel remapping files for linked rules, and keeps the older dashboard-title/folder/slug fallback for cases where no explicit map is provided.

## 2026-03-10 - Task: Rename Grafana Dashboard Export Flag
- State: Done
- Scope: `grafana-utils.py`, `tests/test_dump_grafana_dashboards.py`, `README.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The dashboard export subcommand uses `--output-dir`, which is generic enough to be confused with import behavior now that the CLI has explicit import and export modes.
- Current Update: Renamed the dashboard export flag to `--export-dir`, updated the parsed attribute and help text, and changed dashboard README examples and tests to use the more explicit export-only name.
- Result: The dashboard CLI now uses `--export-dir` for export mode, which better matches the subcommand and reduces mode confusion.

## 2026-03-10 - Task: Add Grafana Dashboard Import and Export Subcommands
- State: Done
- Scope: `grafana-utils.py`, `tests/test_dump_grafana_dashboards.py`, `README.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `grafana-utils.py` decides between export and import implicitly by checking whether `--import-dir` is present, so export-only and import-only flags live in the same top-level parser and can be confused.
- Current Update: Split the dashboard CLI into explicit `export` and `import` subcommands, moved mode-specific flags onto the matching subparser, and added maintainer comments in the parser setup explaining why the split exists. README examples now call the subcommands directly.
- Result: Operators must now choose import or export explicitly at the command line, which removes the ambiguous mode inference and makes misuse harder.

## 2026-03-10 - Task: Change Grafana Default Server URL
- State: Done
- Scope: `grafana-utils.py`, `grafana-alert-utils.py`, `tests/test_dump_grafana_dashboards.py`, `tests/test_grafana_alert_utils.py`, `README.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Both utilities default to `https://10.21.104.120`, which assumes a specific remote server instead of a local Grafana instance.
- Current Update: Changed both CLI defaults to `http://127.0.0.1:3000`, added unit tests that assert the new parse-time default, and updated README command examples to match.
- Result: Operators now get a local Grafana default out of the box and can still override it with `--url` when targeting another instance.

## 2026-03-10 - Task: Make Grafana Utilities RHEL 8 Python Compatible
- State: Done
- Scope: `grafana-utils.py`, `grafana-alert-utils.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Both utility scripts use `from __future__ import annotations`, PEP 585 built-in generics like `list[str]`, and PEP 604 unions like `str | None`, which Python 3.6 on RHEL 8 cannot parse.
- Current Update: Replaced those annotations with `typing` module equivalents such as `List[...]`, `Dict[...]`, `Optional[...]`, and `Tuple[...]`, removed the unsupported future import so both scripts remain parseable on Python 3.6 without changing behavior, added parser-level tests that validate both entrypoints against Python 3.6 grammar, and documented RHEL 8+ support in the README.
- Result: The dashboard and alerting utilities now avoid Python 3.9+/3.10+ annotation syntax, explicitly document RHEL 8+ support, and have automated syntax checks that keep them compatible with RHEL 8's default Python parser.

## 2026-03-10 - Task: Add Grafana Alerting Utility
- State: Done
- Scope: `grafana-alert-utils.py`, `tests/test_grafana_alert_utils.py`, `README.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Alert rules are not supported. The workspace only has dashboard export/import tooling in `grafana-utils.py`.
- Current Update: Expanded the standalone alerting CLI so it now exports and imports four resource types under `alerts/raw/`: rules, contact points, mute timings, and notification policies. Import uses create by default, switches to update with `--replace-existing` for rules/contact points/mute timings, and always applies the notification policy tree with `PUT`. The current increment adds alert-rule linkage metadata export for `__dashboardUid__`/`__panelId__`, plus import-time fallback that rewrites missing dashboard UIDs by matching the target Grafana dashboard on exported title/folder/slug metadata. Validation now includes a live Docker scenario where a linked rule was exported from dashboard UID `source-dashboard-uid`, the source dashboard was deleted, a replacement dashboard with UID `target-dashboard-uid` but the same title/folder/slug was created, and alert import rewrote the rule linkage to the new dashboard UID automatically.
- Result: Grafana alerting backup/restore is now separated from `grafana-utils.py` and covers the core alerting resources needed for notifications. The tool rejects Grafana provisioning `/export` files for API import, documents the limitation, has dedicated unit tests, and now preserves or repairs panel-linked alert rules when dashboard UIDs differ across Grafana systems.

## 2026-03-10 - Task: Export Grafana Dashboards
- State: Done
- Scope: `grafana-utils.py`, `tests/test_dump_grafana_dashboards.py`
- Baseline: Workspace is empty and there is no existing Grafana export utility.
- Current Update: Added `--without-raw` and `--without-prompt` so operators can selectively suppress one export variant while keeping the dual-export default. The exporter now rejects disabling both at once.
- Result: The tool now supports both workflows: export both variants by default, or export only `raw/` or only `prompt/` when needed. API import still requires an explicit path and should point at `raw/`.
