# ai-status.md

Current AI-maintained status only.

- Older trace history moved to [`archive/ai-status-archive-2026-03-24.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-03-24.md).
- Detailed 2026-03-27 entries moved to [`archive/ai-status-archive-2026-03-27.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-03-27.md).
- Detailed 2026-03-28 task notes were condensed into [`archive/ai-status-archive-2026-03-28.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-03-28.md).
- Some older entries below still cite pre-cleanup `docs/internal/...` paths for files that now live under `docs/internal/archive/`.
- Keep this file short and current. Additive historical detail belongs in `docs/internal/archive/`.

## 2026-03-30 - Rename project surfaces and remove the local web workbench
- State: Done
- Scope: `README.md`, `README.zh-TW.md`, `docs/user-guide.md`, `docs/user-guide-TW.md`, `rust/Cargo.toml`, `rust/Cargo.lock`, `rust/src/cli.rs`, `rust/src/cli_help_examples.rs`, `rust/src/cli_rust_tests.rs`, `rust/src/lib.rs`, `rust/src/overview.rs`, `rust/src/overview_rust_tests.rs`, `rust/src/project_status.rs`, `rust/src/project_status_command.rs`, `rust/src/sync/mod.rs`, `rust/src/sync/cli_help_rust_tests.rs`, `rust/src/bin/grafana-util-web.rs`, `rust/src/web/*`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the repo still exposed three overlapping top-level names in the operator surface: `overview`, `project-status`, and `sync`. Public docs had already started moving toward `overview / status / change`, but the Rust CLI and help text still carried the older names. Separately, the optional `grafana-util-web` Axum workbench had become another current surface even though it was no longer wanted.
- Current Update: renamed the active CLI and doc surfaces to `overview`, `status`, and `change` with no compatibility aliases. Updated root parser/help routing, overview live wording, status/change command examples, default review token, and focused Rust parser/help tests to match the new operator model. Then removed the entire local web workbench stack: Cargo `web` feature, the `grafana-util-web` binary, optional web-only deps, and the whole `rust/src/web/` module tree.
- Result: the project now presents two clearer operator entry lanes plus one advanced status surface: `overview` for human-first project understanding, `change` for review/apply workflows, and `status` for canonical readiness. The repo no longer carries an unused localhost web/API stack.

## 2026-03-30 - Alert authoring round-trip normalization and live smoke
- State: Done
- Scope: `rust/src/alert.rs`, `rust/src/alert_support.rs`, `rust/src/alert_runtime_support.rs`, `rust/src/alert_import_diff.rs`, `rust/src/alert_rust_tests.rs`, `scripts/test-rust-live-grafana.sh`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the new Alert V2 authoring docs were live, but `alert apply` followed by `alert plan` still drifted on authoring-created contact points, managed policies, and simple rules because Grafana injects default fields, omits some empty objects, and can reorder compare-relevant arrays.
- Current Update: normalized compare payloads for alert rules, contact points, and notification policies before diff/plan/import comparisons, covering Grafana-added defaults such as `disableResolveMessage: false`, `continue: false`, empty `annotations`, server-managed rule fields, empty `queryType`, matcher ordering, and integral-float number shape. Added focused Rust regressions for the normalization path and extended the Docker live smoke with an Alert V2 authoring flow: `add-contact-point`, `add-rule`, `preview-route`, `plan`, `apply`, post-apply `plan`, prune delete plan, and prune delete apply.
- Result: authoring-created contact points, managed routes, and simple rules now round-trip back to `noop` after apply under the validated Grafana `12.4.1` smoke path, and the repo now has a regression guard for the operator workflow documented in the new user guide.

## 2026-03-30 - Alert plan/apply/delete management lane
- State: Done
- Scope: `rust/src/alert.rs`, `rust/src/alert_cli_defs.rs`, `rust/src/alert_import_diff.rs`, `rust/src/alert_runtime_support.rs`, `rust/src/alert_support.rs`, `rust/src/alert_rust_tests.rs`, `rust/src/cli_help_examples.rs`, `rust/src/cli_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the alert domain already had export/import/diff and list surfaces, and the sync live-apply transport already knew how to mutate Grafana alert provisioning resources, but there was no operator-first alert management lane for reviewable desired-state planning, explicit apply, or safe delete/scaffold workflows.
- Current Update: added first-class `alert plan`, `alert apply`, `alert delete`, `alert init`, `alert new-rule`, `alert new-contact-point`, and `alert new-template` command surfaces, wired them into the Rust alert dispatcher, taught alert desired-state discovery to accept both JSON and YAML, and introduced managed-dir init/scaffold plus plan/apply/delete runtime helpers that reuse the existing alert document shape and provisioning request contracts.
- Result: operators now have a CLI-first alert management lane that can build reviewable create/update/noop/delete plans from YAML or JSON desired files, apply reviewed plans back to Grafana, preview explicit deletes with policy-reset guarding, and scaffold a managed alert desired-state tree without going through the older migration-oriented import path.

## 2026-03-30 - Alert V2 operator docs refresh
- State: Done
- Scope: `README.md`, `README.zh-TW.md`, `docs/user-guide.md`, `docs/user-guide-TW.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the new alert authoring layer had already landed in the runtime and CLI help, but the public docs still centered the older `init/new-*` management wording, under-described the new authoring commands, and did not cleanly separate desired-state authoring from review/apply or the older `raw/` migration lane.
- Current Update: rewrote the public alert docs around three layers: authoring, review/apply, and migration. Added operator-facing boundaries for `add-rule`, `clone-rule`, `set-route`, `preview-route`, `--folder`, and authoring `--dry-run`; refreshed README quick paths in both languages; and expanded the user guides with simple add, complex clone/edit, and prune-delete workflows. Also captured local Docker Grafana `12.4.1` validation data for `plan/apply/prune` plus the new authoring output shapes, while documenting the current live normalization drift that prevents a clean all-`noop` round-trip after apply.
- Result: the public docs now describe the intended Alert V2 operator workflow without implying that authoring commands mutate Grafana directly, and maintainers have one current trace entry that records both the validated doc examples and the remaining live-validation limitation.

## 2026-03-30 - Connection-first web flow and `web/mod.rs` split
- State: Done
- Scope: `rust/src/web/mod.rs`, `rust/src/web/handlers.rs`, `rust/src/web/contracts.rs`, `rust/src/web/registry.rs`, `rust/src/web/page.rs`, `rust/src/web/index.js`, `rust/src/web/index.css`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the new web workspace shell already had dashboard-first actions, but the connection experience was still easy to miss, there was no explicit `Test Connection` confirmation path, and `rust/src/web/mod.rs` had grown into one large mixed file containing contracts, registry data, HTML, handlers, helpers, and tests.
- Current Update: moved the web shell to a clearer three-step flow with a dedicated connection step, session-scoped connection status messaging, and a real `/api/connection/test` path wired to the shared Grafana auth/runtime helpers. Split the oversized `web/mod.rs` into smaller modules by concern so root routing/server wiring stays in `mod.rs` while contracts, registry metadata, HTML, and handlers live in their own files.
- Result: operators now have a clearer place to enter URL/auth data and verify login before running workspace actions, and the Rust web module layout is easier to extend without turning `mod.rs` back into a monolith.

## 2026-03-30 - Wire all-orgs live project-status execution
- State: Done
- Scope: `rust/src/project_status_command.rs`, `rust/src/overview.rs`, `rust/src/cli_rust_tests.rs`, `rust/src/overview_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `project-status live` and delegated `overview live` already expose `--all-orgs` in the CLI contract and help text, but the runtime still builds one unscoped client and ignores both `--all-orgs` fan-out and explicit `--org-id` scoping for the main live domain reads.
- Current Update: wired the shared live execution path to build org-scoped clients with `X-Grafana-Org-Id`, enumerate visible orgs for real `--all-orgs` fan-out, and aggregate live dashboard, datasource, alert, and access status across orgs instead of silently reading only the default scope. Focused parser/help and shared live-path regressions now lock both `project-status live` and delegated `overview live` to the same scoped runtime behavior.
- Result: `project-status live --org-id <id>` now performs real scoped reads, `project-status live --all-orgs` now aggregates live status across visible orgs for the supported domains, and `overview live` inherits the same canonical live path instead of advertising scope flags it does not actually honor.

## 2026-03-30 - Dashboard-first web workspace over shared Rust execution seams
- State: Done
- Scope: `rust/src/dashboard/mod.rs`, `rust/src/dashboard/inspect.rs`, `rust/src/dashboard/inspect_live.rs`, `rust/src/dashboard/inspect_output.rs`, `rust/src/dashboard/list.rs`, `rust/src/dashboard/vars.rs`, `rust/src/web/mod.rs`, `rust/src/web/index.js`, `rust/src/web/index.css`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `grafana-util-web` already existed as a localhost-only Axum binary, but it was still a thin generic capability console with a raw JSON textarea and no dashboard-specific workspace design. Dashboard list/browse/inspect flows still mostly existed as CLI/TUI paths, and the web layer did not yet have a shared connection panel or a browser-native browse surface.
- Current Update: replaced the generic web console with a workspace shell that keeps Grafana connection settings in browser session state, added dashboard-focused workspace actions for `browse`, `list`, `inspect-live`, `inspect-export`, and `inspect-vars`, and introduced reusable Rust execution seams so the web handlers can reuse the same dashboard list and inspect builders without stdout capture or JS-side business-logic reconstruction. The new browse response now ships tree/list/detail data from Rust, while the frontend renders a distinct browser-native layout instead of imitating the TUI.
- Result: operators now get a real dashboard workbench in web, with URL/token or username/password input at the top, workspace/action navigation, browse/list/inspect flows backed by shared Rust contracts, and the earlier overview/project-status/sync web actions still available under the same shell.

## 2026-03-30 - `grafana-util-web` local workbench over shared execution seams
- State: Done
- Scope: `rust/Cargo.toml`, `rust/src/lib.rs`, `rust/src/overview.rs`, `rust/src/project_status_command.rs`, `rust/src/sync/mod.rs`, `rust/src/sync/cli.rs`, `rust/src/web/mod.rs`, `rust/src/web/index.css`, `rust/src/web/index.js`, `rust/src/bin/grafana-util-web.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: there was no dedicated local web workbench binary yet, and the mature whole-project/status surfaces still only existed as CLI/TUI consumers. `overview`, `project-status`, and `sync` already had stable JSON documents, but they were still primarily shaped around direct CLI printing rather than a reusable browser-facing entry seam.
- Current Update: added a separate `grafana-util-web` binary behind a dedicated Cargo `web` feature, kept the default CLI build free of web dependencies, introduced reusable execution seams for `overview`, `project-status`, and selected read-only `sync` commands, and layered a localhost-first Axum workbench on top with a thin capability registry plus one browser page that posts JSON requests and renders text/JSON responses. The server binds to `127.0.0.1` by default, keeps credentials request-scoped and in-memory, and intentionally excludes live apply in web v1.
- Result: operators get a localhost-only browser surface over the same shared Rust contracts, with the web layer staying thin and owned by the existing execution seams.

## 2026-03-30 - `overview live` thin alias over shared project-status live
- State: Done
- Scope: `rust/src/overview.rs`, `rust/src/cli_help_examples.rs`, `rust/src/overview_rust_tests.rs`, `docs/internal/overview-architecture.md`, `docs/internal/current-capability-inventory-2026-03-30.md`
- Baseline: `overview` was staged-only and already intentionally thin, but there was no operator-friendly `overview live` entrypoint. Users had to know to switch to `project-status live` for whole-project live status even when they conceptually wanted the top-level overview surface.
- Current Update: kept the ownership boundary intact and added `overview live` as a thin convenience entry that delegates straight to `project-status live`. The staged path still owns artifact loading and document assembly; the live path reuses the canonical live status contract and renderer family. Help text and maintainer docs now state that `overview` remains staged by default and only exposes live through a delegated entrypoint.
- Result: operators can use `grafana-util overview live ...` as a practical top-level live entry without creating a second live-status engine or forking the project-status contract.

## 2026-03-30 - Faster inspect-export interactive pre-workbench analysis
- State: Done
- Scope: `rust/src/dashboard/inspect_query_report.rs`, `rust/src/dashboard/inspect_extract_query.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `dashboard inspect-export --interactive` now showed progress messages, which confirmed it was not dead, but large legacy export trees could still appear to stall for a long time at `building query report` before the inspect workbench opened.
- Current Update: kept the interactive workflow the same and optimized the hottest pre-workbench parsing path by moving repeated hard-coded regex compilation in query-variable, Prometheus, Flux, InfluxQL, and SQL query extraction helpers to shared `LazyLock<Regex>` statics. This removes thousands of per-query regex recompiles while preserving the same report/build outputs.
- Result: large export trees should spend materially less time in the silent analysis phase before the inspect workbench launches, so interactive inspect-export behaves more like a slow build with visible progress than a hung command.

## 2026-03-30 - Tool-version metadata across staged Rust outputs
- State: Done
- Scope: `rust/src/common.rs`, `rust/src/dashboard/files.rs`, `rust/src/datasource_export_support.rs`, `rust/src/alert_support.rs`, `rust/src/sync/*.rs`, `rust/src/overview.rs`, `rust/src/project_status.rs`
- Baseline: most staged/export/status JSON documents carried `schemaVersion`, and some alert documents also carried `apiVersion`, but there was no consistent field that recorded which `grafana-util` version produced the file.
- Current Update: added a shared `toolVersion` helper sourced from the Rust crate version and emitted it additively on the main staged/export/status documents: dashboard export metadata/root index, datasource export metadata/indexes, alert export documents/root index, sync summary/source-bundle/preflight/plan/apply/audit/promotion-preflight, alert sync assessment, and top-level `overview` / `project-status` JSON documents.
- Result: future compatibility handling can distinguish contract version from producer version without breaking older readers or requiring existing artifacts to be rewritten.

## 2026-03-30 - Shared TUI presentation pass for project-level and browser surfaces
- State: Done
- Scope: `rust/src/tui_shell.rs`, `rust/src/overview_tui_render.rs`, `rust/src/project_status_tui_render.rs`, `rust/src/dashboard/browse_render.rs`, `rust/src/datasource_browse_render.rs`
- Baseline: interactive surfaces already worked, but summary/header/footer presentation was inconsistent. Footer controls were visually ragged, `overview` showed redundant top blocks, and summary rows in different TUI surfaces mixed ad hoc spacing with low visual hierarchy.
- Current Update: kept behavior and ownership the same, but introduced shared summary-cell presentation in `tui_shell`, consolidated `overview` into a single top summary block, aligned footer control cells, and adopted the same brighter label/accent hierarchy for `overview`, `project-status`, dashboard browser, and datasource browser.
- Result: the TUI surfaces now read more like one shell family: context stays in the top summary block, controls stay in the footer, and key status/action fields are easier to scan without widening any status or workflow contract.

## 2026-03-30 - Overview compatibility for legacy dashboard export indexes
- State: Done
- Scope: `rust/src/dashboard/models.rs`, `rust/src/dashboard/export_focus_report_query_presentation_summary_inventory_rust_tests.rs`
- Baseline: `grafana-util overview --dashboard-export-dir ...` rejected older dashboard export roots when their `index.json` rows omitted `org` / `orgId`, even if the rest of the raw export layout matched the staged inspection contract.
- Current Update: made legacy variant-index org identity fields optional and added a focused regression that exercises a raw export root whose `index.json` carries only uid/title/path/format. The shared dashboard inspection summary path now treats that export as valid staged input with unknown export org identity instead of hard failing.
- Result: older single-scope dashboard export trees can feed `overview` and other inspect-summary consumers as long as the raw export shape is otherwise intact.

## 2026-03-30 - Project-status workbench clarity
- State: Done
- Scope: `rust/src/project_status_tui.rs`, `rust/src/project_status_tui_render.rs`
- Baseline: the project-status workbench already surfaced the recommended domain/action handoff, but the home/header/footer copy still left the project-home -> domain -> action flow implicit and the top blocker summary was less explicit than the rest of the document.
- Current Update: tightened the presentation layer only. The workbench copy now explains the handoff path more directly and surfaces the current top blocker plus recommended action using existing `ProjectStatus` data without changing the shared status contract or selection logic.
- Result: targeted Rust tests passed; the workbench remains a thin consumer over `ProjectStatus`.

## 2026-03-30 - Maintenance-mode and consumer-driven reopen guidance
- State: Done
- Scope: `docs/internal/current-capability-inventory-2026-03-30.md`, `docs/internal/current-execution-review-2026-03-29.md`, `docs/internal/project-roadmap.md`
- Baseline: the repo already said all domain producers were stop-for-now, but the current docs still benefited from one clearer statement that the default mode is now maintenance, clarity, and consumer-driven reopen decisions rather than implicit next-lane selection.
- Current Update: added explicit maintenance-mode guidance, a concrete consumer-driven reopen rule, and a narrow list of allowed near-term work that does not reopen a domain lane. The roadmap now also treats docs/help/TUI clarity as the default next work when no domain gap is justified.
- Result: maintainers now have one cleaner current policy: do not assume another deepening pass; prefer stability, contract protection, and thin-consumer polish unless a real consumer proves a missing decision-critical signal.

## 2026-03-29 - Sync staged provider and placeholder review evidence
- State: Done
- Scope: `docs/internal/current-capability-inventory-2026-03-30.md`, `docs/overview-rust.md`, `docs/internal/README.md`
- Baseline: the repo already had roadmap, execution-review, and architecture notes, but it still lacked one current-only maintainer document that answers what the project can do today, which command surfaces are practically mature, and which areas should not be expanded by default.
- Current Update: added a current capability inventory that summarizes the Rust-mainline operator toolkit by domain, explains the practical role of `overview` and `project-status`, and makes the current stop-for-now posture explicit without reopening any domain lane. Maintainer doc routing now points to this inventory from the Rust overview and internal docs index.
- Result: maintainers now have one current snapshot for capability-oriented orientation instead of reconstructing the answer from roadmap, review, and trace documents.

## 2026-03-29 - Sync staged provider and placeholder review evidence
- State: Done
- Scope: `rust/src/sync/project_status.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the staged sync producer already preserved provider and secret-placeholder blocking evidence, but if those assessments were present and reviewable without blockers, the sync row dropped them entirely and only surfaced alert-artifact review evidence.
- Current Update: kept the work bounded to the sync-owned staged producer and added conservative `provider-review` and `secret-placeholder-review` warnings based on existing bundle-preflight assessment plans when those assessments are present without blocking findings.
- Result: sync now keeps more of the existing trust-chain evidence visible for operator review without widening staged inputs or moving more logic into shared consumers.

## 2026-03-29 - Project-level stop/continue decision refresh
- State: Done
- Scope: `docs/internal/current-execution-review-2026-03-29.md`, `docs/internal/domain-producer-maturity-review-2026-03-29.md`, `docs/internal/next-phase-execution-plan-2026-03-29.md`, `docs/internal/project-status-producer-gap-list.md`
- Baseline: after the latest bounded passes, the maintainer docs still treated `dashboard` and `datasource` as active depth lanes even though their staged/live producers now looked much closer to stop-if-good-enough, while `sync` / `promotion` remained the only lane with clearly open trust-chain depth.
- Current Update: refreshed the project-level review so `dashboard`, `datasource`, `alert`, and `access` are all treated as stop-for-now unless a concrete consumer proves a missing decision-critical signal. The current pass now keeps only `sync` / `promotion` as the remaining active depth lane, and the supporting maturity, gap, and execution-plan notes were aligned to the same decision.
- Result: maintainers now have one consistent project-level answer: stop reopening mature lanes by default, keep `overview` / `project-status` thin, and treat sync/promotion trust evidence as the only remaining active bounded follow-through path.

## 2026-03-29 - Sync/promotion trust-chain evidence follow-through
- State: Done
- Scope: `rust/src/sync/project_status.rs`, `rust/src/sync/project_status_promotion.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the staged sync and promotion producers already carried blocker and handoff/apply evidence, but sync still dropped alert-artifact presence when that surface was reviewable but neither blocked nor plan-only, and promotion still flattened resolved continuation evidence down to a count of `1` even when the staged continuation reported a larger resolved inventory.
- Current Update: kept the work bounded to the sync-owned staged producers. Sync now emits a conservative `alert-artifact-review` warning when staged alert artifacts are present without blocked or plan-only findings, and promotion now keeps `continuationSummary.resolvedCount` as a real evidence count when that field is the chosen apply-continuation source.
- Result: the staged trust chain now holds onto two more existing evidence signals without widening sync/promotion inputs or moving logic back into shared consumers.

## 2026-03-29 - Datasource staged import-blocker promotion
- State: Done
- Scope: `rust/src/datasource_project_status.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the staged datasource producer already surfaced import-preview mutation counts, but `wouldBlock` still showed up only as a warning, which made the datasource row look `ready` even when the staged import preview was explicitly blocked.
- Current Update: kept the work inside the datasource-owned staged producer and promoted import-preview `wouldBlock` into a real datasource blocker with `blocked-by-blockers` status and a blocker-first next action, while leaving the rest of the diff/drift and import-preview warning signals unchanged.
- Result: datasource staged status is now closer to the real export/import/sync trust path because explicit import-preview blockers no longer look like just another warning.

## 2026-03-29 - Dashboard staged datasource-coverage readiness follow-through
- State: Done
- Scope: `rust/src/dashboard/project_status.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the staged dashboard producer already used `dashboardDependencies` detail rows for bounded import-readiness evidence, but its stricter detail-ready path still treated a row as structurally ready even when datasource names and datasource-family coverage were missing from that same dependency row.
- Current Update: kept the work inside the dashboard-owned staged producer and tightened the detail-ready predicate so a dependency row now also needs non-empty `datasources` and `datasourceFamilies` arrays before it counts as detail-ready. Focused tests now cover both the earlier panel/query-field gap and the new datasource-coverage gap.
- Result: dashboard staged status now reads the existing dependency contract a little more honestly for import-readiness without adding new inputs, changing shared consumers, or reopening a broader dashboard depth lane.

## 2026-03-29 - Alert staged readiness closure and stop review
- State: Done
- Scope: `rust/src/alert_project_status.rs`, `docs/internal/domain-producer-maturity-review-2026-03-29.md`, `docs/internal/next-phase-execution-plan-2026-03-29.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the staged alert producer already exposed export-summary presence and supporting-surface warnings, but its conservative readiness coverage still stopped short of flagging the two most obvious promotion prerequisites from the existing summary contract, and the maintainer docs still treated alert as an active depth lane.
- Current Update: kept the code bounded to the alert-owned staged producer and reused only the five stable staged summary counts. The producer now warns when staged alert rules exist without any contact points or notification policies, and the maintainer review/execution docs now mark alert as stop-for-now after this bounded pass instead of keeping it in the active deepening lanes.
- Result: alert staging remains document-driven and conservative, but it now carries clearer promotion-readiness coverage from the existing export summary shape while the project-level docs narrow the active follow-through lanes back down to dashboard, datasource, and sync/promotion.

## 2026-03-29 - Sync/promotion staged trust-chain evidence follow-through
- State: Done
- Scope: `rust/src/sync/project_status.rs`, `rust/src/sync/project_status_promotion.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the sync and promotion staged producers already carried bundle/preflight/handoff evidence, but some real blocker and handoff signals still depended on top-level summary fields even when the nested staged assessments carried the more direct evidence.
- Current Update: kept the work bounded to the owning modules and taught the sync producer to fall back to nested `secretPlaceholderAssessment.summary.blockingCount` when the top-level placeholder blocker count is absent. The promotion producer now also treats `handoffSummary.nextStage` and `continuationSummary.nextStage` as valid staged evidence when explicit instruction text is missing, instead of dropping back to a more generic readiness source.
- Result: `sync` and `promotion` stay document-driven and review-first, but their staged trust-chain rows now hold onto the most direct existing evidence instead of collapsing back to weaker summary-only attribution.

## 2026-03-29 - Bounded producer follow-through note
- State: Done
- Scope: `docs/internal/domain-producer-maturity-review-2026-03-29.md`, `docs/internal/project-status-producer-gap-list.md`, `docs/internal/next-phase-execution-plan-2026-03-29.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the maintainer docs still described dashboard, datasource, sync/promotion, and alert depth as queued next work.
- Current Update: rewrote the maturity review, gap list, and next-phase note so those lanes are framed as bounded mainline follow-through inside their owning domains, with access explicitly treated as stop-for-now and only the residual depth items left as current follow-up.
- Result: the internal docs now read as current-only snapshots instead of a stale queue or a new architecture branch.

## 2026-03-29 - Domain producer maturity review
- State: Done
- Scope: `docs/internal/domain-producer-maturity-review-2026-03-29.md`, `docs/overview-rust.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the repo already had a gap list and an execution-order note, but it still lacked one explicit maintainer review that says which domain producers are already good enough to stop, which still justify one more bounded round, and what practical boundary `overview` and `project-status` should keep.
- Current Update: added a dedicated maturity review that classifies dashboard, datasource, alert, access, sync, and promotion into `stop` versus `one-more-round`, records the residual bounded gap for each one, and fixes `overview` / `project-status` as thin practical consumers rather than expanding owners.
- Result: maintainers now have one current note for deciding where to stop polishing, where one more bounded pass is still justified, and which consumer surfaces should stay thin.

## 2026-03-29 - Dashboard staged import-readiness detail gaps
- State: Done
- Scope: `rust/src/dashboard/project_status.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the staged dashboard producer already carried blocker rows, governance warning rows, and one aggregate `import-readiness-gaps` signal from `dashboardDependencies`, but the import-readiness path still stopped at count-level evidence while blocker and governance paths already had detail-backed attribution.
- Current Update: kept the existing summary-backed gap signal, then added a second bounded import-readiness warning that uses richer `dashboardDependencies` rows and flags detail gaps when a dependency row has a file but lacks the structural evidence needed to look import-ready.
- Result: dashboard status now carries one more document-driven import-readiness layer without expanding the live path, adding a new summary surface, or moving logic back into `overview`.

## 2026-03-29 - Domain live-status depth pass
- State: Done
- Scope: `rust/src/dashboard/live_project_status.rs`, `rust/src/datasource_live_project_status.rs`, `rust/src/alert_live_project_status.rs`, `rust/src/access/live_project_status.rs`, `rust/src/sync/live_project_status_sync.rs`, `rust/src/sync/live_project_status_promotion.rs`, `rust/src/project_status_freshness.rs`, `rust/src/project_status_tui.rs`, `rust/src/project_status_tui_render.rs`, `rust/tests/project_status_tui_rust_tests.rs`, `docs/internal/project-status-producer-gap-list.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `project-status live` already existed and could aggregate all domains, but most live rows were still shallow first-pass counts, TUI handoff copy was generic, and freshness only had source-count stamping.
- Current Update: deepened domain-owned live signals without changing the top-level product shape or adding a new project-status branch. Dashboard now surfaces folder-spread and root-scope warnings plus a bounded live import/dry-run readiness follow-up inside the existing project-status live path; datasource adds name/access/org-scope drift signals plus a bounded live provider/placeholder readiness follow-up; alert keeps a bounded live follow-up for import, diff, and promotion readiness behind rule linkage; access groups import-review and drift-severity signals; sync can fall back to generic bundle `blockedCount` and `planOnlyCount`; promotion surfaces staged handoff and apply-continuation evidence; freshness gained an additive timestamp-aware sample path, with broader real-source timestamp freshness left as a bounded follow-up inside the same project-status path; and the project-status TUI now states the recommended domain/action handoff more explicitly.
- Result: the project still behaves like an inventory/live inspection and analysis-output tool, not a hidden orchestration layer, but the live project-status path is more decision-useful while staying bounded to existing domain-owned live/readiness surfaces.

## 2026-03-29 - Current execution arrangement
- State: Done
- Scope: `docs/internal/next-phase-execution-plan-2026-03-29.md`, `docs/overview-rust.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the repo already had the architecture note, gap list, and backlog, but it still lacked one short maintainer-facing arrangement that says what is now considered mainline, what remains bounded follow-up, what not to do next, and how `overview` and `project-status` should stay practical without becoming workflow owners.
- Current Update: added one current execution-plan note that keeps `dashboard -> datasource -> sync/promotion trust -> alert` as the bounded order, keeps domain-owned status producers as the mainline contract, and explicitly positions `overview` as the staged artifact aggregator while `project-status` remains the project-wide status surface above shared contract consumers.
- Result: the current pass is easier to execute without drifting into top-level framework work or single-signal polishing, and the practical role of `overview` and `project-status` is explicit for maintainers.

## 2026-03-29 - Project-status live deepening and dedicated project-home TUI
- State: Done
- Scope: `rust/src/project_status_command.rs`, `rust/src/project_status_freshness.rs`, `rust/src/project_status_tui.rs`, `rust/src/project_status_tui_render.rs`, `rust/src/dashboard/live_project_status.rs`, `rust/src/datasource_live_project_status.rs`, `rust/src/alert_live_project_status.rs`, `rust/src/access/live_project_status.rs`, `rust/src/sync/live_project_status.rs`, `rust/src/sync/live_project_status_sync.rs`, `rust/src/sync/live_project_status_promotion.rs`, `rust/src/sync/mod.rs`, `rust/src/lib.rs`, `rust/src/cli_rust_tests.rs`, `rust/tests/project_status_tui_rust_tests.rs`, `docs/internal/project-status-architecture.md`, `docs/internal/project-status-producer-gap-list.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the shared `project-status` command and first-pass live path already existed, but sync and promotion still fell back to transport-only `unknown` rows, live freshness stayed implicit, and the dedicated project-home TUI lived only as an unwired worker slice.
- Current Update: wired deeper live domain producers into the shared command path, added optional staged summary/preflight/mapping/availability inputs so live sync and promotion can emit conservative staged-backed readiness instead of always returning `unknown`, stamped live domain and overall freshness through the shared freshness helper, and connected the dedicated `project-status` interactive workbench as a first-class output surface.
- Result: project-wide status is now less overview-dependent and more operationally useful. `grafana-util project-status live` can consume deeper cross-domain evidence, sync/promotion live rows can move past transport-only placeholders when staged handoff data exists, and the project-home TUI is now a real consumer of the shared contract instead of a disconnected prototype.

## 2026-03-29 - Project status staged rollout and project-home TUI
- State: Done
- Scope: `rust/src/project_status.rs`, `rust/src/dashboard/project_status.rs`, `rust/src/datasource_project_status.rs`, `rust/src/alert_project_status.rs`, `rust/src/access/project_status.rs`, `rust/src/sync/project_status.rs`, `rust/src/sync/project_status_promotion.rs`, `rust/src/overview_document.rs`, `rust/src/overview_tui.rs`, `rust/src/overview_tui_render.rs`, `rust/src/overview_rust_tests.rs`, `docs/internal/project-status-producer-gap-list.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: project-wide staged status support was split across a shared contract plus only a few domain-owned producers, there were no ranked project-level blockers or freshness fields, and the overview interactive flow still began directly in the section browser.
- Current Update: landed staged domain-owned producers across dashboard, datasource, alert, access, sync, and promotion; extended the shared project-status contract with project/domain freshness plus ranked `topBlockers` and `nextActions`; taught overview aggregation and text rendering to consume that shared status layer; and added a `Project Home` landing plus action handoff flow on top of the existing overview workbench.
- Result: the Rust mainline now has a real staged project-status surface above any single domain command. Domain status ownership lives with each domain, overview acts as a consumer/aggregator, and the TUI now opens on project-level status and handoff instead of dropping users straight into an artifact browser.

## 2026-03-29 - Project-status command and first-pass live path
- State: Done
- Scope: `rust/src/project_status.rs`, `rust/src/project_status_command.rs`, `rust/src/dashboard/live_project_status.rs`, `rust/src/datasource_live_project_status.rs`, `rust/src/alert_live_project_status.rs`, `rust/src/access/live_project_status.rs`, `rust/src/sync/live_project_status.rs`, `rust/src/overview_document.rs`, `rust/src/cli.rs`, `rust/src/cli_rust_tests.rs`, `rust/src/project_status_cli_rust_tests.rs`, `docs/internal/project-status-architecture.md`, `docs/internal/project-status-producer-gap-list.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: staged project-status had already landed through shared contract plus domain-owned staged producers, but there was still no dedicated top-level `project-status` command and no live path beyond design notes and gap-list planning.
- Current Update: added a top-level `grafana-util project-status {staged,live}` command, moved project-level aggregation helpers into the shared `project_status` module so staged and live consumers use the same overall/top-blocker/next-action logic, kept `overview` as a consumer instead of the owner, and landed first-pass live domain producers for dashboard, datasource, alert, access, sync, and promotion. Dashboard/datasource/alert/access now read conservative live surfaces, while sync and promotion stay explicit `unknown` live rows that direct operators back to staged inputs when live-only transport is not enough to judge readiness.
- Result: the Rust mainline now has an explicit staged/live split at the command layer, not only in architecture docs. `project-status` is now a separate cross-domain consumer, `overview` remains one consumer among several, and the first live project-wide path is available without collapsing staged and live semantics into one undocumented heuristic.

## 2026-03-29 - Access domain status producer
- State: Done
- Scope: `rust/src/access/project_status.rs`, `rust/src/access/mod.rs`, `rust/src/overview_document.rs`, `rust/src/overview_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: access overview status was still assembled inside `overview_document.rs` from a generic staged artifact summary, so access-specific bundle presence and per-bundle attribution were not owned by the access domain itself.
- Current Update: added an access-owned staged export-bundle status producer that aggregates users, teams, orgs, and service-accounts bundle summaries into one shared `ProjectDomainStatus`, then rewired overview aggregation to call it and added focused coverage for the access domain row.
- Result: access now has the same domain-owned status pattern as sync and dashboard. Missing bundle kinds are represented as partial status with `missing-bundle-kind` warnings, a bundle-specific next-action string, and source attribution only for the bundles that are actually present.

## 2026-03-29 - Dashboard governance warning rows
- State: Done
- Scope: `rust/src/dashboard/project_status.rs`, `rust/src/overview_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the dashboard domain-status producer only surfaced blocker counts from the staged inspect summary and always emitted empty warning fields, so overview had no dashboard-owned way to report governance-like summary signals.
- Current Update: added conservative staged warning rows for `riskRecordCount`, `highBlastRadiusDatasourceCount`, `queryAuditCount`, and `dashboardAuditCount`, preserved blocker-first behavior, and added a focused regression covering both summary-only and governance-enriched inputs.
- Result: the dashboard producer now emits `warningCount`, `warnings`, and conservative `nextActions` from its own summary inputs without pushing that inference into overview.

## 2026-03-29 - Overview architecture hardening
- State: Done
- Scope: `rust/src/overview.rs`, `rust/src/overview_kind.rs`, `rust/src/overview_summary_projection.rs`, `rust/src/overview_document.rs`, `rust/src/overview_sections.rs`, `rust/src/overview_section_rows.rs`, `rust/src/overview_tui.rs`, `rust/src/overview_tui_render.rs`, `rust/src/overview_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the Rust `overview` stack already has clear top-level module boundaries, but text rendering still depends on section projection, artifact-kind rules are repeated across multiple modules, and the project-status / section-row builders are starting to concentrate too much domain-specific branching into single files.
- Current Update: centralized cross-cutting artifact-kind rules in `overview_kind.rs`, moved summary-fact projection into `overview_summary_projection.rs`, moved overview row builders into `overview_section_rows.rs`, kept `overview_sections.rs` focused on generic section assembly plus summary-item projection, split TUI frame rendering into `overview_tui_render.rs`, and refactored `overview_document.rs` so text rendering now consumes artifact-level summary items while project-status derivation stays in one file through ordered per-domain builder helpers and a small local spec layer for repeated literals.
- Result: the overview stack is still traceable as `args -> artifacts -> document -> text/json/tui`, but the main drift risks are lower: text output no longer back-references section views, supported artifact-kind logic is less scattered, summary projection no longer competes with kind-registry ownership, section-row mapping is no longer mixed into generic assembly, and the TUI file no longer mixes render/layout code with state/event dispatch.

## 2026-03-29 - Shared project-status contract and sync producer
- State: Done
- Scope: `rust/src/project_status.rs`, `rust/src/sync/project_status.rs`, `rust/src/sync/mod.rs`, `rust/src/overview.rs`, `rust/src/overview_document.rs`, `rust/src/overview_rust_tests.rs`, `rust/src/lib.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the staged `projectStatus` shape still lived under `overview`, and the sync row was only a local `overview_document.rs` summary fallback over `sync-summary` while `bundle-preflight` appeared as a separate pseudo-domain.
- Current Update: promoted the reusable project-status contract into a crate-level module, then added the first domain-owned producer in `sync/project_status.rs`. The sync domain row now reuses `sync-summary` plus `bundle-preflight` evidence, carries shared `scope` / `mode` / `warningCount` / `warnings` fields, and replaces the earlier `bundle-preflight` pseudo-domain in overview aggregation.
- Result: project status is now less `overview`-owned, and the first real domain producer lives with its owning workflow. `overview` remains a consumer, while sync status becomes a reusable staged-domain surface instead of a local row builder plus a separate workflow-shaped domain entry.

## 2026-03-29 - Dashboard domain status producer
- State: Done
- Scope: `rust/src/dashboard/project_status.rs`, `rust/src/dashboard/mod.rs`, `rust/src/overview_document.rs`, `rust/src/overview_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: after sync moved to a domain-owned producer, the dashboard row was still assembled directly inside `overview_document.rs` from staged inspect summary keys, so the project-status ownership split was only partial.
- Current Update: added a dashboard-owned status producer in `dashboard/project_status.rs` and rewired overview aggregation to consume it. The first pass stays conservative and reuses the existing inspect summary signals for `dashboardCount`, `queryCount`, `orphanedDatasourceCount`, and `mixedDatasourceDashboardCount` without introducing new governance heuristics yet.
- Result: dashboard now follows the same architectural pattern as sync: domain-owned staged status first, overview consumer second. The visible status semantics stay stable while ownership moves to the domain boundary that should evolve them later.

## 2026-03-29 - Project status architecture mini-spec
- State: Done
- Scope: `docs/internal/project-status-architecture.md`, `docs/overview-rust.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the repo already had a staged `overview` command and an `overview`-specific architecture note, but there was still no explicit top-level design for project-wide progress visibility that was broader than the `overview` subsystem itself.
- Current Update: added a dedicated project-status architecture mini-spec above the `overview` layer. It defines domain status producers, a shared project-status contract, staged vs live separation, and the intended TUI navigation model of `project home -> domain drill-down -> action handoff`, then linked the crate-level maintainer guide to that higher-level design note.
- Result: maintainers now have a clear architectural rule that project-wide status support belongs to a shared cross-domain status model with multiple consumers, not to an ever-growing `overview` command.

## 2026-03-29 - Project status producer gap list
- State: Done
- Scope: `docs/internal/project-status-producer-gap-list.md`, `docs/internal/project-status-architecture.md`, `docs/overview-rust.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the new project-status architecture defined the target layers and TUI model, but it did not yet translate the current Rust codebase into a per-domain execution map of what was already landed versus what was still missing.
- Current Update: added a maintainer gap list that inventories the current status-like producers for dashboard, datasource, alert, access, sync, and promotion, marks each domain as landed/partial/missing, and records the shortest safe next step for turning each domain into a reusable status producer.
- Result: maintainers now have a concrete domain-by-domain implementation order for whole-project progress visibility instead of only a high-level architecture note.

## 2026-03-28 - Rust project overview command
- State: Done
- Scope: `rust/src/overview.rs`, `rust/src/overview_rust_tests.rs`, `rust/src/cli.rs`, `rust/src/cli_rust_tests.rs`, `rust/src/lib.rs`, `rust/src/dashboard/mod.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the Rust CLI had strong dashboard inspect and sync staging surfaces, plus stable datasource, alert, and access export roots, but there was no single top-level entrypoint that could summarize those staged artifacts in one pass.
- Current Update: expanded `grafana-util overview` into a stable artifact source plus UI projection and wired that projection into an interactive workbench. It still reuses dashboard inspect summary, datasource export inventory, alert export root index, access export bundles, sync summary, sync bundle preflight, and sync promotion preflight builders/contracts, but the overview document now also emits a staged-only `projectStatus` contract with overall status plus per-domain readiness/blocker rows, derivation metadata such as `reasonCode`/`sourceKinds`/`signalKeys`, and deterministic `nextActions`, keeps the minimal section model aligned with the existing workbench/browser vocabulary, lets the CLI browse it through `--output interactive` when built with `tui`, exposes richer secondary item views such as datasource inventory rows, alert asset rows, access roster rows, sync resource rows, promotion check rows, and bundle assessment rows, keeps the internals split across `rust/src/overview.rs`, `rust/src/overview_document.rs`, `rust/src/overview_artifacts.rs`, `rust/src/overview_sections.rs`, and `rust/src/overview_support.rs`, and now anchors that split in `docs/internal/overview-architecture.md` so maintainers can follow one stable entrypoint-to-output path instead of reverse-engineering the module graph.
- Result: operators can now use `overview` not just as an artifact browser but as a staged project-status surface that summarizes which domains are ready, partial, or blocked, why they landed there, and what the next deterministic action is, while maintainers now have clearer internal boundaries between overview orchestration, document/render logic, artifact assembly, UI projection, and shared support helpers.

## 2026-03-28 - Governance gate high blast radius policy
- State: Done
- Scope: `rust/src/dashboard/governance_gate_rules.rs`, `rust/src/dashboard/governance_gate_rules_policy.rs`, `rust/src/dashboard/governance_gate_rules_evaluation_apply.rs`, `rust/src/dashboard/export_focus_governance_rule_datasource_routing_complexity_rust_tests.rs`, `rust/src/dashboard/export_focus_governance_rule_threshold_audit_cost_rust_tests.rs`, `rust/src/dashboard/export_focus_governance_output_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: inspect could already flag `highBlastRadius` datasources, but governance gate policy had no way to reject that concentration during CI or promotion checks.
- Current Update: added `datasources.forbidHighBlastRadius` to governance gate policy parsing, checked-rules output, and evaluator logic against `datasourceGovernance`.
- Result: governance gate can now fail when inspect artifacts contain a datasource marked `highBlastRadius`, so dependency concentration is enforceable instead of advisory-only.

## 2026-03-28 - Datasource concentration threshold
- State: Done
- Scope: `rust/src/dashboard/inspect_governance_rows.rs`, `rust/src/dashboard/inspect_governance_coverage.rs`, `rust/src/dashboard/inspect_governance_document.rs`, `rust/src/dashboard/inspect_governance_render.rs`, `rust/src/dashboard/inspect_workbench_content.rs`, `rust/src/dashboard_inspection_dependency_contract.rs`, `rust/src/dashboard/inspect_dependency_render.rs`, `rust/src/dashboard/inspect_governance_document_rust_tests.rs`, `rust/src/dashboard/inspect_live_tui_rust_tests.rs`, `rust/src/dashboard/inspect_workbench_state.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: inspect could already show folder and dashboard blast-radius facts per datasource, but operators still had to decide by eye whether that concentration was risky enough to flag.
- Current Update: added a conservative `highBlastRadius` threshold to datasource governance and dependency usage, then surfaced the count in governance summary and the boolean in governance/dependency/TUI outputs.
- Result: shared datasources that fan out broadly are now flagged explicitly in inspect output, so governance and cleanup work can focus on concentration hotspots instead of scanning raw counts manually.

## 2026-03-28 - Datasource blast-radius outputs
- State: Done
- Scope: `rust/src/dashboard/inspect_governance_rows.rs`, `rust/src/dashboard/inspect_governance_coverage.rs`, `rust/src/dashboard/inspect_governance_render.rs`, `rust/src/dashboard/inspect_workbench_content.rs`, `rust/src/dashboard_reference_models.rs`, `rust/src/dashboard_inspection_dependency_contract.rs`, `rust/src/dashboard/inspect_dependency_render.rs`, `rust/src/dashboard/inspect_governance_document_rust_tests.rs`, `rust/src/dashboard/inspect_live_tui_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: dashboard inspect already counted datasource usage and surfaced some governance risks, but datasource-level blast radius still required reading raw dashboard UID lists and inferring cross-folder concentration by hand.
- Current Update: promoted folder and dashboard blast-radius signals into the shared datasource governance and dependency usage models, then threaded those fields through JSON, table, and workbench renderers.
- Result: inspect outputs now show datasource folder count, cross-folder status, folder paths, and dashboard titles directly, so operators can see dependency concentration and likely blast radius without post-processing the raw report rows.

## 2026-03-28 - Loki pipeline field hints
- State: Done
- Scope: `rust/src/dashboard/inspect_analyzer_loki.rs`, `rust/src/dashboard/export_focus_report_query_family_analysis_rust_tests.rs`, `rust/src/dashboard/export_focus_report_query_family_extraction_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Loki query inspection already captures stream selectors, label matchers, line-filter hints, and range windows, but parsed pipeline fields such as `status`, `level`, or `duration_ms` are still invisible in the shared query report rows.
- Current Update: constrained selector label extraction to actual stream selectors, then taught the Loki analyzer to surface obvious pipeline predicate fields and `unwrap` targets as shared inspection measurements.
- Result: Loki query rows now expose parser-derived field hints such as `status`, `level`, and `duration_ms` through the same shared inspection contract used by dependency and governance outputs, without changing report/render wiring.

## 2026-03-28 - Rust mainline status refresh
- State: Active
- Scope: `rust/src/dashboard/*`, `rust/src/sync/*`, `rust/src/datasource*`, `rust/src/access/*`, `docs/internal/project-roadmap.md`, `docs/internal/maintainer-backlog-2026-03-28.md`
- Baseline: the Rust mainline had already absorbed the operator workflows, but the live status file still carried a rolling task log.
- Current Update: condensed the live view to the current Rust-only mainline. Dashboard, sync, datasource, and access surfaces are still the active runtime; the TUI shell grammar is shared across the main console surfaces; sync promotion now exposes a staged review handoff; datasource secret handling is fail-closed and placeholder-based; and the maintainer backlog now owns the next Rust-only follow-ups.
- Result: the live status file now reads as a current maintainer snapshot instead of a detailed activity log.

## 2026-03-28 - Current follow-up
- State: Planned
- Scope: `docs/internal/project-roadmap.md`, `docs/internal/maintainer-backlog-2026-03-28.md`, `TODO.md`
- Next Step: keep the next round focused on dashboard boundary cleanup, inspection/governance depth, selective crate-boundary cleanup, sync/promotion trust, and datasource secret handling.
## 2026-03-29 - Project-level stop/continue closeout
- State: Done
- Scope: `docs/internal/current-execution-review-2026-03-29.md`, `docs/internal/domain-producer-maturity-review-2026-03-29.md`, `docs/internal/next-phase-execution-plan-2026-03-29.md`, `docs/internal/project-status-producer-gap-list.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: after the final bounded sync/promotion follow-through, the maintainer docs still treated `sync` / `promotion` as the last active depth lane even though their remaining gaps had narrowed to trust-evidence polish rather than missing operator decisions.
- Current Update: reran the project-level stop/continue review and closed the pass with no default depth lane remaining. All six domain-owned producers are now treated as stop-for-now unless a concrete consumer proves a missing decision-critical signal, and `overview` / `project-status` stay thin practical consumers.
- Result: maintainers now have a single current answer for this pass: stop producer deepening by default, protect the existing domain-owned contracts, and only reopen a lane from concrete downstream pressure instead of speculative polishing.
## 2026-03-29 - Roadmap and backlog alignment after producer closeout
- State: Done
- Scope: `docs/internal/project-roadmap.md`, `docs/internal/maintainer-backlog-2026-03-28.md`, `TODO.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the project-level stop/continue review had already closed all domain depth lanes by default, but the roadmap, maintainer backlog, and root TODO still carried older assumptions that dashboard, datasource, and sync/promotion were the active next execution lanes.
- Current Update: aligned the roadmap, maintainer backlog, and active TODO to the current execution stance. They now emphasize stability, consumer-proven reopen criteria, thin `overview` / `project-status` boundaries, and exploratory-only treatment for broader analysis or packaging ideas.
- Result: the repo now has one current maintainer answer across the roadmap stack instead of a closed stop/continue review sitting next to older active-lane queues.
## 2026-03-30 - Shared TUI presentation pass for pane-based interactive surfaces
- State: Done
- Scope: `rust/src/tui_shell.rs`, `rust/src/overview_tui_render.rs`, `rust/src/project_status_tui_render.rs`, `rust/src/interactive_browser.rs`, `rust/src/dashboard/topology_tui.rs`, `rust/src/dashboard/impact_tui.rs`, `rust/src/dashboard/governance_gate_tui.rs`, `rust/src/dashboard/inspect_workbench_render.rs`, `rust/src/dashboard/inspect_orchestration.rs`, `rust/src/dashboard/inspect_live.rs`, `rust/src/sync/audit_tui.rs`, `rust/src/sync/review_tui_helpers.rs`, related focused Rust tests
- Baseline: pane-based interactive screens had drifted apart in presentation. `overview` still rendered a redundant upper block, footer controls were unevenly spaced, and focus state was not consistently highlighted across the tab-driven TUI surfaces.
- Current Update: collapsed `overview` to a single top summary block, changed shared control rows to fixed-width aligned cells, and standardized focus visibility with blue focus chips across the shared/major pane-based interactive views. Footer copy was also shortened to clearer operator-oriented action labels. `dashboard inspect-export --interactive` now also has a focused render smoke test, the inspect workbench shell follows the same visible header/footer treatment instead of an older ad hoc layout, and interactive export/live inspect now print explicit pre-workbench progress steps while summary/query/governance analysis is still running.
- Result: `overview`, `project-status`, shared browser-style screens, inspect workbench, and the main dashboard/sync review panes now read as the same TUI family instead of separate hand-built layouts, and long-running inspect interactive launches no longer fail silently before the first frame.

## 2026-03-30 - Alert scaffold naming and live operator examples
- State: Done
- Scope: `rust/src/alert.rs`, `rust/src/alert_support.rs`, `rust/src/alert_runtime_support.rs`, `rust/src/alert_rust_tests.rs`, `docs/user-guide.md`, `docs/user-guide-TW.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the new alert management lane existed, but scaffolded files still used generic `example-*` identities regardless of `--name`, and the user guides only described the workflow abstractly instead of showing a validated end-to-end operator path.
- Current Update: scaffold generation now carries `--name` into alert rule, contact point, and template identity fields. The alert guides now include Docker-validated Grafana `12.4.1` examples for `export`, `import --dry-run`, `plan`, `apply`, and prune-based delete planning/apply, including explicit add-rule and delete-rule walkthroughs.
- Result: the management lane now starts from correctly named scaffolds, and operators have a concrete full example that covers both migration-style replay and desired-state add/delete flows.

## 2026-03-30 - Alert authoring layer and managed route overwrite contract
- State: Done
- Scope: `rust/src/alert.rs`, `rust/src/alert_cli_defs.rs`, `rust/src/alert_support.rs`, `rust/src/alert_runtime_support.rs`, `rust/src/alert_rust_tests.rs`, `rust/src/cli_help_examples.rs`, `rust/src/cli_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: alert V2 planning had settled on a file-first authoring layer, but the runtime still lacked wired `add-rule` / `clone-rule` / `add-contact-point` / `set-route` / `preview-route` behavior, and the boundary between desired-file authoring versus live Grafana mutation was still easy to blur.
- Current Update: wired the new authoring commands as desired-state writers only. `add-rule` now authors simple threshold/classic-condition rule documents plus optional managed-route updates, `clone-rule` rewrites staged rule files without inventing a second schema, `add-contact-point` writes scaffolded desired documents, and `set-route` / `preview-route` operate only on the managed notification-policy lane. Re-running `set-route` now explicitly overwrites the same tool-owned route instead of attempting field-by-field merges, and all authoring surfaces support `--dry-run`.
- Result: alert V2 now has a concrete authoring layer that feeds the existing `plan/apply` engine without calling Grafana APIs directly, while keeping the managed route contract idempotent and bounded.
