# ai-status.md

Current AI-maintained status only.

- Older trace history moved to [`archive/ai-status-archive-2026-03-24.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-03-24.md).
- Detailed 2026-03-27 entries moved to [`archive/ai-status-archive-2026-03-27.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-03-27.md).
- Keep this file short and current. Additive historical detail belongs in `docs/internal/archive/`.

## 2026-03-27 - Sync staged/live boundary split
- State: Done
- Scope: `rust/src/sync/cli.rs`, `rust/src/sync/live.rs`, `rust/src/sync/live_apply.rs`, `rust/src/sync/live_intent.rs`, `rust/src/sync/staged_documents.rs`, `rust/src/sync/staged_documents_apply.rs`, `rust/src/sync/staged_documents_render.rs`
- Baseline: staged review/apply/preflight helpers and live apply-intent parsing were mixed into broader facade modules, which made sync ownership boundaries harder to trace.
- Current Update: split staged review/apply gating into `staged_documents_apply.rs`, kept `staged_documents_render.rs` focused on rendering and drift display, and moved live apply-intent parsing into `live_intent.rs` so `live_apply.rs` stays request-execution focused.
- Result: the sync CLI now reads through clearer staged-vs-live boundaries without changing staged document contracts or live apply JSON output.

## 2026-03-27 - Sync explainability upgrade
- State: Done
- Scope: `rust/src/sync/blocked_reasons.rs`, `rust/src/sync/staged_documents_apply.rs`, `rust/src/sync/staged_documents_render.rs`, `rust/src/sync/bundle_preflight.rs`, focused sync render/apply tests
- Baseline: sync preflight and bundle-preflight apply rejections mostly surfaced aggregate blocking counts, while the text renderers gave operators limited context on why a plan stayed review-only or why apply stayed blocked.
- Current Update: added a small blocked-reason helper that extracts concrete blocking details from staged check arrays, threaded those reasons into apply rejection messages, and added concise reason lines to plan/apply/bundle-preflight text output.
- Result: operators now see specific blocking causes in sync apply failures and clearer review/apply guidance in the staged text renderers without redesigning the JSON contracts.

## 2026-03-27 - Promotion preflight skeleton
- State: Done
- Scope: `rust/src/sync/promotion_preflight.rs`, `rust/src/sync/cli.rs`, `rust/src/sync/mod.rs`, focused sync help/contract tests
- Baseline: the repo already had source-bundle and bundle-preflight primitives, but there was no first-class staged contract for cross-environment remap visibility before moving into promotion workflows.
- Current Update: added a `sync promotion-preflight` command plus a staged `grafana-utils-sync-promotion-preflight` document that layers folder and datasource remap checks on top of the existing bundle-preflight assessment and optional mapping input.
- Result: maintainers now have a concrete promotion entry point that surfaces direct matches, explicit remaps, missing mappings, and inherited bundle blockers without claiming a full promotion/apply workflow yet.

## 2026-03-27 - Unified CLI help/example source split
- State: Done
- Scope: `rust/src/cli.rs`, `rust/src/cli_help_examples.rs`, `rust/src/lib.rs`, focused unified CLI help tests
- Baseline: the unified CLI help/example strings and color-label table lived as one large block in `rust/src/cli.rs`.
- Current Update: extracted the help/example data into a dedicated helper module while keeping the rendered CLI help paths and command behavior unchanged.
- Result: the unified help source is now split across `rust/src/cli.rs` and `rust/src/cli_help_examples.rs`, and focused help rendering tests passed.

## 2026-03-27 - Dashboard dependency report human-readable output
- State: In Progress
- Scope: `rust/src/dashboard_inspection_dependency_contract.rs`, `rust/src/dashboard/inspect_output.rs`, focused dashboard inspect tests
- Baseline: dependency reporting still rendered `Dependency` and `DependencyJson` through the same pretty-JSON path, and orphaned datasource detail was limited to strings.
- Current Update: enriching the dependency contract with typed usage/orphan rows and splitting the non-JSON dependency renderer into table-style sections.

## 2026-03-27 - Current Maintainer State
- State: Active
- Scope: Rust maintainability cleanup across `dashboard/`, `sync/`, `datasource/`, and `access/`.
- Current Shape:
  - `rust/src/sync/workbench.rs` is now a facade over builder-oriented helpers in `summary_builder.rs`, `bundle_builder.rs`, `plan_builder.rs`, and `apply_builder.rs`.
  - `rust/src/dashboard/import.rs` is now an orchestration layer over `import_lookup.rs`, `import_validation.rs`, `import_render.rs`, `import_compare.rs`, and `import_routed.rs`.
  - Governance rule evaluation lives in `rust/src/dashboard/governance_gate_rules.rs`, with `governance_gate.rs` reduced to command/result orchestration.
  - Recent maintainer work has focused on splitting large orchestration files into smaller helper modules without changing the public CLI or JSON contracts.
  - Large dashboard test coverage has started moving out of `rust/src/dashboard/rust_tests.rs` into feature files such as `inspect_live_rust_tests.rs`, `inspect_query_rust_tests.rs`, `inspect_governance_rust_tests.rs`, `inspect_export_rust_tests.rs`, and `screenshot_rust_tests.rs`.
- Result:
  - Remaining complexity is primarily feature density and contract surface, not missing core architecture direction.
  - The current cleanup theme is to keep facades thin, contracts typed, and feature-specific tests close to the owned behavior.

## 2026-03-27 - Open Follow-Up
- State: Planned
- Scope: `rust/src/dashboard/governance_gate.rs`, related governance gate tests
- Next Step: wire governance-gate runtime evaluation to support JSON, YAML, and built-in policy sources without changing the evaluator contract.
