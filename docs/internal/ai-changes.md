# ai-changes.md

Current AI change log only.

- Older detailed history moved to [`archive/ai-changes-archive-2026-03-24.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-changes-archive-2026-03-24.md).
- Detailed 2026-03-27 entries moved to [`archive/ai-changes-archive-2026-03-27.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-changes-archive-2026-03-27.md).
- Keep this file limited to the latest active architecture and maintenance changes.

## 2026-03-27 - Sync staged/live boundary split
- Summary: split staged review/apply/preflight helper ownership out of `rust/src/sync/staged_documents.rs` into `rust/src/sync/staged_documents_apply.rs`, trimmed `rust/src/sync/staged_documents_render.rs` back to rendering and drift display, and moved live apply-intent parsing from `rust/src/sync/live_apply.rs` into `rust/src/sync/live_intent.rs`.
- Tests: existing sync CLI, staged document, and live-apply coverage were reused; no new behavior-specific tests were needed for this boundary-only refactor.
- Test Run: `cargo fmt --manifest-path rust/Cargo.toml --all --check` passed; `cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings` passed; `cargo test --manifest-path rust/Cargo.toml --quiet sync` passed with 123 sync tests.
- Impact: `rust/src/sync/cli.rs`, `rust/src/sync/live.rs`, `rust/src/sync/live_apply.rs`, `rust/src/sync/live_intent.rs`, `rust/src/sync/mod.rs`, `rust/src/sync/staged_documents.rs`, `rust/src/sync/staged_documents_apply.rs`, `rust/src/sync/staged_documents_render.rs`
- Rollback/Risk: the public sync behavior should remain stable; revert the helper splits if module visibility or staged helper reexports need to be collapsed again.
- Follow-up: none.

## 2026-03-27 - Sync explainability upgrade
- Summary: added `rust/src/sync/blocked_reasons.rs` to pull concrete blocking reasons out of staged preflight and bundle-preflight check arrays, reused it in `staged_documents_apply.rs` for apply rejection messages, and added short operator guidance lines to the sync plan/apply/bundle-preflight text renderers.
- Tests: updated focused sync render and apply regression tests to assert the new reason strings without changing CLI topology or staged JSON payload shapes.
- Test Run: `cargo fmt --manifest-path rust/Cargo.toml --all --check` passed; `cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings` passed; `cargo test --manifest-path rust/Cargo.toml --quiet sync` passed with 123 sync tests.
- Impact: `rust/src/sync/blocked_reasons.rs`, `rust/src/sync/staged_documents_apply.rs`, `rust/src/sync/staged_documents_render.rs`, `rust/src/sync/bundle_preflight.rs`, `rust/src/sync/cli_apply_review_exec_apply_rust_tests.rs`, `rust/src/sync/cli_render_rust_tests.rs`, `rust/src/sync/bundle_contract_preflight_rust_tests.rs`, `rust/src/sync/bundle_exec_rust_tests.rs`
- Rollback/Risk: the change is text-heavy and should not alter sync JSON contracts; revert the helper and focused render assertions if the extra operator guidance proves too noisy.

## 2026-03-27 - Promotion preflight skeleton
- Summary: added a first staged `sync promotion-preflight` workflow around the existing source-bundle and bundle-preflight primitives. The new document reports direct folder/datasource matches, explicit remaps from an optional mapping file, missing target mappings, and inherited bundle blockers in one reviewable contract.
- Tests: added focused promotion-preflight contract/render coverage plus CLI help/parser coverage without attempting a live promotion path yet.
- Test Run: `cargo fmt --manifest-path rust/Cargo.toml --all --check` passed; `cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings` passed; `cargo test --manifest-path rust/Cargo.toml --quiet sync` passed with 128 sync tests.
- Impact: `rust/src/sync/promotion_preflight.rs`, `rust/src/sync/cli.rs`, `rust/src/sync/mod.rs`, `rust/src/sync/promotion_preflight_rust_tests.rs`, `rust/src/sync/cli_help_rust_tests.rs`, `rust/src/sync/bundle_contract_rust_tests.rs`
- Rollback/Risk: this is intentionally a skeleton and only covers staged folder/datasource remap visibility; revert the command/module if the contract needs to be redesigned before broader promotion semantics are added.

## 2026-03-27 - Unified CLI help/example source split
- Summary: moved the unified root help/example blocks and help-label color table out of `rust/src/cli.rs` into a dedicated `rust/src/cli_help_examples.rs` helper so the dispatcher stays focused on rendering and routing.
- Validation: `cargo fmt --manifest-path rust/Cargo.toml --all`; `cargo test --quiet unified_help`
- Test Run: passed, with 7 unified help-focused tests.
- Impact: `rust/src/cli.rs`, `rust/src/cli_help_examples.rs`, `rust/src/lib.rs`, `rust/src/cli_rust_tests.rs`
- Rollback/Risk: the user-facing help text should stay the same; revert the helper extraction if rendered help output changes unexpectedly.

## 2026-03-27 - Dashboard dependency report human-readable output
- Summary: enriched the offline dependency contract with typed datasource usage and orphan records, then split `InspectExportReportFormat::Dependency` onto a table-style text renderer while keeping `DependencyJson` as pretty JSON.
- Validation: focused dependency/inspect tests are pending; full validation will run after the code settles.
- Impact: `rust/src/dashboard_inspection_dependency_contract.rs`, `rust/src/dashboard/inspect_output.rs`, focused dashboard inspect regression tests
- Rollback/Risk: dependency JSON shape now carries richer objects for orphaned datasources and added datasource UID/type fields in usage rows; revert the render split if downstream text expectations need to be restored.

## 2026-03-27 - Current Change Summary
- Summary: archived the older detailed AI trace entries and reset the top-level AI docs to short current-only summaries.
- Validation: confirmed the new archive files exist and the current AI docs now point at both archive generations.
- Impact: `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`, `docs/internal/archive/ai-status-archive-2026-03-27.md`, `docs/internal/archive/ai-changes-archive-2026-03-27.md`

## 2026-03-27 - Current Architecture Summary
- Summary: current maintainer work is centered on shrinking large Rust orchestration modules, keeping facades thin, and preserving stable CLI and JSON contracts while feature-specific test files continue to split out of umbrella suites.
- Validation: repository documentation review only.
- Impact: `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`

## 2026-03-27 - Current Planned Follow-Up
- Summary: next targeted maintainer change is to let dashboard governance-gate load policy from JSON, YAML, or built-in sources without changing the evaluator contract.
- Validation: planning note only.
- Impact: `rust/src/dashboard/governance_gate.rs`, related dashboard governance gate tests, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
