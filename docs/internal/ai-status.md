# ai-status.md

Current AI-maintained status only.

- Older trace history moved to [`archive/ai-status-archive-2026-03-24.md`](docs/internal/archive/ai-status-archive-2026-03-24.md).
- Detailed 2026-03-27 entries moved to [`archive/ai-status-archive-2026-03-27.md`](docs/internal/archive/ai-status-archive-2026-03-27.md).
- Detailed 2026-03-28 task notes were condensed into [`archive/ai-status-archive-2026-03-28.md`](docs/internal/archive/ai-status-archive-2026-03-28.md).
- Detailed 2026-03-29 through 2026-03-31 entries moved to [`archive/ai-status-archive-2026-03-31.md`](docs/internal/archive/ai-status-archive-2026-03-31.md).
- Detailed 2026-04-01 through 2026-04-12 entries moved to [`archive/ai-status-archive-2026-04-12.md`](docs/internal/archive/ai-status-archive-2026-04-12.md).
- Keep this file short and current. Additive historical detail belongs in `docs/internal/archive/`.
- Older entries moved to [`ai-status-archive-2026-04-13.md`](docs/internal/archive/ai-status-archive-2026-04-13.md).
- Older entries moved to [`ai-status-archive-2026-04-14.md`](docs/internal/archive/ai-status-archive-2026-04-14.md).
- Older entries moved to [`ai-status-archive-2026-04-15.md`](docs/internal/archive/ai-status-archive-2026-04-15.md).
- Older entries moved to [`ai-status-archive-2026-04-16.md`](docs/internal/archive/ai-status-archive-2026-04-16.md).
- Older entries moved to [`ai-status-archive-2026-04-17.md`](docs/internal/archive/ai-status-archive-2026-04-17.md).
- Older entries moved to [`ai-status-archive-2026-04-18.md`](docs/internal/archive/ai-status-archive-2026-04-18.md).
- Older entries moved to [`ai-status-archive-2026-04-19.md`](docs/internal/archive/ai-status-archive-2026-04-19.md).

## 2026-04-18 - Fix Rust 1.95 sync review clippy failure
- State: Done
- Scope: Rust sync review TUI key handling, CI failure analysis, focused sync tests, full Rust test, clippy, formatting, architecture gate, and AI trace docs. Public CLI behavior, generated docs, README files, JSON contracts, and Python implementation are out of scope.
- Baseline: GitHub Actions `rust-quality` passed cargo tests on commit `8a6b7d6b`, then failed under Rust 1.95 clippy because nested `if diff_mode` checks inside `review_tui` key handling triggered the new `collapsible_match` lint.
- Current Update: Collapsed the nested diff-mode checks into guarded `match key.code` arms while preserving the same checklist and diff-view key behavior.
- Result: Focused sync tests, full Rust tests, local clippy, formatting, and architecture checks pass. CI must rerun on pushed commits to verify the Rust 1.95 lint gate.

## 2026-04-19 - Clarify contract ownership map
- State: Done
- Scope: `docs/internal/contract-doc-map.md`, contract registry routing notes, and trace docs. Runtime JSON output, schema manifests, public CLI behavior, generated docs, README files, and Python implementation are out of scope.
- Current Update: Clarified the boundary between runtime golden output contracts, CLI/docs routing contracts, docs-entrypoint navigation, and schema/help manifests so the maintainer map now names the source of truth for each layer explicitly.
- Result: The contract map now distinguishes `command-surface.json`, `docs-entrypoints.json`, `output-contracts.json`, and `schemas/manifests/` as separate ownership surfaces.

## 2026-04-19 - Advance status and review-governance cleanup
- State: Done
- Scope: Rust alert live project-status normalization, TODO backlog cleanup, contract promotion guidance, mutation review-envelope inventory, focused tests, formatting, architecture checks, and AI trace docs. Public CLI behavior, generated docs, README files, and Python implementation are out of scope.
- Current Update: Routed the alert live status producer through the shared status reading model, removed stale completed work from the active backlog, documented runtime-vs-schema promotion rules, and captured an internal review-envelope inventory before any public JSON changes.
- Result: Focused alert/status tests, contract/schema checks, full Rust tests, clippy, formatting, architecture, and AI workflow checks pass locally.

## 2026-04-18 - Split oversized Rust test surfaces
- State: Done
- Scope: Rust test-surface maintainability for sync bundle execution, dashboard export/import/topology, dashboard browse workflow, snapshot, access org runtime, TODO backlog, focused tests, full Rust test, clippy, architecture gate, and AI trace docs. README files, generated user docs, public CLI behavior, JSON contracts, and Python implementation are out of scope.
- Baseline: Several Rust regression files mixed unrelated behavior suites in 900+ line modules, which made review and worker assignment harder even after production architecture warnings were clean.
- Current Update: Split the largest test hubs into behavior-named sibling modules while keeping their original files as routing facades and shared fixture homes where appropriate. Restored the existing access user runtime module include and kept dashboard browse's test-only document builder explicit for clippy.
- Result: Focused sync/dashboard/snapshot/access tests pass, full Rust tests pass, clippy and formatting pass, and `make quality-architecture` remains clean.

## 2026-04-18 - Clear Rust architecture warnings
- State: Done
- Scope: Rust architecture-warning cleanup across dashboard plan/export/export-layout/import-apply, status live, datasource CLI defs, alert runtime tests, focused tests, full Rust quality gate, and AI trace docs. README files, generated user docs, and Python implementation are out of scope.
- Baseline: `make quality-architecture` reported warning-threshold files in dashboard export/export-layout/import_apply/plan, status live, datasource CLI defs, and alert runtime tests. Several production files mixed orchestration, rendering, live collection, artifact writing, and tests in one module.
- Current Update: Split each warning surface by responsibility: dashboard plan into input/reconcile/render, dashboard import/apply into backend/prepare/live/render, dashboard export into provisioning/root-bundle helpers, export-layout into apply/render/tests, status live into discovery/domains/multi-org/tests, datasource CLI output-format helpers, and alert runtime tests by scenario group.
- Result: `make quality-architecture` is clean with zero warnings, focused tests pass, full Rust tests pass, clippy passes, and `make quality-rust` passes outside the sandbox. The sandboxed `make quality-rust` run still fails local mock-server socket tests with `Operation not permitted`.

## 2026-04-18 - Fix datasource plan architecture gate
- State: Done
- Scope: Rust datasource plan module split, shared review/action contract vocabulary, focused tests, architecture quality gate, and AI trace docs. README files, generated user docs, and Python implementation are out of scope.
- Baseline: `make quality-architecture` failed because `rust/src/commands/datasource/plan/mod.rs` was 1065 lines, above the 800-line hard limit; review action/status/reason strings were also spread through datasource/access/dashboard/sync plan, preview, apply, and TUI paths.
- Current Update: Split datasource plan into `model`, `builder`, `render`, and `tests` modules, leaving `mod.rs` as a thin re-export layer. Added a shared review contract vocabulary and routed core plan/preview/apply summary filters through it instead of scattering `would-*`, `same`, `blocked`, `warning`, and related reason strings.
- Result: Focused datasource plan tests, full Rust tests, clippy, formatting, and `make quality-architecture` pass. Remaining architecture output contains warning-threshold files only, with zero hard failures.
