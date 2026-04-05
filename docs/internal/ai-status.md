# ai-status.md

Current AI-maintained status only.

- Older trace history moved to [`archive/ai-status-archive-2026-03-24.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-03-24.md).
- Detailed 2026-03-27 entries moved to [`archive/ai-status-archive-2026-03-27.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-03-27.md).
- Detailed 2026-03-28 task notes were condensed into [`archive/ai-status-archive-2026-03-28.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-03-28.md).
- Keep this file short and current. Additive historical detail belongs in `docs/internal/archive/`.
- Detailed 2026-03-29 through 2026-03-31 entries moved to [`archive/ai-status-archive-2026-03-31.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-03-31.md).

## 2026-04-06 - Converge project-status live reads onto shared workflow helpers
- State: Done
- Scope: `rust/src/grafana_api/project_status_live.rs`, `rust/src/grafana_api/datasource_live_project_status.rs`, `rust/src/access/live_project_status.rs`, `rust/src/grafana_api/tests.rs`, `rust/src/access/team_browse.rs`, `rust/src/access/user_browse.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: project-status already reused shared Grafana connection/client wiring, but datasource and access live-status producers still owned a couple of raw `/api/org` and `/api/orgs` reads locally, and the shared project-status helper module only exposed part of the workflow-level read surface.
- Current Update: expanded `grafana_api::project_status_live` with shared current-org, visible-org-list, and alert-surface helpers for both client and request seams; switched datasource and access live-status producers onto those shared helpers; and tightened the access browse imports so `--no-default-features` stays warning-free.
- Result: project-status-related live reads now reuse one workflow-level helper module for shared org and alert document reads, while the status scoring/aggregation logic stays in the runtime layer.

## 2026-04-06 - Split sync live workflow helpers into read/apply siblings
- State: Done
- Scope: `rust/src/grafana_api/sync_live.rs`, `rust/src/grafana_api/sync_live_read.rs`, `rust/src/grafana_api/sync_live_apply.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: sync live still had a single large module that mixed shared client facade wiring, live read collection, request-based test seams, plugin availability, and live apply logic in one file.
- Current Update: split the sync live workflow into a thin facade plus dedicated read/apply sibling modules, keeping `SyncLiveClient` as the shared internal wrapper while moving plugin availability, live spec collection, and apply request seams into the sibling files.
- Result: sync live is now easier to navigate and maintain without turning the workflow layer into a finer-grained SDK.

## 2026-04-06 - Split datasource live project-status analysis from input collection
- State: Done
- Scope: `rust/src/datasource_live_project_status.rs`, `rust/src/datasource_live_project_status_analysis.rs`, `rust/src/grafana_api/datasource_live_project_status.rs`
- Baseline: datasource live project-status still mixed request collection, status/risk calculation, and render-neutral status assembly in one large module.
- Current Update: moved the live request collection into the shared `grafana_api` datasource helper, extracted datasource status/risk calculation plus `ProjectDomainStatus` assembly into `datasource_live_project_status_analysis.rs`, and left `datasource_live_project_status.rs` as the thin orchestration and test wrapper.
- Result: datasource live project-status now has a clearer workflow-level split without changing the live behavior or adding a deeper abstraction layer.

## 2026-04-06 - Converge sync live workflow helpers onto shared Grafana resource clients
- State: Done
- Scope: `rust/src/grafana_api/dashboard.rs`, `rust/src/grafana_api/datasource.rs`, `rust/src/grafana_api/alerting.rs`, `rust/src/grafana_api/sync_live.rs`, `rust/src/grafana_api/tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: sync live fetch/apply had already been moved onto the shared `SyncLiveClient` workflow wrapper, but several concrete Grafana path contracts still lived directly in `sync_live.rs` instead of routing through the shared dashboard/datasource/alerting resource helpers.
- Current Update: added shared folder/list/update helpers to `grafana_api::dashboard`, datasource CRUD helpers to `grafana_api::datasource`, and alert delete/policy helpers to `grafana_api::alerting`, then rewired `SyncLiveClient` to use those shared methods for folder, dashboard, datasource, and alert workflow actions.
- Result: sync live now keeps the reviewed intent and guard logic in the sync workflow layer while the concrete Grafana endpoint ownership sits in the shared internal resource clients.

## 2026-04-06 - Converge datasource and alert live workflow helpers and extend sync live smoke
- State: Done
- Scope: `rust/src/datasource_live_project_status.rs`, `rust/src/alert.rs`, `rust/src/alert_runtime_support.rs`, `rust/src/grafana_api/mod.rs`, `rust/src/grafana_api/datasource_live_project_status.rs`, `rust/src/grafana_api/alert_live.rs`, `scripts/test-rust-live-grafana.sh`, `docs/internal/maintainer-quickstart.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `project-status` and `sync` had already moved onto workflow-level shared live helpers, but datasource live project-status still owned raw `/api/datasources`, `/api/orgs`, and `/api/org` reads locally, alert runtime support still kept one large in-module provisioning request layer, and the maintained Docker smoke path did not yet assert the `change preview --fetch-live` contract from the repo-owned script.
- Current Update: moved datasource live inventory gathering behind a dedicated `grafana_api::datasource_live_project_status` helper, extracted the alert provisioning request/apply helpers into `grafana_api::alert_live`, kept `alert_runtime_support.rs` focused on plan/build/apply orchestration, and extended `scripts/test-rust-live-grafana.sh` so the sync smoke now writes and validates a real `change preview --fetch-live` artifact. Also added a maintainer quickstart rule that new live workflow code should centralize raw Grafana path ownership in one workflow-level helper under `rust/src/grafana_api/` instead of reintroducing a second production request path inside command runtimes.
- Result: datasource and alert live workflow code now follow the same shared-live ownership pattern as `project-status` and `sync`, and the repo-owned Docker smoke now covers the live sync preview contract instead of relying on one-off localhost checks.

## 2026-04-05 - Reuse resolved Grafana clients within one command to avoid repeated auth prompts
- State: Done
- Scope: `rust/src/dashboard/cli_defs.rs`, `rust/src/dashboard/dashboard_runtime.rs`, `rust/src/dashboard/list.rs`, `rust/src/dashboard/export.rs`, `rust/src/dashboard/mod.rs`, `rust/src/datasource.rs`, `rust/src/datasource_import_export.rs`, `rust/src/datasource_import_export_routed.rs`, `rust/src/grafana_api/tests.rs`
- Baseline: several dashboard and datasource command paths built a root client and then rebuilt scoped org clients from `CommonCliArgs`, which re-ran auth resolution and could prompt for `--prompt-password` multiple times within one command.
- Current Update: added shared dashboard runtime helpers that derive org-scoped clients from an already-resolved root API client, then rewired dashboard list/export and datasource list/export/routed-import paths to reuse that root client instead of resolving auth again for each org scope. Added a shared-client regression in `grafana_api/tests.rs`, reran focused slices, and live-validated `dashboard list --prompt-password` against localhost Grafana.
- Result: the fixed command paths now resolve prompt-based auth once per command and reuse the same root connection/client when they need additional org-scoped HTTP clients.

## 2026-04-05 - Add generic resource queries plus dashboard serve/edit-live authoring surfaces
- State: Done
- Scope: `rust/src/resource.rs`, `rust/src/cli.rs`, `rust/src/cli_rust_tests.rs`, `rust/src/dashboard/serve.rs`, `rust/src/dashboard/edit_live.rs`, `rust/src/dashboard/cli_defs_command.rs`, `rust/src/dashboard/dashboard_cli_parser_help_rust_tests.rs`, `rust/src/sync/plan_builder.rs`, `rust/src/sync/staged_documents_render.rs`, `rust/src/sync/rust_tests.rs`, `docs/commands/en/*.md`, `docs/commands/zh-TW/*.md`, `docs/user-guide/en/dashboard.md`, `docs/user-guide/zh-TW/dashboard.md`, `docs/user-guide/en/reference.md`, `docs/user-guide/zh-TW/reference.md`, generated man/html, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the repo already had deep workflow-specific surfaces for dashboards, alerts, datasources, access, and change review, but it still lacked a small generic live resource query surface, a local dashboard preview server, a safe live-dashboard editor flow, and explicit dependency-order metadata in staged sync plans.
- Current Update: expanded the read-only `resource` namespace with `resource describe`, clearer selector/help messaging, and command-index routing; upgraded `dashboard serve` with browser-open convenience plus persistent reload-error state in the preview document/page; upgraded `dashboard edit-live` so edited drafts always print a review summary and `--apply-live` is gated by that review; and documented the sync-plan ordering contract (`ordering.mode`, `orderIndex`, `orderGroup`, `kindOrder`, `blocked_reasons`) in both command docs and the technical reference.
- Result: the Rust CLI now has a clearer generic read-only resource query surface, a safer single-dashboard authoring loop with review-first live editing, and explicit staged sync ordering evidence that both reviewers and CI can rely on from the preview artifact itself.

## 2026-04-05 - Move dashboard list flow onto shared Grafana resource clients
- State: Done
- Scope: `rust/src/dashboard/list.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: dashboard list already had with-request seams, but the public client-backed path still handed raw `request_json` closures through to the helper flow, so folder path and source enrichment were not using the new shared concrete dashboard/datasource resource methods yet.
- Current Update: added a small local dashboard-list resource wrapper in `dashboard/list.rs` and switched the public client-backed list path to use shared concrete dashboard and datasource methods for summaries, folder lookup, dashboard fetches, and datasource listing. The request-injection helpers remain intact for tests and other consumers.
- Result: the dashboard list flow now uses the shared concrete Grafana resource layer where possible without changing the public CLI behavior or the with-request seam.

## 2026-04-05 - Move dashboard inspect-live fast path onto shared Grafana resource clients
- State: Done
- Scope: `rust/src/dashboard/inspect_live.rs`, `rust/src/grafana_api/dashboard.rs`, `rust/src/grafana_api/tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the single-org inspect-live fast path had already started consuming shared dashboard and datasource methods for summary/dashboard/datasource reads, but it still kept one direct current-org read outside the shared dashboard client.
- Current Update: added shared current-org support to `DashboardResourceClient` and switched the inspect-live single-org fast path to use that shared method alongside the existing shared summary, dashboard fetch, folder lookup, and datasource-list calls.
- Result: the inspect-live fast path no longer owns a direct `/api/org` read in the client-backed lane, so its main live reads now route through the shared dashboard resource layer.

## 2026-04-05 - Move dashboard live wrappers onto shared Grafana resource clients
- State: Done
- Scope: `rust/src/grafana_api/dashboard.rs`, `rust/src/grafana_api/datasource.rs`, `rust/src/grafana_api/tests.rs`, `rust/src/dashboard/live.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the shared-client refactor had already centralized connection wiring, but the public helpers in `dashboard/live.rs` still mostly treated `DashboardResourceClient` and `DatasourceResourceClient` as thin `request_json` pass-through adapters instead of calling concrete shared endpoint methods. The new `grafana_api` dashboard and datasource resource modules also still lacked most of the dashboard live endpoint surface.
- Current Update: added concrete dashboard/datasource resource methods for dashboard search, paged dashboard summaries, folder lookup, dashboard fetch, dashboard/folder permission reads, dashboard import, dashboard delete, folder delete, and datasource listing. The public wrappers in `dashboard/live.rs` now call those methods directly, while the existing `with_request` helpers stay in place for orchestration and test seams.
- Result: dashboard live reads and mutations now rely on one shared home for those Grafana endpoint contracts instead of re-declaring the API shape in the public wrapper layer. Focused `grafana_api` and alert regressions passed after the migration.

## 2026-04-05 - Verify repo-local smoke regression for the task-first change lane
- State: Done
- Scope: `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the repo already had a temp-workspace smoke that walks `inspect -> check -> preview -> apply` through one local staged workspace and checks the preview/apply handoff.
- Current Update: re-validated that smoke against the current tree so the trace log still matches the existing task-first lane coverage and its local preview artifact flow.
- Result: the task-first lane still has a repo-local smoke regression that catches workspace-discovery or preview/apply handoff breakage without needing a live Grafana instance.

## 2026-04-05 - Centralize Rust Grafana connection wiring behind a shared internal client layer
- State: Done
- Scope: `rust/src/grafana_api/**`, `rust/src/lib.rs`, `rust/src/dashboard/dashboard_runtime.rs`, `rust/src/alert.rs`, `rust/src/alert_client.rs`, `rust/src/alert_cli_defs.rs`, `rust/src/access/access_cli_runtime.rs`, `rust/src/project_status_support.rs`, `rust/src/sync/live.rs`, focused Rust tests, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Rust already shares a low-level `JsonHttpClient`, but profile resolution, auth header selection, org scoping, CA-cert propagation, and per-domain client construction are still duplicated across dashboard, alert, access, sync, and project-status runtime helpers. Alert also owns a thin domain-local client wrapper instead of building from one shared root client layer.
- Current Update: added a new internal `grafana_api` module that owns connection resolution, root client construction, org scoping, and resource wrappers. Dashboard, access, alert, and project-status runtime builders now resolve their live clients through the same shared connection path, and the alert thin client now delegates its endpoint methods through the new shared alerting resource client instead of building its own raw transport wrapper.
- Result: the repo now has one internal Grafana connection/client layer for live runtime paths, with focused Rust regressions covering auth-mode resolution, org-header injection, and the migrated alert/project-status paths. CLI behavior stayed unchanged, and the staged dashboard/datasource/access resource wrappers are in place for future endpoint migration without forcing more command-flow churn in this change.

## 2026-04-05 - Accept common export-tree roots in task-first `change --workspace` discovery
- State: Done
- Scope: `rust/src/sync/guided.rs`, focused Rust tests in `rust/src/sync/guided.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: task-first `change` discovery already worked from a repo/workspace root, but pointing `--workspace` at common subtree roots such as `dashboards/`, `dashboards/raw/`, or `datasources/provisioning/` still failed or felt inconsistent even though operators naturally land there after export commands.
- Current Update: refactored staged-input discovery into a workspace-root pass plus a direct-input overlay so `change` can infer the real workspace root from common export/provisioning subtrees while still honoring the exact subtree the operator pointed at. Added tempfile regressions for `dashboards/`, `dashboards/raw/`, and `datasources/provisioning/`, then live-validated `change inspect` and `change preview` against a local Grafana export tree rooted under `dashboards/raw/`.
- Result: `change --workspace` now tolerates the common export-tree entrypoints operators actually have on disk instead of forcing them back to a higher repo root before inspect/check/preview can work.

## 2026-04-05 - Harden dashboard authoring around watch UX, General folder publish semantics, and live smoke coverage
- State: Done
- Scope: `rust/src/dashboard/files.rs`, `rust/src/dashboard/authoring.rs`, `rust/src/dashboard/dashboard_export_import_inventory_rust_tests.rs`, `rust/src/dashboard/dashboard_authoring_rust_tests.rs`, `scripts/test-rust-live-grafana.sh`, `README.md`, `README.zh-TW.md`, `docs/commands/en/dashboard-publish.md`, `docs/commands/zh-TW/dashboard-publish.md`, `docs/user-guide/en/dashboard.md`, `docs/user-guide/zh-TW/dashboard.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: stdin-aware dashboard authoring and `publish --watch` were already in place, but live validation against Grafana 12.4.0 exposed one real edge: sending a literal `folderUid: general` through the import payload could fail against the built-in General folder even though the default root publish path worked. The watch loop also only printed minimal status, and the Rust live smoke script did not yet cover the new stdin/watch authoring lane end to end.
- Current Update: normalized the built-in General folder back to the default root publish path during dashboard import/publish payload assembly, so authoring can still preserve `meta.folderUid = general` on disk without forcing a fragile live API path. The watch loop now prints a clearer start/stop hint plus explicit change-detected and restabilizing messages before reruns. Added focused Rust regressions for omitting `folderUid` when the effective target is General, extended `scripts/test-rust-live-grafana.sh` with stdin review/patch/publish and watch-recovery smoke coverage, and updated the README plus dashboard command/handbook docs to describe the new authoring loop and General-folder normalization.
- Result: dashboard authoring now behaves more predictably against real Grafana instances, the watch loop is easier to operate, and the repo-owned live smoke path covers the stdin and watch authoring flow instead of leaving it as manual validation only.

## 2026-04-05 - Wire dashboard authoring live smoke into CI and freeze polling-watch policy
- State: Done
- Scope: `.github/workflows/ci.yml`, `Makefile`, `docs/DEVELOPER.md`, `TODO.md`, `rust/src/dashboard/authoring.rs`, `rust/src/dashboard/dashboard_authoring_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the repo already had `make test-rust-live` and the expanded dashboard authoring smoke inside `scripts/test-rust-live-grafana.sh`, but the maintained CI path still stopped at `make quality-rust`. The newer watch-status wording also lived only in runtime output, not in a focused regression, and there was no written maintainer decision about whether the polling watcher should be replaced by an event-based implementation.
- Current Update: added a dedicated `rust-live-smoke` GitHub Actions job that installs the shell dependencies and runs `make test-rust-live`, then made the release jobs depend on that live gate as well as `rust-quality`. Clarified the `make test-rust-live` help text, documented in `docs/DEVELOPER.md` that this is the maintained end-to-end validation entrypoint for dashboard authoring, added focused Rust tests for the exact watch status messages, and recorded the current engineering decision in both `docs/DEVELOPER.md` and `TODO.md`: keep the repo-owned polling watcher unless live validation proves a concrete portability, latency, or missed-save problem.
- Result: dashboard authoring live smoke is now part of the fixed CI path, watch UX wording is under direct regression coverage, and the watcher implementation has an explicit maintained policy instead of an open-ended "maybe event-based later" ambiguity.

## 2026-04-05 - Consolidate persisted-output routing for reviewable artifacts
- State: Done
- Scope: `rust/src/common.rs`, `rust/src/common_rust_tests.rs`, `rust/src/dashboard/inspect_paths.rs`, `rust/src/dashboard/vars.rs`, `rust/src/dashboard/topology.rs`, `rust/src/dashboard/validate.rs`, `rust/src/sync/bundle_exec_rust_tests.rs`, `docs/internal/maintainer-quickstart.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: several dashboard and sync paths already tried to keep output-file artifacts plain and avoid duplicate stdout, but they still implemented that contract through repeated command-local trim/write/print branches. ANSI stripping also rebuilt its regex each call.
- Current Update: added a shared `emit_plain_output` helper and switched the representative dashboard paths that persist operator-facing text or JSON renderings onto the same output-routing contract. `strip_ansi_codes` now uses a precompiled regex, maintainer guidance now states the persisted-artifact rule explicitly, and focused regressions cover `common`, `inspect_paths`, `inspect-vars`, `topology`, `validate-export`, and `change bundle`.
- Result: persisted artifacts now follow one clearer repo-owned rule: on-disk output stays plain and deterministic, and stdout is only duplicated when `--also-stdout` is set.

## 2026-04-05 - Fix change preview workspace discovery and Grafana null template handling
- State: Done
- Scope: `rust/src/sync/guided.rs`, `rust/src/sync/live_fetch.rs`, `rust/src/sync/live_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: live validation against local Grafana 12.4.0 showed two real `change preview` failures. Workspace auto-discovery found both `dashboards/raw` and `dashboards/provisioning` under one export tree and treated that as a hard conflict, so the task-first `change preview --workspace ...` path failed on repo-shaped staged inputs. The live alert template fetch path also treated `/api/v1/provisioning/templates` returning `null` as an invalid response and aborted preview even when the Grafana instance simply had no notification templates configured.
- Current Update: added one repo-owned dashboard source selector for task-first preview so explicit dashboard flags still conflict as before, but auto-discovery now prefers `dashboards/raw` over `dashboards/provisioning` instead of blocking the workflow. The live template fetch path now treats `null` template lists as empty rather than as an unexpected response, and focused regressions cover both the preview-source selection rules and the `null` template-list case.
- Result: `change preview` now stays usable on workspace-shaped dashboard exports that include both `raw` and `provisioning`, and live preview no longer fails on Grafana instances that return `null` for the notification-template list endpoint.

## 2026-04-05 - Add stdin-friendly dashboard authoring input and publish watch mode
- State: Done
- Scope: `rust/src/dashboard/authoring.rs`, `rust/src/dashboard/cli_defs_command.rs`, `rust/src/dashboard/dashboard_authoring_rust_tests.rs`, `rust/src/dashboard/dashboard_cli_parser_help_rust_tests.rs`, `docs/commands/en/dashboard-patch-file.md`, `docs/commands/en/dashboard-review.md`, `docs/commands/en/dashboard-publish.md`, `docs/commands/zh-TW/dashboard-patch-file.md`, `docs/commands/zh-TW/dashboard-review.md`, `docs/commands/zh-TW/dashboard-publish.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: dashboard authoring already supported local file review, patch, and publish, but it still assumed file-only input and did not provide a repo-owned watch loop for repeated draft saves. Generator-based authoring had to stop at an intermediate file, and `publish` lacked a fast local feedback mode.
- Current Update: added `--input -` support for `dashboard review`, `dashboard patch-file`, and `dashboard publish` through a shared authoring input loader that accepts wrapped or bare dashboard JSON from standard input. `patch-file --input -` now requires `--output` explicitly, `publish` rejects `--watch` with stdin, and `dashboard publish --watch` now polls one local file and re-runs publish or dry-run after each stabilized save while continuing to watch through validation or API failures. Updated CLI help/examples plus English and Traditional Chinese command docs to show the stdin and watch workflows, and added focused regressions for parser/help coverage, stdin-reader behavior, patch-file stdin validation, and publish watch/stdin guardrails.
- Result: dashboard authoring now supports generator-to-CLI pipelines without an extra temp file, and local file-based publish has a built-in watch loop without changing the existing import-tree contract.

## 2026-04-05 - Complete dashboard manual and manpage coverage for stdin/watch authoring
- State: Done
- Scope: `docs/user-guide/en/dashboard.md`, `docs/user-guide/zh-TW/dashboard.md`, `docs/commands/en/dashboard.md`, `docs/commands/zh-TW/dashboard.md`, `docs/man/grafana-util-dashboard*.1`, `docs/html/commands/**/dashboard*.html`, `docs/html/handbook/**/dashboard.html`, `docs/html/man/grafana-util-dashboard*.html`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the command-reference pages for `dashboard review`, `patch-file`, and `publish` already documented stdin and watch support, but the broader dashboard handbook and namespace overview still described the older file-first authoring flow. Generated manpages and HTML also still reflected that older source state.
- Current Update: added a dedicated draft-authoring section to the dashboard handbook in English and Traditional Chinese, expanded the dashboard namespace overview pages to explain the single-dashboard authoring path plus stdin/watch boundaries, and regenerated the manpages and HTML site from the updated source docs. Validation now includes both `make man-check` and `make html-check` after regeneration.
- Result: the source handbook, namespace overview, generated manpages, and generated HTML now all describe the same stdin-aware and watch-aware dashboard authoring workflow.

## 2026-04-05 - Add repo-specific AI workflow note, drift checks, GitHub templates, and AGENTS routing
- State: Done
- Scope: `AGENTS.md`, `docs/internal/ai-workflow-note.md`, `docs/internal/task-brief-template.md`, `.github/ISSUE_TEMPLATE/ai-task-brief.md`, `.github/PULL_REQUEST_TEMPLATE.md`, `scripts/check_ai_workflow.py`, `python/tests/test_python_check_ai_workflow.py`, `Makefile`, `docs/DEVELOPER.md`, `docs/internal/maintainer-quickstart.md`, `docs/internal/README.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the repo already had strong maintainer routing, contract layering, generated-doc rules, and AI trace files, but it did not yet have one current note that translated those existing rules into a concrete workflow for AI-assisted maintenance. It also lacked a stable task brief shape, any repo-owned automated check for the most obvious source/generated/trace drift rules, and any GitHub-native place to reuse the same task fields during collaborative review.
- Current Update: added a dedicated internal workflow note that maps AI-assisted maintenance onto the repo's existing raw-source, maintained-knowledge, and workflow-schema layers. The note defines repo-specific `ingest`, `query`, and `lint` expectations, reinforces generated-versus-source boundaries, gives a minimal task-brief shape for agent work, and frames the final review step as `Diff Review` for solo work or `PR Review` for collaborative work. Added `task-brief-template.md` as a reusable handoff shape, mirrored the same fields into `.github/ISSUE_TEMPLATE/ai-task-brief.md` and `.github/PULL_REQUEST_TEMPLATE.md`, added `scripts/check_ai_workflow.py` as a lightweight drift checker for current-path changes, and wired the new AI workflow entrypoints and `make quality-ai-workflow` validation path into `AGENTS.md` so first-entry agents see the repo-specific workflow immediately.
- Result: the repo now has a written AI workflow, a small executable enforcement layer, GitHub-native templates that keep issue and PR context aligned with the same repo-specific task brief shape, and top-level agent routing that points new agents at the workflow instead of relying on discovery by accident.

## 2026-04-04 - Start template-backed HTML shell rendering
- State: Done
- Scope: `scripts/generate_command_html.py`, `scripts/templates/base.html.tmpl`, `scripts/templates/article_layout.html.tmpl`, `scripts/templates/page_header.html.tmpl`, `scripts/templates/right_sidebar.html.tmpl`, `python/tests/test_python_generate_command_html.py`, `python/tests/test_python_docgen_landing.py`, `docs/internal/generated-docs-architecture.md`, `docs/internal/generated-docs-playbook.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the HTML renderer had already separated some content metadata, but the shared page shell, article layout, header block, and right sidebar were still assembled inline as large Python strings inside `scripts/generate_command_html.py`. That kept layout work tightly coupled to renderer logic and made simple shell edits noisy.
- Current Update: added a minimal file-backed template layer under `scripts/templates/` and moved the shared shell markup, article layout, page header, and right sidebar into template files. `generate_command_html.py` now loads those templates, fills them with existing escaped view data, and keeps the same content contracts for handbook, landing, and command pages.
- Result: the renderer now has a clearer separation between view-model assembly and shared shell markup, while generated output stays in sync with the checked-in HTML tree and focused generator tests still pass.

## 2026-04-04 - Group handbook sidebar navigation by information architecture
- State: Done
- Scope: `scripts/docgen_handbook.py`, `scripts/generate_command_html.py`, `python/tests/test_python_generate_command_html.py`, `docs/internal/generated-docs-architecture.md`, `docs/internal/generated-docs-playbook.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the HTML sidebar rendered every handbook chapter from one flat `HANDBOOK_ORDER` sequence, then appended a second flat command list underneath. That mixed onboarding, role paths, asset guides, governance chapters, and command reference into one scan surface, which made the handbook feel fragmented even when individual pages were fine.
- Current Update: split handbook navigation concerns into two metadata layers. `HANDBOOK_ORDER` still owns linear reading order for previous/next, while new sidebar groups in `scripts/docgen_handbook.py` now define the actual information architecture shown in the HTML nav. The renderer now reads those groups and reduces command reference to a single hub entry instead of a second mini index.
- Result: handbook and command pages now share a grouped handbook sidebar that matches the handbook index IA more closely, while command docs are represented as one secondary entrypoint instead of a competing taxonomy. Validation passed with regenerated HTML plus focused landing and HTML generator tests.

## 2026-04-04 - Separate landing-page content from the HTML renderer
- State: Done
- Scope: `docs/landing/`, `scripts/docgen_landing.py`, `scripts/generate_command_html.py`, `python/tests/test_python_docgen_landing.py`, `python/tests/test_python_generate_command_html.py`, `docs/internal/generated-docs-architecture.md`, `docs/internal/generated-docs-playbook.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the generated homepage had already moved to a task-oriented layout, but the actual landing-page copy, task order, locale strings, and curated links were still hardcoded directly in `scripts/generate_command_html.py`. That made simple homepage content edits look like renderer changes and blurred the source-of-truth boundary.
- Current Update: added a dedicated `docs/landing/{en,zh-TW}.md` source layer plus `scripts/docgen_landing.py` to parse a fixed Markdown contract for hero copy, search copy, task sections, and maintainer links. The HTML generator now renders the landing page from those parsed structures, keeps only UI chrome and version metadata in Python, and still auto-selects `en` or `zh-TW` on first homepage load while preserving manual switching in local storage.
- Result: homepage content now lives in Markdown instead of hardcoded Python, the landing renderer is materially thinner, and landing-page maintenance follows the same metadata-vs-renderer split already used by the handbook generator.

## 2026-04-03 - Tighten dashboard raw-to-prompt semantic compatibility
- State: Done
- Scope: `rust/src/dashboard/prompt.rs`, `rust/src/dashboard/raw_to_prompt_rust_tests.rs`, `scripts/compare_prompt_semantics.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the first `dashboard raw-to-prompt` implementation produced usable prompt JSON, but when compared against a real historical export bundle it only matched 40 of 83 prompt dashboards semantically. The main drift was missing datasource template variables, treating generic `type: datasource` or `-- Mixed --` selectors incorrectly, and collapsing repeated same-family datasource requirements too aggressively.
- Current Update: updated the shared prompt builder so single-family dashboards preserve the Grafana-style datasource templating variable even when multiple prompt slots exist, generic/mixed datasource selectors now become prompt slots without rewriting builtin Grafana annotation selectors, and `__requires` now keeps one datasource entry per prompt slot instead of deduplicating by plugin family. Added focused regression tests for single-family templating, mixed-selector rewriting, and builtin Grafana annotation handling, plus a semantic compare script for replaying historical prompt bundles against generated output. The final historical edge case is now handled in the `raw-to-prompt` runtime itself: it records which panel-subtree datasource paths were placeholders in the raw dashboard and rewrites only those same prompt-output paths back to `$datasource`.
- Result: on the Pontus dashboard export sample, semantic compatibility improved from 40/83 prompt dashboards to 83/83 without modifying the source bundle.

## 2026-04-03 - Thin the unified CLI and type the sync apply intent envelope
- State: Done
- Scope: `rust/src/cli.rs`, `rust/src/cli_help.rs`, `rust/src/lib.rs`, `rust/src/sync/apply_contract.rs`, `rust/src/sync/apply_builder.rs`, `rust/src/sync/live.rs`, `rust/src/sync/live_apply.rs`, `rust/src/sync/workbench.rs`, `rust/src/alert_client.rs`, `rust/src/http.rs`, `rust/src/dashboard/export.rs`, `rust/src/dashboard/live.rs`, `rust/src/sync/preflight.rs`, `rust/src/sync/rust_tests.rs`, `rust/src/sync/live_rust_tests.rs`, `docs/DEVELOPER.md`, `docs/overview-rust.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `cli.rs` still mixed command topology with large help-rendering/example blocks, the sync apply-intent contract was still passed around mostly as ad hoc JSON, and several Rust files still carried low-signal comments that narrated signatures instead of explaining boundaries or invariants.
- Current Update: moved unified help rendering and long example blocks into `cli_help.rs`, introduced `sync/apply_contract.rs` as the typed apply-intent envelope shared by the local builder and live execution, kept the operation loader backward-compatible for existing review/render/live tests, and removed boilerplate comments from the touched Rust files. Maintainer docs now point to the new helper modules and restate the comment-signal / thin-facade rule alongside the concrete refactor.
- Result: the root CLI entrypoint is thinner, the sync apply-intent path now has a repo-owned typed contract instead of only loose JSON, comment noise is lower in the touched files, and the full Rust test suite still passes.

## 2026-04-03 - Add dashboard raw-to-prompt migration workflow
- State: Done
- Scope: `rust/src/dashboard/cli_defs_command.rs`, `rust/src/dashboard/cli_defs_shared.rs`, `rust/src/dashboard/raw_to_prompt.rs`, `rust/src/dashboard/raw_to_prompt_rust_tests.rs`, `rust/src/dashboard/mod.rs`, `rust/src/dashboard/test_support.rs`, `rust/src/cli.rs`, `rust/src/cli_help.rs`, `rust/src/cli_help_examples.rs`, `rust/src/cli_rust_tests.rs`, `rust/src/dashboard/dashboard_cli_parser_help_rust_tests.rs`, `README.md`, `README.zh-TW.md`, `docs/commands/en/*.md`, `docs/commands/zh-TW/*.md`, `docs/user-guide/en/dashboard.md`, `docs/user-guide/en/reference.md`, `docs/user-guide/zh-TW/dashboard.md`, `docs/user-guide/zh-TW/reference.md`, `docs/man/*.1`, `docs/html/**`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the dashboard surface could export a `prompt/` lane from live Grafana, but there was no offline migration path for ordinary dashboard JSON or `raw/` files. Operators had to hand-edit datasource prompts or misuse `dashboard import` for files that really belonged in the Grafana UI import flow.
- Current Update: added a dedicated `dashboard raw-to-prompt` command with explicit file/dir input modes, sibling/default output rules, `infer-family|exact|strict` datasource repair policy, summary/log output controls, optional live datasource lookup through `--profile` or direct live auth flags, and prompt-lane metadata/index writing for `raw/` directory conversions. The command docs, handbook, README, generated manpages, and generated HTML now all explain that `raw/` remains the API replay lane, `prompt/` is for Grafana UI import, and `raw-to-prompt` is the migration bridge between them.
- Result: the repo now has an operator-facing migration path for raw dashboard JSON to prompt JSON, plus optional live datasource augmentation when operators need to repair prompt files against a target Grafana inventory. Focused Rust tests and docs generation now pass for this slice.

## 2026-04-03 - Add maintainer quickstart for first-entry repo orientation
- State: Done
- Scope: `README.md`, `README.zh-TW.md`, `docs/DEVELOPER.md`, `docs/internal/README.md`, `docs/internal/maintainer-quickstart.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the repo already had several strong entrypoints, but a first-time AI agent or new maintainer still had to infer the right reading order from multiple files. There was no single short page that answered which files to open first, which layers are source of truth, which outputs are generated, and which safe validation commands to prefer while still orienting.
- Current Update: added a dedicated `maintainer-quickstart.md` under `docs/internal/` and linked it from README, the Traditional Chinese README, `docs/DEVELOPER.md`, and the internal docs index. The new page defines the first files to read, the maintained surfaces, source-of-truth boundaries, task routing, safe validation defaults, and repo-specific gotchas.
- Result: future AI agents and human maintainers now have one explicit first-stop orientation page instead of reconstructing the repo map from scattered entrypoints.

## 2026-04-03 - Document generated docs architecture for maintainers
- State: Done
- Scope: `docs/DEVELOPER.md`, `docs/internal/generated-docs-architecture.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the repo already had working Markdown-to-manpage and Markdown-to-HTML generators, but the maintainer-facing explanation was still scattered across script docstrings and short notes. Future maintainers would have had to infer source-of-truth rules, Markdown subset limits, locale asymmetry, and Pages deployment behavior from code.
- Current Update: added a dedicated internal design document for the generated docs system and linked it from `docs/DEVELOPER.md`. The new doc explains source layers, output trees, module responsibilities, supported Markdown subset, command/handbook schema expectations, test flow, and GitHub Pages deployment rules.
- Result: maintainers now have one explicit design/maintenance reference for the generated docs pipeline instead of reconstructing it from the generators.

## 2026-04-03 - Add generated docs maintainer playbook
- State: Done
- Scope: `docs/DEVELOPER.md`, `docs/internal/generated-docs-architecture.md`, `docs/internal/generated-docs-playbook.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the new generated-docs architecture note explained the system well, but it was still architecture-first. A maintainer adding a new command page, handbook chapter, namespace manpage, locale, or Pages-facing output file still had to translate the design into concrete repo steps by hand.
- Current Update: added a task-oriented playbook for the generated docs pipeline and linked it from both `docs/DEVELOPER.md` and the architecture note. The playbook covers the common change types, the exact files to edit, the generator hooks to update, and the standard validation loop.
- Result: maintainers now have both the design reference and an operational cookbook for common generated-docs changes.

## 2026-04-03 - Reorganize DEVELOPER.md as a maintainer routing map
- State: Done
- Scope: `docs/DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `docs/DEVELOPER.md` already contained the right pointers, but the content was still arranged more like a short note dump than a high-signal maintainer map. It mixed code architecture, generated-docs notes, contract pointers, and validation guidance without a strong routing structure.
- Current Update: reorganized `docs/DEVELOPER.md` into explicit maintainer sections: start-here routing, repo priorities, code architecture map, documentation map, validation/build guidance, project rules, and a quick routing table by task type.
- Result: maintainers can now navigate by concern instead of reading the whole page linearly to discover where to go next.

## 2026-04-03 - Tighten maintainer guidance for comment signal and facade thinning
- State: Done
- Scope: `docs/DEVELOPER.md`, `docs/overview-rust.md`, `docs/internal/maintainer-quickstart.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the maintainer docs already described the Rust routing layers, but they did not yet state the quality bar clearly enough for comment signal, repo-owned envelopes, or how thin the facades should stay.
- Current Update: added short maintainer guidance that prefers repo-owned typed envelopes over ad hoc shapes, keeps `cli.rs` and domain facades focused on routing/normalize/re-export, and treats comments as signal for ownership or non-obvious behavior rather than narration.
- Result: maintainers now have a concise policy for keeping the Rust surface thinner and the comments more useful without turning the docs into a longer design note.

## 2026-04-03 - Document profile secret storage across user and maintainer docs
- State: Done
- Scope: `README.md`, `README.zh-TW.md`, `docs/DEVELOPER.md`, `docs/internal/README.md`, `docs/internal/profile-secret-storage-architecture.md`, `docs/user-guide/en/reference.md`, `docs/user-guide/zh-TW/reference.md`, `docs/commands/en/profile.md`, `docs/commands/zh-TW/profile.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the repo already supported environment-backed secrets, OS secret storage, and encrypted secret files, but that guidance was scattered. Operators could see fragments in `profile` docs, and maintainers could infer the platform backends from Rust code, but there was no single complete explanation of what the modes are, why they exist, what platforms they support, or how to troubleshoot them.
- Current Update: added a dedicated internal secret-storage architecture note, linked it from the maintainer entrypoints, and expanded the user-facing reference/profile docs with complete mode descriptions, macOS/Linux OS-store notes, usage guidance, caveats, and troubleshooting.
- Result: both operators and maintainers now have a clear documented model for profile secret storage instead of piecing it together from examples and code.

## 2026-04-03 - Add role-based doc entrypoints for operators and maintainers
- State: Done
- Scope: `README.md`, `README.zh-TW.md`, `docs/user-guide/en/index.md`, `docs/user-guide/zh-TW/index.md`, `docs/user-guide/en/role-new-user.md`, `docs/user-guide/en/role-sre-ops.md`, `docs/user-guide/en/role-automation-ci.md`, `docs/user-guide/zh-TW/role-new-user.md`, `docs/user-guide/zh-TW/role-sre-ops.md`, `docs/user-guide/zh-TW/role-automation-ci.md`, `docs/DEVELOPER.md`, `docs/internal/README.md`, `docs/internal/maintainer-role-map.md`, `scripts/docgen_handbook.py`, `scripts/generate_command_html.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the docs already had substantial content, but the primary navigation was still file-family oriented. New users, SREs, automation owners, and maintainers had to infer their own reading order from README sections, handbook chapter lists, and internal indexes.
- Current Update: added dedicated public role-guide handbook pages in English and Traditional Chinese, added an internal maintainer-role map, updated handbook ordering to treat the public role pages as first-class chapters, and upgraded README, handbook indexes, and the generated HTML landing page to route readers by role as well as by document type.
- Result: the docs now support both content-type navigation and role-based navigation, which makes the full document set easier to approach without already knowing the repo’s file layout.

## 2026-04-03 - Split default and browser-enabled Rust release artifacts
- State: Done
- Scope: `rust/Cargo.toml`, `Makefile`, `scripts/build-rust-macos-arm64.sh`, `scripts/build-rust-linux-amd64.sh`, `scripts/build-rust-linux-amd64-zig.sh`, `scripts/validate-rust-linux-amd64-artifact.sh`, `scripts/install.sh`, `.github/workflows/ci.yml`, `docs/DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the default Rust feature set included `browser`, so every normal build and release artifact pulled in `headless_chrome`. That made cross-builds slower and heavier, while Linux validation kept getting tangled with the browser-enabled dependency set.
- Current Update: switched the default feature set to lean/TUI-only, added explicit browser-enabled build targets and release assets, aligned install/build/validation tooling to choose standard versus browser artifacts intentionally, and cleaned up browser-disabled screenshot warnings triggered by the new default feature policy.
- Result: standard builds now omit `headless_chrome`, browser support is shipped through explicit `*-browser` artifacts and CI jobs, and both the default and browser-enabled Rust compile paths validate cleanly.

## 2026-04-02 - Consolidate contract docs into summary/spec/trace layers
- State: Done
- Scope: `docs/DEVELOPER.md`, `docs/internal/contract-doc-map.md`, `docs/internal/export-root-output-layering-policy.md`, `docs/internal/dashboard-export-root-contract.md`, `docs/internal/datasource-masked-recovery-contract.md`, `docs/internal/alert-access-contract-policy.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: current contract guidance was split awkwardly across maintainer summary notes and trace files, which made navigation noisier and encouraged repeating the same detailed rules in multiple places.
- Current Update: created dedicated current spec docs for repo-level export-root policy, dashboard export-root, and datasource masked-recovery contracts; kept `docs/DEVELOPER.md` as the short summary layer; and aligned the AI trace files to stay trace-only.
- Result: maintainers now have one clear summary/spec/trace split for the active contract topics instead of overlapping note fragments.

## 2026-04-02 - Clarify export-root/output layering scope
- State: Done
- Scope: `docs/DEVELOPER.md`, `docs/internal/export-root-output-layering-policy.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the maintainer notes already documented dashboard and datasource export/projection boundaries, but they did not yet spell out the repo-level pattern clearly enough to prevent overgeneralizing it to every resource kind.
- Current Update: added the short repo-level policy that reserves the explicit export-root/output-layering pattern for `dashboard` and `datasource`, with the detailed domain rule now anchored in a dedicated policy doc.
- Result: maintainers now have one concise place to read where the pattern applies and one detailed current policy doc for the full rule.

## 2026-04-02 - Clarify alert/access contract boundaries
- State: Done
- Scope: `docs/DEVELOPER.md`, `docs/internal/alert-access-contract-policy.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the repo-level export-root note already kept `alert` and `access` outside the dashboard/datasource pattern, but it still left too much room for maintainers to infer that any root index or staged bundle set should automatically grow `scopeKind` or aggregate-root semantics.
- Current Update: moved the detailed requirements into a dedicated policy doc that defines the current `alert` and `access` contract types, promotion criteria, and documentation split between summary docs and trace docs.
- Result: the repo now has one stable requirements doc for this boundary instead of repeating the same policy text across multiple maintainer notes.

## 2026-04-02 - Formalize dashboard export-root contract
- State: Done
- Scope: `docs/DEVELOPER.md`, `docs/internal/dashboard-export-root-contract.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: dashboard runtime and help text already treated `raw/`, `provisioning/`, and combined roots as different staged contract shapes, but the maintainer docs did not yet define the dashboard root contract as explicitly as the datasource masked-recovery contract.
- Current Update: moved the detailed dashboard root-manifest, scope semantics, and output-layering rules into a dedicated current contract doc while leaving only the short summary in `docs/DEVELOPER.md`.
- Result: dashboard now has a stable spec doc that can be updated without turning the maintainer summary or trace files into duplicate contract inventories.

## 2026-04-02 - Close datasource masked-recovery bookkeeping
- State: Done
- Scope: `TODO.md`, `docs/DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the datasource masked-recovery export/import/inspect lane was already complete, but the active backlog and maintainer notes still read as if the work was open.
- Current Update: removed the datasource masked-recovery item from the active TODO backlog and recorded the current maintainer contract at a concise level: `datasources.json` stays the canonical replay/masked-recovery artifact, `provisioning/datasources.yaml` stays a projection, and inspect/output notes should keep the masked secret boundary intact.
- Result: the bookkeeping now matches the finished datasource contract and no longer advertises the lane as active work.

## 2026-04-02 - Formalize datasource masked-recovery schema policy
- State: Done
- Scope: `docs/DEVELOPER.md`, `docs/internal/datasource-masked-recovery-contract.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the datasource maintainer notes already described the masked-recovery lane, but the schema compatibility rules were still implicit instead of written down as a stable contract policy.
- Current Update: moved the detailed stable fields, additive-versus-breaking rules, and `schemaVersion` guidance into a dedicated current contract doc while leaving the short summary in `docs/DEVELOPER.md`.
- Result: maintainers now have one current datasource contract spec to read before making export/import or help-text changes.

## 2026-04-01 - Add repo-owned install script for release binaries
- State: Done
- Scope: `scripts/install.sh`, `python/tests/test_python_install_script.py`, `README.md`, `README.zh-TW.md`, `docs/user-guide.md`, `docs/user-guide-TW.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: operators could build from source or download release assets manually, but there was no supported one-line install path that fetched the right Rust binary and placed it into a common executable directory.
- Current Update: added a repo-owned POSIX install script that detects `linux-amd64` and `macos-arm64`, downloads the matching GitHub release archive, installs `grafana-util` into `/usr/local/bin` when writable or falls back to `~/.local/bin`, and supports explicit `BIN_DIR`, `VERSION`, `REPO`, and `ASSET_URL` overrides. Public English and Traditional Chinese docs now show the one-line `curl ... | sh` path plus the direct local-checkout fallback.
- Result: users now have a documented one-line installer for the maintained Rust binary without needing to compile from source or hand-place the executable.

## 2026-04-05 - Converge project-status and sync onto the shared live layer
- State: Done
- Scope: `rust/src/grafana_api/mod.rs`, `rust/src/grafana_api/tests.rs`, `rust/src/grafana_api/project_status_live.rs`, `rust/src/grafana_api/sync_live.rs`, `rust/src/project_status_live_runtime.rs`, `rust/src/project_status_support.rs`, `rust/src/sync/live.rs`, `rust/src/sync/live_apply.rs`, `rust/src/sync/live_fetch.rs`, `rust/src/sync/mod.rs`
- Baseline: the repository had already introduced `GrafanaConnection` and `GrafanaApiClient`, but `project-status` and `sync` still owned major live endpoint contracts directly. That left the repo in a half-migrated state where shared live wiring and request-closure flows coexisted as parallel main paths.
- Current Update: added workflow-level shared live helpers for `project-status` and `sync`, moved org listing / alert-surface reads / dashboard version-history reads behind `grafana_api::project_status_live`, and moved sync live fetch/apply endpoint ownership behind `grafana_api::sync_live`. `project-status` now resolves one root `GrafanaApiClient` and derives org-scoped clients from it, while `sync` now resolves one client per command and routes client-backed fetch/apply through `SyncLiveClient` instead of owning raw Grafana path handling in the command runtime.
- Result: `grafana_api` is now acting as an internal shared live layer for the two biggest remaining workflow-heavy live paths, reducing duplicate endpoint ownership without turning the repo into a generic endpoint SDK.

## 2026-04-01 - Extend alert list output formats
- State: Blocked
- Scope: `rust/src/alert_cli_defs.rs`, `rust/src/alert_list.rs`, `rust/src/alert_rust_tests.rs`
- Baseline: alert list commands (`list-rules`, `list-contact-points`, `list-mute-timings`, `list-templates`) only normalized `table`, `csv`, and `json` flags, with table as the default. The runtime list renderer also only handled table/csv/json.
- Current Update: widened the list parser/output normalization to include `text` and `yaml`, updated the list help text examples, and added focused parser/rendering tests for the expanded output set. Focused `cargo test` validation is blocked by unrelated compile errors elsewhere in the crate.
- Result: code changes are in place, but the focused Rust test slice does not complete because the repository currently fails to compile in unrelated dashboard/datasource files.

## 2026-04-01 - Record baseline-five live defaults and dashboard review output inventory
- State: Done
- Scope: `rust/src/profile_config.rs`, `rust/src/profile_cli.rs`, `rust/src/cli.rs`, `rust/src/cli_help_examples.rs`, `rust/src/dashboard/cli_defs_shared.rs`, `rust/src/dashboard/dashboard_runtime.rs`, `rust/src/access/access_cli_shared.rs`, `rust/src/access/access_cli_runtime.rs`, `rust/src/alert_cli_defs.rs`, `rust/src/project_status_command.rs`, `rust/src/project_status_support.rs`, `rust/src/dashboard/authoring.rs`, `rust/src/dashboard/cli_defs_command.rs`, `rust/src/dashboard/dashboard_cli_parser_help_rust_tests.rs`, `rust/src/dashboard/authoring_rust_tests.rs`, `rust/src/dashboard/mod.rs`, `rust/src/cli_rust_tests.rs`, `docs/user-guide.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the shared live connection baseline still required repeating URL/auth/TLS flags by hand, and the dashboard authoring lane had not yet been documented as a local-only specialization with an explicit output-mode inventory.
- Current Update: expanded the repo-local profile baseline across the five live surfaces on the baseline-five rule (`dashboard`, `datasource`, `access`, `alert`, and `status live`) so they now inherit named live defaults from `grafana-util.yaml` while preserving explicit CLI overrides and environment fallbacks. In the same wave, documented the dashboard-only authoring/review lane as intentionally specialized: `get` and `clone-live` create local drafts, `patch-file` and `publish` reuse the import pipeline, and `review` now makes its output coverage explicit across text, table, CSV, JSON, and YAML.
- Result: the Rust CLI now has a five-surface shared live baseline, while the dashboard authoring/review surfaces stay deliberately specialized instead of being folded into the shared live connection layer.
## 2026-04-05 - Refactor `change` into a task-first lane
- State: In Progress
- Scope: `rust/src/sync/mod.rs`, `rust/src/sync/guided.rs`, `rust/src/sync/cli.rs`, `rust/src/sync/*_rust_tests.rs`, `rust/src/cli_help.rs`, `rust/src/cli_help_examples.rs`, `rust/src/cli_rust_tests.rs`, `rust/src/dashboard/mod.rs`, `rust/src/dashboard/history_cli_rust_tests.rs`, `docs/commands/en/change.md`, `docs/commands/en/index.md`, `docs/commands/zh-TW/index.md`, `docs/user-guide/en/change-overview-status.md`, `docs/user-guide/en/role-sre-ops.md`, `docs/user-guide/zh-TW/role-sre-ops.md`, `README.md`, `README.zh-TW.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `change` exposed the lower-level sync lifecycle directly (`summary`, `plan`, `review`, `preflight`, `apply`, plus bundle/promotion lanes), so first-run users had to understand staged artifact names before they could tell which command to run.
- Current Update: added task-first `change inspect`, `change check`, and `change preview` routing on top of the existing staged sync builders, moved the old low-level workflow under `change advanced`, taught `change apply` to look for `--preview-file` or common repo-local preview artifacts, and updated Rust help/parser tests plus the first-entry docs to describe the new lane. The same work also finished wiring the already-added dashboard `history` command surface so the crate compiles and tests cleanly with that command present.
- Result: the staged sync contract still exists underneath, but the operator-facing `change` entrypoint now starts from task intent instead of internal document names.
## 2026-04-05 - Centralize dashboard import lookup live calls
- State: Done
- Scope: `rust/src/dashboard/import_lookup.rs`, `rust/src/dashboard/live.rs`, `rust/src/grafana_api/dashboard.rs`, `rust/src/grafana_api/tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: dashboard import lookup still owned one raw create-folder API contract locally and spread its request-based live calls across multiple direct helper invocations, even after the shared `grafana_api` layer existed for dashboard resources.
- Current Update: added shared `create_folder_entry(...)` coverage to `grafana_api/dashboard.rs`, moved the request-based folder-create helper into `dashboard/live.rs`, and wrapped `import_lookup.rs` live reads/writes behind a local `ImportLookupRequestClient` so dashboard summary loading, dashboard/folder fetches, current-org lookup, org listing, and folder creation now route through one lookup-scoped client seam.
- Result: `import_lookup.rs` no longer owns a raw folder-create endpoint contract, and the remaining request-based import live calls are centralized enough to make a later `DashboardResourceClient` threading pass smaller.
## 2026-04-05 - Add client-backed dashboard import preflight
- State: Done
- Scope: `rust/src/dashboard/import_apply.rs`, `rust/src/dashboard/import_validation.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: dashboard import runtime still validated export org and preflight dependencies through request-closure helpers only, even when a concrete `JsonHttpClient` was already present on the client-backed path.
- Current Update: added client-backed import validation helpers that use `DashboardResourceClient` for current-org lookup, datasource listing, and plugin availability reads, then wired `import_dashboards_with_client` to invoke them before the existing request-based execution path.
- Result: the client-backed import entrypoint now touches the shared dashboard resource layer directly for live preflight reads, while the request-closure seam remains in place for the generic execution path.
## 2026-04-05 - Centralize datasource/import org lookups
- State: Done
- Scope: `rust/src/grafana_api/access.rs`, `rust/src/grafana_api/tests.rs`, `rust/src/datasource_import_export_support.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: datasource import/export helpers still read `/api/org` and `/api/orgs` directly from `JsonHttpClient`, even though the shared Grafana client layer already owned the same org contracts for dashboard flows.
- Current Update: added shared `fetch_current_org(...)` and `list_orgs(...)` methods to `AccessResourceClient`, and switched datasource import/export support helpers to use `DashboardResourceClient` for the same org lookup reads.
- Result: the shared client layer now owns the org lookup contract in one place, and datasource import/export no longer hardcodes those paths locally.
## 2026-04-05 - Route dashboard import client path through shared lookup backend
- State: Done
- Scope: `rust/src/dashboard/import_apply.rs`, `rust/src/dashboard/import_lookup.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the dashboard import client-backed entrypoint only used the shared dashboard client for preflight checks, then dropped back to the request-closure execution path for the real import loop.
- Current Update: added a shared lookup backend in `import_lookup.rs` that supports both request closures and `DashboardResourceClient`, then rewired `import_dashboards_with_client` to use the client-backed lookup path for dashboard existence checks, folder-path resolution, folder ensuring, and final dashboard import requests while keeping the interactive selection seam request-based.
- Result: the main dashboard import client path now runs through the shared Grafana client layer end-to-end for its live lookup/apply flow, while tests and edge orchestration can still use the request seam.
## 2026-04-05 - Align dashboard import dry-run and interactive review with shared lookup backend
- State: Done
- Scope: `rust/src/dashboard/import_dry_run.rs`, `rust/src/dashboard/import_interactive_review.rs`, `rust/src/dashboard/import_apply.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: dashboard import dry-run and interactive review still only had request-based lookup flows, even after the import client path had moved onto the shared lookup backend.
- Current Update: added a client-backed dry-run report builder in `import_dry_run.rs` and used it from `import_dashboards_with_client` for dry-run execution. Also added a client-backed interactive review path in `import_interactive_review.rs` so review resolution can use the same shared lookup backend when a concrete dashboard client is available, while the current TUI caller keeps its request seam unchanged.
- Result: the main import, dry-run, and review code paths now share the same lookup model instead of diverging into separate endpoint ownership patterns.
## 2026-04-05 - Route TUI dashboard import review through the shared client path
- State: Done
- Scope: `rust/src/dashboard/import_interactive.rs`, `rust/src/dashboard/import_interactive_render.rs`, `rust/src/dashboard/import_interactive_state.rs`, `rust/src/dashboard/import_apply.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the interactive dashboard import lane still always resolved focused review rows through the request-closure path, even when the surrounding import entrypoint already had a concrete `DashboardResourceClient`.
- Current Update: added a client-backed interactive selector entrypoint plus client-backed focused-review resolution in the TUI state/render path, then wired the client-backed import entrypoint to use that selector when `--interactive` is enabled.
- Result: the TUI import lane now follows the same shared client path as the rest of the client-backed dashboard import flow instead of dropping back to request-only review resolution.
## 2026-04-05 - Merge dashboard import request/client main loops
- State: Done
- Scope: `rust/src/dashboard/import_apply.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `import_apply.rs` still kept two large import execution flows, one for request closures and one for `DashboardResourceClient`, so behavior was aligned but the main orchestration loop was still duplicated and guarded by a temporary `#![allow(dead_code)]`.
- Current Update: finished wiring the existing shared `LiveImportBackend`, `prepare_import_run(...)`, `run_live_import(...)`, and `render_dry_run_report(...)` helpers into the real request/client entrypoints. Dry-run rendering now uses one shared renderer, and both live paths now share the same import preparation and main loop while keeping backend-specific lookup/apply hooks.
- Result: the dashboard import runtime no longer maintains parallel request/client main loops for the same behavior, and `import_apply.rs` no longer needs the dead-code escape hatch to compile cleanly.
