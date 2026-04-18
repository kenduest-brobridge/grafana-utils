# ai-changes-archive-2026-04-19

## 2026-04-18 - Add review plans for access, dashboard, alert, and workspace
- Summary: extended the review-first plan model beyond datasource. `access plan` now reviews user export bundles against live Grafana, `dashboard plan` reviews single-org dashboard export trees against live Grafana, alert plan rows now carry stable action/status/review metadata, and workspace preview normalizes legacy operations into shared `actions`, `domains`, and `blockedReasons` fields for future TUI consumers.
- Tests: added focused access plan and dashboard plan tests; extended alert plan tests for action metadata and linked-dashboard warnings; added workspace preview contract tests for normalized action ordering and domain summaries.
- Test Run: `cargo test --manifest-path rust/Cargo.toml --quiet access_plan --lib`; `cargo test --manifest-path rust/Cargo.toml --quiet dashboard_plan --lib`; `cargo test --manifest-path rust/Cargo.toml --quiet alert --lib`; `cargo test --manifest-path rust/Cargo.toml --quiet workspace_preview --lib`.
- Impact: Rust access, dashboard, alert, and workspace preview command surfaces; command docs in English and zh-TW; docs entrypoint contract; generated man/html docs; and AI trace docs. README files and Python implementation were intentionally left unchanged.
- Rollback/Risk: medium new CLI surfaces and additive JSON fields. Rollback should remove `access plan`, `dashboard plan`, the workspace preview enrichment adapter, alert plan field additions, docs/tests, generated pages, and docs-entrypoint links; existing import/apply paths should remain separate.
- Follow-up: extend access plan beyond user bundles, add dashboard multi-org routing for `--use-export-org`, and wire a TUI consumer to the shared action contract instead of adding UI concerns to plan builders.

## 2026-04-18 - Extend access plan resource coverage
- Summary: extended `grafana-util access plan` beyond user bundles. The planner now supports concrete `--resource org`, `--resource team`, and `--resource service-account` modes with the same stable action contract, opt-in prune handling, changed fields, target evidence, and review hints. `--resource all` remains a future aggregate layer.
- Tests: added focused Rust plan regressions for org, team, and service-account create/same/update/extra/delete-candidate rows, including team membership/provisioned hints and service-account role/disabled hints.
- Test Run: `cargo test --manifest-path rust/Cargo.toml --quiet access --lib`; `cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings`.
- Impact: Rust access plan routing and resource-specific plan helpers, access CLI help text, access command docs in English and zh-TW, generated docs, and AI trace docs.
- Rollback/Risk: medium access plan behavior expansion. Rollback should remove the new org/team/service-account planner modules and route those selectors back to unsupported, while keeping the user plan path intact.
- Follow-up: implement `--resource all` as an aggregate layer after the concrete resource contracts settle, preferably without duplicating per-resource planners.
