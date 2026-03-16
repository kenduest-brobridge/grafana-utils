# ai-status.md

Historical note:

- Older entries describe the repo state and `TODO.md` backlog as they existed on the entry date.
- `TODO.md` now tracks only the active backlog; completed or superseded TODO items moved to `docs/internal/todo-archive.md`.

## 2026-03-16 - Task: Let Rust Bundle Preflight Fall Back To Raw Alert Rule Documents
- State: Done
- Scope: `rust/src/sync_bundle_preflight.rs`, `rust/src/sync_bundle_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Rust bundle-preflight only consumed top-level `alerts` from a source bundle. When a bundle still carried alert exports only under `alerting.rules[*].document`, the staged sync-preflight path ignored those alert rules entirely and missed alert datasource and contact-point dependency checks.
- Current Update: Added a Rust bundle-preflight fallback that derives minimal staged alert sync specs from raw alert rule export documents under `alerting.rules`, while still preferring explicit top-level `alerts` when those are already present.
- Result: Rust bundle-preflight now surfaces alert dependency failures from bundled raw alert rule exports even before the source-bundle builder fully materializes top-level alert specs, which closes most of the remaining alert-dependency blind spot in the current bundle contract.

## 2026-03-16 - Task: Normalize Rust Source Bundle Alert Specs
- State: Done
- Scope: `rust/src/sync.rs`, `rust/src/sync_workbench.rs`, `rust/src/sync_cli_rust_tests.rs`, `rust/src/sync_bundle_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Rust `sync bundle` already carried raw `alerting.rules[*].document` exports, but the emitted source bundle still left top-level `alerts` empty. That meant downstream staged sync flows had to rely on bundle-preflight fallbacks instead of receiving normalized alert sync specs directly from the source-bundle artifact.
- Current Update: Added Rust alert normalization that derives top-level alert sync specs from bundled alert rule exports, supports both `grafana-alert-rule` tool documents and raw `groups[].rules[]` documents, carries safe dependency-oriented fields such as `condition`, `annotations`, `contactPoints`, `datasourceUids`, `datasourceNames`, `pluginIds`, and `data`, and passes those normalized alerts into `build_sync_source_bundle_document(...)`.
- Result: Rust source bundles now emit non-empty top-level `alerts` when the alert export contains enough rule detail, which lets downstream bundle review and staged sync paths consume normalized alert specs directly instead of depending solely on raw `alerting` documents.

## 2026-03-16 - Task: Clarify Remaining Rust Sync Bundle Alert Gap
- State: Done
- Scope: `TODO.md`, `docs/DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The active backlog still described the Rust export package/bundle work as if the whole workflow were missing, even though the current tree already packages dashboards, folders, datasource inventory, bundle metadata, and raw `alerting` sections. That wording no longer pointed maintainers at the actual remaining alert-side gap.
- Current Update: Narrowed the backlog and maintainer notes to the real remaining Rust sync bundle problem: deriving safe normalized top-level alert sync specs from raw alert exports so bundle-preflight and downstream sync stages can consume `alerts` directly instead of relying only on raw `alerting` documents.
- Result: The maintainer docs and active backlog now describe the bundle workflow as partially complete and identify the remaining alert normalization step as the next focused Rust gap.

## 2026-03-16 - Task: Tighten Rust Sync Bundle And Preflight Dependency Coverage
- State: Done
- Scope: `rust/src/sync.rs`, `rust/src/sync_preflight.rs`, `rust/src/sync_workbench.rs`, `rust/src/sync_rust_tests.rs`, `rust/src/sync_cli_rust_tests.rs`, `rust/src/sync_bundle_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Rust sync preflight already checked datasource uid/name availability plus datasource plugin types for datasource resources, and the first-pass Rust `sync bundle` workflow could package dashboards, datasource inventory, and raw alerting sections. But dashboard/alert specs could not declare their own plugin dependencies in preflight, bundled datasource inventory dropped secret-provider metadata needed by provider assessment, and the bundle CLI tests did not prove that provider metadata survived into downstream bundle-preflight evaluation.
- Current Update: Added optional dashboard and alert `pluginIds` checks to Rust sync preflight so staged specs can block on missing plugin availability before mutation. Preserved `secureJsonDataProviders` and `secureJsonDataPlaceholders` when normalizing datasource inventory into the Rust source bundle, recorded `alertExportDir` in bundle metadata, and added focused Rust tests that verify bundle output preserves provider metadata and that bundle-preflight can read provider references back out of a source-bundle document.
- Result: Rust sync preflight now covers one broader class of non-datasource plugin dependencies, and the Rust source-bundle workflow now carries enough datasource provider metadata for bundle-preflight/provider assessment to work against bundle artifacts instead of only hand-built fixture documents.

## 2026-03-16 - Task: Exercise Rust Sync Bundle Alert Normalization Paths
- State: Done
- Scope: `rust/src/sync_cli_rust_tests.rs`, `rust/src/sync_bundle_rust_tests.rs`, `scripts/test-rust-live-grafana.sh`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Rust sync bundle and bundle-preflight coverage already proved datasource provider metadata and alert export directory provenance, but they still did not exercise the expected top-level `alerts` normalization path end to end. The live smoke harness also had no `grafana-util sync ...` path at all.
- Current Update: Added bundle CLI test coverage with a realistic raw alert-rule export fixture that asserts the source bundle carries a normalized top-level alert spec, added bundle-preflight coverage that counts a source bundle containing alert specs alongside dashboard and datasource records, and extended the Rust live smoke script to package the seeded dashboard/alert exports through `grafana-util sync bundle` plus `sync bundle-preflight`.
- Result: Once the source-bundle normalization path is present, the checked-in Rust tests and live smoke harness now exercise it directly instead of only validating adjacent alerting summary/provider metadata behavior.

## 2026-03-16 - Task: Split Rust Full-Page Screenshot Output
- State: Done
- Scope: `rust/src/dashboard.rs`, `rust/src/dashboard_cli_defs.rs`, `rust/src/dashboard_screenshot.rs`, `rust/src/dashboard_rust_tests.rs`, `docs/user-guide.md`, `docs/user-guide-TW.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Rust `dashboard screenshot --full-page` always stitched the entire page into one raster file. That worked, but very tall dashboards produced oversized PNG/JPEG artifacts and there was no supported way to emit smaller segmented files.
- Current Update: Added `--full-page-output single|tiles|manifest` to the Rust screenshot CLI. The runtime now captures full-page segments once, then either stitches them into the existing single-file output, writes numbered `part-0001.*` tiles into a derived output directory, or writes the same tile set plus `manifest.json` metadata with viewport/crop/scroll details. Validation now rejects `tiles` or `manifest` without `--full-page`, and rejects PDF split output because PDF remains single-file only.
- Result: Operators can keep the old one-file behavior by default or switch long dashboard captures to multi-file output that is easier to store, review, and post-process without changing the existing screenshot route for normal captures.

## 2026-03-16 - Task: Add Rust Screenshot Device Scale Factor
- State: Done
- Scope: `rust/src/dashboard_cli_defs.rs`, `rust/src/dashboard_screenshot.rs`, `rust/src/dashboard_rust_tests.rs`, `docs/user-guide.md`, `docs/user-guide-TW.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Rust screenshot capture exposed viewport `--width` and `--height`, but it had no Python-style `--device-scale-factor`. Operators could make the CSS viewport bigger, but they could not request denser PNG/JPEG output without also changing layout width and height.
- Current Update: Added `--device-scale-factor` with default `1.0`, validated it as greater than zero, and wired Chromium `Emulation.setDeviceMetricsOverride` into the Rust screenshot runtime. Updated full-page clip/stitch math to convert crop and scroll offsets from CSS pixels into output pixels so high-density full-page captures stay aligned.
- Result: Rust screenshot capture now accepts commands such as `--device-scale-factor 2 --width 2200 --height 1600`, including full-page PNG/JPEG output at higher raster density without changing the dashboard's CSS viewport geometry.

## 2026-03-16 - Task: Add Rust Dashboard Screenshot Command
- State: Done
- Scope: `rust/Cargo.toml`, `rust/src/cli.rs`, `rust/src/dashboard.rs`, `rust/src/dashboard_cli_defs.rs`, `rust/src/dashboard_screenshot.rs`, `rust/src/dashboard_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Rust dashboard CLI currently supports list/export/import/diff/inspect workflows only. There is no screenshot or PDF capture subcommand, no browser automation runtime, and no code path that can capture the rendered Grafana dashboard UI as an image or PDF.
- Current Update: Added a Rust-only `dashboard screenshot` workflow with clap args for dashboard UID, slug, panel id, time range, repeatable `--var`, theme, viewport, wait time, browser path, org header, and output format selection. Added a new browser-capture module that launches Chromium through `headless_chrome`, reuses the existing auth header builder, constructs dashboard or solo-panel URLs, reserves a local debug port explicitly, hides Grafana chrome before capture, derives a clip from live page dimensions for `--full-page`, and writes PNG, JPEG, or PDF output after a render wait. Fixed the initial screenshot bugs where the capture path passed the wrong `headless_chrome` flag and where full-page dimension reads expected a JSON object instead of primitive values, then tightened the DOM cleanup to collapse the left menu first, hide sticky/fixed top bars more aggressively, and preserve datasource-style template variable routing through repeatable `--var NAME=VALUE` query pairs.
- Result: The Rust dashboard CLI now supports `grafana-util dashboard screenshot ...` through unified dispatch, the new screenshot helper behavior is covered by focused parser/help/URL/validation tests, the Rust `dashboard` plus `cli` test targets pass, and live checks against both local Docker Grafana and the user-provided remote dashboard produced browser-rendered full-page PNG output with the sidebar removed while keeping dark-mode and template-variable-driven dashboard selection intact.

## 2026-03-16 - Task: Add Python Dashboard Inspect Vars And Screenshot Commands
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `grafana_utils/unified_cli.py`, `grafana_utils/dashboards/variable_inspection.py`, `grafana_utils/dashboards/screenshot.py`, `tests/test_python_dashboard_variable_inspection.py`, `tests/test_python_dashboard_screenshot_helper.py`, `tests/test_python_dashboard_capture_cli.py`, `tests/test_python_unified_cli_dashboard_capture.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Python dashboard support already covered export/list/import/diff/inspect-export/inspect-live, but it still lacked Rust's `dashboard inspect-vars` and `dashboard screenshot` commands. Python unified CLI also did not expose those dashboard subcommands or the short `db` / `sy` aliases that Rust already supported.
- Current Update: Added a Python live variable-inspection helper that resolves dashboard UID from `--dashboard-uid` or Grafana URLs, extracts `templating.list`, applies `vars-query` overlays, and renders table/CSV/JSON output. Added a Python screenshot helper that validates capture args, parses `--var` and `vars-query` state, builds `/d/...` and `/d-solo/...` capture URLs, preserves a `/render/...` builder for future/fallback use, and now defaults to a browser-driven Chromium DevTools flow that injects Grafana auth headers directly into the page session instead of routing through `/render`. Follow-up work on the same path added live slug fail-open for direct `--dashboard-url` capture, Rust-style DOM readiness checks, sidebar/topbar cleanup, a Rust-style full-page scroll-and-stitch PNG/JPEG path so long dashboards no longer rely on one giant browser clip, and Python runtime support for Rust-like screenshot metadata/header fields (`print-capture-url`, header title/url/captured-at/text) on raster outputs.
- Result: The Python CLI now accepts `grafana-util dashboard inspect-vars ...` and `grafana-util dashboard screenshot ...`, the unified Python entrypoint routes both commands through the dashboard namespace, focused Python screenshot/CLI tests pass, and live checks against `https://192.168.1.112:3000` succeeded for both variable inspection and browser-driven PNG capture without using Grafana `/render`, including stitched full-page PNG output for the long `node-exporter-full` dashboard URL. The screenshot runtime now also has the image-header composition path needed for Rust-style title/url/captured-at annotations on PNG/JPEG outputs.

## 2026-03-16 - Task: Recover Sync And Workbench Modules After Reverse
- State: Done
- Scope: `grafana_utils/unified_cli.py`, `grafana_utils/sync_cli.py`, `grafana_utils/gitops_sync.py`, `grafana_utils/*workbench.py`, `tests/test_python_unified_cli.py`, `tests/test_python_sync_cli.py`, `tests/test_python_*workbench.py`, `rust/src/lib.rs`, `rust/src/cli.rs`, `rust/src/sync.rs`, `rust/src/sync_*.rs`, `rust/src/alert_sync.rs`, `rust/src/bundle_preflight.rs`, `rust/src/datasource_provider.rs`, `rust/src/cli_rust_tests.rs`, `docs/internal/*.md`, `VERSION`, `scripts/set-version.sh`
- Baseline: The current tree had lost the Rust sync command surface plus its supporting sync/preflight/provider modules, and the same reverse also removed the Python sync/workbench modules, their tests, and several internal design notes. Rust unified CLI help and dispatch no longer exposed `sync`, while Python unified CLI also no longer routed `grafana-util sync ...`.
- Current Update: Restored the deleted Python and Rust sync/workbench module files from the earlier dev commit, reconnected Python unified CLI routing for `grafana-util sync ...`, and reconnected Rust unified CLI routing for `grafana-util sync ...` while keeping the requested canonical-only Rust rule that rejects legacy direct forms. Preserved the short Rust namespace aliases `db` and `sy`.
- Result: The repo again contains the staged sync/preflight/workbench implementation set in both runtimes, Python and Rust unified CLIs both route `sync`, and the focused recovered test suites pass.

## 2026-03-16 - Task: Remove Rust Unified CLI Legacy Direct Commands
- State: Done
- Scope: `rust/src/cli.rs`, `rust/src/dashboard_cli_defs.rs`, `rust/src/cli_rust_tests.rs`, `rust/src/dashboard_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Rust unified CLI still accepted top-level legacy direct commands such as `list-dashboard` and `export-alert`, and `--help` still advertised compatibility direct forms. That regressed the intended canonical command shape where operators should use `grafana-util <module> <command>`.
- Current Update: Removed Rust top-level legacy direct routing and dashboard command aliases for `list-dashboard`, `export-dashboard`, and `import-dashboard`. Kept a short namespace alias so `grafana-util db ...` still routes to `dashboard ...`, and updated focused Rust parser/help tests to reject the removed direct forms while accepting `db`.
- Result: The Rust unified CLI now enforces namespaced commands such as `grafana-util dashboard list` or `grafana-util db list`, and its help output no longer reintroduces removed compatibility direct forms.

## 2026-03-15 - Task: Align Shared CLI Help And User Guides
- State: Done
- Scope: `grafana_utils/unified_cli.py`, `grafana_utils/datasource/parser.py`, `tests/test_python_unified_cli.py`, `tests/test_python_datasource_cli.py`, `rust/src/cli.rs`, `rust/src/cli_rust_tests.rs`, `docs/user-guide.md`, `docs/user-guide-TW.md`, `docs/DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The shared user guides still framed examples around Rust source-tree commands, datasource help examples were inconsistent between root and subcommand help, and the Traditional Chinese user guide contained malformed Markdown tables plus mixed terminology around legacy compatibility paths.
- Current Update: Switched shared user-guide examples to the neutral `grafana-util ...` / `grafana-access-utils ...` command shape, refreshed unified CLI help text to describe legacy entrypoints as compatibility forms without runtime warnings, expanded datasource root/subcommand help examples, and repaired the malformed Markdown tables plus terminology in the Traditional Chinese guide.
- Result: Operators now see one shared CLI shape in the public guides, datasource help output includes actionable examples at both the group and subcommand level, and the compatibility-path wording stays visible in help/docs without changing legacy command behavior.

## 2026-03-15 - Task: Add Python Datasource Org-Scoped Export And Routed Import
- State: Done
- Scope: `grafana_utils/datasource/parser.py`, `grafana_utils/datasource_cli.py`, `grafana_utils/datasource/workflows.py`, `tests/test_python_datasource_cli.py`, `docs/DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Python datasource export stayed current-org-only, and datasource import only supported the current org or one explicit `--org-id`. There was no Python support for `datasource export --org-id`, `datasource export --all-orgs`, `datasource import --use-export-org`, repeatable `--only-org-id`, or `--create-missing-orgs`.
- Current Update: Added Python datasource export org scoping with Basic-auth-only `--org-id` and `--all-orgs`, writing `org_<id>_<name>/` subdirectories plus one aggregate root manifest for multi-org exports. Added Python datasource routed import with `--use-export-org`, repeatable `--only-org-id`, and `--create-missing-orgs`, including org-level dry-run preview for `missing-org` and `would-create-org`.
- Result: The Python datasource CLI now supports the same explicit-org and routed-org operator model already used by dashboard import/export, while preserving the existing single-org datasource export/import behavior when the new flags are not used.

## 2026-03-15 - Task: Add Routed Dashboard Import Live Smoke Coverage
- State: Done
- Scope: `scripts/test-rust-live-grafana.sh`, `docs/DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The existing Rust live Grafana smoke script covered single-org dashboard export/import/diff/dry-run and alerting flows, but it did not exercise routed dashboard import from a combined multi-org export root. There was no live smoke path for `--use-export-org`, `--only-org-id`, `--create-missing-orgs`, or the new routed dry-run org preview semantics.
- Current Update: Extended the Rust live smoke script to create a second org and dashboard, export dashboards with `--all-orgs`, verify routed dry-run preview for one selected org, verify routed `--create-missing-orgs --dry-run` reports a would-create state after deleting that org, and verify live `--use-export-org --create-missing-orgs` recreates the org and restores its dashboard.
- Result: The checked-in Rust live smoke harness now covers routed multi-org dashboard import preview and recreate behavior in addition to the existing single-org dashboard and alerting checks.

## 2026-03-15 - Task: Add Dashboard Import Org-Aware Dry-Run Preview
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `grafana_utils/dashboards/import_workflow.py`, `tests/test_python_dashboard_cli.py`, `docs/user-guide.md`, `docs/user-guide-TW.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Routed dashboard import already supported `--use-export-org`, `--only-org-id`, and live `--create-missing-orgs`, but dry-run still failed closed for missing destination orgs and refused `--create-missing-orgs --dry-run`. Operators could not preview whether each exported org already existed or would need creation before import.
- Current Update: Changed routed dry-run so it now emits one org-level preview line per selected exported org, reporting `orgAction=exists`, `orgAction=missing-org`, or `orgAction=would-create-org` plus the source/target org ids and dashboard count. Existing target orgs still run through the current per-dashboard dry-run path, while missing-org cases stay non-mutating and skip live org creation.
- Result: `dashboard import --use-export-org --dry-run` now previews destination-org existence and would-create behavior without mutating Grafana, both with and without `--create-missing-orgs`.

## 2026-03-15 - Task: Add Dashboard Import Routing By Exported Org
- State: Done
- Scope: `grafana_utils/clients/dashboard_client.py`, `grafana_utils/dashboard_cli.py`, `grafana_utils/dashboards/export_inventory.py`, `grafana_utils/dashboards/import_runtime.py`, `grafana_utils/dashboards/import_workflow.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard_cli_defs.rs`, `rust/src/dashboard_import.rs`, `rust/src/dashboard_rust_tests.rs`, `docs/user-guide.md`, `docs/user-guide-TW.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Dashboard import only supported one destination org per run. Operators could target the current org or one explicit `--org-id`, and `--require-matching-export-org` only acted as a safety guard. There was no way to point import at a combined `--all-orgs` export root, filter selected exported orgs, or create missing destination orgs before routed import.
- Current Update: Added `--use-export-org` to route one combined multi-org export root back into Grafana by each exported orgId, added repeatable `--only-org-id` filtering, and added `--create-missing-orgs` so missing destination orgs can be created from the exported org name before import continues. Kept `--use-export-org` Basic-auth-only, blocked incompatible flag combinations, and later extended routed dry-run so `--create-missing-orgs --dry-run` now previews `would-create` org state instead of failing closed.
- Result: Dashboard import can now replay multi-org exports back into matching org contexts with explicit filtering and optional destination-org creation, while the existing single-org import workflow remains unchanged.

## 2026-03-15 - Task: Add Safer Access User Password Input
- State: Done
- Scope: `grafana_utils/access/parser.py`, `grafana_utils/access/workflows.py`, `grafana_utils/access_cli.py`, `tests/test_python_access_cli.py`, `rust/src/access_cli_defs.rs`, `rust/src/access_user.rs`, `rust/src/access_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `access user add --password` and `access user modify --set-password` currently take cleartext passwords directly from CLI flags, and user import only accepts inline `password` fields when creating missing global users. There is no prompt-based or file-based password input path for access user lifecycle commands.
- Current Update: Added prompt/file-based password input for Python and Rust `access user add` and `access user modify`, kept existing explicit `--password` and `--set-password` behavior, and resolved password values before user lifecycle requests are sent.
- Result: Operators can now use `--password-file` or `--prompt-user-password` on create and `--set-password-file` or `--prompt-set-password` on modify, reducing the need to pass cleartext passwords directly on the command line.

## 2026-03-15 - Task: Add Access Org Management
- State: Done
- Scope: `grafana_utils/access/parser.py`, `grafana_utils/access/workflows.py`, `grafana_utils/clients/access_client.py`, `grafana_utils/access_cli.py`, `tests/test_python_access_cli.py`, `rust/src/access.rs`, `rust/src/access_cli_defs.rs`, `rust/src/access_org.rs`, `rust/src/access_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The access CLIs supported users, teams, and service accounts, but there was no first-class org surface for list/add/modify/delete or snapshot export/import. Existing `access user` flows could only target org-local behavior indirectly through `--org-id`, `--set-org-role`, or `--scope org`, and there was no explicit org membership replay path.
- Current Update: Added `access org` to the Python and Rust CLIs with Basic-auth-only list/add/modify/delete/export/import workflows, org export bundles (`orgs.json` plus `export-metadata.json`), and import replay that can create missing orgs plus add or role-update org users from snapshot records.
- Result: Python and Rust now both expose explicit organization management in the access domain, and the current user-management semantics remain available for direct global user creation plus org-scoped role/removal targeting.

## 2026-03-15 - Task: Add Service-Account Snapshot Export Import Diff
- State: In Progress
- Scope: `grafana_utils/access/parser.py`, `grafana_utils/access/workflows.py`, `grafana_utils/clients/access_client.py`, `grafana_utils/access_cli.py`, `tests/test_python_access_cli.py`, `rust/src/access.rs`, `rust/src/access_cli_defs.rs`, `rust/src/access_service_account.rs`, `rust/src/access_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `access service-account` only supported list/add/delete and token lifecycle operations. There was no snapshot bundle for service accounts, no import replay path, and no live-vs-file drift command in either implementation.
- Current Update: Added CLI surface and request/client plumbing for `access service-account export`, `import`, and `diff`. The new snapshot contract uses `service-accounts.json` plus `export-metadata.json`, keys records by service-account name, and treats `role` plus `disabled` as the mutable reconciliation fields for import and diff.
- Result: Python and Rust now expose matching service-account snapshot workflows in the access domain, with create/update replay, dry-run import reporting, and drift summary output designed to mirror the existing access snapshot model.

## 2026-03-15 - Task: Add Service-Account Snapshot Export Import And Diff
- State: Planned
- Scope: `grafana_utils/access/parser.py`, `grafana_utils/access/workflows.py`, `grafana_utils/access_cli.py`, `grafana_utils/clients/access_client.py`, `tests/test_python_access_cli.py`, `rust/src/access.rs`, `rust/src/access_cli_defs.rs`, `rust/src/access_service_account.rs`, `rust/src/access_rust_tests.rs`, `README.md`, `README.zh-TW.md`, `docs/user-guide.md`, `docs/user-guide-TW.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `access service-account` currently supports list/add/delete and token add/delete, but does not provide snapshot-style export/import/diff workflows like `access user` and `access team`. The public docs and support tables still describe service accounts as lifecycle-only resources.
- Current Update: Scoping Python and Rust changes so service-account snapshots follow the same bundle metadata, dry-run import, and drift diff model already used by user/team workflows.
- Result: Pending implementation.

## 2026-03-15 - Task: Align Rust Inspect JSON Contract With Python
- State: Done
- Scope: `rust/src/dashboard.rs`, `rust/src/dashboard_inspect.rs`, `rust/src/dashboard_inspect_summary.rs`, `rust/src/dashboard_inspect_report.rs`, `rust/src/dashboard_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Rust `inspect-export --json` still serialized the internal `ExportInspectionSummary` struct directly, which kept snake_case count fields, exposed Rust-only field names such as `datasource_usage` and `mixed_dashboards`, and diverged from the Python summary/report JSON contract that already uses a wrapper document with camelCase keys.
- Current Update: Added dedicated Rust JSON document builders for summary and report inspection output, kept the internal runtime structs unchanged for text/table rendering, and switched the Rust inspect JSON paths to emit the Python-shaped document keys such as `summary.dashboardCount`, `datasourceInventory`, `orphanedDatasources`, `mixedDatasourceDashboards`, and `queries[*].query`.
- Result: Rust `inspect-export --json` and `inspect-live --output-format report-json` now emit a much closer machine-readable contract to the Python inspection output without changing the existing text, table, or governance renderers.

## 2026-03-15 - Task: Standardize Python Development On Poetry
- State: Done
- Scope: `pyproject.toml`, `poetry.lock`, `Makefile`, `README.md`, `DEVELOPER.md`, `AGENTS.md`, `tests/test_python_packaging.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Python development instructions were split between direct `python3 -m unittest`, source-tree module execution, and packaged `pip install` examples without one declared standard environment manager for maintainers. The Python build shortcut also only emitted a wheel, leaving no committed Poetry lockfile or standard sdist path for downstream package-install workflows.
- Current Update: Declared Poetry as the standard Python development environment workflow, added a Poetry dev dependency group plus committed `poetry.lock`, introduced Poetry-oriented `make` targets, and switched `make build-python` to build both `sdist` and `wheel` through the Poetry-managed environment while keeping the existing setuptools packaging backend and `pip install` validation paths.
- Result: The repo now has one documented Python development workflow for maintainers, one committed Poetry lockfile for reproducible dev environments, and a standard Python build path that emits both `sdist` and `wheel` for downstream installation and release checks.

## 2026-03-15 - Task: Add Maintainer Architecture Comments for Python CLI Facades
- State: Done
- Scope: `DEVELOPER.md`, `grafana_utils/unified_cli.py`, `grafana_utils/alert_cli.py`, `grafana_utils/dashboard_cli.py`, `grafana_utils/access_cli.py`, `grafana_utils/datasource_cli.py`, `grafana_utils/datasource_contract.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Python CLI entrypoint and facade files were functionally stable but carried fewer maintainer-facing boundary notes than their module layout, making future refactors slower to reason about at a glance.
- Current Update: Added explicit module and function docstrings for unified routing, parser normalization, dispatch boundaries, and datasource contract semantics; added a DEVELOPER section for Python CLI boundary responsibilities.
- Result: No behavior changes. Future maintainers can now infer the intended separation between entrypoint routing and domain workflow ownership directly from source and maintainer documentation.

## 2026-03-16 - Task: Cache Rust Dashboard Import Org Lookups
- State: Done
- Scope: `rust/src/dashboard_import.rs`, `rust/src/dashboard_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Rust dashboard import already cached per-dashboard UID and folder lookups inside one import run, but routed import preview and `--use-export-org` planning still re-fetched `/api/orgs` for each exported org scope, and matching-export-org validation kept current-org lookup outside the same reusable import cache.
- Current Update: Extended the Rust import lookup cache to retain current-org ID and the admin org inventory, rewired matching-export-org validation to reuse the same cache, and rewired routed import planning/preview to reuse one `/api/orgs` fetch across all export scopes in the same run.
- Result: Large routed Rust imports now avoid repeated admin org inventory lookups during planning and dry-run preview while preserving existing import behavior and output contracts.

## 2026-03-16 - Task: Tighten Rust Prompt Export Naming And Typed Alert Linkage
- State: Done
- Scope: `rust/src/dashboard_prompt.rs`, `rust/src/alert.rs`, `rust/src/dashboard_rust_tests.rs`, `rust/src/alert_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Rust prompt export already rewrote datasource inputs and `__requires`, but plugin display names still fell back to generic title-casing for datasource types such as OpenSearch and PostgreSQL, and the prompt tests only covered a narrow Prometheus/Loki subset. The alert import path also still passed raw dashboard/panel linkage maps down through rewrite helpers instead of one typed linkage context.
- Current Update: Added Grafana-style datasource plugin display names for common datasource families, carried those names through prompt input and `__requires` rendering, and extended Rust dashboard tests with broader same-type object-reference coverage plus OpenSearch/PostgreSQL naming/version assertions. Replaced the alert import path's raw dashboard/panel linkage map pair with a typed `AlertLinkageMappings` helper that loads and resolves dashboard and panel remaps through explicit methods.
- Result: Rust prompt export is closer to Grafana external export naming for common datasource plugins, the prompt test surface now covers broader same-type and non-Prometheus/Loki cases, and alert linkage rewriting no longer threads ad hoc nested maps through the import/diff path.

## 2026-03-16 - Task: Deepen Rust Inspection Dependency Summaries
- State: Done
- Scope: `rust/src/dashboard_inspect.rs`, `rust/src/dashboard_inspect_report.rs`, `rust/src/dashboard_inspect_render.rs`, `rust/src/dashboard_inspect_governance.rs`, `rust/src/dashboard_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Rust inspect report output already exposed flat per-query rows plus governance family coverage, but the grouped tree output still summarized dashboards and panels only by counts, governance JSON did not include explicit dashboard dependency rows, and `--report-filter-datasource` only matched the rendered datasource label rather than the uid/type/family fields already present in the report rows.
- Current Update: Extended normalized grouped query models with dashboard/panel datasource families, datasource labels, query-field lists, and dashboard file paths so tree and tree-table output can surface dependency shape directly in section headers. Added governance `dashboardDependencies` rows with dashboard file, datasource, family, panel-count, and query-count coverage, and widened Rust datasource report filtering to match exact datasource label, uid, type, or family values.
- Result: Rust inspect-export / inspect-live output now carries richer dependency summaries inside the existing tree/governance surfaces, and operators can narrow report output by datasource uid/type/family without adding new CLI flags.

## 2026-03-16 - Task: Drop Python 3.6 Compatibility As A Contributor Constraint
- State: Done
- Scope: `AGENTS.md`, `docs/DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The published package metadata already required Python 3.9+, and the active syntax-floor tests already validated Python 3.9 parseability, but the repo instructions still told contributors to keep new code parseable under Python 3.6 and framed RHEL 8-era compatibility as an active coding constraint.
- Current Update: Updated the repo contributor instructions and maintainer notes to make Python 3.9+ the explicit development baseline, removed the remaining guidance that told contributors to preserve Python 3.6 parser compatibility, and clarified that new work can use Python 3.9-era language features where appropriate.
- Result: Repo policy now matches packaging metadata, current tests, and current maintainer intent: Python 3.9+ is the supported baseline, and Python 3.6 compatibility is no longer a requirement for new code.

## 2026-03-16 - Task: Expand Rust Access Live Smoke And Clear Stale TLS TODOs
- State: Done
- Scope: `scripts/test-rust-live-grafana.sh`, `rust/src/access_rust_tests.rs`, `TODO.md`, `docs/DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The checked-in Rust live smoke covered dashboard, alerting, and datasource workflows, but it did not enter the `access` namespace at all. At the same time, `TODO.md` still claimed shared access TLS flags such as `--insecure` and `--ca-cert` were unimplemented even though parser/runtime support had already landed in both runtimes.
- Current Update: Extended the Rust Docker smoke script to exercise representative destructive access paths with `--insecure`, including user org/global delete, team delete, org delete, and service-account token/service-account delete after creating the corresponding live objects. Added focused Rust parser coverage that asserts those destructive commands accept `--insecure`, refreshed maintainer notes for the expanded Rust smoke surface, and cleaned the stale TODO entries that still listed `--insecure` / `--ca-cert` as missing.
- Result: Rust now has a checked-in live validation path for the newer access delete surface instead of relying on Python-only smoke coverage, and the active backlog no longer misreports already-implemented access TLS flags as unfinished work.

## 2026-03-15 - Task: Align Rust Inspection Orphaned Datasource Summary
- State: Done
- Scope: `rust/src/dashboard_inspect_summary.rs`, `rust/src/dashboard_inspect.rs`, `rust/src/dashboard_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Python `inspect-export` and `inspect-live` summary output already exposed `orphanedDatasourceCount` and `orphanedDatasources`, but the Rust inspection summary only carried datasource inventory and mixed-dashboard aggregates. That left the two runtimes with a visible summary capability gap even though the Rust governance path already knew how to derive orphaned datasource risk from the same inventory.
- Current Update: Added orphaned datasource count and orphaned datasource inventory rows to the Rust inspection summary model, wired the summary builder to materialize those rows directly from the datasource inventory usage counts, and extended the Rust dashboard tests to lock in the new summary fields.
- Result: Rust inspection summary output now exposes the same orphaned-datasource concept that Python summary output already had, reducing one concrete inspect-export/inspect-live drift point without changing the existing governance risk behavior.

## 2026-03-15 - Task: Raise Python Baseline To 3.9
- State: Done
- Scope: `pyproject.toml`, `README.md`, `README.zh-TW.md`, `DEVELOPER.md`, `grafana_utils/auth_staging.py`, `grafana_utils/http_transport.py`, `grafana_utils/unified_cli.py`, `grafana_utils/datasource_contract.py`, `tests/test_python_packaging.py`, `tests/test_python_unified_cli.py`, `tests/test_python_auth_staging.py`, `tests/test_python_access_cli.py`, `tests/test_python_dashboard_cli.py`, `tests/test_python_dashboard_inspection_governance.py`, `tests/test_python_access_pending_cli_staging.py`, `tests/test_python_datasource_cli.py`, `tests/test_python_alert_cli.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Python packaging metadata and current docs still declared `>=3.6`, the maintainer notes still told contributors to avoid Python 3.9 built-in generics, and the syntax-floor tests still locked covered Python modules to Python 3.6 parseability. That left the repo policy, contributor guidance, and static validation tied to an older compatibility floor than the current code needs.
- Current Update: Raised the published Python floor to 3.9 in `pyproject.toml`, updated current operator and maintainer docs to describe Python 3.9+ as the supported syntax/runtime baseline, switched representative shared Python modules to built-in generic annotations, and updated the syntax-floor tests to assert Python 3.9 parseability instead of Python 3.6.
- Result: The repo now consistently advertises and validates Python 3.9+ as the supported Python baseline, and touched shared modules can use Python 3.9 typing syntax without conflicting with packaging metadata or static syntax-floor tests.

## 2026-03-15 - Task: Split Dashboard Inspection Models And Dispatch
- State: Done
- Scope: `rust/src/dashboard.rs`, `rust/src/dashboard_models.rs`, `rust/src/dashboard_inspect_summary.rs`, `grafana_utils/dashboards/inspection_workflow.py`, `grafana_utils/dashboards/inspection_dispatch.py`, `tests/test_python_dashboard_cli.py`, `DEVELOPER.md`, `TODO.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `rust/src/dashboard.rs` still owns the remaining export/index/inventory and inspection summary structs even after the earlier module split, while Python inspection still keeps the output-mode validation and report/summary dispatch logic inline in one workflow module. That leaves the Rust root module carrying typed payload details that belong with helper ownership and leaves the Python inspection path with more duplicated branch logic than needed to stay aligned with Rust.
- Current Update: Moved the remaining Rust dashboard export/index/inventory structs into `rust/src/dashboard_models.rs`, moved the inspection summary payload structs into `rust/src/dashboard_inspect_summary.rs`, and kept the existing `crate::dashboard` imports stable through re-exports. On the Python side, extracted inspect output-mode validation plus report/summary rendering dispatch into `grafana_utils/dashboards/inspection_dispatch.py` so `inspection_workflow.py` now focuses on temporary live-export materialization plus high-level workflow entrypoints.
- Result: `rust/src/dashboard.rs` now drops to a much smaller orchestration/root module instead of owning typed export and report payload shapes, while Python inspection output routing now has one shared dispatch path that stays easier to keep aligned with the Rust inspect behavior.

## 2026-03-15 - Task: Rename Unified CLI To grafana-util
- State: Done
- Scope: `pyproject.toml`, `grafana_utils/__main__.py`, `grafana_utils/unified_cli.py`, `tests/test_python_packaging.py`, `tests/test_python_unified_cli.py`, `tests/test_python_access_cli.py`, `tests/test_python_dashboard_cli.py`, `tests/test_python_alert_cli.py`, `README.md`, `README.zh-TW.md`, `DEVELOPER.md`, `AGENTS.md`, `rust/src/bin/grafana-util.rs`, `rust/src/cli.rs`, `rust/src/alert.rs`, `rust/src/alert_cli_defs.rs`, `rust/src/dashboard_cli_defs.rs`, `rust/src/dashboard_help.rs`, `rust/src/datasource.rs`, `rust/src/cli_rust_tests.rs`, `rust/src/alert_rust_tests.rs`, `rust/src/dashboard_rust_tests.rs`, `rust/src/datasource_rust_tests.rs`, `scripts/test-python-access-live-grafana.sh`, `scripts/test-rust-live-grafana.sh`, `scripts/build-rust-linux-amd64.sh`, `scripts/build-rust-linux-amd64-zig.sh`, `scripts/build-rust-macos-arm64.sh`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The unified Python and Rust CLIs, repo-local wrapper path, packaging metadata, tests, and current docs all still present the primary tool name as `grafana-utils`, while Python packaging also still only includes the top-level `grafana_utils` package and would omit newly added subpackages on install.
- Current Update: Renamed the unified installed command and repo-local wrapper usage to `grafana-util`, renamed the Rust unified binary source entrypoint to `rust/src/bin/grafana-util.rs`, updated help text, tests, scripts, and current docs to the singular command name, and widened the Python setuptools package discovery to include `grafana_utils.*` so the split access and datasource subpackages remain installable.
- Result: The repo now presents one singular unified command name, `grafana-util`, across Python packaging, source-tree wrapper usage, Rust unified binary/help, tests, and current operator docs, while keeping existing export/import metadata kinds unchanged for compatibility and keeping Python subpackages included in packaged installs.

## 2026-03-15 - Task: Split Python Access CLI Facade
- State: Done
- Scope: `grafana_utils/access_cli.py`, `grafana_utils/access/parser.py`, `grafana_utils/access/workflows.py`, `tests/test_python_access_cli.py`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `grafana_utils/access_cli.py` is still the largest Python CLI facade in the repo and currently mixes argparse wiring, auth validation, identity lookup helpers, user/team/service-account workflows, and top-level dispatch in one file even after earlier support-module extractions.
- Current Update: Split the argparse and CLI-shape wiring into `grafana_utils/access/parser.py`, moved access validation/lookup/workflow logic into `grafana_utils/access/workflows.py`, and reduced `grafana_utils/access_cli.py` to a stable facade that re-exports the tested helper surface while keeping auth prompting and top-level client dispatch local. Extended focused access tests with Python 3.6 syntax coverage for the new modules and updated maintainer notes to document the new boundaries.
- Result: Python access code now has a real `grafana_utils/access/` submodule layout instead of one oversized facade, while `grafana_utils.access_cli` and the unified CLI still expose the same external command and helper API expected by the existing tests.

## 2026-03-15 - Task: Split Python Datasource CLI Facade
- State: Done
- Scope: `grafana_utils/datasource_cli.py`, `grafana_utils/datasource/__init__.py`, `grafana_utils/datasource/parser.py`, `grafana_utils/datasource/workflows.py`, `tests/test_python_datasource_cli.py`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `grafana_utils/datasource_cli.py` still mixes argparse wiring, export/import/diff workflow logic, JSON/file bundle helpers, and top-level dispatch in one module even though the repo already uses subpackage boundaries for dashboard and access code.
- Current Update: Split datasource argparse wiring and dry-run column metadata into `grafana_utils/datasource/parser.py`, moved export/import/diff execution plus datasource bundle helpers into `grafana_utils/datasource/workflows.py`, and reduced `grafana_utils/datasource_cli.py` to a stable facade that re-exports the existing helper surface while forwarding execution through the submodules. Extended focused datasource tests with Python 3.6 syntax coverage for the new modules and updated maintainer notes to describe the datasource package layout.
- Result: Python datasource code now has a real `grafana_utils/datasource/` submodule boundary, while `grafana_utils.datasource_cli` and unified CLI dispatch still preserve the existing parser/help and helper behavior expected by the focused tests.

## 2026-03-15 - Task: Split Rust Dashboard Import Dry-Run Helpers
- State: Done
- Scope: `rust/src/dashboard.rs`, `rust/src/dashboard_import.rs`, `rust/src/dashboard_rust_tests.rs`, `TODO.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: After the shared live-helper extraction, `rust/src/dashboard.rs` was smaller but still carried the folder inventory dry-run record builder and table renderer used only by dashboard import. That left the root module holding import-only presentation logic that did not belong with the top-level dashboard facade.
- Current Update: Moved the folder inventory dry-run record builder and folder dry-run table renderer into `rust/src/dashboard_import.rs`, then kept the existing dashboard test import path stable through a test-only re-export from `dashboard.rs`.
- Result: `rust/src/dashboard.rs` dropped again from 568 lines to 478 lines, and the folder dry-run rendering logic now sits with the import workflow that actually owns it.

## 2026-03-15 - Task: Split Rust Dashboard Help Surface
- State: Done
- Scope: `rust/src/dashboard.rs`, `rust/src/dashboard_help.rs`, `rust/src/dashboard_rust_tests.rs`, `TODO.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Even after the live-helper and import dry-run cleanup, `rust/src/dashboard.rs` still owned the `--help-full` rendering helpers and long inspect example strings for `inspect-export` and `inspect-live`. Those helpers were stable, but they were unrelated to runtime orchestration and kept extra presentation-only detail in the root module.
- Current Update: Moved the dashboard `--help-full` rendering helpers and extended example text into `rust/src/dashboard_help.rs`, then kept the public `crate::dashboard` API stable by re-exporting the moved functions from `dashboard.rs`.
- Result: The Rust dashboard root module is smaller again, and the full-help rendering surface now lives in a dedicated module instead of in the orchestration root.

## 2026-03-15 - Task: Split Rust Dashboard Live Orchestration Helpers
- State: Done
- Scope: `rust/src/dashboard.rs`, `rust/src/dashboard_live.rs`, `rust/src/dashboard_export.rs`, `rust/src/dashboard_import.rs`, `rust/src/dashboard_list.rs`, `rust/src/dashboard_rust_tests.rs`, `TODO.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Rust dashboard behavior was already split across CLI definitions, export, import, list, prompt, and inspect modules, but `rust/src/dashboard.rs` still held a large shared block of live Grafana request helpers plus folder inventory orchestration that export, import, and list all depended on through the root module. That made the root dashboard module keep growing even after earlier feature splits.
- Current Update: Extracted the shared live request and folder inventory helpers into `rust/src/dashboard_live.rs`, rewired `rust/src/dashboard.rs` to re-export the moved helpers, and left the existing export/import/list modules and dashboard tests on the same crate-level names.
- Result: The Rust dashboard root module dropped from 1017 lines to 568 lines without changing operator-facing behavior, and the shared Grafana fetch/folder reconciliation logic now lives in one dedicated helper module instead of continuing to accrete in `dashboard.rs`.
## 2026-03-14 - Task: Block Datasource Name-Match UID Drift Updates
- State: Done
- Scope: `grafana_utils/datasource_cli.py`, `tests/test_python_datasource_cli.py`, `rust/src/datasource.rs`, `rust/src/datasource_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `TODO.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Datasource import already blocked plugin-type changes and ambiguous matches, but `--replace-existing` still allowed updates that matched a live datasource only by exact `name` even when the exported `uid` and live `uid` disagreed. That left room for one datasource identity to overwrite another same-name datasource by mistake.
- Current Update: Added a shared update-safety rule in Python and Rust that turns same-name matches with differing non-empty UIDs into an explicit blocked action instead of a normal update, and added focused tests that lock in the new `would-fail-uid-mismatch` behavior.
- Result: Datasource import still allows normal UID matches and missing-datasource creates, but it no longer silently updates a same-name datasource when the underlying datasource identity has drifted.

## 2026-03-14 - Task: Add Prompt Token Auth Flags
- State: Done
- Scope: `grafana_utils/auth_staging.py`, `grafana_utils/dashboard_cli.py`, `grafana_utils/alert_cli.py`, `grafana_utils/access_cli.py`, `tests/test_python_dashboard_cli.py`, `tests/test_python_alert_cli.py`, `tests/test_python_access_cli.py`, `tests/test_python_auth_staging.py`, `rust/src/common.rs`, `rust/src/common_rust_tests.rs`, `rust/src/dashboard_cli_defs.rs`, `rust/src/dashboard_rust_tests.rs`, `rust/src/alert_cli_defs.rs`, `rust/src/alert_rust_tests.rs`, `rust/src/access_cli_defs.rs`, `rust/src/access_rust_tests.rs`, `rust/src/access_pending_delete.rs`, `README.md`, `README.zh-TW.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The CLI families already supported `--token` and `--prompt-password`, but operators still had to paste API tokens directly onto the command line or rely on env vars. That left token auth less safe for manual use than Basic auth, even though both secrets can leak through shell history or process args.
- Current Update: Added `--prompt-token` across the shared Python and Rust auth paths, wired the common parsers to accept it, prompted for the token without echo, and tightened validation so prompted token auth stays mutually exclusive with explicit token and Basic auth flags.
- Result: Operators can now use token auth interactively without exposing the token in shell history or process arguments, using a flag pattern that matches the existing `--prompt-password` behavior.

## 2026-03-14 - Task: Add Python Prompt Token Support
- State: Done
- Scope: `grafana_utils/auth_staging.py`, `grafana_utils/dashboard_cli.py`, `grafana_utils/alert_cli.py`, `grafana_utils/access_cli.py`, `tests/test_python_auth_staging.py`, `tests/test_python_dashboard_cli.py`, `tests/test_python_alert_cli.py`, `tests/test_python_access_cli.py`, `README.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Python CLIs already supported explicit token auth and prompted Basic-auth passwords, but there was no secure interactive equivalent for token auth. Operators who wanted to avoid putting a Grafana API token in shell history still had to pass `--token` directly or rely on environment variables.
- Current Update: Added `--prompt-token` to the shared Python auth path and the dashboard, alert, and access parsers, wired it through the shared auth resolver, and added focused success/conflict coverage for prompted token input.
- Result: Python operators can now enter a Grafana API token through a non-echoed prompt with `--prompt-token`, while the CLIs still reject mixing token and Basic-auth inputs or combining `--prompt-token` with an explicit `--token`.

## 2026-03-14 - Task: Reject Extra Datasource Contract Fields
- State: Done
- Scope: `grafana_utils/datasource_contract.py`, `grafana_utils/datasource_cli.py`, `grafana_utils/datasource_diff.py`, `tests/test_python_datasource_cli.py`, `tests/test_python_datasource_diff.py`, `rust/src/datasource.rs`, `rust/src/datasource_rust_tests.rs`, `rust/src/datasource_diff_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `TODO.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Datasource export/import had a narrow normalized contract in practice, but the import and diff loaders still accepted `datasources.json` entries with extra fields and silently normalized them away. That meant server-managed fields, secret-bearing settings, and datasource-type-specific config blobs could still appear in import/diff inputs without an explicit failure.
- Current Update: Added shared datasource contract validation in Python, mirrored the same fail-closed validation in Rust datasource import/diff loaders, and added focused tests that reject extra fields such as `id`, `jsonData`, `secureJsonData`, and `password` instead of silently dropping them.
- Result: Datasource import and diff now enforce the documented normalized contract directly in both runtimes, so secret-bearing or server-managed datasource fields cause an explicit error instead of being ignored.

## 2026-03-14 - Task: Align Datasource Contract Fixtures Across Python and Rust
- State: Done
- Scope: `grafana_utils/datasource_contract.py`, `grafana_utils/datasource_cli.py`, `grafana_utils/datasource_diff.py`, `tests/fixtures/datasource_contract_cases.json`, `tests/test_python_datasource_cli.py`, `tests/test_python_datasource_diff.py`, `rust/src/datasource_rust_tests.rs`, `rust/src/datasource_diff_rust_tests.rs`, `TODO.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Datasource export/import already used a deliberately small normalized contract, but Python still normalized some values more loosely than Rust, especially for `isDefault` and `orgId` when fixture inputs used booleans or numbers. The repo also did not yet have one shared cross-language datasource fixture set covering secret-bearing Prometheus, Loki, and InfluxDB cases.
- Current Update: Added a shared Python datasource contract helper for canonical string/bool normalization, rewired the Python datasource CLI and datasource diff helpers to use it, introduced one cross-language fixture file for Prometheus, Loki, and InfluxDB datasource cases with mixed auth/secret fields, and updated both Python and Rust datasource tests to validate normalized export records and import payloads against that same fixture set.
- Result: Python and Rust datasource tests now enforce the same normalized export/import contract from one fixture source, and Python no longer drifts on bool/int normalization for datasource inventory records.

## 2026-03-14 - Task: Remove Rust Auth Legacy Aliases
- State: Done
- Scope: `rust/src/access_cli_defs.rs`, `rust/src/alert_cli_defs.rs`, `rust/src/dashboard_cli_defs.rs`, `rust/src/common.rs`, `rust/src/access_rust_tests.rs`, `rust/src/alert_rust_tests.rs`, `rust/src/common_rust_tests.rs`, `rust/src/dashboard_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Rust dashboard, alert, and access CLIs still advertised the legacy Basic-auth aliases `--username` and `--password` alongside `--basic-user` and `--basic-password`. That kept help output noisy, left the shared Rust auth errors pointing at both old and new spellings, and directly conflicted with business flags like `access user add --password`.
- Current Update: Removed the Rust CLI auth aliases from the shared/common Clap definitions, updated the shared Rust auth validation messages to mention only `--basic-user`, `--basic-password`, and `--prompt-password`, and refreshed help/auth tests so the removed aliases stay gone.
- Result: Rust CLI Basic auth now uses one consistent flag pair across dashboard, alert, and access, and the shared help/error output no longer mixes canonical and legacy spellings.

## 2026-03-14 - Task: Remove Python Auth Legacy Aliases
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `grafana_utils/alert_cli.py`, `grafana_utils/access_cli.py`, `grafana_utils/auth_staging.py`, `tests/test_python_dashboard_cli.py`, `tests/test_python_alert_cli.py`, `tests/test_python_access_cli.py`, `tests/test_python_auth_staging.py`, `README.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Python dashboard, alert, and access CLIs still accepted legacy Basic-auth aliases `--username` and `--password` alongside `--basic-user` and `--basic-password`. That made the help text noisier and left a naming collision with business flags such as `access user add --password`.
- Current Update: Removed the Python CLI parser aliases for `--username` and auth `--password`, updated shared auth error wording to mention only `--basic-user` and `--basic-password`, refreshed parser/auth tests to assert the new contract, and trimmed the README auth section to the new flag vocabulary.
- Result: Python CLI Basic auth now uses one unambiguous flag pair, `--basic-user` and `--basic-password`, while business flags such as `access user add --password` keep their existing meaning.

## 2026-03-14 - Task: Resolve Rust Access Password Flag Collision
- State: Done
- Scope: `rust/src/access_cli_defs.rs`, `rust/src/access_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Rust access CLI reused the common Basic-auth legacy alias `--password` while `access user add` also defined `--password` for the new user's local password. Clap tolerated normal parsing in some paths, but long-help rendering for `access user add` failed with a duplicate long-option panic.
- Current Update: Removed the Rust access common-auth `--password` legacy alias, kept `--basic-password` as the supported Basic-auth flag for that CLI family, and re-enabled focused help coverage for `access user add` so the collision stays fixed.
- Result: `cargo run --quiet --bin grafana-utils -- access user add -h` and the corresponding test help paths now work again, while the user-creation password flag keeps its existing `--password` spelling.

## 2026-03-14 - Task: Consolidate Python CLI Auth Error Resolution
- State: Done
- Scope: `grafana_utils/auth_staging.py`, `grafana_utils/dashboard_cli.py`, `grafana_utils/alert_cli.py`, `grafana_utils/access_cli.py`, `tests/test_python_auth_staging.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Python dashboard, alert, and access CLIs already delegated the raw token-vs-Basic auth decision to `auth_staging.resolve_auth_from_namespace()`, but each CLI still carried its own copy of the same `AuthConfigError` to operator-facing `GrafanaError` message mapping. That left three near-identical `resolve_auth()` implementations to keep in sync whenever auth wording changed.
- Current Update: Added shared CLI-facing auth error formatting and a `resolve_cli_auth_from_namespace()` helper in `auth_staging.py`, then rewired the dashboard, alert, and access CLIs to use that shared helper while preserving each module's existing `GrafanaError` wrapper and return shape.
- Result: Python auth resolution now has one shared source of truth for CLI-facing error wording across dashboard, alert, and access commands, which removes duplicated message-mapping logic without changing the operator-visible auth behavior.

## 2026-03-14 - Task: Fill Rust Access CLI Help Text
- State: Done
- Scope: `rust/src/access_cli_defs.rs`, `rust/src/access_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Rust access CLI already exposed a broad user/team/service-account command surface, but many `--...` flags in `access_cli_defs.rs` still relied on bare `#[arg(long)]` declarations with no operator-facing help text. That made `-h` output inconsistent with the more fully described dashboard, datasource, and alert command families.
- Current Update: Added concrete help text for the previously bare access list, add, modify, delete, and token-creation flags, and added focused help-output tests so the most important user/team/service-account subcommands now assert the presence of those descriptions.
- Result: Rust access `-h` output is now much closer to the rest of the repo: most operator-facing flags explain what they target, what they filter, or what they change instead of only listing the flag names.

## 2026-03-14 - Task: Add Import Dry-Run Output Columns
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `grafana_utils/dashboards/import_support.py`, `grafana_utils/dashboards/import_workflow.py`, `grafana_utils/datasource_cli.py`, `tests/test_python_dashboard_cli.py`, `tests/test_python_datasource_cli.py`, `rust/src/dashboard_cli_defs.rs`, `rust/src/dashboard_import.rs`, `rust/src/dashboard_rust_tests.rs`, `rust/src/datasource.rs`, `rust/src/datasource_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Dashboard and datasource import dry-run output could now be switched between text, table, and JSON, but the table view still had a fixed column set. Dashboard table output also carried newer folder-match details in the record flow without giving operators a way to narrow the rendered columns to the fields they actually needed for review.
- Current Update: Added `--output-columns` for dashboard and datasource import dry-run table output in Python and Rust, normalized the supported column ids and aliases, kept the default tables unchanged when the flag is omitted, and tightened validation so the selector is only accepted together with table-like dry-run output.
- Result: Operators can now trim import dry-run tables down to the specific fields they care about, such as `uid,action,file` for datasource review or `uid,source_folder_path,destination_folder_path,reason` for dashboard folder-mismatch review, while the existing default summaries still render exactly as before.

## 2026-03-14 - Task: Trim Dashboard CLI Compatibility Wrappers
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `grafana_utils/dashboards/import_runtime.py`, `tests/test_python_dashboard_cli.py`, `tests/test_python_dashboard_inspection_cli.py`, `tests/test_python_dashboard_integration_flow.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: After the export/import/inspection/diff runtime splits and the dashboard test decoupling work, `dashboard_cli.py` still carried a large block of thin compatibility wrappers for output support, export inventory, folder inventory, listing helpers, and inspection materialization. Most of those names no longer had active runtime or test callers, but they still kept the CLI facade larger and harder to reason about.
- Current Update: Removed the now-unused wrapper layer from `dashboard_cli.py`, rewired import and diff dependency assembly to call canonical helper modules directly, and kept only the real CLI entrypoints plus dependency-bundle factories in the facade.
- Result: `dashboard_cli.py` is now much closer to a true CLI facade instead of a mixed facade-and-helper module, and the remaining dashboard helper logic now lives in the dedicated `grafana_utils.dashboards.*` modules where it belongs.

## 2026-03-14 - Task: Decouple Dashboard Tests From CLI Compatibility Wrappers
- State: Done
- Scope: `tests/test_python_dashboard_cli.py`, `tests/test_python_dashboard_inspection_cli.py`, `tests/test_python_dashboard_integration_flow.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The dashboard Python test suites still reached many helper functions through `grafana_utils.dashboard_cli`, including output-support, export-inventory, folder-inventory, and listing helpers whose real implementations now live under `grafana_utils.dashboards.*`. That meant the remaining compatibility wrappers in `dashboard_cli.py` were still pinned in place by tests even after the workflow/runtime wiring had moved out.
- Current Update: Repointed the dashboard test helpers and fixtures to the canonical `grafana_utils.dashboards.*` modules for export metadata, output-path builders, export inventory discovery/validation, folder inventory loading, dashboard write helpers, and datasource-source attachment. Kept `dashboard_cli` itself for real CLI entrypoint coverage, but stopped using its wrapper surface for fixture construction and helper-unit assertions.
- Result: The dashboard Python tests now validate the real helper modules directly instead of indirectly through `dashboard_cli` compatibility wrappers, which clears the way for later cleanup of that facade without losing behavior coverage.

## 2026-03-14 - Task: Split Python Dashboard Import Runtime Wiring
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `grafana_utils/dashboards/import_runtime.py`, `tests/test_python_dashboard_cli.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Dashboard import already had a dedicated `import_workflow.py` module, but `dashboard_cli.py` still assembled the full import dependency map locally and therefore kept another large import-only runtime wiring block in the CLI facade. The CLI still needed to preserve its public helper surface, but the import workflow did not need to depend on that local assembly directly.
- Current Update: Added `grafana_utils/dashboards/import_runtime.py` to own the import dependency-map assembly and rewired `dashboard_cli._build_import_workflow_deps()` to delegate through that runtime helper while preserving the existing `dashboard_cli` entrypoints and helper names.
- Result: Python dashboard import runtime wiring now sits in a dedicated helper module instead of in the CLI facade, which trims another large behavior-preserving dependency bundle out of `dashboard_cli.py` without changing import behavior or the public helper names used by tests.

## 2026-03-14 - Task: Unify Output Format Flags
- State: Done
- Scope: `grafana_utils/access_cli.py`, `grafana_utils/alert_cli.py`, `grafana_utils/dashboard_cli.py`, `grafana_utils/datasource_cli.py`, `tests/test_python_access_cli.py`, `tests/test_python_alert_cli.py`, `tests/test_python_dashboard_cli.py`, `tests/test_python_datasource_cli.py`, `rust/src/access_cli_defs.rs`, `rust/src/access.rs`, `rust/src/alert_cli_defs.rs`, `rust/src/alert.rs`, `rust/src/dashboard_cli_defs.rs`, `rust/src/dashboard.rs`, `rust/src/datasource.rs`, `rust/src/access_rust_tests.rs`, `rust/src/alert_rust_tests.rs`, `rust/src/dashboard_rust_tests.rs`, `rust/src/datasource_rust_tests.rs`, `README.md`, `README.zh-TW.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The repo currently uses `--table` in multiple incompatible ways: sometimes as the default list output, sometimes as a dry-run mode switch, and dashboard inspect already has a separate `--output-format` selector. That makes operators guess whether `--table` is redundant, required, or unsupported depending on the command family.
- Current Update: Added a consistent `--output-format` selector across the existing table/csv/json-like command families without changing current defaults, kept the legacy flags working as compatibility aliases, and documented the new single-flag path in the READMEs.
- Result: Python and Rust now both accept `--output-format` for access list, alert list, dashboard list, dashboard datasource list, datasource list, and the dashboard/datasource import dry-run summaries. Mixed use with old selector flags now fails cleanly, but existing defaults and old flags continue working unchanged.

## 2026-03-14 - Task: Split Python Dashboard Export Runtime Wiring
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `grafana_utils/dashboards/export_runtime.py`, `tests/test_python_dashboard_cli.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Dashboard export already had a dedicated `export_workflow.py` module, but `dashboard_cli.py` still assembled the full export dependency map locally and therefore kept a large cluster of export-only runtime wiring in the CLI facade. The public helper names in `dashboard_cli.py` still needed to stay available for compatibility and tests, but the export workflow did not need to depend on those local wrappers directly.
- Current Update: Added `grafana_utils/dashboards/export_runtime.py` to own the export dependency-map assembly and rewired `dashboard_cli._build_export_workflow_deps()` to delegate through that runtime helper while keeping the existing `dashboard_cli` helper surface stable.
- Result: Python dashboard export runtime wiring now sits in a dedicated helper module instead of inside the CLI facade, which trims another large behavior-preserving dependency bundle out of `dashboard_cli.py` without changing export behavior or the public helper names used by tests.

## 2026-03-14 - Task: Wire Datasource Diff CLI
- State: Done
- Scope: `grafana_utils/datasource_cli.py`, `grafana_utils/datasource_diff.py`, `grafana_utils/unified_cli.py`, `tests/test_python_datasource_cli.py`, `tests/test_python_unified_cli.py`, `rust/src/datasource.rs`, `rust/src/datasource_diff.rs`, `rust/src/datasource_rust_tests.rs`, `rust/src/cli.rs`, `rust/src/cli_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Datasource diff scaffolds now exist in standalone Python and Rust files, but neither runtime exposes a `datasource diff` subcommand yet. The unified CLIs, operator help, and crate test graph still describe datasource as `list/export/import` only, so the new compare logic is unreachable.
- Current Update: Wired both runtimes to expose datasource diff through the existing datasource namespace, kept the compare helpers as the implementation base, extended focused parser/help/behavior coverage, and updated the README datasource command summaries so operator docs no longer claim datasource only supports list/export/import.
- Result: `grafana-utils datasource diff --diff-dir ...` now works in both Python and Rust, unified CLI help exposes the new subcommand, Python prints per-item unified diffs for changed datasource records, Rust returns a non-zero CLI result when differences are found, and the previously standalone Rust diff scaffold is now part of the crate test graph.

## 2026-03-14 - Task: Split Python Dashboard Inspection Runtime Wiring
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `grafana_utils/dashboards/inspection_runtime.py`, `grafana_utils/dashboards/inspection_workflow.py`, `tests/test_python_dashboard_inspection_cli.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Dashboard inspection already has dedicated workflow, summary, report, and governance modules, but `dashboard_cli.py` still assembles a large inspection dependency map and owns a local `iter_dashboard_panels()` helper just to feed those modules. That leaves too much inspection runtime wiring in the CLI facade even after the earlier dashboard refactors.
- Current Update: Added `grafana_utils/dashboards/inspection_runtime.py` to own the inspection dependency-map assembly and moved `iter_dashboard_panels()` there. Updated `inspection_workflow.py` to own its own `json`/`sys`/`tempfile` usage and to call `run_inspect_export()` directly for live inspection instead of routing back through a CLI callback.
- Result: `dashboard_cli.py` now delegates the inspection runtime wiring to a dedicated helper module and keeps only thin compatibility wrappers for inspection entrypoints, which reduces the remaining inspection-specific bulk in the CLI facade without changing the public CLI surface.

## 2026-03-14 - Task: Split Python Dashboard Export Org Resolution
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `grafana_utils/dashboards/export_inventory.py`, `tests/test_python_dashboard_cli.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Dashboard import already delegates most raw-export file discovery and manifest validation to `grafana_utils/dashboards/export_inventory.py`, but `dashboard_cli.py` still owns the separate `resolve_export_org_id()` scan that walks raw `index.json`, `folders.json`, and `datasources.json` directly. That leaves one more raw-export inventory concern in the CLI facade even though the surrounding metadata helpers have already moved out.
- Current Update: Moved `resolve_export_org_id()` into `grafana_utils/dashboards/export_inventory.py` and rewired the CLI wrapper to delegate through that module, keeping the existing wrapper signature and import-org-guard behavior intact.
- Result: Raw export metadata resolution now lives with the other export inventory helpers instead of inside the dashboard CLI facade, which further narrows `dashboard_cli.py` toward parser and wiring ownership.

## 2026-03-14 - Task: Split Python Dashboard Diff Workflow
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `grafana_utils/dashboards/diff_workflow.py`, `tests/test_python_dashboard_cli.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Python dashboard export, import, and inspection already delegate their main orchestration into dedicated `grafana_utils/dashboards/*_workflow.py` modules, but `dashboard_cli.py` still owns the remaining dashboard diff loop directly. That keeps one more live orchestration path coupled to the CLI facade even though the rest of the dashboard runtime has mostly been split by responsibility.
- Current Update: Moved the dashboard diff compare loop into a new `grafana_utils/dashboards/diff_workflow.py` module and rewired `dashboard_cli.diff_dashboards()` to delegate through a focused dependency bundle, matching the existing export/import/inspection workflow pattern.
- Result: Python dashboard diff now follows the same orchestration split as the other major dashboard flows, and `dashboard_cli.py` keeps only the stable CLI wrapper/dependency wiring for diff instead of the full compare loop.

## 2026-03-14 - Task: Consolidate Shared Python Auth Helper
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `grafana_utils/alert_cli.py`, `grafana_utils/auth_staging.py`, `tests/test_python_dashboard_cli.py`, `tests/test_python_alert_cli.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Python dashboard and alert CLIs still each carried their own inline token-vs-Basic auth resolution logic even though `grafana_utils/auth_staging.py` already existed and access had started delegating through it. The auth rules matched, but operator-facing error wording still lived separately inside each CLI.
- Current Update: Rewired `dashboard_cli.py` and `alert_cli.py` to resolve auth through the shared staging helper while preserving each CLI's existing auth error messages and prompt behavior. Extended focused dashboard and alert auth tests to cover the shared helper path plus env-backed and partial-env validation.
- Result: All three Python CLI families now share one auth-resolution implementation path, while dashboard and alert keep their established CLI-facing validation text and auth fallback behavior.

## 2026-03-14 - Task: Wire Quality Gate Scripts
- State: Done
- Scope: `Makefile`, `.github/workflows/ci.yml`, `scripts/check-quality.sh`, `scripts/check-python-quality.sh`, `scripts/check-rust-quality.sh`, `README.md`, `DEVELOPER.md`, `TODO.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The repo already had staged quality scripts, but `make quality` still expanded directly to hard-coded test/fmt/clippy targets and CI still duplicated that logic as separate workflow steps. The new scripts existed only as unattached assets, so local and CI quality behavior could drift again.
- Current Update: Wired `make quality`, `make quality-python`, and `make quality-rust` to the staged scripts, and updated CI to call those targets directly instead of re-declaring the checks inline. Updated maintainer and user docs to describe the new script-backed quality gate path and the optional-tool skip behavior.
- Result: Local `make` entrypoints and CI now share the same quality gate scripts, so future gate changes can happen in one place instead of being duplicated across shell, Makefile, and workflow YAML.

## 2026-03-14 - Task: Finish Access Delete Commands And Group Alias
- State: Done
- Scope: `grafana_utils/access_cli.py`, `grafana_utils/auth_staging.py`, `grafana_utils/access/pending_cli_staging.py`, `grafana_utils/clients/access_client.py`, `tests/test_python_access_cli.py`, `rust/src/access.rs`, `rust/src/access_cli_defs.rs`, `rust/src/access_pending_delete.rs`, `rust/src/access_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `TODO.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The access CLI already handled user CRUD, team list/add/modify, and service-account list/add/token-add, but `team delete`, `service-account delete`, `service-account token delete`, and the `group` compatibility alias were still unfinished. Access auth resolution also still carried its own inline implementation instead of delegating to the new shared staging helper.
- Current Update: Wired the shared Python auth helper into `access_cli.py` while preserving the existing access-facing error text, added Python and Rust parser/dispatch/client support for `team delete`, `service-account delete`, and `service-account token delete`, and exposed `group` as a compatibility alias for `team`. Extended focused Python and Rust access tests around the new destructive flows and alias parsing.
- Result: Both runtimes now expose the full planned access command surface except for the still-unimplemented shared TLS flags, and the Python access CLI no longer owns a private copy of the token-vs-Basic auth resolution logic.

## 2026-03-14 - Task: Add Actionable Governance Risk Metadata
- State: Done
- Scope: `grafana_utils/dashboards/inspection_governance.py`, `grafana_utils/dashboards/inspection_governance_render.py`, `tests/test_python_dashboard_inspection_governance.py`, `tests/test_python_dashboard_inspection_cli.py`, `rust/src/dashboard_inspect_governance.rs`, `rust/src/dashboard_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Governance reports in both Python and Rust already exposed `kind`, `severity`, `datasource`, and `detail`, but operators still had to infer whether a finding was inventory drift, analyzer coverage debt, or datasource topology risk, and there was no stable remediation hint for automation to consume.
- Current Update: Added additive `category` and `recommendation` fields to governance `riskRecords` in both Python and Rust for the four current governance risk kinds: `mixed-datasource-dashboard`, `orphaned-datasource`, `unknown-datasource-family`, and `empty-query-analysis`. Updated the governance table renderers so the risk section now shows those fields directly, and extended focused Python and Rust governance tests to lock the new JSON/table contract.
- Result: Governance JSON now carries a stable actionability layer for follow-up tooling, and governance table output is more operator-actionable without changing report flags or removing any existing fields.

## 2026-03-14 - Task: Add Dashboard Import Export-Org Guard
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `grafana_utils/dashboards/export_inventory.py`, `grafana_utils/dashboards/import_workflow.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard_cli_defs.rs`, `rust/src/dashboard_import.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Dashboard import already respected the active token org or explicit `--org-id`, but it did not warn or fail when a raw export from one org was replayed into a different target org. The raw export inventory recorded `orgId`, yet import treated that metadata as informational only.
- Current Update: Added opt-in `--require-matching-export-org` to Python and Rust dashboard import. The new guard resolves one stable source export `orgId` from raw metadata files, resolves the target org from `--org-id` or the active current-org lookup, and fails early when those org IDs differ or when the raw export does not provide one stable source org.
- Result: Operators can now keep token-based current-org import behavior by default, but they can also enable an explicit safety check that blocks accidental cross-org dry-runs or live imports.

## 2026-03-14 - Task: Wire Inspection Governance Reports
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `grafana_utils/dashboards/inspection_report.py`, `grafana_utils/dashboards/inspection_workflow.py`, `grafana_utils/dashboards/inspection_governance.py`, `grafana_utils/dashboards/inspection_governance_render.py`, `tests/test_python_dashboard_inspection_cli.py`, `tests/test_python_dashboard_inspection_governance.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_cli_defs.rs`, `rust/src/dashboard_inspect.rs`, `rust/src/dashboard_inspect_governance.rs`, `rust/src/dashboard_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Python had standalone governance builder/render helper modules available locally but no CLI wiring, while Rust inspection still exposed only the existing summary, flat query report, CSV, tree, and tree-table paths with no governance-focused report model at all. Operators could not yet request governance-focused table or JSON output through either inspection CLI, and `--report-columns` validation had no governance-specific guard.
- Current Update: Added `--report governance` and `--report governance-json` to both Python and Rust inspection CLI help/choices, wired each inspection workflow to build governance output from the existing summary document plus the datasource/panel-filtered per-query report document, and kept datasource/panel filtering applied at the report-document layer before governance aggregation. Python now owns dedicated governance builder/render modules; Rust now owns a dedicated `dashboard_inspect_governance.rs` module with governance document and table rendering helpers. Added focused parser/output/validation coverage on both runtimes.
- Result: Both Python and Rust inspection paths now expose governance-focused table and JSON report modes through `inspect-export` and `inspect-live`, while keeping governance aggregation isolated behind dedicated builder/render ownership instead of spreading the logic through the older summary/report paths.
## 2026-03-14 - Task: Add Dashboard Import Org Scoping
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `grafana_utils/dashboards/import_workflow.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_cli_defs.rs`, `rust/src/dashboard_import.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Dashboard list and export already supported explicit org switching through `--org-id` and used a Basic-auth-only org-scoped client model, but dashboard import still always ran in the current org context. Raw exports recorded `org` and `orgId`, yet import had no way to target one explicit destination org for the whole run.
- Current Update: Added `--org-id` to both Python and Rust dashboard import flows. The new flag scopes the entire import run, including dry-run checks and live writes, to one explicit destination Grafana org, requires Basic auth, and keeps raw export `orgId` metadata as informational only rather than automatic routing input.
- Result: Operators can now re-import one raw dashboard batch directly into a chosen Grafana org without manually switching org context first, while preserving the existing import behavior when `--org-id` is not set.

## 2026-03-14 - Task: Add Dashboard Import Folder-Path Guard
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `grafana_utils/dashboards/folder_path_match.py`, `grafana_utils/dashboards/import_support.py`, `grafana_utils/dashboards/import_workflow.py`, `grafana_utils/dashboards/progress.py`, `tests/test_python_dashboard_cli.py`, `tests/test_python_dashboard_folder_path_match.py`, `rust/src/dashboard_cli_defs.rs`, `rust/src/dashboard_import.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Dashboard import already supported create-only, create-or-update, and update-or-skip-missing modes keyed by dashboard `uid`, but it had no way to protect existing dashboards that had drifted into a different destination folder path. Operators could preserve or override destination folder UIDs, but they could not require the exported raw folder path to match the current Grafana folder path before updating an existing dashboard.
- Current Update: Added `--require-matching-folder-path` in both Python and Rust dashboard import flows. The new guard compares the raw source folder path against the current destination Grafana folder path only for existing dashboards, rewrites update actions to `skip-folder-mismatch` when those paths differ, extends dry-run table/json output with source and destination folder-path columns/details, and rejects the guard when combined with `--import-folder-uid`.
- Result: Operators can now keep the existing batch import workflow while safely blocking updates to dashboards that have moved to a different folder path in the target Grafana, and they can see the exact source/destination path mismatch in dry-run output before running a live import.

## 2026-03-14 - Task: Strengthen Loki Inspection Analyzers
- State: Done
- Scope: `grafana_utils/dashboards/inspection_analyzers/loki.py`, `rust/src/dashboard_inspect_analyzer_loki.rs`, `rust/src/dashboard_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Loki already had dedicated analyzer boundaries in both Python and Rust, but Python still returned an empty placeholder analysis and Rust did not yet extract Loki-specific inspection signals beyond the generic report flow. That left Loki inspection rows structurally present but materially less informative than Prometheus, Flux, or SQL rows.
- Current Update: Implemented conservative Loki heuristics in both runtimes. Python now extracts stream matchers into `measurements` and common LogQL range/aggregation functions plus pipeline/filter stages into `metrics` without widening the existing report contract. Rust now extracts stream selectors, label matchers, common functions/stages, and range windows into the existing `metrics` / `measurements` / `buckets` fields, with focused regression coverage for a real Loki query fixture.
- Result: Loki inspection rows now expose useful best-effort query signals in both Python and Rust while preserving the established inspection schema and keeping future Loki refinement isolated behind dedicated analyzer modules.

## 2026-03-14 - Task: Split Rust Dashboard Inspection Renderers
- State: Done
- Scope: `rust/src/dashboard.rs`, `rust/src/dashboard_inspect.rs`, `rust/src/dashboard_inspect_render.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Rust already had separate inspection report-model and analyzer modules, but `rust/src/dashboard_inspect.rs` still owned the shared CSV/table rendering helpers plus the grouped tree and tree-table report renderers. That kept inspection orchestration and output formatting coupled in one large Rust module even after earlier inspection splits.
- Current Update: Extracted the Rust inspection CSV/table/tree/tree-table renderers into `rust/src/dashboard_inspect_render.rs`, rewired `rust/src/dashboard.rs` to re-export the renderer helpers, and updated `rust/src/dashboard_inspect.rs` to consume the new renderer boundary without changing CLI/help/output behavior.
- Result: Rust inspection now has a clearer three-way ownership split across orchestration (`dashboard_inspect.rs`), report-model helpers (`dashboard_inspect_report.rs`), and renderers (`dashboard_inspect_render.rs`), which is closer to the current Python structure.

## 2026-03-14 - Task: Split Python Loki And Generic Inspection Analyzers
- State: Done
- Scope: `grafana_utils/dashboards/inspection_analyzers/__init__.py`, `grafana_utils/dashboards/inspection_analyzers/dispatcher.py`, `grafana_utils/dashboards/inspection_analyzers/generic.py`, `grafana_utils/dashboards/inspection_analyzers/loki.py`, `tests/test_python_dashboard_cli.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: After the Prometheus / Flux / SQL analyzer split, `inspection_analyzers/` still lacked a real explicit fallback boundary and Loki analysis was only represented by an empty placeholder. That left the analyzer package incomplete even though `inspection_report.py` was already dispatching through it.
- Current Update: Added an explicit `generic` analyzer module, wired unknown datasource families through it in the dispatcher, and kept Loki analysis behind its own dedicated analyzer boundary. Added focused syntax and dispatcher coverage for the new generic path and preserved the existing Loki/generic inspection contract expectations in the dashboard CLI suite.
- Result: The Python inspection analyzer package now has an explicit ownership path for every routed datasource family, including Loki and the generic fallback, so future family-specific work can keep shrinking `inspection_report.py` without routing unknown cases back through the report layer.

## 2026-03-14 - Task: Split Python Dashboard Inspection Analyzers
- State: Done
- Scope: `grafana_utils/dashboards/inspection_report.py`, `grafana_utils/dashboards/inspection_analyzers/contract.py`, `grafana_utils/dashboards/inspection_analyzers/prometheus.py`, `grafana_utils/dashboards/inspection_analyzers/flux.py`, `grafana_utils/dashboards/inspection_analyzers/sql.py`, `tests/test_python_dashboard_cli.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `grafana_utils/dashboards/inspection_report.py` still carried datasource-family dispatch plus Prometheus, Flux, and SQL-specific query heuristics inline even after the renderer split. The `inspection_analyzers/` package existed, but most of the real family-specific logic still lived in the report module instead of behind the analyzer boundary.
- Current Update: Moved the active Prometheus, Flux, and SQL query-analysis heuristics into `inspection_analyzers/` and rewired `inspection_report.py` to use `dispatch_query_analysis()` plus the shared `build_query_field_and_text()` helper from the analyzer package. Added focused dashboard CLI coverage for one mixed Prometheus/Flux/SQL report JSON fixture so the analyzer split keeps the current inspection contract and values stable.
- Result: Python inspection analysis is now actually decomposed by datasource family instead of only having a placeholder analyzer package, while `inspection_report.py` focuses more narrowly on row/document construction and preserves the existing CLI/report surface.

## 2026-03-14 - Task: Split Python Dashboard Inspection Renderers
- State: Done
- Scope: `grafana_utils/dashboards/inspection_report.py`, `grafana_utils/dashboards/inspection_render.py`, `grafana_utils/dashboards/inspection_summary.py`, `tests/test_python_dashboard_cli.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `grafana_utils/dashboards/inspection_report.py` still mixed the canonical inspection document builders with CSV/table/tree/tree-table output rendering. That kept report modeling and output formatting coupled inside one 1100+ line module even after earlier dashboard facade reductions, and the focused CLI suite did not yet pin the `inspect-export --json` or `inspect-export --report json` output contracts.
- Current Update: Extracted the inspection report render helpers into `grafana_utils/dashboards/inspection_render.py` and rewired `inspection_report.py` to re-export the stable renderer names already used by `dashboard_cli.py` and the inspection workflow dependency bundle. `inspection_summary.py` now imports the shared table-section helper from the renderer module directly. Added Python 3.6 syntax coverage for the new module, kept the grouped tree-table renderer test, and added focused execution coverage for `inspect-export --json` plus `inspect-export --report json`.
- Result: Python dashboard inspection now has a clearer boundary between report document building and output rendering, while the existing CLI wiring and helper surface stay behavior-compatible and the inspect JSON contracts are now covered before deeper analyzer refactors.

## 2026-03-13 - Task: Split Python Dashboard Output Support Helpers
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `grafana_utils/dashboards/output_support.py`, `tests/test_python_dashboard_cli.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: After the progress, folder-support, and import-support splits, `grafana_utils/dashboard_cli.py` still kept the remaining export/output path builders, file-write helpers, and export index/metadata builders inline. That left one cohesive export-support block in the facade even though the Rust-side structure already treats those responsibilities as helper-owned instead of top-level CLI-owned.
- Current Update: Extracted the Python dashboard export/output helper cluster into `grafana_utils/dashboards/output_support.py` and rewired `grafana_utils/dashboard_cli.py` to import and re-export the stable helper names used by tests and workflow dependency bundles. Added Python 3.6 syntax coverage for the new output-support module in the dashboard CLI test suite.
- Result: The Python dashboard facade is now closer to a parser/dispatch/dependency-bundle host, while output-path generation, export manifest/index construction, and JSON/dashboard file writes live behind a focused helper boundary.

## 2026-03-13 - Task: Split Python Dashboard Progress Helpers
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `grafana_utils/dashboards/progress.py`, `tests/test_python_dashboard_cli.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: After the listing, import/diff, and folder-support splits, `grafana_utils/dashboard_cli.py` was already much smaller but still kept the remaining export/import progress rendering helpers inline. That left one small but cohesive output-formatting block in the facade instead of with other focused helper modules.
- Current Update: Extracted the dashboard export/import progress renderers into `grafana_utils/dashboards/progress.py` and rewired `grafana_utils/dashboard_cli.py` to import and re-export the same helper names used by the workflow dependency bundles. Added Python 3.6 syntax coverage for the new progress helper module.
- Result: The Python dashboard facade is now closer to a pure parser/dispatch/dependency-bundle host, while progress output behavior stays unchanged for export and import workflows.

## 2026-03-13 - Task: Split Python Dashboard Folder Support Helpers
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `grafana_utils/dashboards/folder_support.py`, `tests/test_python_dashboard_cli.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `grafana_utils/dashboard_cli.py` had already shed listing, import/diff helpers, and workflow bodies, but it still kept the remaining folder inventory collection, live folder status checks, export-manifest wrapper loads, and import-folder path resolution logic inline. That left one large folder-oriented helper block in the facade instead of behind a focused Python module matching the Rust split direction.
- Current Update: Extracted the folder inventory and import-folder helper cluster into `grafana_utils/dashboards/folder_support.py` and rewired `grafana_utils/dashboard_cli.py` to import and re-export the stable helper names used by tests and workflows. Added Python 3.6 syntax coverage for both extracted helper modules so the reduced facade and new helper files stay parseable under the repo's compatibility target.
- Result: The Python dashboard facade now carries less folder-specific plumbing, helper ownership is closer to the Rust dashboard split without changing Rust itself, and the existing `grafana_utils.dashboard_cli` helper surface remains behavior-compatible for callers and tests.

## 2026-03-13 - Task: Split Python Dashboard Import Diff Helpers
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `grafana_utils/dashboards/import_support.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `grafana_utils/dashboard_cli.py` had already moved high-level import orchestration into `grafana_utils/dashboards/import_workflow.py`, but the facade still owned the low-level import payload parsing, export-manifest validation wrappers, dry-run renderers, and diff comparison helpers in one large inline block. That kept import/diff-specific logic mixed into the top-level dashboard CLI module even after earlier workflow and listing splits.
- Current Update: Extracted the import/diff helper cluster into `grafana_utils/dashboards/import_support.py` and rewired `grafana_utils/dashboard_cli.py` to import and re-export the stable helper surface used by the CLI workflows and tests. The refactor also drops the duplicate local `build_preserved_web_import_document()` implementation in favor of the existing transformer helper.
- Result: The Python dashboard facade carries less import/diff plumbing, helper ownership is more explicit for future dashboard complexity reduction work, and the public `grafana_utils.dashboard_cli` helper surface remains behavior-compatible for tests and callers.

## 2026-03-13 - Task: Split Python Dashboard Listing Helpers
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `grafana_utils/dashboards/listing.py`, `tests/test_python_dashboard_cli.py`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `grafana_utils/dashboard_cli.py` had already shed export/import/inspection responsibilities, but live dashboard listing, datasource-list rendering, and dashboard datasource-source enrichment still lived inline in the main CLI facade. That left one large block mixing list command orchestration, table/CSV/JSON renderers, and datasource lookup helpers in the same file as unrelated dashboard flows.
- Current Update: Extracted the live dashboard/datasource listing helpers into `grafana_utils/dashboards/listing.py`, including table/CSV/JSON renderers, folder-path/org/source enrichment, datasource UID/name resolution, and the two list command bodies. `grafana_utils/dashboard_cli.py` now re-exports the existing helper names and delegates `list-dashboard` / `list-data-sources` through the extracted module so the stable test and CLI surface stays intact.
- Result: The Python dashboard facade carries less list-specific logic, the list responsibilities now live behind a focused helper boundary similar to Rust `dashboard_list.rs`, and operator-facing behavior stays unchanged.

## 2026-03-14 - Task: Add Inspect Output Format Alias
- State: In Progress
- Scope: `grafana_utils/dashboard_cli.py`, `grafana_utils/dashboards/inspection_workflow.py`, `tests/test_python_dashboard_cli.py`, `tests/test_python_dashboard_inspection_cli.py`, `rust/src/dashboard_cli_defs.rs`, `rust/src/dashboard_inspect.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Dashboard inspect output was split across legacy `--json` / `--table` summary flags plus `--report[=...]` for query-level/governance modes, which made the output contract harder to remember and explain.
- Current Update: Added `--output-format` to both `inspect-export` and `inspect-live` as a single explicit selector for `text`, `table`, `json`, `report-*`, and governance modes, while preserving the older flags for compatibility and rejecting mixed selector combinations.
- Result: Inspect output can now be requested with one clearer flag without removing old CLI spellings. The remaining work is keeping docs/examples biased toward `--output-format` over time.

## 2026-03-13 - Task: Add Datasource Inventory CLI
- State: Done
- Scope: `grafana_utils/datasource_cli.py`, `grafana_utils/unified_cli.py`, `tests/test_python_datasource_cli.py`, `tests/test_python_unified_cli.py`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The repo only exposed live datasource inventory through `grafana-utils dashboard list-data-sources`, so datasource state still lived as a dashboard-adjacent helper instead of a first-class CLI surface and there was no standalone datasource export contract yet.
- Current Update: Added a Python `grafana-utils datasource` entrypoint with `list` and `export` subcommands, kept `dashboard list-data-sources` unchanged as a compatibility path, and defined a minimal datasource export root that writes normalized `datasources.json`, `index.json`, and `export-metadata.json` files for the current org.
- Result: Datasource inventory is now available through a dedicated Python CLI surface without broad import/update semantics yet. The main remaining gaps are the later roadmap items: multi-org datasource workflows plus import/diff support and Python/Rust parity for the new resource family.

## 2026-03-14 - Task: Add Datasource Import
- State: In Progress
- Scope: `grafana_utils/datasource_cli.py`, `tests/test_python_datasource_cli.py`, `rust/src/datasource.rs`, `rust/src/datasource_rust_tests.rs`, `rust/src/cli.rs`, `rust/src/cli_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Datasource inventory could be listed and exported, but there was still no supported path to replay the normalized datasource contract back into Grafana, no dry-run for datasource imports, and no Rust datasource namespace in the unified CLI.
- Current Update: Added first-pass datasource import in both Python and Rust with dry-run/table/JSON output, explicit `--org-id` import scoping, opt-in `--require-matching-export-org`, and create/update/update-existing-only reconciliation using live datasource `uid` then exact `name` matching.
- Result: Datasource export now round-trips through a guarded import workflow on both runtimes. The main remaining gaps are secret-bearing datasource settings, broader conflict/mapping controls, and live Docker validation similar to dashboard import.

## 2026-03-13 - Task: Add Flux And SQL Dashboard Inspection Extraction
- State: Done
- Scope: `grafana_utils/dashboards/inspection_report.py`, `tests/test_python_dashboard_inspection_cli.py`, `rust/src/dashboard_inspect.rs`, `rust/src/dashboard_rust_tests.rs`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Dashboard inspection already exposed one stable per-query report contract, but Flux extraction only covered `_measurement`/`bucket` heuristics and SQL-family queries still fell back to the generic token extractor, so table/source references and coarse query shape were not surfaced usefully.
- Current Update: Kept the shared report contract unchanged and added conservative Flux/SQL-family extraction on both implementations. Flux now maps pipeline/source function names into `metrics` while keeping `_measurement` values in `measurements` and `bucket` values in `buckets`. SQL-family queries now map coarse query-shape hints into `metrics`, table/source references into `measurements`, and leave `buckets` empty because the current contract does not expose dedicated SQL fields.
- Result: Inspect report rows stay schema-compatible, but Flux and SQL-family dashboards now produce more useful best-effort extraction without widening CLI/report scope. The main remaining constraint is contractual: table refs, query-shape hints, and Flux pipeline stages still share the existing generic list fields instead of dedicated report columns.

## 2026-03-13 - Task: Split Dashboard Export Inventory Helpers
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `grafana_utils/dashboards/export_inventory.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_files.rs`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Even after the earlier workflow and inspection splits, the dashboard facades still kept raw-export file discovery, folder/datasource inventory loading, and export metadata validation inline, so both Python and Rust entry modules still mixed low-level filesystem concerns with higher-level orchestration.
- Current Update: Extracted the remaining Python raw-export helpers into `grafana_utils/dashboards/export_inventory.py`, routed the Python facade through those helpers, and kept the Rust side aligned by moving the matching helper ownership under `rust/src/dashboard_files.rs` behind the existing `dashboard.rs` re-export surface.
- Result: The Python and Rust dashboard facades now carry less raw-export plumbing, which reduces the chance that future inspect/import changes re-entangle file inventory logic with top-level CLI orchestration. Validation passed with `python3 -m unittest -v tests/test_python_dashboard_cli.py tests/test_python_dashboard_inspection_cli.py tests/test_python_unified_cli.py`, `cargo test dashboard --manifest-path rust/Cargo.toml --quiet`, and `make quality`.

## 2026-03-13 - Task: Split Python Dashboard Inspection Summary Internals
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `grafana_utils/dashboards/inspection_summary.py`, `tests/test_python_dashboard_cli.py`, `tests/test_python_dashboard_inspection_cli.py`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: After the inspection report split, `dashboard_cli.py` still kept the higher-level inspection summary document builder and summary/table renderers inline, so the Python inspection surface was still only partially decomposed and the summary-focused tests still lived in the broader dashboard CLI suite.
- Current Update: Extracted the summary document builder plus summary/table renderers into `grafana_utils/dashboards/inspection_summary.py`, routed `inspect-export` and `inspect-live` through that module using the existing inspection dependency bundle, and moved the summary-specific inspection behavior tests into `tests/test_python_dashboard_inspection_cli.py`.
- Result: Python dashboard inspection now has a clearer internal boundary between summary inspection and per-query reporting, and `dashboard_cli.py` shrank again without changing operator-facing behavior. Validation passed with `python3 -m unittest -v tests/test_python_dashboard_cli.py tests/test_python_dashboard_inspection_cli.py`.

## 2026-03-13 - Task: Stabilize Dashboard Inspection Report Internals
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `grafana_utils/dashboards/inspection_report.py`, `tests/test_python_dashboard_cli.py`, `tests/test_python_dashboard_inspection_cli.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_inspect.rs`, `rust/src/dashboard_inspect_report.rs`, `rust/src/dashboard_rust_tests.rs`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: After the earlier dashboard workflow split, inspection was still the path most likely to re-tangle. Python still mixed report constants/row normalization/rendering into the CLI surface, Rust still kept most inspection report model helpers inside the broader dashboard modules, and the Python inspection-heavy tests were still largely concentrated in the main dashboard CLI test file.
- Current Update: Centralized the Python inspection report contract in `grafana_utils/dashboards/inspection_report.py`, moved the inspection-heavy Python behavior coverage into `tests/test_python_dashboard_inspection_cli.py`, and split the Rust inspection report model/column contract into `rust/src/dashboard_inspect_report.rs` so both implementations now route flat/tree/tree-table output through a narrower dedicated inspection-report layer.
- Result: Dashboard inspection behavior stays unchanged for operators, but the canonical inspection model is now much more explicit in both implementations and the Python inspection tests are no longer piled into one giant dashboard CLI file. Validation passed with `python3 -m unittest -v tests/test_python_dashboard_cli.py tests/test_python_dashboard_inspection_cli.py tests/test_python_unified_cli.py`, `cargo test dashboard --manifest-path rust/Cargo.toml --quiet`, and `make quality`.

## 2026-03-13 - Task: Add Full Inspect Help For Dashboard CLI
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_cli_defs.rs`, `rust/src/dashboard_rust_tests.rs`, `rust/src/bin/grafana-utils.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The dashboard `inspect-export` and `inspect-live` help output stayed concise, but operators had no built-in way to ask either CLI for a richer inspect-specific examples block covering report modes like `tree-table`, filters, and `--report-columns`.
- Current Update: Added `--help-full` for `inspect-export` and `inspect-live` in both Python and Rust. The new flag prints the normal subcommand help first, then appends a short extended examples section focused on report modes, datasource/panel filters, and column trimming. Normal `-h/--help` remains unchanged.
- Result: Inspect users can now ask either CLI for richer examples without making standard help noisier. Validation passed with `python3 -m unittest -v tests/test_python_dashboard_cli.py` and `cargo test dashboard --manifest-path rust/Cargo.toml --quiet`.

## 2026-03-13 - Task: Refine Python Tree-Table Dashboard Inspect Report
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `grafana_utils/dashboards/inspection_workflow.py`, `tests/test_python_dashboard_cli.py`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The repo already had a `tree-table` trace entry, but this Python task specifically needed the Python CLI/parser/docs/tests to accept `tree-table`, honor `--report-columns`, and keep the existing flat and tree modes unchanged without touching Rust files.
- Current Update: Added Python `tree-table` support to the `inspect-export` and `inspect-live` `--report` choices, allowed `--report-columns` for that mode, and rendered grouped dashboard-first sections with one per-dashboard query table using the filtered flat query-record model.
- Result: Python operators can now use `--report tree-table` with either default or custom columns, while `table`, `csv`, `json`, and `tree` behavior remains intact. Validation passed with `python3 -m unittest -v tests/test_python_dashboard_cli.py`.

## 2026-03-13 - Task: Add Tree Dashboard Inspect Report
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `grafana_utils/dashboards/inspection_workflow.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_cli_defs.rs`, `rust/src/dashboard_inspect.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Dashboard inspection could already emit either a high-level summary or a flat row-per-query report through `inspect-export --report` / `inspect-live --report`, but operators had to scan a wide flat table or JSON array when they wanted to read one dashboard at a time.
- Current Update: Added a `--report tree` mode for both Python and Rust `inspect-export` and `inspect-live`. The new mode keeps the existing flat report model as the source of truth, applies the existing datasource and panel-id filters first, then renders the filtered records as a dashboard -> panel -> query tree without changing the existing flat `table`, `csv`, or `json` report contracts.
- Result: Operators can now inspect dashboard exports or live dashboards in a hierarchy that mirrors how Grafana is read in practice, while existing flat report automation remains unchanged. Validation passed with `python3 -m unittest -v tests/test_python_dashboard_cli.py` and `cargo test dashboard --manifest-path rust/Cargo.toml --quiet`.

## 2026-03-13 - Task: Add Tree-Table Dashboard Inspect Report
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `grafana_utils/dashboards/inspection_workflow.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_cli_defs.rs`, `rust/src/dashboard_inspect.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `--report tree` improved readability for dashboard-first inspection, but it intentionally rendered free-form text lines instead of preserving a columnar view. Operators who wanted dashboard-first grouping still had to switch back to the flat table when they needed aligned columns.
- Current Update: Added `--report tree-table` for both Python and Rust `inspect-export` and `inspect-live`. The new mode keeps the same filtered flat query-record model as the source of truth, groups rows by dashboard, then renders one compact table per dashboard section. `--report-columns` now also applies to `tree-table`, and Python `--no-header` handling now treats `tree-table` as a supported table-like mode.
- Result: Operators can inspect one dashboard at a time without giving up column alignment. Validation passed with `python3 -m unittest -v tests/test_python_dashboard_cli.py`, `cargo test dashboard --manifest-path rust/Cargo.toml --quiet`, `python3 python/grafana-utils.py dashboard inspect-export --help`, and `cargo run --manifest-path rust/Cargo.toml --quiet --bin grafana-utils -- dashboard inspect-export --help`.

## 2026-03-13 - Task: Add Basic Quality Gates
- State: Done
- Scope: `.github/workflows/ci.yml`, `Makefile`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The repo had strong unit test coverage, but quality enforcement still depended on developers manually running local commands. There was no checked-in CI workflow and no shared shortcut that matched the repo's baseline automated gates.
- Current Update: Added a baseline GitHub Actions workflow with separate Python and Rust jobs, and introduced `make quality`, `make fmt-rust-check`, and `make lint-rust` so local and CI checks use the same entrypoints. The first baseline intentionally stays pragmatic: Python unit tests plus Rust tests, `cargo fmt --check`, and `cargo clippy --all-targets -- -D warnings`.
- Result: The repo now has a minimum automated quality gate instead of relying only on local discipline, and maintainers have one documented local command that matches the CI baseline. Validation passed with `make quality`.

## 2026-03-13 - Task: Split Rust Dashboard Orchestration Modules
- State: Done
- Scope: `rust/src/dashboard.rs`, `rust/src/dashboard_import.rs`, `rust/src/dashboard_inspect.rs`, `rust/src/dashboard_rust_tests.rs`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `rust/src/dashboard.rs` had regrown past 3000 lines and still mixed shared types/helpers with import/diff orchestration plus inspect-export/inspect-live analysis and rendering. The Rust dashboard surface was behaviorally healthy again, but the main module had resumed accumulating too many responsibilities.
- Current Update: Extracted import and diff orchestration into `rust/src/dashboard_import.rs`, moved inspect-export and inspect-live analysis/rendering into `rust/src/dashboard_inspect.rs`, and kept the `crate::dashboard` API stable through targeted re-exports used by the CLI paths and tests. The remaining `rust/src/dashboard.rs` now focuses more clearly on shared types/helpers plus top-level entrypoints.
- Result: The Rust dashboard implementation is materially easier to evolve: `rust/src/dashboard.rs` dropped to roughly 1287 lines, while import/diff and inspect/live flows now live in dedicated modules without changing operator-facing behavior. Validation passed with `cargo test dashboard --manifest-path rust/Cargo.toml --quiet` and `make quality`.

## 2026-03-13 - Task: Split Python Dashboard Orchestration Modules
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `grafana_utils/dashboards/__init__.py`, `grafana_utils/dashboards/export_workflow.py`, `grafana_utils/dashboards/inspection_workflow.py`, `grafana_utils/dashboards/import_workflow.py`, `tests/test_python_dashboard_cli.py`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `grafana_utils/dashboard_cli.py` has grown into a 3700+ line module that still mixes CLI parsing, rendering helpers, data-shape helpers, and the high-level export/import/inspect orchestration flows in one file. The Python dashboard path works, but the orchestration layer is harder to change safely than the already-split Rust implementation.
- Current Update: Extracted the high-level Python dashboard export, import, and inspection workflow bodies into `grafana_utils/dashboards/export_workflow.py`, `grafana_utils/dashboards/import_workflow.py`, and `grafana_utils/dashboards/inspection_workflow.py`. `grafana_utils/dashboard_cli.py` now delegates through explicit dependency bundles so the existing CLI entrypoints, shared helpers, and direct test imports stay stable while the main module shrinks materially.
- Result: The Python dashboard CLI keeps the same operator-facing behavior, but its top-level module is smaller and future workflow changes can now land in focused orchestration modules instead of growing one file. Validation passed with `python3 -m unittest -v tests/test_python_dashboard_cli.py`.

## 2026-03-13 - Task: Add Inspect Export Orphaned Datasources
- State: Done
- Scope: `grafana_utils/dashboards/inspection_summary.py`, `tests/test_python_dashboard_inspection_cli.py`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `inspect-export` already surfaced datasource inventory records with per-datasource reference and dashboard counts, but operators still had to scan the whole inventory manually to spot datasources that were exported yet unused by any dashboard.
- Current Update: Added explicit orphaned-datasource accounting to the Python inspection summary path so `inspect-export` now records `orphanedDatasourceCount`, exposes `orphanedDatasources` in JSON output, and renders a dedicated orphaned-datasource section in both the human summary and `--table` output.
- Result: Operators can now identify unused exported datasources directly from the inspection summary without scripting against the inventory rows or manually filtering for zero-reference entries.

## 2026-03-13 - Task: Add Dashboard Inspect Live Command
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Dashboard inspection currently requires a raw export directory on disk via `inspect-export`. Operators can inspect exported data offline, but there is no direct live Grafana inspection command that reuses the same summary/report output contract.
- Current Update: Added an `inspect-live` dashboard subcommand in both Python and Rust that accepts live auth/common args plus `inspect-export`-style summary/report flags, materializes a temporary raw-export-like layout from live dashboards, folders, and datasources, and then reuses the existing `inspect-export` analysis/rendering pipeline. Added parser/help coverage and focused report-path tests, then updated the public and maintainer docs.
- Result: Operators can now inspect live Grafana dashboards with the same summary/report surface they already use for raw export directories, without manually running export first. Validation passed with `python3 -m unittest -v tests/test_python_dashboard_cli.py`.

## 2026-03-13 - Task: Add Inspect Report Datasource UID
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_cli_defs.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `inspect-export --report` already carried datasource labels, but JSON rows did not expose datasource UIDs and the table/CSV column contract had no way to opt them in without widening the default report layout.
- Current Update: Added best-effort `datasourceUid` to the per-query inspection row model, kept it in JSON report output by default, and exposed it as an opt-in `datasource_uid` column for table/CSV output so the common default report shape stays unchanged. The CLI help and docs now describe that split behavior.
- Result: Operators can now script against datasource UIDs from JSON output immediately, while table and CSV users can request `datasource_uid` only when they need it. Validation passed with `python3 -m unittest -v tests/test_python_dashboard_cli.py` and `cargo test dashboard --manifest-path rust/Cargo.toml --quiet`.

## 2026-03-13 - Task: Add Dashboard Inspect Query Report
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_cli_defs.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `inspect-export` could summarize dashboard, folder, panel, query, and datasource counts plus mixed-datasource usage, but it did not emit one per-target query report and did not extract metric-like identifiers from query expressions for table or JSON inspection output.
- Current Update: Added `inspect-export --report[=table|json]` in both Python and Rust, built a per-query offline inspection model with dashboard/panel/datasource/query context, extracted heuristic `metrics`, `measurements`, and `buckets`, added `--report-columns`, `--report-filter-datasource`, and `--report-filter-panel-id` for narrower operator workflows, aligned the new flags in docs, and noted that future parser growth should stay split by datasource family.
- Result: Operators can now inspect exported dashboards at query-target granularity from raw export directories, use table output by default or JSON for downstream analysis, narrow the report to one datasource or one panel id, and trim table output to selected columns. Validation passed with `python3 -m unittest -v tests/test_python_dashboard_cli.py`, `cargo test dashboard --manifest-path rust/Cargo.toml --quiet`, and real sample runs against `tmp/recheck-export-20260313/raw`.

## 2026-03-13 - Task: Tighten Dashboard Typed Records And Integration Coverage
- State: Done
- Scope: `grafana_utils/dashboards/common.py`, `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `tests/test_python_dashboard_integration_flow.py`, `rust/src/dashboard_prompt.rs`, `rust/src/dashboard_list.rs`, `rust/src/dashboard_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Dashboard code still repeated fallback literals such as `General`, `Main Org.`, and `unknown` across Python export/import/inspect flows, Rust prompt export still passed datasource catalogs around as anonymous tuple maps, and the Python dashboard suite mostly validated helpers in isolation rather than one end-to-end raw-export inspection and dry-run import flow.
- Current Update: Extracted shared Python dashboard fallback constants into `grafana_utils/dashboards/common.py`, updated dashboard summary and export/import inspection paths to reuse them, replaced Rust's tuple-shaped datasource catalog with a named `DatasourceCatalog { by_uid, by_name }`, and added focused Python integration-style tests for offline `inspect-export --json` plus `import-dashboard --dry-run --json --ensure-folders`.
- Result: Dashboard fallback behavior is easier to keep consistent, Rust datasource resolution now has a typed boundary instead of anonymous paired maps, and the Python suite now covers a higher-value raw-export to inspect/import dry-run workflow without depending on live Grafana.

## 2026-03-13 - Task: Include Dashboard Sources By Default In JSON List Output
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard_list.rs`, `rust/src/dashboard_cli_defs.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `list-dashboard --with-sources` existed mainly to keep text and table output from getting too wide and expensive, but JSON mode also required the extra flag even though machine-readable output benefits more from completeness than compactness.
- Current Update: Changed both Python and Rust dashboard list flows so `--json` automatically fetches dashboard payloads plus the datasource catalog and includes `sources` and `sourceUids` by default, while plain, table, and CSV output still require `--with-sources` to opt into the more expensive datasource expansion.
- Result: JSON list output is now self-contained for script consumers, while operator-facing table and CSV output remain compact unless users explicitly ask for datasource expansion.

## 2026-03-13 - Task: Export Datasource Inventory With Raw Dashboard Exports
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_export.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Raw dashboard export already wrote `folders.json`, but it did not persist the live Grafana datasource catalog anywhere. `inspect-export` could summarize datasource references seen inside dashboard JSON, but it could not report the exported datasource inventory or compare unused datasources against dashboard usage offline.
- Current Update: Added `raw/datasources.json` plus `export-metadata.json::datasourcesFile`, wrote datasource inventory records during Python and Rust raw exports, and extended `inspect-export` human, table, and JSON outputs to include datasource inventory records with usage counts derived from dashboard references.
- Result: Raw exports now carry both folder and datasource inventories, and offline inspection can show which exported datasources are used, unused, or only partially referenced across the exported dashboards.

## 2026-03-12 - Task: Align Prompt Export Labels With Grafana External Export
- State: Done
- Scope: `grafana_utils/dashboards/transformer.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard_prompt.rs`, `rust/src/dashboard_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Dashboard prompt export used Grafana-style `__inputs`, but the human-facing fields still drifted from Grafana external export behavior. Input `name` used stable internal placeholders such as `DS_PROMETHEUS_1`, while `label` and `pluginName` were generated from datasource type strings like `Prometheus datasource` and `prometheus` instead of preserving the original datasource name and a human-readable plugin title.
- Current Update: Changed both Python and Rust prompt-export rewrite paths to carry datasource display names through resolution, keep `DS_*` internal placeholder keys stable, emit `__inputs.label` from the original datasource name when known, and emit human-readable `pluginName` values such as `Prometheus` instead of raw type ids.
- Result: Prompt exports now stay closer to Grafana external export shape for human-facing datasource prompts while preserving the existing placeholder mapping strategy and prompt rewrite flow.

## 2026-03-12 - Task: Split Python Access Client And Models
- State: Done
- Scope: `grafana_utils/access_cli.py`, `grafana_utils/clients/access_client.py`, `grafana_utils/access/common.py`, `grafana_utils/access/models.py`, `tests/test_python_access_cli.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `grafana_utils/access_cli.py` was still the largest Python module in the repo and mixed CLI parsing, Grafana access-management HTTP client behavior, row normalization, table/CSV/JSON rendering, and user/team/service-account workflows in one file.
- Current Update: Extracted the Grafana access API wrapper into `grafana_utils/clients/access_client.py`, moved row normalization and rendering helpers into `grafana_utils/access/models.py`, added `grafana_utils/access/common.py` for shared access constants and exceptions, and kept `grafana_utils/access_cli.py` as the stable facade by importing and re-exporting the moved pieces.
- Result: All three large Python CLIs now follow the same direction: the top-level `*_cli.py` modules are more orchestration-focused, while transport and domain-formatting logic live in smaller reusable modules.

## 2026-03-12 - Task: Split Rust Alert Module Internals
- State: Done
- Scope: `rust/src/alert.rs`, `rust/src/alert_cli_defs.rs`, `rust/src/alert_client.rs`, `rust/src/alert_list.rs`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `rust/src/alert.rs` had grown into a 2200+ line mixed module that combined clap definitions, auth-context building, the Grafana provisioning client, list rendering, export/import/diff orchestration, and shared alert document helpers in one file.
- Current Update: Split the Rust alert implementation into internal modules without changing the public alert CLI API or the existing test imports. `alert_cli_defs.rs` now owns clap parsing and auth normalization, `alert_client.rs` owns the Grafana alert provisioning client plus shared response parsers, and `alert_list.rs` owns list rendering and list-command dispatch. `alert.rs` now keeps the remaining alert document helpers plus export/import/diff orchestration.
- Result: The Rust alert implementation is materially easier to navigate and extend while preserving the existing `crate::alert` API, unified CLI behavior, and focused Rust tests.

## 2026-03-12 - Task: Split Python Alert Client And Provisioning Helpers
- State: Done
- Scope: `grafana_utils/alert_cli.py`, `grafana_utils/clients/alert_client.py`, `grafana_utils/alerts/common.py`, `grafana_utils/alerts/provisioning.py`, `tests/test_python_alert_cli.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `grafana_utils/alert_cli.py` still mixed CLI parsing, Grafana alerting HTTP client behavior, linked-dashboard rewrite logic, alert provisioning import/export normalization, and list/export/import/diff orchestration in one 2100+ line Python module.
- Current Update: Extracted the alerting API wrapper into `grafana_utils/clients/alert_client.py`, moved provisioning import/export and linked-dashboard rewrite helpers into `grafana_utils/alerts/provisioning.py`, added `grafana_utils/alerts/common.py` for shared alert constants and exceptions, and kept `grafana_utils/alert_cli.py` as the stable CLI-facing facade by importing and re-exporting the moved helpers.
- Result: The Python alert implementation now follows the same split direction as the dashboard refactor and the existing Rust design: `alert_cli.py` is more focused on orchestration, while transport and provisioning logic live in dedicated Python modules that are easier to test and reuse.

## 2026-03-12 - Task: Split Python Dashboard Client And Prompt Transformer
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `grafana_utils/clients/dashboard_client.py`, `grafana_utils/dashboards/common.py`, `grafana_utils/dashboards/transformer.py`, `tests/test_python_dashboard_cli.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `grafana_utils/dashboard_cli.py` still mixed CLI parsing, Grafana HTTP transport behavior, prompt-export datasource rewrite helpers, and dashboard list/export/import orchestration in one 2400+ line Python module.
- Current Update: Extracted the dashboard HTTP wrapper into `grafana_utils/clients/dashboard_client.py`, moved prompt-export datasource rewrite and datasource-resolution helpers into `grafana_utils/dashboards/transformer.py`, added `grafana_utils/dashboards/common.py` for shared dashboard constants and exceptions, and kept `grafana_utils/dashboard_cli.py` as the stable facade by importing and re-exporting the moved pieces.
- Result: The Python dashboard implementation now follows the same split direction as the Rust dashboard modules: the CLI module stays focused on orchestration, while the client and prompt-transform code live in dedicated Python modules that are easier to test and reuse.

## 2026-03-12 - Task: Split Rust Access Module Internals
- State: Done
- Scope: `rust/src/access.rs`, `rust/src/access_cli_defs.rs`, `rust/src/access_render.rs`, `rust/src/access_user.rs`, `rust/src/access_team.rs`, `rust/src/access_service_account.rs`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `rust/src/access.rs` had grown into an 1800-line mixed module that combined clap definitions, auth/client setup, output rendering, request helpers, user flows, team flows, service-account flows, and top-level dispatch.
- Current Update: Split the Rust access implementation into internal modules without changing the public access CLI API or test entrypoints. `access_cli_defs.rs` now owns clap/auth/client setup, `access_render.rs` owns formatting and normalization helpers, `access_user.rs` owns user flows, `access_team.rs` owns team flows, and `access_service_account.rs` owns service-account flows. `access.rs` now keeps shared request wrappers, re-exports, and top-level dispatch.
- Result: The Rust access implementation is materially easier to navigate and evolve while preserving the existing `crate::access` API, CLI behavior, and focused test imports.

## 2026-03-12 - Task: Type Rust Dashboard Export Metadata And Index Models
- State: Done
- Scope: `rust/src/dashboard.rs`, `rust/src/dashboard_export.rs`, `rust/src/dashboard_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Rust dashboard export flow already validated fixed-schema files like `export-metadata.json` and `index.json`, but it still built and re-read those documents through ad hoc `Map<String, Value>` objects.
- Current Update: Replaced the fixed-schema dashboard export metadata and index helpers with typed Rust structs using `serde` derives, kept JSON field names stable through `serde` renames, and added focused serialization tests for the root index and export metadata shapes.
- Result: The dashboard export manifest path now gets stronger compile-time structure without changing the on-disk JSON format or the existing import/export CLI behavior.

## 2026-03-12 - Task: Move Python Source-Tree Wrapper To python/ And Remove Python Access Shim
- State: Done
- Scope: `python/grafana-utils.py`, `grafana_utils/unified_cli.py`, `grafana_utils/access_cli.py`, `pyproject.toml`, `scripts/test-python-access-live-grafana.sh`, `tests/test_python_packaging.py`, `tests/test_python_unified_cli.py`, `tests/test_python_access_cli.py`, `tests/test_python_dashboard_cli.py`, `README.md`, `DEVELOPER.md`, `AGENTS.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Python source-tree usage still lived under `cmd/`, and the repo still shipped a Python `grafana-access-utils` wrapper plus console-script entry even after `grafana-utils access ...` became the primary Python access path.
- Current Update: Moved the source-tree Python wrapper to `python/grafana-utils.py`, removed the Python `grafana-access-utils` wrapper and console-script entry, updated the live access smoke script to invoke `python/grafana-utils.py access ...`, and refreshed current docs/tests to use the single Python command shape.
- Result: Python checkout usage now matches the unified CLI direction more cleanly: one source-tree wrapper under `python/` and one Python command surface built around `grafana-utils ...`.

## 2026-03-12 - Task: Split Rust Dashboard Prompt Rewrite Module
- State: Done
- Scope: `rust/src/dashboard.rs`, `rust/src/dashboard_prompt.rs`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: After the first dashboard module split, `rust/src/dashboard.rs` still carried the largest remaining pure-transformation block: datasource resolution, prompt-export templating rewrites, and `build_external_export_document`.
- Current Update: Moved the dashboard prompt-export datasource resolution and template-rewrite pipeline into `rust/src/dashboard_prompt.rs`, then kept the existing `crate::dashboard` API stable through re-exports needed by sibling modules and tests.
- Result: The remaining `dashboard.rs` now reads more like orchestration plus shared IO/import/diff logic, while the prompt-export transformation logic lives in its own focused internal module.

## 2026-03-12 - Task: Split Rust Dashboard Module Internals
- State: Done
- Scope: `rust/src/dashboard.rs`, `rust/src/dashboard_cli_defs.rs`, `rust/src/dashboard_list.rs`, `rust/src/dashboard_export.rs`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `rust/src/dashboard.rs` had grown into a 2700+ line module that mixed clap definitions, auth/client setup, dashboard and datasource list rendering, multi-org list/export orchestration, prompt-export rewrite logic, import flow, diff flow, and shared file helpers in one file.
- Current Update: Split the Rust dashboard implementation into internal modules without changing the public dashboard API or CLI behavior. `dashboard_cli_defs.rs` now owns clap/auth/client setup, `dashboard_list.rs` owns dashboard and datasource listing plus renderers, and `dashboard_export.rs` owns export pathing plus multi-org export orchestration. `dashboard.rs` now re-exports the same public entrypoints and keeps the remaining shared helpers, prompt rewrite, import, and diff flows.
- Result: The Rust dashboard implementation is materially smaller and easier to navigate while preserving the existing CLI surface and test entrypoints.

## 2026-03-12 - Task: Remove grafana-alert-utils Compatibility Shim
- State: Done
- Scope: `pyproject.toml`, `grafana_utils/unified_cli.py`, `grafana_utils/alert_cli.py`, `tests/test_python_alert_cli.py`, `tests/test_python_packaging.py`, `rust/src/alert.rs`, `rust/src/cli.rs`, `rust/src/alert_rust_tests.rs`, `rust/src/cli_rust_tests.rs`, `scripts/build-rust-macos-arm64.sh`, `scripts/build-rust-linux-amd64.sh`, `scripts/build-rust-linux-amd64-zig.sh`, `scripts/test-rust-live-grafana.sh`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The repo had already consolidated alert workflows under `grafana-utils alert ...`, but still shipped a separate `grafana-alert-utils` Python wrapper, console script, Rust binary, and build artifacts as a compatibility shim.
- Current Update: Removed the Python wrapper, Python console-script entry, Rust standalone alert binary, and build-script artifact copies for `grafana-alert-utils`. Current docs, help text, smoke scripts, and tests now use `grafana-utils alert ...` as the only alert entrypoint.
- Result: The repo now exposes one primary alert command surface instead of keeping a second standalone alert executable alive after the unified CLI migration.

## 2026-03-12 - Task: Add Alert List Commands And Direct Alert Aliases
- State: Done
- Scope: `grafana_utils/alert_cli.py`, `grafana_utils/unified_cli.py`, `tests/test_python_alert_cli.py`, `tests/test_python_unified_cli.py`, `tests/test_python_packaging.py`, `rust/src/alert.rs`, `rust/src/cli.rs`, `rust/src/alert_rust_tests.rs`, `rust/src/cli_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Alert workflows already had explicit `export`, `import`, and `diff`, but there was still no read-only alert listing surface and no direct-form aliases such as `export-alert` or `list-alert-rules`.
- Current Update: Added `grafana-utils alert list-rules`, `list-contact-points`, `list-mute-timings`, and `list-templates` in Python and Rust, with default table output plus `--csv`, `--json`, and `--no-header`. Also added top-level direct aliases `export-alert`, `import-alert`, `diff-alert`, and `list-alert-*`.
- Result: Alert workflows now match the dashboard command family more closely: there is an explicit read-only surface for common alert resource types, and operators can use either the canonical namespace form or the shorter direct alert aliases.

## 2026-03-12 - Task: Split Alert CLI Into Export Import Diff Subcommands
- State: Done
- Scope: `grafana_utils/alert_cli.py`, `grafana_utils/unified_cli.py`, `tests/test_python_alert_cli.py`, `tests/test_python_unified_cli.py`, `rust/src/alert.rs`, `rust/src/cli.rs`, `rust/src/alert_rust_tests.rs`, `rust/src/cli_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Alerting workflows still used one flat CLI surface driven by `--output-dir`, `--import-dir`, or `--diff-dir`. That made `grafana-utils alert` inconsistent with the dashboard namespace and hid the available alert modes from command help.
- Current Update: Added explicit `export`, `import`, and `diff` alert subcommands in both Python and Rust. The unified command now supports `grafana-utils alert export|import|diff ...`, while the standalone compatibility shim also supports `grafana-alert-utils export|import|diff ...`. Legacy flag-only invocation still works for compatibility.
- Result: The alert CLI now advertises its three modes directly in help output and matches the namespace style already used by `grafana-utils dashboard ...` and `grafana-utils access ...`.

## 2026-03-12 - Task: Make Dashboard List Default To Tables And Add Progress Flags
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The dashboard list commands still defaulted to compact single-line text output, table headers could not be suppressed, and dashboard export/import printed per-dashboard progress lines by default instead of only when explicitly requested.
- Current Update: Changed Python and Rust `list-dashboard` plus `list-data-sources` to default to table output, added `--no-header` for those table-oriented list commands, and added `--progress` to `export-dashboard` and `import-dashboard` so per-dashboard progress lines are opt-in.
- Result: Operators now get a more readable default listing format, can remove table headers for scripts or copy/paste workflows, and can choose whether dashboard export/import should stay quiet or show item-by-item progress.

## 2026-03-12 - Task: Add Concise And Verbose Dashboard Progress Modes
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard_cli_defs.rs`, `rust/src/dashboard_export.rs`, `rust/src/dashboard.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Dashboard export and import only had a single `--progress` mode, which printed detailed per-item lines and did not provide a lighter-weight progress view for long runs.
- Current Update: Added a concise `--progress` mode for both Python and Rust dashboard export/import that prints one `current/total` line per dashboard, plus a new `-v/--verbose` mode that keeps detailed path/status output and supersedes the concise progress form.
- Result: Operators can now choose between quiet summary-only runs, compact progress for long jobs, or detailed item-by-item logging for troubleshooting.

## 2026-03-13 - Task: Add Dry-Run Import Table Output
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard_cli_defs.rs`, `rust/src/dashboard.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Dashboard import dry-run output was line-oriented only, so operators could not switch to a compact summary table when reviewing a larger batch.
- Current Update: Added `import-dashboard --dry-run --table` plus `--no-header` support in both Python and Rust, while rejecting `--table` outside dry-run mode.
- Result: Operators can keep the default line-oriented dry-run output or opt into a summary table that is easier to scan or pipe into snapshots.

## 2026-03-13 - Task: Add Update-Existing-Only Dashboard Import Mode
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard_cli_defs.rs`, `rust/src/dashboard.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Dashboard import either created missing dashboards or failed on existing ones unless `--replace-existing` was set, but there was no mode for large local batches that should update only existing dashboard UIDs and ignore everything else.
- Current Update: Added `--update-existing-only` in Python and Rust dashboard import flows so matching UIDs update, missing UIDs are skipped, dry-run predicts `skip-missing`, and the summary/output modes report skipped counts clearly.
- Result: Operators can now point a large local raw export set at Grafana and safely reconcile only the dashboards that already exist there without accidentally creating the rest.

## 2026-03-13 - Task: Add Folder Inventory Export And Ensure-Folders Import
- State: Done
- Scope: `grafana_utils/clients/dashboard_client.py`, `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_export.rs`, `rust/src/dashboard_cli_defs.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Raw dashboard export preserved each dashboard's `folderUid`, but there was no exported folder inventory for rebuilding missing destination folders, so cross-environment imports still required manual folder setup.
- Current Update: Raw dashboard export now writes `raw/folders.json` and records `foldersFile` in the raw export manifest. Dashboard import gained `--ensure-folders`, which uses that inventory to create missing parent/child folders before importing dashboards, and `--dry-run --ensure-folders` now reports folder missing/match/mismatch state so operators can spot folder drift before a real run.
- Result: Operators can export one environment, move the raw payloads, let the importer recreate the referenced folder chain automatically, and validate folder path parity in dry-run mode instead of pre-creating every folder UID by hand.

## 2026-03-12 - Task: Consolidate Python And Rust CLIs Under grafana-utils
- State: Done
- Scope: `grafana_utils/unified_cli.py`, `grafana_utils/dashboard_cli.py`, `grafana_utils/alert_cli.py`, `cmd/grafana-utils.py`, `cmd/grafana-alert-utils.py`, `cmd/grafana-access-utils.py`, `pyproject.toml`, `tests/test_python_unified_cli.py`, `tests/test_python_packaging.py`, `rust/src/cli.rs`, `rust/src/cli_rust_tests.rs`, `rust/src/bin/grafana-utils.rs`, `rust/src/dashboard.rs`, `rust/src/alert.rs`, `rust/src/lib.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The repo had three split command names across Python and Rust. Dashboard already lived under `grafana-utils`, but alerting and access used separate primary binaries and docs still described the access path as split or Python-first.
- Current Update: Added a unified Python dispatcher and a unified Rust dispatcher so `grafana-utils` is now the primary command for `dashboard`, `alert`, and `access` workflows. Old dashboard direct forms such as `grafana-utils export-dashboard ...` still work as compatibility paths, and `grafana-alert-utils` plus `grafana-access-utils` remain available as shims.
- Result: Operators can now use one primary command shape in both implementations, while older scripts and muscle memory keep working through compatibility entrypoints during the transition.

## 2026-03-12 - Task: Add Developer Grafana Sample-Data Seed Script
- State: Done
- Scope: `scripts/seed-grafana-sample-data.sh`, `Makefile`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Developer live testing was relying on one-off manual API calls to create sample datasources, folders, dashboards, and extra orgs. That made repeated verification of `list-dashboard`, `export-dashboard`, and `list-data-sources` less reproducible.
- Current Update: Added `make seed-grafana-sample-data`, `make destroy-grafana-sample-data`, `make reset-grafana-all-data`, and a dedicated shell script that seeds, removes, or aggressively resets a running Grafana test dataset with stable sample orgs, datasources, folders, and dashboards using fixed ids and overwrite-friendly upserts.
- Result: Developers now have repo-owned setup, cleanup, and disposable-instance reset commands for rebuilding the same manual test dataset instead of repeating ad hoc setup steps during local Grafana testing.

## 2026-03-12 - Task: Add Prompted Basic-Auth Password Support
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `grafana_utils/alert_cli.py`, `grafana_utils/access_cli.py`, `tests/test_python_dashboard_cli.py`, `tests/test_python_alert_cli.py`, `tests/test_python_access_cli.py`, `rust/Cargo.toml`, `rust/src/common.rs`, `rust/src/common_rust_tests.rs`, `rust/src/dashboard.rs`, `rust/src/dashboard_rust_tests.rs`, `rust/src/alert.rs`, `rust/src/alert_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The CLIs only supported token auth, explicit `--basic-password`, or environment fallback password input. Operators who wanted Basic auth had to expose the password through shell history, process arguments, or environment variables.
- Current Update: Added `--prompt-password` everywhere Basic auth is supported, wired it into the shared Python and Rust auth resolvers, and added validation that rejects mixing prompt mode with token auth or explicit `--basic-password`.
- Result: Operators can now run Basic-auth commands with `--basic-user ... --prompt-password` and enter the password securely without echo while keeping the existing token and environment-based auth paths.

## 2026-03-12 - Task: Add Platform-Specific Rust Build Paths
- State: Done
- Scope: `Makefile`, `scripts/build-rust-macos-arm64.sh`, `scripts/build-rust-linux-amd64.sh`, `scripts/build-rust-linux-amd64-zig.sh`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The repo could build native Rust release binaries on the current host only, but there was no explicit platform-targeted release workflow. In particular, macOS Apple Silicon and Linux `amd64` outputs did not have named Make targets or stable artifact directories.
- Current Update: Added `make build-rust-macos-arm64` for native Apple Silicon builds into `dist/macos-arm64/`, `make build-rust-linux-amd64` for Docker-based Linux `amd64` builds into `dist/linux-amd64/`, and `make build-rust-linux-amd64-zig` for non-Docker Linux `amd64` builds using local `zig`.
- Result: Operators on macOS now have explicit repo-owned release paths for native Apple Silicon binaries plus Linux `amd64` binaries through either Docker or local zig.

## 2026-03-12 - Task: Update Dashboard Help Examples And Local Default URL
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The dashboard CLI still defaulted to `http://127.0.0.1:3000`, and the real `-h` output either lacked examples entirely or only showed token-based remote examples. That made first-run local usage harder, especially for operators using Basic auth.
- Current Update: Changed the dashboard CLI default URL to `http://localhost:3000`, updated Python and Rust help output to show local Basic-auth examples plus token examples, and refreshed the public and maintainer docs to match the new local-first help text.
- Result: The shipped Python and Rust dashboard CLIs now guide operators toward a working local Grafana flow directly from `-h`, while still documenting token auth when needed.

## 2026-03-12 - Task: Add Dashboard Multi-Org Export
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `export-dashboard` only operated in the current Grafana org context. Operators could not export one explicit org or aggregate exports across all visible orgs, even after `list-dashboard` gained org selection support.
- Current Update: Added `--org-id` and `--all-orgs` to Python and Rust `export-dashboard`. Both paths are Basic-auth-only. Explicit-org export reuses the existing layout, while multi-org export writes `org_<id>_<name>/raw/...` and `org_<id>_<name>/prompt/...` trees plus aggregate root-level variant indexes so cross-org dashboards do not overwrite each other.
- Result: Operators can now export dashboards from one chosen org or every visible org without manually switching Grafana org context first.

## 2026-03-12 - Task: Add Dashboard Multi-Org Listing
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `list-dashboard` already exposed current-org metadata in each row, but it still only listed dashboards in the current request org context. Operators could not point the command at another org or aggregate dashboards across all visible orgs from one run.
- Current Update: Added `--org-id` and `--all-orgs` to Python and Rust `list-dashboard`. The command now accepts one explicit org override or enumerates `/api/orgs` and aggregates dashboard results across all visible orgs. Both paths are Basic-auth-only and preserve the existing `org` and `orgId` output fields for every listed dashboard.
- Result: Operators can now inspect one chosen Grafana org or all visible orgs from a single `list-dashboard` run instead of being limited to the auth context's current org.

## 2026-03-12 - Task: Add Dashboard Datasource Listing Command
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The dashboard CLI could list dashboards and could fetch the datasource catalog internally, but there was no dedicated operator command to inspect Grafana data sources directly with table, CSV, or JSON output.
- Current Update: Added `list-data-sources` in both Python and Rust, reusing the existing datasource list API path and adding compact text, `--table`, `--csv`, and `--json` renderers for `uid`, `name`, `type`, `url`, and `isDefault`.
- Result: Operators can now inspect live Grafana data sources directly from `grafana-utils` without exporting dashboards or reading raw API responses.

## 2026-03-12 - Task: Rename Dashboard CLI Subcommands
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The dashboard CLI exposed short subcommand names `export`, `list`, and `import`, while the repo now also contains separate alerting and access CLIs. The shorter names made the dashboard actions look inconsistent next to the more explicit access subcommands and left room for ambiguity when reading docs quickly.
- Current Update: Renamed the dashboard CLI subcommands to `export-dashboard`, `list-dashboard`, and `import-dashboard` in both Python and Rust, updated focused parser/help coverage, and refreshed public and maintainer docs to use the new names consistently.
- Result: Dashboard operations now read explicitly at the CLI boundary, and both Python and Rust `grafana-utils` help/output surfaces match the renamed operator workflow.

## 2026-03-12 - Task: Add Dashboard List Org Metadata
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The dashboard `list` subcommand already showed folder and datasource context, but operators still could not see which Grafana organization the current authenticated view belonged to in text, table, CSV, or JSON output.
- Current Update: Added one current-org fetch through `GET /api/org` in both Python and Rust dashboard list paths, attached `org` and `orgId` to every listed dashboard summary, and extended the renderer/tests so compact text, table, CSV, and JSON output all include those fields alongside the existing folder and optional datasource metadata.
- Result: Operators can now tell which Grafana org produced a given dashboard list result without guessing from the base URL or credentials, and machine-readable list consumers now receive stable `org` and `orgId` fields in both Python and Rust.

## 2026-03-12 - Task: Add Dashboard List Datasource Display
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The dashboard `list` subcommand already showed `uid`, `name`, `folder`, `folderUid`, and resolved folder path, but it could not show which datasource names each dashboard used.
- Current Update: Added an opt-in `--with-sources` flag to both Python and Rust dashboard list paths. When enabled, the command fetches the datasource catalog and each dashboard payload, resolves datasource references into display names, and appends those names to text, table, CSV, and JSON output. CSV output also carries a best-effort `sourceUids` column.
- Result: Operators can now inspect dashboard datasource usage directly from `grafana-utils list-dashboard --with-sources` without exporting dashboard files, while plain `list-dashboard` remains unchanged and cheaper. CSV consumers can also capture concrete datasource UIDs when Grafana exposed them.

## 2026-03-12 - Task: Add Python Access Live Smoke Test
- State: Done
- Scope: `scripts/test-python-access-live-grafana.sh`, `Makefile`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Python access CLI had live Docker validation recorded in docs, but there was no checked-in script to reproduce those user, team, and service-account workflows end to end.
- Current Update: Added a Docker-backed smoke script for the Python access CLI and a `make test-access-live` target. The script starts Grafana, bootstraps a token, then validates user add/modify/delete, team add/list/modify, and service-account add/token/list flows with the auth modes each command expects.
- Result: The repo now has a repeatable live validation path for the Python access CLI instead of relying only on ad hoc one-off Docker checks.

## 2026-03-15 - Task: Expand Python Access Service-Account Live Smoke
- State: Done
- Scope: `scripts/test-python-access-live-grafana.sh`, `docs/DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The checked-in Python access live smoke covered service-account add, token add, and list, but it did not validate the newer snapshot workflows or destructive cleanup flows against a real Grafana instance.
- Current Update: Extended the Docker-backed Python access smoke script to export service-account snapshots, validate delete and token-delete flows, replay the exported snapshot through dry-run and live import, rewrite the exported role to force a diff, confirm dry-run/live update import behavior, and finish with a no-drift diff check.
- Result: The checked-in live access smoke now exercises the service-account snapshot lifecycle end to end in addition to the earlier create/list flows.

## 2026-03-15 - Task: Expand Rust Datasource Live Smoke
- State: Done
- Scope: `scripts/test-rust-live-grafana.sh`, `docs/DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Rust live smoke already validated datasource export/import and routed multi-org replay, but it did not exercise datasource add/delete against a real Grafana instance.
- Current Update: Extended the Rust Docker smoke script to create a second datasource through the Rust `datasource add` path, validate dry-run JSON output for add/delete, verify the created datasource through the Grafana API, and then remove it through the Rust `datasource delete` path before continuing with the existing export/import coverage.
- Result: The checked-in Rust live smoke now covers datasource add/delete in addition to the earlier datasource export/import and multi-org routing paths.

## 2026-03-15 - Task: Add Python Datasource Live Smoke
- State: Done
- Scope: `scripts/test-python-datasource-live-grafana.sh`, `Makefile`, `docs/DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Datasource real-Grafana validation had become Rust-heavy. The repo had no checked-in Python datasource live smoke path for add/delete/export/import or the new multi-org routed datasource replay flow.
- Current Update: Added a Docker-backed Python datasource smoke script and a `make test-python-datasource-live` target. The script bootstraps Grafana, validates datasource add/delete dry-run and live CRUD, validates single-org export/import dry-run, validates `export --all-orgs`, and validates `import --use-export-org --only-org-id --create-missing-orgs` dry-run/live replay.
- Result: The repo now has a repeatable Python datasource live validation path that complements the existing Rust datasource smoke coverage.

## 2026-03-12 - Task: Add Access Utility Team Add
- State: Done
- Scope: `grafana_utils/access_cli.py`, `tests/test_python_access_cli.py`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`, `TODO.md`
- Baseline: The Python access CLI already covered `team list` and `team modify`, but `TODO.md` still listed `team add` as one of the remaining team-lifecycle gaps.
- Current Update: Added `grafana-access-utils team add` with parser/help wiring, Grafana team creation through the org-scoped team API, optional initial `--member` and `--admin` seeding, and aligned public and maintainer docs. The command creates the team first, then reuses the existing exact org-user resolution and safe membership/admin update flow.
- Result: At this point the Python access CLI now covered `team add` alongside the existing user, team-list, team-modify, and service-account workflows, leaving only `team delete` plus the `group` alias in the then-current team/group backlog.

## 2026-03-11 - Task: Add Access Utility User List
- State: Done
- Scope: `grafana_utils/access_cli.py`, `tests/test_python_access_cli.py`, `pyproject.toml`, `cmd/grafana-access-utils.py`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The repo currently has dashboard and alerting CLIs only. `TODO.md` defines a future `grafana-access-utils` command shape, but there is no packaged script, wrapper, or public documentation for access-management workflows yet.
- Current Update: Added `grafana_utils/access_cli.py` with an initial Python access-management surface that now covers `user list` plus `service-account list`, `service-account add`, and `service-account token add`. Packaging wiring, focused unit coverage, and public/maintainer docs now describe the access CLI as Python-only for this first cut. The auth split is explicit: org-scoped user listing may use token or Basic auth, global user listing requires Basic auth, and the service-account commands are org-scoped and may use token or Basic auth.
- Result: The repo now ships a first Python access-management CLI surface for user listing and service-account creation flows, with focused tests plus a full Python suite pass confirming the new command does not regress the existing dashboard and alerting tools.

## 2026-03-11 - Task: Add Access Utility Team List
- State: Done
- Scope: `grafana_utils/access_cli.py`, `tests/test_python_access_cli.py`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Python access CLI already supports `user list` plus initial service-account commands, but `TODO.md` still lists all `team` operations as not started and the public docs say no `team` command exists yet.
- Current Update: Added a read-only `grafana-access-utils team list` command with org-scoped team search, optional member lookup, standard `--table|--csv|--json` output modes, and incomplete-command help for `grafana-access-utils team`. Public and maintainer docs now include the command and its auth expectations.
- Result: The Python access CLI now covers `user list`, `team list`, and the initial service-account workflows, with targeted and full Python test suite passes confirming the new command surface.

## 2026-03-11 - Task: Add Access Utility User Add
- State: Done
- Scope: `grafana_utils/access_cli.py`, `tests/test_python_access_cli.py`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Python access CLI already supports `user list`, `team list`, and the initial service-account commands, but it still cannot create Grafana users even though `TODO.md` calls out `user add` as one of the next lifecycle steps.
- Current Update: Added `grafana-access-utils user add` as a Basic-auth server-admin workflow that creates Grafana users through the admin API, supports optional org-role and Grafana-admin follow-up updates, and avoids the `--basic-password` versus new-user `--password` flag collision by separating the internal parser destinations and help text.
- Result: The Python access CLI now covers `user list`, `user add`, `team list`, and the initial service-account workflows, with targeted tests, the full Python suite, and a Docker-backed Grafana `12.4.1` smoke test confirming the new command path.

## 2026-03-11 - Task: Add Access Utility Team Modify
- State: Done
- Scope: `grafana_utils/access_cli.py`, `tests/test_python_access_cli.py`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Python access CLI can now list teams, but it still cannot add or remove team members or admins even though `TODO.md` puts `team modify` next in the planned access-management sequence.
- Current Update: Added `grafana-access-utils team modify` with `--team-id` or exact `--name` targeting, add/remove member actions, add/remove admin actions, and text or `--json` output. The command resolves users by exact login or email, uses org-scoped team APIs, and preserves admin changes safely by reading current member permission metadata before issuing the bulk admin update payload.
- Result: The Python access CLI now covers `user list`, `user add`, `team list`, `team modify`, and the initial service-account workflows, with targeted tests, the full Python suite, and Docker-backed Grafana `12.4.1` smoke tests confirming member and admin modification flows with both Basic auth and token auth.

## 2026-03-12 - Task: Add Access Utility User Modify
- State: Done
- Scope: `grafana_utils/access_cli.py`, `tests/test_python_access_cli.py`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Python access CLI can now create users and modify teams, but it still cannot update an existing user's identity fields, password, org role, or Grafana-admin state even though `TODO.md` lists `user modify` as the next user-lifecycle step.
- Current Update: Added `grafana-access-utils user modify` with id, login, or email targeting; explicit setters for login, email, name, password, org role, and Grafana-admin state; and text or `--json` output. The command is Basic-auth-only, updates profile fields and password through the global/admin user APIs, and reuses the existing org-role and permission update paths for role changes.
- Result: The Python access CLI now covers `user list`, `user add`, `user modify`, `team list`, `team modify`, and the initial service-account workflows, with targeted tests, the full Python suite, and a Docker-backed Grafana `12.4.1` smoke test confirming the update path.

## 2026-03-12 - Task: Add Access Utility User Delete
- State: Done
- Scope: `grafana_utils/access_cli.py`, `tests/test_python_access_cli.py`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Python access CLI can now create and modify users, but it still cannot remove users even though `TODO.md` keeps `user delete` as the next unfinished user-lifecycle step.
- Current Update: Added `grafana-access-utils user delete` with id, login, or email targeting; `--scope org|global`; required `--yes` confirmation; and text or `--json` output. Global deletion uses the admin delete API and requires Basic auth, while org-scoped removal uses the org user API and works with token or Basic auth.
- Result: At this point the Python access CLI now covered `user list`, `user add`, `user modify`, `user delete`, `team list`, `team modify`, and the initial service-account workflows, with targeted tests, the full Python suite, and Docker-backed Grafana `12.4.1` smoke tests confirming both global delete and org-scoped removal flows.

## 2026-03-11 - Task: Remove Python Dependency From Rust Live Smoke Test
- State: Done
- Scope: `scripts/test-rust-live-grafana.sh`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Rust Docker smoke script required `python3` only to extract simple JSON fields while creating a Grafana API token.
- Current Update: Replaced the JSON field helper with `jq`, removed the explicit `python3` prerequisite from the script, replaced the last Perl-based in-place JSON rewrite with a `jq` temp-file rewrite, and now check for `jq` at startup.
- Result: The Rust live smoke test no longer depends on Python or Perl and now keeps its runtime requirements to Docker, curl, and `jq`.

## 2026-03-11 - Task: Clarify Rust CLI Help Text
- State: Done
- Scope: `rust/src/dashboard.rs`, `rust/src/alert.rs`, `rust/src/dashboard_rust_tests.rs`, `rust/src/alert_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Rust `-h` and `--help` output listed many flags without operator-facing explanations, so switches like `--flat` were hard to understand from the CLI alone.
- Current Update: Added explicit clap help text for common auth/TLS flags plus dashboard and alerting mode flags, and added help-output tests that assert the Rust help explains flat export layout and includes examples.
- Result: `grafana-utils export-dashboard -h` and `grafana-alert-utils -h` now explain what options do instead of only showing their names, reducing the need to cross-reference README or Python help for common workflows.

## 2026-03-11 - Task: Add Preferred Auth Flag Aliases
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `grafana_utils/alert_cli.py`, `tests/test_python_dashboard_cli.py`, `tests/test_python_alert_cli.py`, `README.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Python dashboard and alerting CLIs only advertise `--api-token`, `--username`, and `--password`, even though the auth TODO now prefers `--token`, `--basic-user`, and `--basic-password`. Mixed token and Basic-auth input also resolves implicitly instead of failing early.
- Current Update: Added preferred CLI aliases for token and Basic auth in both Python CLIs while keeping the legacy flag names accepted, updated help text to advertise the preferred flags, and tightened `resolve_auth` so mixed token plus Basic input and partial Basic-auth input fail with clear operator-facing errors.
- Result: Operators can now use `--token`, `--basic-user`, and `--basic-password` consistently across both Python CLIs, while older flag names still parse. `python3 -m unittest -v tests/test_python_dashboard_cli.py`, `python3 -m unittest -v tests/test_python_alert_cli.py`, and `python3 -m unittest -v` all pass after the auth validation change.

## 2026-03-11 - Task: Add Dashboard List Subcommand
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `README.zh-TW.md`, `DEVELOPER.md`, `AGENTS.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The dashboard CLIs currently expose `export`, `import`, and `diff`, but there is no standalone operator command for listing dashboards without writing export files. The underlying `/api/search` lookup already exists only as an internal export helper.
- Current Update: Added a new explicit `list` subcommand in both Python and Rust dashboard CLIs, reusing the existing `/api/search` pagination path and enriching summaries with folder tree path from `GET /api/folders/{uid}` when `folderUid` is present. The command now supports compact text output, `--table`, `--csv`, and `--json`, with tests covering parser support, machine-readable renderers, table formatting, and folder hierarchy resolution.
- Result: Operators can now run `grafana-utils list` to inspect live dashboard summaries without exporting files first, and choose human-readable or machine-readable output with `--table`, `--csv`, or `--json`. The output fields are `uid`, `name`, `folder`, `folderUid`, and resolved folder tree path. Both `python3 -m unittest -v tests/test_python_dashboard_cli.py` and `cd rust && cargo test dashboard` pass, and the full Python and Rust test suites still pass after the new list formatting work.

## 2026-03-11 - Task: Add Docker-Backed Rust Grafana Smoke Test
- State: Done
- Scope: `scripts/test-rust-live-grafana.sh`, `Makefile`, `README.md`, `README.zh-TW.md`, `DEVELOPER.md`, `AGENTS.md`, `rust/src/alert.rs`, `rust/src/alert_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Rust CLIs already have unit coverage, but the repo has no repeatable live Grafana validation path for the Rust export/import/diff/dry-run workflows. Manual Docker validation knowledge is scattered, and the Rust alerting client still rejects Grafana template-list responses when the API returns JSON `null`.
- Current Update: Added `scripts/test-rust-live-grafana.sh` plus `make test-rust-live` to start a temporary Grafana Docker container, seed a datasource/dashboard/contact point, and exercise Rust dashboard export/import/diff/dry-run plus Rust alerting export/import/diff/dry-run. The script now defaults to pinned image `grafana/grafana:12.4.1`, auto-selects a free localhost port when `GRAFANA_PORT` is unset, and cleans up the container automatically. Also fixed the Rust alerting template-list path so `GET /api/v1/provisioning/templates` returning JSON `null` is treated as an empty list, matching the Python behavior.
- Result: `make test-rust-live` now passes locally against a temporary Docker Grafana instance, and `cd rust && cargo test` still passes after the Rust alerting null-handling fix. Maintainer and public docs now point at the live smoke-test entrypoint and its overrides.

## 2026-03-11 - Task: Add Versioned Export Schema, Dry-Run, and Diff Workflows
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `grafana_utils/alert_cli.py`, `tests/test_python_dashboard_cli.py`, `tests/test_python_alert_cli.py`, `README.md`, `README.zh-TW.md`, `DEVELOPER.md`, `AGENTS.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Python CLIs can export and import Grafana dashboards and alerting resources, but there is no versioned export schema marker for dashboards, no dry-run path to preview import behavior safely, and no built-in diff workflow to compare local exports against live Grafana state.
- Current Update: Added versioned export metadata for dashboard exports and extended alerting tool documents/root indexes with `schemaVersion`, while keeping older alerting `apiVersion`-only tool docs importable. Added non-mutating import `--dry-run` behavior for both CLIs, added dashboard `diff` as an explicit subcommand, and added alerting `--diff-dir` to compare exported files with live Grafana resources. Both diff paths now print unified diffs for changed documents.
- Result: Operators can validate export shape compatibility, preview create/update behavior safely, and compare local exports against Grafana before applying changes. The focused Python dashboard and alerting suites plus the full Python suite pass with the new workflows.

## 2026-03-11 - Task: Distinguish Python and Rust Test File Names
- State: Done
- Scope: `tests/test_python_dashboard_cli.py`, `tests/test_python_alert_cli.py`, `tests/test_python_packaging.py`, `rust/src/common.rs`, `rust/src/http.rs`, `rust/src/alert.rs`, `rust/src/dashboard.rs`, `rust/src/common_rust_tests.rs`, `rust/src/http_rust_tests.rs`, `rust/src/alert_rust_tests.rs`, `rust/src/dashboard_rust_tests.rs`, `DEVELOPER.md`, `AGENTS.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Python tests are named generically under `tests/test_*.py`, while Rust unit tests are inline inside implementation files. That makes it hard to distinguish Python and Rust test files by filename alone.
- Current Update: Renamed the Python test files to `test_python_*`, moved the Rust unit tests into dedicated `*_rust_tests.rs` files loaded from their parent modules, and updated maintainer docs to use the new test names and layout.
- Result: Python and Rust test files are now distinguishable by filename, and both `python3 -m unittest -v` and `cd rust && cargo test` still pass with the new layout.

## 2026-03-11 - Task: Add Unified Build Makefile
- State: Done
- Scope: `Makefile`, `.gitignore`, `README.md`, `README.zh-TW.md`, `DEVELOPER.md`, `AGENTS.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The repo supports Python packaging and a separate Rust crate, but build commands are split across `pip` and `cargo` examples in the docs. There is no single root command surface for building the Python wheel and Rust release binaries together.
- Current Update: Added a root `Makefile` with Python, Rust, aggregate build, and aggregate test targets. Updated the English and Traditional Chinese README files plus maintainer docs to document those commands, and extended `.gitignore` for Python build outputs created by `make build-python`.
- Result: `make help`, `make build-python`, and `make build-rust` all pass locally. The Python target writes the wheel to `dist/`, and the Rust target produces release binaries under `rust/target/release/`.

## 2026-03-11 - Task: Rename Dashboard Export Variant Flags
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `rust/src/dashboard.rs`, `tests/test_dump_grafana_dashboards.py`, `README.md`, `README.zh-TW.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Both the packaged Python dashboard CLI and the Rust dashboard CLI expose short export-suppression flags, `--without-raw` and `--without-prompt`, with matching internal field names. The current docs and tests also use those shorter names.
- Current Update: Renamed the public export flags to `--without-dashboard-raw` and `--without-dashboard-prompt` in both implementations, renamed the corresponding Python namespace attributes and Rust struct fields, updated the rejection error text for disabling both variants, and refreshed the dashboard tests plus English and Traditional Chinese README examples.
- Result: The Python and Rust dashboard CLIs now use the longer dashboard-specific variant flag names consistently, and the focused dashboard unittest suite plus the full Rust and Python test suites pass with the new flag names.

## 2026-03-11 - Task: Port Grafana HTTP and API Flows Into Rust
- State: Done
- Scope: `rust/Cargo.toml`, `rust/Cargo.lock`, `rust/src/lib.rs`, `rust/src/common.rs`, `rust/src/http.rs`, `rust/src/dashboard.rs`, `rust/src/alert.rs`, `rust/src/bin/grafana-utils.rs`, `rust/src/bin/grafana-alert-utils.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Rust crate can parse CLI arguments and normalize dashboard and alerting documents, but the actual Grafana HTTP client and live export/import flows are still stubbed with explicit not-implemented errors.
- Current Update: Added a shared Rust JSON HTTP client on top of `reqwest`, wired real dashboard raw export/import flows into `rust/src/dashboard.rs`, and wired real alerting export/import flows into `rust/src/alert.rs` for rules, contact points, mute timings, policies, and templates. The Rust alerting path now also includes linked-dashboard metadata export plus import-time dashboard UID repair logic. The remaining dashboard gap, prompt-export datasource rewrite, is now ported as well, including datasource-template-variable input generation and dependent-variable placeholder rewrites.
- Result: The Rust crate now executes the real Grafana HTTP/API flows and can produce both raw and prompt-style dashboard exports instead of relying on Python for datasource rewrite parity. `/opt/homebrew/bin/cargo test` passes, the targeted dashboard Rust tests pass, and the existing Python `python3 -m unittest -v` suite still passes.

## 2026-03-11 - Task: Add Rust Rewrite Scaffold for Grafana Utilities
- State: Done
- Scope: `rust/Cargo.toml`, `rust/Cargo.lock`, `rust/src/lib.rs`, `rust/src/common.rs`, `rust/src/dashboard.rs`, `rust/src/alert.rs`, `rust/src/bin/grafana-utils.rs`, `rust/src/bin/grafana-alert-utils.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The repo ships only Python implementations. There is no Rust crate, no Rust CLI entrypoints, and no shared Rust model for dashboard or alerting document normalization.
- Current Update: Added an isolated `rust/` crate with shared auth and path helpers, a first-pass dashboard module, a first-pass alerting module, and Rust binary entrypoints for `grafana-utils` and `grafana-alert-utils`. The Rust port currently covers CLI parsing, auth/header resolution, path-building helpers, file discovery, and dashboard/alerting document normalization helpers, while the live HTTP flows still return explicit not-implemented errors.
- Result: The repository now contains a concrete Rust rewrite scaffold that can be extended incrementally without disturbing the shipping Python package. Existing Python tests still pass, and the new Rust crate now passes `cargo test` after the Rust toolchain was installed locally.

## 2026-03-11 - Task: Package Grafana Utilities for Installation
- State: Done
- Scope: `pyproject.toml`, `grafana_utils/__init__.py`, `grafana_utils/dashboard_cli.py`, `grafana_utils/alert_cli.py`, `grafana_utils/http_transport.py`, `cmd/grafana-utils.py`, `cmd/grafana-alert-utils.py`, `tests/test_dump_grafana_dashboards.py`, `tests/test_grafana_alert_utils.py`, `tests/test_packaging.py`, `README.md`, `README.zh-TW.md`, `DEVELOPER.md`, `AGENTS.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The repo runs from source, but it is not structured as an installable Python package. The implementation lives under `cmd/`, there is no packaging metadata, and there are no console entry points for global or per-user installs on other systems.
- Current Update: Moved the implementation into the `grafana_utils/` package, kept `cmd/` as thin source-tree wrappers, added `pyproject.toml` with console scripts for `grafana-utils` and `grafana-alert-utils`, and updated the English and Traditional Chinese docs plus maintainer guidance to cover normal, `--user`, and optional HTTP/2 installs. Packaging validation now includes package metadata tests and an isolated local `pip install --target` run.
- Result: The repo now supports installation as a Python package for either system/global environments or user-local environments while preserving direct repo execution through `cmd/`. Targeted tests and the full unittest suite passed. Local package installation also succeeded into `/tmp` with `--no-build-isolation`; a post-install `pyenv` rehash hook reported a local permissions warning after the install completed.

## 2026-03-11 - Task: Enable Persistent Grafana HTTP Connections
- State: Done
- Scope: `cmd/grafana_http_transport.py`, `tests/test_dump_grafana_dashboards.py`, `tests/test_grafana_alert_utils.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The shared transport abstraction exists, but both transport adapters still issue one-shot requests. That means no deliberate connection reuse, and HTTP/2 is not attempted even when the runtime could support it.
- Current Update: Changed the `requests` transport to use a persistent `requests.Session`, changed the `httpx` transport to use a persistent `httpx.Client`, and added automatic HTTP/2 enablement for `httpx` only when the runtime has `h2` support available. The default transport selector now uses `auto`, which prefers HTTP/2-capable `httpx` when possible and otherwise falls back to `requests` keep-alive sessions.
- Result: Grafana HTTP requests now reuse connections by default, and HTTP/2 is enabled automatically only in environments that can actually negotiate it. Full unit tests still pass after the transport behavior change.

## 2026-03-11 - Task: Make Grafana HTTP Transport Replaceable
- State: Done
- Scope: `cmd/grafana_http_transport.py`, `cmd/grafana-utils.py`, `cmd/grafana-alert-utils.py`, `tests/test_dump_grafana_dashboards.py`, `tests/test_grafana_alert_utils.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Both CLI tools embed `urllib` request handling directly inside their Grafana client classes. That makes the HTTP implementation fixed, mixes transport concerns into the resource clients, and leaves no clean seam for swapping `requests`, `httpx`, or a test transport.
- Current Update: Added a shared replaceable JSON transport module with `RequestsJsonHttpTransport` and `HttpxJsonHttpTransport`, changed both CLI clients to depend on an injected transport object, and kept `requests` as the default transport selected by the client constructors. Updated tests to load the shared transport module, verify both transport adapters build successfully, and exercise the new injected-transport seam directly.
- Result: The Grafana dashboard and alerting clients now use a replaceable transport architecture instead of hard-wired `urllib` calls. Full unit tests pass, and both CLIs can now switch HTTP engines by swapping the transport implementation rather than rewriting client logic.

## 2026-03-11 - Task: Refactor Grafana CLI Readability
- State: Done
- Scope: `cmd/grafana-utils.py`, `cmd/grafana-alert-utils.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Both CLI modules are functionally covered by tests, but several import/export and API-normalization flows are long enough that humans need to read multiple branches at once to understand them. The current structure leans on comments and helper names, but key paths such as datasource rewriting and alert import dispatch still need cleaner decomposition.
- Current Update: Refactored the dashboard CLI by splitting datasource resolution, templating rewrite, and export index generation into smaller helpers. Refactored the alerting CLI by splitting linked-dashboard repair, export document generation, and per-resource import handling into clearer dispatcher-style helpers with smaller units of work.
- Result: Both CLIs now read more like orchestration code with named helper steps instead of large inline branches. Full unit tests still pass, so the refactor changed structure and readability without changing behavior.

## 2026-03-11 - Task: Move Grafana CLIs Into cmd
- State: Done
- Scope: `cmd/grafana-utils.py`, `cmd/grafana-alert-utils.py`, `tests/test_dump_grafana_dashboards.py`, `tests/test_grafana_alert_utils.py`, `tests/__init__.py`, `README.md`, `README.zh-TW.md`, `DEVELOPER.md`, `AGENTS.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Both CLI entrypoints currently live at the repository root as `grafana-utils.py` and `grafana-alert-utils.py`. Unit tests import those scripts by direct filesystem path, and the public and maintainer docs still show root-level invocation examples.
- Current Update: Moved both CLI entrypoints into `cmd/`, updated the path-sensitive test loaders and CLI help strings, refreshed the English and Traditional Chinese docs plus maintainer guidance to use `python3 cmd/...`, and added `tests/__init__.py` so the documented `python3 -m unittest -v` command discovers the suite.
- Result: The repository now keeps both CLIs under `cmd/` without changing their behavior, unit tests load the new file locations correctly, and both targeted test runs plus the full unittest command pass from the repo root.

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
## 2026-03-15 - Task: Promote Python Access User/Team Diff to Supported Status
- State: Done
- Scope: `grafana_utils/access_cli.py`, `tests/test_python_access_cli.py`, `README.md`, `README.zh-TW.md`, `docs/user-guide.md`, `docs/user-guide-TW.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Python access workflows already had `diff_users_with_client` and `diff_teams_with_client`, but the top-level Python facade did not re-export those helpers and the public docs still described Python access snapshots and drift comparison as Rust-only.
- Current Update: Re-exported Python access export/import/diff helpers from `grafana_utils.access_cli`, added dispatch coverage for `access user diff` and `access team diff`, and updated the English/Traditional Chinese README plus both user guides so access user/team export, import, and diff are documented as supported Python workflows.
- Result: Python and Rust now present the same supported access command surface for user/team snapshot export, import, and diff in the operator docs, and the Python facade/tests explicitly cover the diff entrypoints.
## 2026-03-16 - Task: Refine Rust Dashboard Screenshot Chrome Hiding
- State: Done
- Scope: `rust/src/dashboard_screenshot.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Rust `dashboard screenshot` command could hide the Grafana sidebar, but remote full-page captures still kept the top toolbar/header in the output.
- Current Update: Extended the browser DOM preparation step to hide fixed/sticky top chrome in addition to the sidebar before stitched full-page capture. Live validation against the remote Grafana dashboard `eei8l48f3s3k0f` now produces a dark-mode full-page PNG without the top toolbar.
- Result: Browser-rendered full-page screenshots are cleaner and closer to report-style output, while dashboard variables can still be selected through repeatable `--var name=value` assignments.

## 2026-03-16 - Task: Accept Full Grafana Dashboard URLs for Screenshot and Variable Inspection
- State: Done
- Scope: `rust/src/cli.rs`, `rust/src/dashboard.rs`, `rust/src/dashboard_cli_defs.rs`, `rust/src/dashboard_screenshot.rs`, `rust/src/dashboard_vars.rs`, `rust/src/dashboard_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Rust screenshot capture required separate `--url` plus `--dashboard-uid`, and the new variable inspection helper did not exist.
- Current Update: Added `dashboard inspect-vars`, allowed both `dashboard screenshot` and `dashboard inspect-vars` to accept a full Grafana dashboard URL, and taught screenshot URL-building to reuse URL state such as `var-*`, `from`, `to`, `orgId`, and `panelId` while still letting explicit CLI flags override those values.
- Result: Operators can now paste a browser dashboard URL directly into the Rust CLI and preserve its current variable/query-string state without manually re-entering every `--var`.

## 2026-03-16 - Task: Strengthen Rust Screenshot Query-State Debugging and Readiness
- State: Done
- Scope: `rust/Cargo.toml`, `rust/src/http.rs`, `rust/src/dashboard_cli_defs.rs`, `rust/src/dashboard_screenshot.rs`, `rust/src/dashboard_vars.rs`, `rust/src/dashboard_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `--vars-query` only overlaid `var-*` values, screenshot runs could not print the final resolved URL, API inspection on one remote Grafana hit response-body decode errors, and browser capture could stop at Grafana's loading spinner or SPA navigation wait.
- Current Update: Extended screenshot query-fragment parsing to preserve non-variable query keys such as `refresh`, `showCategory`, and `timezone`; added `--print-capture-url`; forced the Rust HTTP client onto a more compatible JSON-bytes/HTTP1.1 path; and tightened screenshot readiness to wait for panel content while avoiding the stuck navigation event path. Live validation against dashboard `rYdddlPWk` confirmed the final capture URL and preserved query state.
- Result: Screenshot/debug workflows now expose the exact resolved URL, keep more Grafana browser state intact, and are more reliable against remote Grafana instances that previously failed in API decode or browser-ready handling.
