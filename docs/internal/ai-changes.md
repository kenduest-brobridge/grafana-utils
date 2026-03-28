# ai-changes.md

Current AI change log only.

- Older detailed history moved to [`archive/ai-changes-archive-2026-03-24.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-changes-archive-2026-03-24.md).
- Detailed 2026-03-27 entries moved to [`archive/ai-changes-archive-2026-03-27.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-changes-archive-2026-03-27.md).
- Keep this file limited to the latest active architecture and maintenance changes.

## 2026-03-28 - Browse TUI interaction wording convergence
- Summary: aligned the dashboard, datasource, user, and team browsers around the same interaction wording so browse summaries and footers now use `Mode=...`, `active-pane=...`, `next pane`, `previous pane`, `search`, `next match`, and consistent confirm-delete or cancel labels.
- Tests: extended focused browse-render assertions for the new dashboard and datasource wording; access browse remained render-only wiring with no new dedicated test module.
- Test Run: `cargo fmt --manifest-path rust/Cargo.toml --all --check` passed; `cargo test --manifest-path rust/Cargo.toml --quiet browse_render` passed; `cargo test --manifest-path rust/Cargo.toml --quiet datasource_browse_render` passed.
- Impact: `rust/src/dashboard/browse_render.rs`, `rust/src/datasource_browse_render.rs`, `rust/src/access/user_browse_render.rs`, `rust/src/access/team_browse_render.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: low. This changes operator-facing wording only and keeps browse state, input handling, and destructive behavior unchanged.
- Follow-up: none.

## 2026-03-28 - Interactive dashboard import boundary split
- Summary: split the growing `dashboard import --interactive` workbench into explicit render, review, and context helper modules while keeping the existing `crate::dashboard::import_interactive` entrypoint, state types, and test paths stable. The TUI frame/layout lives in `import_interactive_render.rs`, local artifact loading plus live review/diff resolution now live in `import_interactive_review.rs`, and the `Summary / Destination / Diff` context-pane builders now live in `import_interactive_context.rs`.
- Tests: reused the focused dashboard import workflow regression suite to validate the unchanged grouping, review-on-focus, dry-run review wording, context view cycling, and diff-depth behavior after the split.
- Test Run: `cargo fmt --manifest-path rust/Cargo.toml --all` passed; `cargo test --manifest-path rust/Cargo.toml --quiet dashboard_browse_workflow_rust_tests` passed; `cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings` passed.
- Impact: `rust/src/dashboard/import_interactive.rs`, `rust/src/dashboard/import_interactive_render.rs`, `rust/src/dashboard/import_interactive_review.rs`, `rust/src/dashboard/import_interactive_context.rs`, `rust/src/dashboard/mod.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: low-to-moderate. This is an internal ownership cleanup around already-landed interactive import behavior; revert the split if a different long-term module shape is needed, but keep the workbench state, renderer, and live review helpers separated.
- Follow-up: if the import workbench keeps growing, the next split should isolate diff formatting helpers or destination-summary builders instead of folding them back into the renderer.

## 2026-03-28 - Interactive dashboard import state/model split
- Summary: split `dashboard import --interactive` again by separating the shared review/context model types plus `InteractiveImportState` into `import_interactive_state.rs`, reducing ownership overlap inside `import_interactive.rs` without changing the existing TUI flow, review semantics, or public entrypoint.
- Tests: relied on the existing focused interactive import workflow regressions, which continue to reach the same `crate::dashboard::import_interactive::*` symbols through re-exports.
- Test Run: `cargo fmt --manifest-path rust/Cargo.toml --all --check` passed; `cargo test --manifest-path rust/Cargo.toml --quiet dashboard_browse_workflow_rust_tests` passed; `cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings` passed.
- Impact: `rust/src/dashboard/import_interactive.rs`, `rust/src/dashboard/import_interactive_state.rs`, `rust/src/dashboard/mod.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: low. This is a follow-on ownership cleanup that keeps the same entrypoint and re-export paths while narrowing the orchestration file to TTY gating and top-level delegation.
- Follow-up: the next useful split is to separate import-context loading from live review/diff synthesis inside `import_interactive_review.rs` if that module keeps growing.

## 2026-03-28 - Interactive dashboard import loader/review split
- Summary: split `dashboard import --interactive` once more by moving local import artifact and folder-context loading into `import_interactive_loader.rs`, leaving `import_interactive_review.rs` focused on live review and diff construction instead of mixed local-plus-live responsibilities.
- Tests: reused the focused dashboard import workflow regressions because the public `crate::dashboard::import_interactive::*` paths and interactive behavior stay unchanged.
- Test Run: `cargo fmt --manifest-path rust/Cargo.toml --all --check` passed; `cargo test --manifest-path rust/Cargo.toml --quiet dashboard_browse_workflow_rust_tests` passed; `cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings` passed.
- Impact: `rust/src/dashboard/import_interactive_loader.rs`, `rust/src/dashboard/import_interactive_review.rs`, `rust/src/dashboard/import_interactive.rs`, `rust/src/dashboard/mod.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: low. This is a narrow subsystem cleanup that preserves behavior and symbol paths while reducing ownership overlap inside the import workbench helpers.
- Follow-up: if the import review module grows again, the next split should isolate diff formatting from live review resolution instead of recombining the loader path.

## 2026-03-28 - Interactive dashboard import dry-run review mode
- Summary: made `dashboard import --interactive --dry-run` an explicit operator-facing mode by switching the interactive header, status copy, Enter action label, help text, and cancellation message over to dry-run review wording instead of reusing the import wording.
- Tests: added a focused dry-run state regression for the interactive import workbench and extended the import help regression to assert the new dry-run wording.
- Test Run: `cargo fmt --manifest-path rust/Cargo.toml --all` passed; `cargo test --manifest-path rust/Cargo.toml --quiet dashboard_browse_workflow_rust_tests` passed; `cargo test --manifest-path rust/Cargo.toml --quiet dashboard_cli_parser_help_rust_tests` passed; `cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings` passed.
- Impact: `rust/src/dashboard/import_interactive.rs`, `rust/src/dashboard/import_apply.rs`, `rust/src/dashboard/cli_defs_command.rs`, `rust/src/dashboard/dashboard_browse_workflow_rust_tests.rs`, `rust/src/dashboard/dashboard_cli_parser_help_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: low. This does not add a new command path; it only makes the existing interactive dry-run path explicit and less confusing for operators.
- Follow-up: none.

## 2026-03-28 - Interactive dashboard import context views
- Summary: upgraded the interactive dashboard import workbench into a single-screen review surface with switchable `Summary`, `Destination`, and `Diff` context panes. The lower pane now lets operators switch scope across focused, selected, and all dashboards, and the diff pane can switch between summary, structural, and raw views without leaving the import workflow.
- Tests: extended the focused dashboard import workflow regressions to cover the new context-view, scope, and diff-depth state transitions.
- Test Run: `cargo fmt --manifest-path rust/Cargo.toml --all` passed; `cargo test --manifest-path rust/Cargo.toml --quiet dashboard_browse_workflow_rust_tests` passed; `cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings` passed.
- Impact: `rust/src/dashboard/import_interactive.rs`, `rust/src/dashboard/dashboard_browse_workflow_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: moderate. This keeps the same import entrypoint and selection model, but it expands the review model and makes the import workbench materially richer than the original selector. Current ownership is now tracked by the later `Interactive dashboard import boundary split` entry.
- Follow-up: completed by the later same-day boundary split that moved the context-pane builders out of `import_interactive.rs`.

## 2026-03-28 - Interactive dashboard import review workbench
- Summary: upgraded `dashboard import --interactive` from a local file picker into a review-first TUI that reuses the existing import dry-run/import lookup semantics for the focused dashboard. The workbench now resolves review state on focus, caches create/update/skip/block outcomes, exposes folder/action/flat grouping, and keeps `Enter` as the direct import boundary for the selected files.
- Tests: extended focused dashboard import workflow tests for grouping and on-focus review resolution, and added a parser/help regression for the updated `--interactive` wording.
- Test Run: `cargo fmt --manifest-path rust/Cargo.toml --all` passed; `cargo test --manifest-path rust/Cargo.toml --quiet dashboard_browse_workflow_rust_tests` passed; `cargo test --manifest-path rust/Cargo.toml --quiet dashboard_cli_parser_help_rust_tests` passed; `cargo test --manifest-path rust/Cargo.toml --quiet import_edge_dry_run_preflight_rust_tests` passed; `cargo test --manifest-path rust/Cargo.toml --quiet import_edge_dry_run_update_existing_rust_tests` passed; `cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings` passed.
- Impact: `rust/src/dashboard/import.rs`, `rust/src/dashboard/import_apply.rs`, `rust/src/dashboard/import_dry_run.rs`, `rust/src/dashboard/import_interactive.rs`, `rust/src/dashboard/cli_defs_command.rs`, `rust/src/dashboard/dashboard_browse_workflow_rust_tests.rs`, `rust/src/dashboard/dashboard_cli_parser_help_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: moderate. The CLI entrypoint and final import boundary are unchanged, but interactive import now performs live review lookups while the TUI is open, so future changes to import semantics should keep the review helper and non-interactive import path aligned.
- Follow-up: a later phase can add a dedicated promotion-style review artifact or diff-aware live-vs-import panes, but this pass intentionally stops at review-then-import.

## 2026-03-28 - Interactive dashboard import summary and diff review
- Summary: extended the new interactive import workbench with operator-facing review summaries and a lightweight focused live diff. The header now shows pending/reviewed/create/update/skip/block counts plus the current folder/action group summary, and the review pane now shows a compact live-vs-import delta for existing dashboards instead of only the resolved action badge.
- Tests: expanded the focused dashboard import workflow regression to cover summary counting and changed-live diff summaries in addition to the existing on-focus review cases.
- Test Run: `cargo fmt --manifest-path rust/Cargo.toml --all` passed; `cargo test --manifest-path rust/Cargo.toml --quiet dashboard_browse_workflow_rust_tests` passed; `cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings` passed.
- Impact: `rust/src/dashboard/import_interactive.rs`, `rust/src/dashboard/dashboard_browse_workflow_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: low-to-moderate. This keeps the same review-then-import boundary, but it adds more operator-facing review text and focused live fetch usage, so revert if the import TUI needs to stay narrower or if the compact diff summary should move to a different pane later.
- Follow-up: if this surface grows further, the next step should be a dedicated import-review document/model rather than piling more semantics into one renderer.

## 2026-03-28 - Inspect workbench modal state split
- Summary: extracted the inspect workbench search prompt, repeat-search memory, and full-detail viewer fields into a dedicated `InspectWorkbenchModalState` helper, then rewired the workbench input loop and modal renderer to read through the nested modal state instead of the flattened top-level fields.
- Tests: added a focused repeat-search regression alongside the existing inspect workbench state coverage.
- Test Run: `cargo fmt --manifest-path rust/Cargo.toml --all --check` passed; `cargo test --manifest-path rust/Cargo.toml --quiet inspect_workbench_state` passed with 6 tests.
- Impact: `rust/src/dashboard/inspect_workbench_modal_state.rs`, `rust/src/dashboard/inspect_workbench_state.rs`, `rust/src/dashboard/inspect_workbench.rs`, `rust/src/dashboard/inspect_workbench_render.rs`, `rust/src/dashboard/inspect_workbench_render_modal_sections.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: low. This is an internal ownership split with no intended operator-facing behavior change, but it does move the modal/search state access path, so revert if a later workbench change wants the fields flattened again.
- Follow-up: none.

## 2026-03-28 - Sync review TUI interaction grammar alignment
- Summary: tightened the sync review checklist/diff/footer copy to use clearer staged-review language, replacing older phrases like "workspace primary", "toggle operations", and "confirm the reviewed selection" with wording that more closely matches the shared Rust TUI interaction grammar.
- Tests: updated the focused sync review regression to pin the new header, diff-controls, and status copy.
- Test Run: `cargo fmt --manifest-path rust/Cargo.toml --all --check` passed; `cargo test --manifest-path rust/Cargo.toml --quiet cli_review_tui_rust_tests` passed with 6 tests.
- Impact: `rust/src/sync/review_tui.rs`, `rust/src/sync/review_tui_helpers.rs`, `rust/src/sync/cli_review_tui_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: low. This is TUI copy only and the staged-only selection/filter behavior did not change, but revert the wording if downstream operators prefer the older phrasing.
- Follow-up: none.

## 2026-03-28 - Dashboard and datasource browse shell grammar convergence
- Summary: moved dashboard browse onto `tui_shell::build_header` and `tui_shell::build_footer`, shifted browse status text into the shared footer path for both browse surfaces, and rewired the datasource browse control rows to reuse shared key-chip/plain helpers instead of duplicating shell styling locally.
- Tests: added focused browse-render unit coverage for the header/status split in both dashboard and datasource browse.
- Test Run: `cargo fmt --manifest-path rust/Cargo.toml --all --check` passed; `cargo test --manifest-path rust/Cargo.toml --quiet browse_render` passed; `cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings` passed.
- Impact: `rust/src/dashboard/browse_render.rs`, `rust/src/datasource_browse_render.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: low-to-moderate. This is TUI chrome only, but it does move visible status placement and shared helper usage, so revert if the footer/header split needs to stay on the older layout.
- Follow-up: none.

## 2026-03-28 - TUI overlay and workbench state cleanup
- Summary: moved dashboard and datasource destructive confirmations off the detail pane into a shared centered overlay pattern, dropped the extra preview pane from sync review diff mode so the diff workspace stays dominant, and split inspect workbench full-detail viewer state into its own `InspectFullDetailState`.
- Tests: kept the changes pinned with focused browse/review/inspect render-state tests.
- Test Run: `cargo fmt --manifest-path rust/Cargo.toml --all --check` passed; `cargo test --manifest-path rust/Cargo.toml --quiet inspect_workbench_state` passed; `cargo test --manifest-path rust/Cargo.toml --quiet cli_review_tui_rust_tests` passed; `cargo test --manifest-path rust/Cargo.toml --quiet datasource_browse_render` passed; `cargo test --manifest-path rust/Cargo.toml --quiet browse_input` passed; `cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings` passed.
- Impact: `rust/src/tui_shell.rs`, `rust/src/dashboard/browse_render.rs`, `rust/src/datasource_browse_render.rs`, `rust/src/sync/review_tui.rs`, `rust/src/dashboard/inspect_workbench.rs`, `rust/src/dashboard/inspect_workbench_render.rs`, `rust/src/dashboard/inspect_workbench_render_modal_sections.rs`, `rust/src/dashboard/inspect_workbench_state.rs`
- Rollback/Risk: low-to-moderate. These are TUI-only behavior and structure changes, but they do alter how destructive confirmation and diff review occupy terminal space.

## 2026-03-28 - Shared Rust TUI shell pass
- Summary: introduced a crate-private `tui_shell` helper and moved the main Rust TUI surfaces onto a common shell grammar so `dashboard inspect workbench`, `sync review`, and `datasource browse` now share the same header/footer/control vocabulary and stronger active-workspace hierarchy.
- Tests: added focused TUI assertions for sync review header state and datasource browse header mode text, while keeping existing inspect workbench summary tests green.
- Test Run: `cargo fmt --manifest-path rust/Cargo.toml --all --check` passed; `cargo test --manifest-path rust/Cargo.toml --quiet inspect_live_tui_rust_tests` passed; `cargo test --manifest-path rust/Cargo.toml --quiet cli_review_tui_rust_tests` passed; `cargo test --manifest-path rust/Cargo.toml --quiet datasource_browse_render` passed; `cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings` passed.
- Impact: `rust/src/tui_shell.rs`, `rust/src/lib.rs`, `rust/src/dashboard/inspect_workbench_render.rs`, `rust/src/dashboard/inspect_workbench_render_helpers.rs`, `rust/src/dashboard/inspect_workbench_support.rs`, `rust/src/sync/review_tui.rs`, `rust/src/sync/cli_review_tui_rust_tests.rs`, `rust/src/datasource_browse_render.rs`
- Rollback/Risk: low-to-moderate. This changes only TUI presentation and helper wiring, not CLI contracts or live behavior, but it does reshape the operator-facing terminal hierarchy across multiple domains.

## 2026-03-28 - Datasource secret resolution aggregation
- Summary: updated the shared datasource secret resolver so live mutation/import now accumulates every missing or empty placeholder name and returns one fail-closed error before any write attempt, instead of stopping at the first unresolved secret.
- Tests: refreshed the focused secret helper regression to cover aggregate missing/empty reporting and updated the import preflight regression to assert the new later-stage failure text.
- Test Run: `cargo fmt --manifest-path rust/Cargo.toml --all --check` failed because the worktree already contains unrelated dashboard formatting diffs; `rustfmt --check rust/src/datasource_secret.rs rust/src/datasource_secret_rust_tests.rs rust/src/datasource_import_export.rs` passed; `cargo test --manifest-path rust/Cargo.toml --quiet datasource_secret_rust_tests` failed because unrelated dashboard compile errors are still present in the worktree; `cargo test --manifest-path rust/Cargo.toml --quiet datasource_rust_tests` failed for the same reason.
- Impact: `rust/src/datasource_secret.rs`, `rust/src/datasource_secret_rust_tests.rs`, `rust/src/datasource_import_export.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: low. The behavior stays staged/fail-closed and only expands the unresolved-secret error report; revert if downstream consumers depend on the older single-placeholder error wording.
- Follow-up: none.

## 2026-03-28 - Maintainer backlog phase/status sync
- Summary: updated the internal maintainer backlog so it reflects the current Rust state more accurately: dashboard inspect cleanup is described as landed in its current slices, datasource secret handling is described as already having a usable operator contract plus dry-run `secretVisibility`, and promotion is described as a partially landed staged review handoff instead of a pure skeleton.
- Tests: not applicable. This is docs-only.
- Test Run: not run.
- Validation: reread the backlog and current AI trace entries to make sure the phase language and progress wording match the current Rust architecture notes.
- Impact: `docs/internal/maintainer-backlog-2026-03-28.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: low. Revert if the backlog wording needs to go back to the prior phase framing.

## 2026-03-28 - Promotion preflight review handoff
- Summary: added a structured `handoffSummary` to the staged sync promotion-preflight document so operators can see whether the result is ready to move into review, then rendered the same handoff state in the text output with a `review-required` and `next-stage` line.
- Tests: extended promotion-preflight regression coverage to assert the new handoff summary in both blocked and clean cases, plus the rendered handoff line in the text output.
- Test Run: `cargo fmt --manifest-path rust/Cargo.toml --all --check` passed; `cargo test --manifest-path rust/Cargo.toml --quiet sync` passed with 132 sync tests.
- Impact: `rust/src/sync/promotion_preflight.rs`, `rust/src/sync/promotion_preflight_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: low. This is a staged-only contract addition and does not touch live apply wiring; revert the new `handoffSummary` field and render line if downstream consumers need the preflight document shape restored.

## 2026-03-28 - Dashboard inspect/workbench ownership cleanup
- Summary: moved dashboard inspect report-format rendering into a dedicated `inspect_output_report.rs` helper while keeping summary rendering in `inspect_output.rs`, then split the inspect workbench BrowserItem builders out of `inspect_workbench_support.rs` into `inspect_workbench_content.rs` so document/group wiring stays separate from content assembly.
- Tests: kept the behavior pinned through focused `inspect_output` and `inspect_live_tui` Rust tests.
- Test Run: `cargo fmt --manifest-path rust/Cargo.toml --all` passed; `cargo test --manifest-path rust/Cargo.toml --quiet inspect_output` passed; `cargo test --manifest-path rust/Cargo.toml --quiet inspect_live_tui` passed.
- Impact: `rust/src/dashboard/inspect_output.rs`, `rust/src/dashboard/inspect_output_report.rs`, `rust/src/dashboard/inspect_orchestration.rs`, `rust/src/dashboard/inspect_workbench_support.rs`, `rust/src/dashboard/inspect_workbench_content.rs`, `rust/src/dashboard/inspect_live_tui.rs`
- Rollback/Risk: low. This is internal ownership cleanup only; revert the helper extractions if future dashboard inspect changes need a different report/workbench module shape.

## 2026-03-28 - Datasource secret dry-run visibility and staged sync wording
- Summary: added `secretVisibility` and `secretVisibilityCount` to datasource import dry-run JSON, expanded missing-secret mutation/import errors with summarized placeholder-plan detail, and aligned staged sync bundle/apply wording around `secretPlaceholderNames` and `secret-placeholder-blocking`.
- Tests: extended focused datasource and sync regressions to assert import dry-run visibility, richer secret-plan errors, updated bundle-preflight detail strings, and the new staged render labels.
- Test Run: `cargo fmt --manifest-path rust/Cargo.toml --all` passed; `cargo test --manifest-path rust/Cargo.toml --quiet datasource_rust_tests` passed; `cargo test --manifest-path rust/Cargo.toml --quiet sync` passed.
- Impact: `rust/src/datasource_import_export.rs`, `rust/src/datasource_mutation_payload.rs`, `rust/src/datasource_secret.rs`, `rust/src/datasource_rust_tests.rs`, `rust/src/sync/bundle_preflight.rs`, `rust/src/sync/staged_documents_render.rs`, `rust/src/sync/cli_render_rust_tests.rs`, `rust/src/sync/bundle_contract_preflight_rust_tests.rs`, `rust/src/sync/bundle_exec_rust_tests.rs`
- Rollback/Risk: low-to-moderate. The flow stays staged-only and fail-closed, but the dry-run JSON and error text now expose more operator guidance, so downstream consumers should be checked if they parse those strings rigidly.

## 2026-03-28 - Safe crate boundary tightening
- Summary: kept the CLI-facing public crate surface intact while retaining only the clearly internal helper modules as `pub(crate)` in `lib.rs`, avoiding the broader dead-code fallout from shrinking every top-level module.
- Tests: reused full Rust library and clippy validation to ensure the narrower boundary still compiles across test and non-test targets.
- Test Run: `cargo test --manifest-path rust/Cargo.toml --quiet --lib` passed with 804 tests; `cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings` passed.
- Impact: `rust/src/lib.rs`
- Rollback/Risk: low. This keeps the prior public CLI-oriented surface and only narrows helper modules already used internally.

## 2026-03-28 - Sync/promotion secret placeholder UX alignment
- Summary: aligned staged bundle/promotion help examples with the datasource secret contract by showing `secretPlaceholderNames` in the availability-file examples so sync and promotion expose the same secret vocabulary as datasource import/mutation.
- Tests: updated focused Rust sync help and promotion render tests to pin the secret-placeholder example strings and placeholder-name expectations.
- Test Run: `cargo test --manifest-path rust/Cargo.toml --quiet sync` passed with 132 sync tests.
- Impact: `rust/src/sync/mod.rs`, `rust/src/sync/cli_help_rust_tests.rs`, `rust/src/sync/promotion_preflight_rust_tests.rs`
- Rollback/Risk: low; this is help/render/test alignment only and does not alter live apply semantics or datasource runtime behavior.

## 2026-03-28 - Dashboard governance/render boundary and crate visibility cleanup
- Summary: moved the governance text renderer out of `rust/src/dashboard/inspect_governance.rs` into a dedicated `rust/src/dashboard/inspect_governance_render.rs` helper so the governance facade can stay focused on document/row ownership, and tightened several internal helper modules in `rust/src/lib.rs` from `pub` to `pub(crate)`.
- Tests: reused focused bundle/datasource helper tests for the `lib.rs` visibility tightening and validated the dashboard boundary change through formatting, clippy, and focused dashboard inspect coverage.
- Test Run: `cargo fmt --manifest-path rust/Cargo.toml --all --check`; `cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings`; `cargo test --manifest-path rust/Cargo.toml --quiet inspect_output`; `cargo test --manifest-path rust/Cargo.toml --quiet dashboard_inspection_dependency_contract`; `cargo test --manifest-path rust/Cargo.toml --quiet --lib bundle_preflight_rust_tests`; `cargo test --manifest-path rust/Cargo.toml --quiet --lib datasource_provider_rust_tests`; `cargo test --manifest-path rust/Cargo.toml --quiet --lib datasource_secret_rust_tests`
- Impact: `rust/src/dashboard/inspect_governance.rs`, `rust/src/dashboard/inspect_governance_render.rs`, `rust/src/lib.rs`
- Rollback/Risk: low. This is ownership/visibility cleanup around existing behavior; revert the renderer split or specific `pub(crate)` changes if a downstream internal call path still depends on the older surface.

## 2026-03-28 - Datasource secret operator contract follow-through
- Summary: clarified the wired Rust secret-placeholder flow for datasource import and live mutation by documenting `--secret-values` directly in datasource import help, adding focused help assertions for `secureJsonDataPlaceholders`, and updating the maintainer note so it no longer describes the import/mutation contract as unwired.
- Tests: extended focused datasource CLI help coverage for the import-side secret wording and headings.
- Test Run: `cargo test --manifest-path rust/Cargo.toml --quiet datasource_cli_mutation_rust_tests` passed with 30 tests.
- Impact: `rust/src/datasource_cli_defs.rs`, `rust/src/datasource_cli_mutation_rust_tests.rs`, `docs/internal/datasource-secret-handling-unwired.md`
- Rollback/Risk: low. This is help/doc contract alignment only; revert if the import-side secret flag names or placeholder contract change again.

## 2026-03-27 - Datasource secret placeholder preflight
- Summary: added `rust/src/datasource_secret.rs` for `${secret:...}` placeholder parsing and staged plan summaries, then wired `secretPlaceholderAssessment` into Rust sync bundle-preflight so missing placeholder availability becomes an explicit blocking check alongside provider and alert-artifact assessments.
- Tests: added focused datasource secret helper coverage and extended sync bundle-preflight/apply/render/promotion regressions to assert the new `secretPlaceholderBlockingCount`, staged review output, and apply rejection reason source.
- Test Run: `cargo fmt --manifest-path rust/Cargo.toml --all` passed; `cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings` passed; `cargo test --manifest-path rust/Cargo.toml --quiet sync` passed with 131 sync tests.
- Impact: `rust/src/datasource_secret.rs`, `rust/src/datasource_secret_rust_tests.rs`, `rust/src/lib.rs`, `rust/src/sync/bundle_preflight.rs`, `rust/src/sync/staged_documents_apply.rs`, `rust/src/sync/staged_documents_render.rs`, `rust/src/sync/promotion_preflight.rs`, `rust/src/sync/bundle_contract_preflight_rust_tests.rs`, `rust/src/sync/cli_apply_review_exec_apply_rust_tests.rs`, `rust/src/sync/cli_render_rust_tests.rs`, `rust/src/sync/bundle_exec_rust_tests.rs`, `rust/src/sync/promotion_preflight_rust_tests.rs`
- Rollback/Risk: this is still staged-only secret handling and does not resolve secrets; revert the new assessment if the placeholder contract or availability naming needs to change before wiring later resolution flows.

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

## 2026-03-27 - Promotion mapping help example
- Summary: added a minimal `grafana-utils-sync-promotion-mapping` JSON example directly to `sync promotion-preflight --help` so the mapping file contract is discoverable from the CLI instead of only from tests and source.
- Tests: extended focused sync help coverage to assert the mapping document kind and environment metadata snippet appear in the rendered help output.
- Test Run: `cargo fmt --manifest-path rust/Cargo.toml --all --check` passed; `cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings` passed; `cargo test --manifest-path rust/Cargo.toml --quiet sync` passed with 129 sync tests.
- Impact: `rust/src/sync/mod.rs`, `rust/src/sync/cli_help_rust_tests.rs`
- Rollback/Risk: low; revert the extra help block if the long-help output becomes too noisy or if the mapping contract changes again.

## 2026-03-27 - Unified CLI help/example source split
- Summary: moved the unified root help/example blocks and help-label color table out of `rust/src/cli.rs` into a dedicated `rust/src/cli_help_examples.rs` helper so the dispatcher stays focused on rendering and routing.
- Validation: `cargo fmt --manifest-path rust/Cargo.toml --all`; `cargo test --quiet unified_help`
- Test Run: passed, with 7 unified help-focused tests.
- Impact: `rust/src/cli.rs`, `rust/src/cli_help_examples.rs`, `rust/src/lib.rs`, `rust/src/cli_rust_tests.rs`
- Rollback/Risk: the user-facing help text should stay the same; revert the helper extraction if rendered help output changes unexpectedly.

## 2026-03-27 - Dashboard dependency report human-readable output
- Summary: finished the dashboard dependency report cleanup by extracting dependency-table rendering out of `rust/src/dashboard/inspect_output.rs` into `rust/src/dashboard/inspect_dependency_render.rs`, added focused text coverage for orphan-cell normalization and dashboard dependency sections, and moved datasource family normalization into `rust/src/dashboard/inspect_family.rs` so inspect reporting no longer depends on governance internals for that shared helper.
- Validation: `cargo fmt --manifest-path rust/Cargo.toml --all --check` passed after formatting; `cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings` passed; `cargo test --manifest-path rust/Cargo.toml --quiet inspect_output` passed; `cargo test --manifest-path rust/Cargo.toml --quiet dashboard_inspection_dependency_contract` passed.
- Impact: `rust/src/dashboard_inspection_dependency_contract.rs`, `rust/src/dashboard/inspect_output.rs`, `rust/src/dashboard/inspect_dependency_render.rs`, `rust/src/dashboard/inspect_family.rs`, `rust/src/dashboard/inspect_governance.rs`, `rust/src/dashboard/inspect_governance_coverage.rs`, `rust/src/dashboard/inspect_query_report.rs`, `rust/src/dashboard/mod.rs`, `rust/src/lib.rs`
- Rollback/Risk: low. This is an internal ownership cleanup around already-exposed report behavior; revert the helper extraction if the inspect/governance helper split needs a different shared-module shape.

## 2026-03-27 - Current Change Summary
- Summary: archived the older detailed AI trace entries and reset the top-level AI docs to short current-only summaries.
- Validation: confirmed the new archive files exist and the current AI docs now point at both archive generations.
- Impact: `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`, `docs/internal/archive/ai-status-archive-2026-03-27.md`, `docs/internal/archive/ai-changes-archive-2026-03-27.md`

## 2026-03-27 - Current Architecture Summary
- Summary: current maintainer work is centered on shrinking large Rust orchestration modules, keeping facades thin, and preserving stable CLI and JSON contracts while feature-specific test files continue to split out of umbrella suites.
- Validation: repository documentation review only.
- Impact: `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`

## 2026-03-27 - Current Planned Follow-Up
- Summary: next targeted maintainer change is to continue dashboard subsystem boundary cleanup beyond the dependency report path, keep tightening crate visibility boundaries, and extend datasource secret handling from the now-wired add/modify mutation path into datasource import-side record and payload workflows before returning to narrower promotion-only refinements.
- Validation: planning note only.
- Impact: `rust/src/dashboard/`, `rust/src/datasource.rs`, `rust/src/datasource_import_export.rs`, `rust/src/lib.rs`, related dashboard and datasource tests, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`

## 2026-03-28 - Dashboard dependency boundary and datasource mutation secret wiring
- Summary: finished the current dashboard dependency-report cleanup by keeping dependency rendering in an inspect-owned helper module and moving shared datasource family normalization out of governance into `rust/src/dashboard/inspect_family.rs`. Also wired the datasource secret placeholder resolution contract into both live datasource add/modify payload builders and datasource import payloads through explicit `--secure-json-data-placeholders` and `--secret-values` JSON inputs plus import-side `secureJsonDataPlaceholders` record support.
- Tests: extended focused dashboard dependency output assertions and datasource regressions to cover dependency sections, orphan rendering normalization, placeholder resolution, fail-closed mutation input errors, import parser support, import payload resolution, and import contract acceptance of `secureJsonDataPlaceholders`.
- Test Run: `cargo fmt --manifest-path rust/Cargo.toml --all` passed; `cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings` passed; `cargo test --manifest-path rust/Cargo.toml --quiet inspect_output` passed; `cargo test --manifest-path rust/Cargo.toml --quiet dashboard_inspection_dependency_contract` passed; `cargo test --manifest-path rust/Cargo.toml --quiet datasource_rust_tests` passed; `cargo test --manifest-path rust/Cargo.toml --quiet datasource_cli_mutation_tail_rust_tests` passed; `cargo test --manifest-path rust/Cargo.toml --quiet datasource_rust_tests_tail_rust_tests` passed; `cargo test --manifest-path rust/Cargo.toml --quiet datasource_secret_rust_tests` passed.
- Impact: `rust/src/dashboard/inspect_dependency_render.rs`, `rust/src/dashboard/inspect_family.rs`, `rust/src/dashboard/inspect_output.rs`, `rust/src/dashboard/inspect_query_report.rs`, `rust/src/dashboard/inspect_governance.rs`, `rust/src/dashboard/inspect_governance_coverage.rs`, `rust/src/dashboard/mod.rs`, `rust/src/datasource.rs`, `rust/src/datasource_cli_defs.rs`, `rust/src/datasource_import_export.rs`, `rust/src/datasource_import_export_support.rs`, `rust/src/datasource_mutation_payload.rs`, `rust/src/datasource_secret.rs`, `rust/src/datasource_rust_tests.rs`, `rust/src/datasource_rust_tests_tail_rust_tests.rs`, `rust/src/datasource_cli_mutation_tail_rust_tests.rs`, `rust/src/datasource_secret_rust_tests.rs`, `rust/src/lib.rs`, `docs/internal/datasource-secret-handling-unwired.md`
- Rollback/Risk: low-to-moderate. The dashboard side is internal ownership cleanup only; the datasource side adds new explicit CLI secret-input surfaces and import-side placeholder support, but still does not extend dry-run or sync/promotion secret explainability.
