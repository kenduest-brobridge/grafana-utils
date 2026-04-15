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
- Older entries moved to [`ai-status-archive-2026-04-15.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-04-15.md).

## 2026-04-15 - Fix stale dashboard command references
- State: Done
- Scope: Public docs and maintainer guidance that still present removed dashboard command names or stale inspect-artifact wording. Python implementation is out of scope.
- Baseline: `grafana-util dashboard analyze` and `grafana-util dashboard inspect-export` are rejected by the CLI, but README and maintainer docs still mention them as command paths.
- Current Update: Replaced README dependency examples with `dashboard summary`, updated policy/about artifact wording to dashboard summary JSON artifacts, refreshed generated man/html docs, and clarified maintainer docs that `inspect` is an internal artifact flow rather than a public dashboard command.
- Result: CLI smoke checks confirm `dashboard analyze` and `dashboard inspect-export` are rejected while `dashboard summary` is accepted. Docs, generated-doc, Rust help, AI workflow, and whitespace checks pass.

## 2026-04-15 - Clear remaining Rust architecture warnings
- State: Done
- Scope: Rust maintainability cleanup for sync help assertions plus large dashboard dependency, sync live-apply, datasource staged-reading, and dashboard browse-support modules. README files, Python implementation, and dashboard summary/analyze public naming are out of scope.
- Baseline: `make quality-architecture` reports five warnings: `sync/cli_help_rust_tests.rs` direct help assertions plus four production files over the 900-line warning threshold.
- Current Update: Added grouped sync help assertions and split dependency contract tests, sync request-json live-apply shim, datasource staged-reading tests, and dashboard local browse tests into focused sibling modules.
- Result: Focused tests, full Rust tests, clippy, formatting, and architecture guardrails pass. `make quality-architecture` now reports no warnings. Dashboard summary/analyze naming cleanup remains deferred.

## 2026-04-15 - Reduce dashboard help assertions
- State: Done
- Scope: Rust dashboard help-test maintainability. README files and Python implementation are out of scope.
- Baseline: `make quality-architecture` warned that `dashboard_cli_inspect_help_rust_tests.rs` used many direct `help.contains()` assertions.
- Current Update: Added a small `assert_help_includes` helper and routed grouped dashboard help assertions through it while preserving the same expected help text coverage.
- Result: Dashboard help focused tests, full Rust tests, clippy, architecture guardrails, formatting, and whitespace checks pass. The dashboard help-test warning is cleared.

## 2026-04-15 - Split datasource supported catalog tests
- State: Done
- Scope: Rust datasource test maintainability. README files and Python implementation are out of scope.
- Baseline: `make quality-architecture` still warned on `datasource/tests/cli_mutation.rs` after previous datasource test splits.
- Current Update: Moved supported datasource catalog JSON/text/table/csv/yaml tests into `cli_mutation_supported_catalog.rs`, leaving `cli_mutation.rs` focused on datasource command help, parser compatibility, and add-payload behavior.
- Result: Datasource focused tests, full Rust tests, clippy, architecture guardrails, formatting, and whitespace checks pass. `datasource/tests/cli_mutation.rs` is no longer an architecture warning.

## 2026-04-15 - Split datasource tail import and inspect tests
- State: Done
- Scope: Rust datasource test maintainability. README files and Python implementation are out of scope.
- Baseline: `make quality-architecture` still warned on `datasource/tests/tail.rs` after previous tail diff and fixture splits.
- Current Update: Moved datasource import validation/loader coverage into `tail_import.rs` and inspect-export/local source/manifest coverage into `tail_inspect.rs`, leaving `tail.rs` focused on routed import summary and export-org routing behavior.
- Result: Datasource focused tests, full Rust tests, clippy, architecture guardrails, formatting, and whitespace checks pass. `datasource/tests/tail.rs` is no longer an architecture warning.

## 2026-04-15 - Split snapshot review tests
- State: Done
- Scope: Rust snapshot test maintainability. README files and Python implementation are out of scope.
- Baseline: `make quality-architecture` still warned on `snapshot/tests.rs` after earlier maintainability passes.
- Current Update: Moved staged export scope resolver coverage into `tests_staged_scopes.rs` and snapshot review wrapper/warning coverage into `tests_review_warnings.rs`, leaving the main snapshot test module focused on shared fixtures and broader snapshot export/review behavior.
- Result: Snapshot focused tests, full Rust tests, clippy, architecture guardrails, formatting, and whitespace checks pass. `snapshot/tests.rs` is no longer an architecture warning.
