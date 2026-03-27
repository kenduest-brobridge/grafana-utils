# ai-status.md

Current AI-maintained status only.

- Older trace history moved to [`archive/ai-status-archive-2026-03-24.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-03-24.md).
- Detailed 2026-03-27 entries moved to [`archive/ai-status-archive-2026-03-27.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-03-27.md).
- Keep this file short and current. Additive historical detail belongs in `docs/internal/archive/`.

## 2026-03-28 - Maintainer backlog phase/status sync
- State: Done
- Scope: `docs/internal/maintainer-backlog-2026-03-28.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the maintainer backlog still described dashboard cleanup as only starting, datasource secret handling as future work, and promotion as a skeleton even though the Rust state had already moved forward.
- Current Update: synchronized the backlog to call out landed dashboard inspect splits, the now-usable datasource secret operator contract and import dry-run visibility, and promotion as a partially landed staged review handoff.
- Result: the maintainer docs now match the current Rust architecture and progress language more closely without changing any code-facing docs.

## 2026-03-28 - Promotion preflight review handoff
- State: Done
- Scope: `rust/src/sync/promotion_preflight.rs`, `rust/src/sync/promotion_preflight_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: promotion preflight already reports remap checks, aggregate blocker counts, and bundle-preflight context, but it did not expose a staged handoff signal that tells operators whether the result was ready to move into review.
- Current Update: added a structured `handoffSummary` with review readiness and next-stage state, and rendered the same signal in the text output.
- Result: the promotion preflight contract now carries a small staged handoff step that tells operators when the result is ready for review versus when blockers still need to be resolved.

## 2026-03-28 - Dashboard inspect/workbench ownership cleanup
- State: Done
- Scope: `rust/src/dashboard/inspect_output.rs`, `rust/src/dashboard/inspect_output_report.rs`, `rust/src/dashboard/inspect_orchestration.rs`, `rust/src/dashboard/inspect_workbench_support.rs`, `rust/src/dashboard/inspect_workbench_content.rs`, `rust/src/dashboard/inspect_live_tui.rs`
- Baseline: dashboard inspect summary rendering still lived beside the broader report renderer, and workbench document assembly still mixed high-level group wiring with all BrowserItem content builders in one support module.
- Current Update: moved report-format rendering into `inspect_output_report.rs`, kept summary rendering in `inspect_output.rs`, and split workbench content builders into `inspect_workbench_content.rs` so `inspect_workbench_support.rs` stays focused on document/group construction and live-TUI item selection.
- Result: the dashboard inspect surface now has clearer ownership between orchestration, report rendering, summary rendering, and workbench content assembly without changing CLI behavior.

## 2026-03-28 - Datasource secret dry-run visibility and staged sync wording
- State: Done
- Scope: `rust/src/datasource_import_export.rs`, `rust/src/datasource_mutation_payload.rs`, `rust/src/datasource_secret.rs`, `rust/src/datasource_rust_tests.rs`, `rust/src/sync/bundle_preflight.rs`, `rust/src/sync/staged_documents_render.rs`, focused sync render/preflight tests
- Baseline: datasource import dry-run did not expose any structured secret-placeholder visibility, mutation failures only said that secret values were missing, and sync staged text used shorter secret-blocking wording that did not match the datasource placeholder vocabulary.
- Current Update: added import dry-run `secretVisibility` output plus summary counts, improved mutation/import missing-secret errors with compact placeholder-plan detail, and aligned staged sync text and bundle-preflight checks around `secretPlaceholderNames` and `secret-placeholder-blocking`.
- Result: datasource and sync now speak a more consistent secret placeholder language, and operators can see secret-placeholder review signals earlier in the Rust-only flow.

## 2026-03-28 - Safe crate boundary tightening
- State: Done
- Scope: `rust/src/lib.rs`
- Baseline: several helper modules were already internal-only in practice, but the crate root still exposed them more broadly than needed.
- Current Update: kept the CLI-facing crate surface intact while retaining only the safe internal modules as `pub(crate)` such as `alert_sync`, `cli_help_examples`, `dashboard_inspection_*`, `datasource_provider`, `datasource_secret`, `help_styles`, and `interactive_browser`.
- Result: the crate boundary is narrower than before without collapsing the public CLI-oriented surface into dead-code errors.

## 2026-03-28 - Sync/promotion secret placeholder UX alignment
- State: Done
- Scope: `rust/src/sync/mod.rs`, `rust/src/sync/cli_help_rust_tests.rs`, `rust/src/sync/promotion_preflight_rust_tests.rs`
- Baseline: bundle/promotion surfaces already tracked `secretPlaceholderNames`, but the visible help examples and Python bundle-preflight summary wording were not aligned with the datasource secret workflow vocabulary.
- Current Update: aligned bundle/promotion help examples to show an explicit availability file with `secretPlaceholderNames`, and pinned the placeholder-name expectations in focused Rust sync help and promotion tests.
- Result: the staged sync/promotion surfaces now make placeholder expectations visible in the same naming style as the datasource secret workflow without changing live semantics.

## 2026-03-28 - Dashboard governance/render boundary and crate visibility cleanup
- State: Done
- Scope: `rust/src/dashboard/inspect_governance.rs`, `rust/src/dashboard/inspect_governance_render.rs`, `rust/src/lib.rs`
- Baseline: the dashboard governance facade still mixed document/row exports with the full table renderer, and `lib.rs` still exposed several internal sync/datasource helper modules as effectively public surface.
- Current Update: moved governance table rendering into a dedicated `inspect_governance_render.rs` helper and narrowed several internal helper modules in `lib.rs` from `pub` to `pub(crate)`.
- Result: dashboard inspect/governance ownership is more explicit and the crate surface is less likely to grow accidental semi-public modules.

## 2026-03-28 - Datasource secret operator contract follow-through
- State: Done
- Scope: `rust/src/datasource_cli_defs.rs`, `rust/src/datasource_cli_mutation_rust_tests.rs`, `docs/internal/datasource-secret-handling-unwired.md`
- Baseline: Rust datasource import and mutation flows already accepted placeholder-based secret inputs, but the import-side operator help and the maintainer note still described the contract as partly unwired.
- Current Update: documented `--secret-values` as part of datasource import help with explicit `secureJsonDataPlaceholders` wording, added focused CLI help assertions, and updated the maintainer note to describe the import/mutation contract as wired while calling out remaining provider limitations.
- Result: datasource secret handling now has a clearer operator-facing contract for import and live mutation, even though provider-backed resolution and richer dry-run visibility are still follow-up work.

## 2026-03-27 - Datasource secret placeholder preflight
- State: Done
- Scope: `rust/src/datasource_secret.rs`, `rust/src/sync/bundle_preflight.rs`, `rust/src/sync/staged_documents_apply.rs`, `rust/src/sync/staged_documents_render.rs`, focused sync datasource secret tests
- Baseline: Rust sync source bundles preserved `secureJsonDataPlaceholders`, but bundle-preflight and apply gating only assessed sync checks, provider references, and alert artifacts. Missing datasource secret placeholder availability stayed invisible until a later manual step.
- Current Update: added a Rust datasource secret placeholder helper, wired a staged `secretPlaceholderAssessment` into bundle-preflight, counted placeholder blockers in the bundle summary, and threaded that count into apply rejection and apply-intent text output.
- Result: datasource secret handling now has a first fail-closed staged contract on the Rust path, so missing placeholder availability is reviewable and blocks apply before any live mutation path is considered.

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

## 2026-03-27 - Promotion mapping help example
- State: Done
- Scope: `rust/src/sync/mod.rs`, `rust/src/sync/cli_help_rust_tests.rs`
- Baseline: the promotion mapping contract was enforced in code and focused tests, but operators still had to infer the file shape from source or test fixtures.
- Current Update: embedded a minimal promotion mapping JSON example directly in `sync promotion-preflight --help` and locked the example strings in focused help tests.
- Result: operators can discover the mapping file kind, schema version, and environment metadata shape directly from CLI help before reading source or maintainer docs.

## 2026-03-27 - Unified CLI help/example source split
- State: Done
- Scope: `rust/src/cli.rs`, `rust/src/cli_help_examples.rs`, `rust/src/lib.rs`, focused unified CLI help tests
- Baseline: the unified CLI help/example strings and color-label table lived as one large block in `rust/src/cli.rs`.
- Current Update: extracted the help/example data into a dedicated helper module while keeping the rendered CLI help paths and command behavior unchanged.
- Result: the unified help source is now split across `rust/src/cli.rs` and `rust/src/cli_help_examples.rs`, and focused help rendering tests passed.

## 2026-03-27 - Dashboard dependency report human-readable output
- State: Done
- Scope: `rust/src/dashboard_inspection_dependency_contract.rs`, `rust/src/dashboard/inspect_output.rs`, `rust/src/dashboard/inspect_dependency_render.rs`, `rust/src/dashboard/inspect_family.rs`, focused dashboard inspect tests
- Baseline: dependency reporting had richer contract data available, but the dedicated dependency text renderer was still coupled to the broader inspect output module and inspect query reporting still depended on governance-owned family normalization helpers.
- Current Update: moved dependency table rendering into `inspect_dependency_render.rs`, added focused orphan-cell normalization coverage plus stronger output assertions for dashboard dependency sections, and extracted datasource family normalization into `inspect_family.rs` so the core inspect pipeline no longer reaches into governance internals for that shared helper.
- Result: the dependency report path is now a clearer inspect-owned subsystem slice with focused tests, the explicit `In Progress` item is closed, and the dashboard inspect path no longer depends on governance for basic family normalization.

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
- Scope: `rust/src/dashboard/`, `rust/src/datasource.rs`, `rust/src/datasource_import_export.rs`, `rust/src/lib.rs`, related dashboard and datasource tests
- Next Step: continue dashboard subsystem boundary cleanup beyond the already-landed inspect/report/governance splits, keep narrowing public-vs-internal crate boundaries, and then decide what follows the now-usable datasource secret operator contract and import dry-run secretVisibility.
