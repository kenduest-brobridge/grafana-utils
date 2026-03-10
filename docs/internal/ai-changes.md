# ai-changes.md

## 2026-03-11 - Refactor Grafana CLI Readability
- Summary: Refactored `cmd/grafana-utils.py` and `cmd/grafana-alert-utils.py` for human readability without changing behavior. The dashboard CLI now uses smaller helpers for dashboard object extraction, datasource lookup and normalization, template-variable rewrite steps, and export index construction. The alerting CLI now uses smaller helpers for linked-dashboard mapping, per-resource export handling, and per-kind import dispatch.
- Tests: No new tests were needed because the refactor preserved behavior. Existing coverage was used to validate the structural changes.
- Test Run: `python3 -m unittest -v` (pass)
- Validation: Local full-suite unit tests passed after the refactor. The resulting top-level flows are shorter and easier to scan, with behavior-sensitive logic moved into named helpers.
- Impact: `cmd/grafana-utils.py`, `cmd/grafana-alert-utils.py`, `docs/internal/ai-status.md`
- Rollback/Risk: Low to moderate risk because logic moved across helper boundaries, but no contracts or CLI behavior were intentionally changed and the existing test suite passed after the refactor.
- Follow-up: If readability needs more work later, the next candidates are normalizing repeated JSON write patterns and grouping the client API methods by resource family.

## 2026-03-11 - Move Grafana CLIs Into cmd
- Summary: Moved the dashboard and alerting CLI entrypoints from the repository root into `cmd/`, updated the scripts' embedded help/output strings to reflect the new invocation paths, and refreshed public and maintainer docs to use `python3 cmd/grafana-utils.py ...` and `python3 cmd/grafana-alert-utils.py ...`.
- Tests: Updated `tests/test_dump_grafana_dashboards.py` and `tests/test_grafana_alert_utils.py` to load the scripts from `cmd/`, and added `tests/__init__.py` so default `unittest` discovery reaches both modules.
- Test Run: `python3 -m unittest -v tests/test_dump_grafana_dashboards.py` (pass); `python3 -m unittest -v tests/test_grafana_alert_utils.py` (pass); `python3 -m unittest -v` (pass)
- Validation: Local unit tests passed after the move. `README.md`, `README.zh-TW.md`, `DEVELOPER.md`, and `AGENTS.md` now point at the new `cmd/` entrypoints, and the documented full-suite test command now discovers all tests.
- Impact: `cmd/grafana-utils.py`, `cmd/grafana-alert-utils.py`, `tests/test_dump_grafana_dashboards.py`, `tests/test_grafana_alert_utils.py`, `tests/__init__.py`, `README.md`, `README.zh-TW.md`, `DEVELOPER.md`, `AGENTS.md`, `docs/internal/ai-status.md`
- Rollback/Risk: Moderate path-change risk for anyone invoking the old root-level scripts directly. The docs and tests now consistently point at `cmd/`, but external automation will need the same path update.
- Follow-up: If backward compatibility is required later, add thin root-level wrapper scripts instead of moving the implementation back out of `cmd/`.

## 2026-03-10 - Extend Grafana Alerting Resource Coverage
- Summary: Extended `grafana-alert-utils.py` beyond rules, contact points, mute timings, and notification policies by adding notification template export/import support, explicit dashboard UID and panel ID mapping files for linked alert rules, and richer linked-dashboard metadata capture during export. Template import now uses the template name as the stable identity, fetches the current template version before `PUT` updates, and tolerates Grafana returning `null` from the template list endpoint when no templates exist.
- Tests: Expanded `tests/test_grafana_alert_utils.py` to cover template export documents, template import payload validation, template create/update conflict handling, parser support for the new mapping flags, empty template list handling, and linked alert-rule rewrite behavior when dashboard and panel maps are provided.
- Test Run: `python3 -m unittest -v` (pass)
- Validation: Local unit tests passed for the full project test target. README was updated to document `alerts/raw/templates/`, `--dashboard-uid-map`, `--panel-id-map`, and template update behavior.
- Impact: `grafana-alert-utils.py`, `tests/test_grafana_alert_utils.py`, `README.md`, `docs/internal/ai-status.md`
- Rollback/Risk: Moderate risk is limited to the standalone alert CLI. Template updates still depend on Grafana's provisioning API behavior and linked-rule automatic fallback only rewrites dashboard UID unless a panel map is supplied.
- Follow-up: If environments rely on template groups or panel IDs that are regenerated during dashboard migration, add an optional live validation flow against Grafana 9.x/10.x in addition to the current unit coverage.

## 2026-03-10 - Rename Grafana Dashboard Export Flag
- Summary: Renamed the dashboard export CLI flag from `--output-dir` to `--export-dir` in `grafana-utils.py`. The change updates the parser, the parsed argument name, the help text, and the dashboard README examples so export mode reads clearly next to the explicit `import` subcommand.
- Tests: Updated the dashboard CLI parse test to assert the default `export_dir` value and reran the dashboard test suite.
- Test Run: `python3 -m unittest tests.test_dump_grafana_dashboards` (pass)
- Validation: Local dashboard unit tests passed and the dashboard CLI help now shows `--export-dir` under the `export` subcommand.
- Impact: `grafana-utils.py`, `tests/test_dump_grafana_dashboards.py`, `README.md`, `docs/internal/ai-status.md`
- Rollback/Risk: Low to moderate operator-facing change because older `--output-dir` dashboard export invocations will no longer parse. The rename makes export intent more explicit.
- Follow-up: None.

## 2026-03-10 - Add Grafana Dashboard Import and Export Subcommands
- Summary: Changed `grafana-utils.py` so dashboard mode selection is explicit at the CLI level. The script now requires `export` or `import` subcommands, and export-only and import-only options live on separate subparsers instead of being mixed together on one parser.
- Tests: Updated the dashboard CLI tests to cover required subcommand selection, export defaults, import parsing, and the export validation path under the new command layout.
- Test Run: `python3 -m unittest tests.test_dump_grafana_dashboards` (pass)
- Validation: Local dashboard unit tests passed and README examples were updated to use `python3 grafana-utils.py export ...` and `python3 grafana-utils.py import ...`.
- Impact: `grafana-utils.py`, `tests/test_dump_grafana_dashboards.py`, `README.md`, `docs/internal/ai-status.md`
- Rollback/Risk: Moderate operator-facing change because old invocations without subcommands will now fail argument parsing. The benefit is that import and export intent is explicit.
- Follow-up: If backward compatibility is needed later, add a deliberate legacy shim rather than returning to implicit mode inference.

## 2026-03-10 - Change Grafana Default Server URL
- Summary: Changed the default Grafana base URL in both utilities from a hardcoded remote host to `http://127.0.0.1:3000`. Updated README examples and added direct unit tests so the new default is locked in.
- Tests: Added parse-args assertions for the default URL in both dashboard and alert utility test suites.
- Test Run: `python3 -m unittest tests.test_dump_grafana_dashboards tests.test_grafana_alert_utils` (pass)
- Validation: Local unit tests passed and README examples now match the CLI defaults.
- Impact: `grafana-utils.py`, `grafana-alert-utils.py`, `tests/test_dump_grafana_dashboards.py`, `tests/test_grafana_alert_utils.py`, `README.md`, `docs/internal/ai-status.md`
- Rollback/Risk: Low risk. This only changes the CLI default target; explicit `--url` values still override it.
- Follow-up: None.

## 2026-03-10 - Make Grafana Utilities RHEL 8 Python Compatible
- Summary: Reworked type annotations in both Grafana utility scripts so they no longer depend on Python 3.9+ built-in generics or Python 3.10+ union syntax. Removed `from __future__ import annotations` and converted signatures and local annotations to `typing.List`, `typing.Dict`, `typing.Optional`, `typing.Set`, and `typing.Tuple`. Latest change: documented RHEL 8+ support in the README and turned the Python 3.6 syntax parse check into permanent unit tests for both entrypoints.
- Tests: Reused the existing dashboard and alerting `unittest` suites to confirm the syntax-only compatibility refactor did not change behavior. Added parser-level unit tests that validate both scripts with `ast.parse(..., feature_version=(3, 6))`.
- Test Run: `python3 -m unittest -v` (pass)
- Validation: Local unit tests passed, both scripts parsed successfully as Python 3.6 grammar, and the README now states RHEL 8+ support explicitly.
- Impact: `grafana-utils.py`, `grafana-alert-utils.py`, `docs/internal/ai-status.md`
- Rollback/Risk: Low risk. This is a syntax-compatibility refactor only; behavior should remain unchanged.
- Follow-up: If RHEL 8 deployment uses a stricter runtime baseline than Python 3.6, validate the full CLI workflows there against the target Grafana instance.

## 2026-03-10 - Add Grafana Alerting Utility
- Summary: Expanded the standalone CLI, `grafana-alert-utils.py`, from rule-only backup/restore into a broader Grafana alerting utility. Export now writes a tool-owned JSON format under `alerts/raw/` with separate subdirectories for rules, contact points, mute timings, and notification policies. Import reads that same format and uses the Grafana alerting provisioning API to create or update rules/contact points/mute timings and to apply the notification policy tree. Latest change: alert-rule exports now capture linked dashboard metadata when a rule carries `__dashboardUid__` / `__panelId__`, and import now repairs `__dashboardUid__` automatically when the source dashboard UID is missing on the target Grafana but a unique dashboard match exists by exported title, folder title, and slug.
- Tests: Added `unittest` coverage for alert CLI argument parsing, auth handling, SSL behavior, per-resource path generation, export-root rejection on import, server-managed field stripping for all supported resource kinds, import payload validation, provisioning-export rejection, resource kind detection, export file/index generation across all resource types, create/update dispatch for rules/contact points/mute timings, policy import safety checks, linked-dashboard metadata preservation, and dashboard-UID fallback rewrite behavior for linked alert rules.
- Test Run: `python3 -m unittest -v` (pass)
- Reason: Unit tests cover the code paths locally, and live validation was performed against a temporary Docker Grafana instance rather than the user's Grafana environment.
- Validation: `python3 -m unittest -v`; updated `README.md`; live Docker validation against Grafana 12.4.1 on `http://127.0.0.1:33000` by creating folder `codex-alert-folder`, alert rule `afflu1oeeir5sd`, contact point `codex-webhook`, mute timing `codex-mute`, and a notification policy tree pointing at them; exporting all resources with `grafana-alert-utils.py`; resetting Grafana state; importing from `/tmp/grafana-alert-export-v2/raw`; and confirming the recreated resources preserved the rule UID, folder UID, rule group, contact point UID, mute timing name, and policy references. Additional live validation created a dashboard-linked alert rule with `__dashboardUid__=\"source-dashboard-uid\"` and `__panelId__=\"7\"`, exported it, deleted the source dashboard, created a same-title same-folder replacement dashboard with UID `target-dashboard-uid`, imported the alert backup, and confirmed the rule annotations were rewritten to `__dashboardUid__=\"target-dashboard-uid\"` while preserving `__panelId__=\"7\"`.
- Impact: `grafana-alert-utils.py`, `tests/test_grafana_alert_utils.py`, `README.md`, `docs/internal/ai-status.md`
- Rollback/Risk: Low to moderate risk. The new tool is isolated from `grafana-utils.py`, but import still depends on the target Grafana having any referenced folders and other alerting dependencies available or being restored in the same import set.
- Follow-up: If needed later, extend the separate alert CLI to cover message templates and other remaining Grafana alerting resources without folding that logic into the dashboard utility.

## 2026-03-10 - Export Grafana Dashboards
- Summary: Added a standalone Python utility to export Grafana dashboards by UID into local JSON files, extended it with import support for recursively loading exported dashboard JSON back into Grafana, and added datasource-prompt export behavior that now follows the import-critical pattern from the provided `1-prompt.json`. Current architecture writes both `dashboards/raw/` and `dashboards/prompt/` by default, with `raw/` intended for preserved-UID/API-safe imports and `prompt/` intended for Grafana web imports that ask for datasource mapping. Latest change: added `--without-raw` and `--without-prompt` so one export run can still be selective when needed, while rejecting the invalid case where both are disabled.
- Tests: Added `unittest` coverage for auth handling, CLI SSL behavior, dual export variant directory layout, variant suppression flags, rejection of disabling all export variants, path generation, pagination, overwrite protection, import file discovery, rejection of the combined export root, import payload shaping, preserved-uid web-import export shape, website-import placeholder export behavior, generic datasource input generation, datasource placeholder object rewriting, conversion of typed datasource variables into import placeholders, creation of import placeholders from datasource template variables, synthesized datasource template variables for single-type dashboards, passthrough handling for untyped datasource variables, passthrough handling for Grafana built-in datasource aliases, resolution of datasource references expressed as plain-string UIDs, and datasource type alias fallback.
- Test Run: `python3 -m unittest -v` (pass)
- Reason: Live Grafana export was not run because this turn did not include usable credentials or a network execution request against the target instance.
- Validation: `python3 -m unittest -v`; updated `README.md`
- Impact: `grafana-utils.py`, `tests/test_dump_grafana_dashboards.py`
- Rollback/Risk: Low risk. Revert by deleting the new utility and test files. Website-import exports with `__inputs` are meant for Grafana’s web UI and are not accepted by the script’s API import mode.
- Follow-up: Run one export and confirm `dashboards/raw/` and `dashboards/prompt/` are both populated, then use `dashboards/raw/` for API imports and `dashboards/prompt/` for Grafana web imports.
