# ai-changes-archive-2026-04-13

## 2026-04-12 - Externalize docs entry taxonomy and add handbook command maps
- Summary: added `scripts/contracts/docs-entrypoints.json` as the shared definition file for landing quick commands, jump-select command entries, and handbook command-relationship maps; replaced the hard-coded Python metadata with a validating loader in `scripts/docgen_entrypoints.py`.
- User impact: the generated docs homepage now exposes a stable first-run path panel, jump navigation includes `version` and `config profile`, and handbook pages such as dashboard show grouped subcommand relationships in both the left nav and an in-page command map.
- Validation: `make html`; `make html-check`; `make quality-docs-surface`; `python3 -m unittest -v python.tests.test_python_docgen_entrypoints python.tests.test_python_docgen_command_docs python.tests.test_python_check_docs_surface`

## 2026-04-12 - Re-scope Developer Guide as a short maintainer router
- Summary: rewrote `docs/DEVELOPER.md` into a shorter maintainer landing page, tightened `docs/internal/maintainer-quickstart.md` into the first-entry reading-order and source-of-truth map, extracted stable closure rules into `docs/internal/ai-change-closure-rules.md`, and routed the maintainer and AI-workflow docs to that shared closure contract so future routing changes update the right maintainer docs together.
- Validation: `make quality-ai-workflow`; `git diff --check`

## 2026-04-12 - Remove legacy CLI compatibility
- Summary: removed the binary-level legacy command hint layer, deleted the legacy help module, removed unused old alert grouping schema from `cli.rs`, removed `legacy_replacements` support from the docs-surface contract/checker, kept `grafana-util alert --help` focused on real flat commands, and updated colored help rendering so option entries, inline `--flag` references, and example captions are highlighted.
- Tests: updated CLI tests to assert removed roots and old alert grouped forms are rejected through the normal Clap path, are not intercepted by custom help preflight, and colored contextual help highlights option entries, inline flags, and example captions across dashboard, alert, datasource, and profile help.
- Test Run: `cargo fmt --manifest-path rust/Cargo.toml --all`; `cargo test --manifest-path rust/Cargo.toml --quiet cli_rust_tests -- --test-threads=1`; `cargo fmt --manifest-path rust/Cargo.toml --all --check`; `make man`; `make html`; `make man-check`; `make html-check`; `make quality-docs-surface`; `make quality-ai-workflow`; `python3 -m json.tool scripts/contracts/command-surface.json`; `python3 -m json.tool scripts/contracts/command-reference-index.json`.
- Validation: manually checked `grafana-util --color always das ex --help` through `cargo run --manifest-path rust/Cargo.toml --quiet --bin grafana-util -- ... | cat -v` and confirmed option entries, aliases, and inline flags emit highlight ANSI.
- Impact: `rust/src/bin/grafana-util.rs`, `rust/src/cli.rs`, `rust/src/cli_help.rs`, `rust/src/cli_help_examples.rs`, `rust/src/cli_help/grouped_specs.rs`, `rust/src/cli_rust_tests.rs`, `scripts/check_docs_surface.py`, `scripts/contracts/command-surface.json`, `scripts/contracts/command-reference-index.json`, `docs/commands/{en,zh-TW}/`, `docs/internal/ai-status.md`.
- Rollback/Risk: old commands now receive standard parser errors with no compatibility mapping; rollback would restore the deleted legacy hint module and contract field.
- Follow-up: none.

## 2026-04-12 - Infer unique long option prefixes
- Summary: enabled unique long-option prefix inference on the unified CLI root and the standalone access parser so shortcuts such as `--all-o` and `--tab` resolve when they match exactly one known option.
- Tests: added parser coverage for successful unique long option inference and for rejected ambiguous/invalid prefixes in both unified CLI and access parser paths.
- Test Run: `cargo fmt --manifest-path rust/Cargo.toml --all`; `cargo test --manifest-path rust/Cargo.toml --quiet long_option -- --test-threads=1`; `cargo test --manifest-path rust/Cargo.toml --quiet access_cli_rust_tests -- --test-threads=1`; `cargo test --manifest-path rust/Cargo.toml --quiet cli_rust_tests -- --test-threads=1`; `cargo fmt --manifest-path rust/Cargo.toml --all --check`; `git diff --check`; `cargo build --manifest-path rust/Cargo.toml --quiet --bin grafana-util`; `make quality-ai-workflow`.
- Validation: `./rust/target/debug/grafana-util access user list --all-o --list-col` prints the user list columns without calling Grafana; `./rust/target/debug/grafana-util access user list --output json` remains rejected with a suggestion for `--output-format`.
- Impact: `rust/src/cli.rs`, `rust/src/access/cli_defs.rs`, `rust/src/access/access_cli_rust_tests.rs`, `rust/src/cli_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`, `docs/internal/ai-learnings.md`.
- Rollback/Risk: command-line abbreviations now work for unique long option prefixes; ambiguous or invalid prefixes still fail, so scripts should continue to prefer full canonical flag names for clarity.
- Follow-up: none.

## 2026-04-12 - Show org users in list table output
- Summary: fixed `grafana-util access org list --with-users` human-readable output so table, CSV, and text modes include user summaries when user details are requested; default org list output remains the original `id/name/userCount` shape.
- Tests: added formatter tests for org list headers, table rows, CSV headers, and text summary lines with and without `--with-users`.
- Test Run: `cargo fmt --manifest-path rust/Cargo.toml --all`; `cargo test --manifest-path rust/Cargo.toml --quiet org_ -- --test-threads=1`; `cargo fmt --manifest-path rust/Cargo.toml --all --check`; `git diff --check`.
- Impact: `rust/src/access/org.rs`, `rust/src/access/org_workflows.rs`, `rust/src/access/rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`.
- Rollback/Risk: only `--with-users` table/CSV/text rendering gains a user-summary column or suffix; scripts parsing fixed three-column table output with `--with-users` should switch to JSON or omit `--with-users`.
- Follow-up: none.

## 2026-04-12 - Add flat CLI help inventory
- Summary: added root `grafana-util --help-flat` output that expands the visible public Clap command tree into a grep-friendly table with command path, group/command kind, and operator-facing purpose text.
- Tests: added CLI help coverage for root pre-parse routing, colorized output, public command inclusion, hidden command exclusion, and rejection of leaked internal `Struct definition` / `Arguments for` wording.
- Test Run: `cargo fmt --manifest-path rust/Cargo.toml --all`; `cargo test --manifest-path rust/Cargo.toml --quiet cli_rust_tests -- --test-threads=1`; `cargo fmt --manifest-path rust/Cargo.toml --all --check`; `git diff --check`; `python3 -m json.tool scripts/contracts/command-surface.json`.
- Validation: manually checked `cargo run --manifest-path rust/Cargo.toml --quiet --bin grafana-util -- --help-flat` and confirmed all public roots render with purpose text and access leaf commands use operator-facing descriptions.
- Impact: `rust/src/cli.rs`, `rust/src/cli_help.rs`, `rust/src/cli_help/routing.rs`, access CLI command definitions, CLI help tests, command-surface contract/checker, command-reference index docs, and maintainer workflow docs that reference root help inventory support.
- Rollback/Risk: root pre-parse now reserves `--help-flat`; command purposes depend on command-level Clap `about` metadata, so new commands should provide product-facing `about` text instead of relying on Args struct comments.
- Follow-up: none.

## 2026-04-12 - Add AI trace maintenance tool
- Summary: added `scripts/ai_trace.py` with structured `add`, `compact`, and `check-size` commands for maintaining AI trace files, and wired trace size enforcement into `scripts/check_ai_workflow.py`.
- Tests: added Python unittest coverage for trace insertion, compact/archive append behavior, size-limit checks, and workflow-gate integration.
- Test Run: `python3 -m unittest -v python.tests.test_python_ai_trace python.tests.test_python_check_ai_workflow`; `python3 scripts/ai_trace.py check-size`; `make quality-ai-workflow`; `git diff --check`.
- Impact: `scripts/ai_trace.py`, `scripts/check_ai_workflow.py`, `python/tests/test_python_ai_trace.py`, `python/tests/test_python_check_ai_workflow.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`, and current AI trace archives after compaction.
- Rollback/Risk: internal maintainer tooling only; rollback removes the helper and the size check, but manual trace maintenance would again be required.
- Follow-up: none.

## 2026-04-12 - Split unified CLI help routing helpers
- Summary: split unified CLI help routing into a thinner orchestration layer plus focused `contextual` and `flat` helper modules, keeping the existing public help entrypoints and inferred-subcommand behavior unchanged.
- Tests: re-ran focused unified help, dashboard help parser, and dashboard inspect/help-full Rust suites after the module split.
- Test Run: `cargo test --manifest-path rust/Cargo.toml --quiet cli_rust_tests`; `cargo test --manifest-path rust/Cargo.toml --quiet dashboard_cli_parser_help_rust_tests`; `cargo test --manifest-path rust/Cargo.toml --quiet dashboard_cli_inspect_help_rust_tests`.
- Impact: `rust/src/cli_help.rs`, `rust/src/cli_help/routing.rs`, `rust/src/cli_help/contextual.rs`, `rust/src/cli_help/flat.rs`, and AI trace docs.
- Rollback/Risk: low to moderate. The refactor is behavior-preserving and covered by focused help tests, but future help work should extend the focused helper modules instead of re-growing `routing.rs` into another mixed-responsibility file.
- Follow-up: none.

## 2026-04-12 - Split snapshot review shaping and browser behavior
- Summary: split snapshot review into shared validation, text rendering, tabular/output shaping, and browser-specific helper modules, keeping the public snapshot review entrypoints unchanged while making `snapshot_review.rs` a thin module hub.
- Tests: relied on the existing snapshot review Rust coverage for behavior; reran the focused snapshot review Rust target after the split.
- Test Run: `cargo fmt --manifest-path rust/Cargo.toml --all`; `cargo test --manifest-path rust/Cargo.toml --quiet snapshot_rust_tests -- --test-threads=1`.
- Impact: `rust/src/snapshot_review.rs`, `rust/src/snapshot_review_common.rs`, `rust/src/snapshot_review_render.rs`, `rust/src/snapshot_review_browser.rs`, `rust/src/snapshot_review_output.rs`.
- Rollback/Risk: low. The refactor is behavior-preserving and only changes module boundaries, but the full crate still has unrelated `access` / `alert` compile failures in the current worktree, so broader verification remains blocked until those existing edits are resolved.
- Follow-up: none.

## 2026-04-12 - Split Rust architecture hotspots and test modules
- Summary: split unified help routing, snapshot review, access rendering, alert CLI runtime/output, and the largest Rust test suites into focused helper modules with thin aggregators.
- Tests: no behavior changes; preserved existing coverage and re-ran focused Rust targets for CLI, dashboard help, access CLI, overview, alert, and snapshot review.
- Test Run: `cargo test --manifest-path rust/Cargo.toml --quiet cli_rust_tests -- --test-threads=1`; `cargo test --manifest-path rust/Cargo.toml --quiet dashboard_cli_parser_help_rust_tests -- --test-threads=1`; `cargo test --manifest-path rust/Cargo.toml --quiet access_cli_rust_tests -- --test-threads=1`; `cargo test --manifest-path rust/Cargo.toml --quiet overview_rust_tests -- --test-threads=1`; `cargo test --manifest-path rust/Cargo.toml --quiet alert_rust_tests -- --test-threads=1`; `cargo test --manifest-path rust/Cargo.toml --quiet snapshot_rust_tests -- --test-threads=1`; `make quality-architecture`; `make quality-ai-workflow`; `git diff --check`.
- Impact: `rust/src/cli_help.rs`, `rust/src/cli_help/routing.rs`, `rust/src/cli_help/contextual.rs`, `rust/src/cli_help/flat.rs`, `rust/src/snapshot_review*.rs`, `rust/src/access/render*.rs`, `rust/src/alert*.rs`, `rust/src/*_rust_tests.rs`, `rust/src/access/*_rust_tests.rs`, and dashboard/overview test children.
- Rollback/Risk: behavior-preserving module-boundary refactor; rollback would collapse the helper modules back into the original large files. Remaining architecture warnings are pre-existing hotspots outside this pass.
- Follow-up: consider a later pass for remaining warnings in access live status/tests, dashboard import/browse/inspect surfaces, datasource status/import-export, `snapshot.rs`, and the remaining brittle help tests in dashboard inspect and sync.

## 2026-04-13 - Improve public docs voice and hygiene
- Summary: refreshed handbook and command-reference wording so docs explain user workflows first, added explicit workflow maps plus task-first guidance sections for alert, dashboard, datasource, access, and status/workspace subcommand families, documented that handbooks should not duplicate one page per leaf command, removed generated-looking `Purpose` / `用途` example-comment labels, removed decorative handbook heading emoji, kept command maps out of handbook bodies, renamed the sidebar command map to command shortcuts, removed handbook chapter-count chrome, and tightened zh-TW product terminology.
- Tests: regenerated command HTML and manpages, checked docs surface, and ran whitespace validation.
- Test Run: `make html`; `make man`; `make html-check`; `make man-check`; `make quality-docs-surface`; `git diff --check`.
- Impact: `README.zh-TW.md`, `docs/commands/{en,zh-TW}/`, `docs/user-guide/{en,zh-TW}/`, generated `docs/html/`, `docs/man/`, `docs/internal/zh-tw-style-guide.md`, `docs/internal/generated-docs-playbook.md`, and AI trace docs.
- Rollback/Risk: docs-only wording and generated-output refresh; broad command-doc comment cleanup touches many files, so review should focus on example captions and generated HTML/man parity.
- Follow-up: consider a later targeted pass for command pages whose examples still share similar captions after the global label cleanup.

## 2026-04-13 - Fix docs portal, landing, and manpage HTML navigation
- Summary: fixed the GitHub Pages root version portal so handbook, command reference, and manpage shortcuts deep-link to the selected output under latest/dev lanes instead of all pointing at the same lane index; removed latest minor duplication from version choices; shortened the local landing page into recommended starts, common jobs, and complete reference; widened the landing layout for common 1366 and 1920 width viewports; moved root manpage router copy into a JSON contract; shortened root manpage subcommand listings; changed generated manpage HTML definition lists from a wide two-column grid to stacked readable entries; rendered root subcommand manpages as collapsible linked groups; linked manpage references in index and SEE ALSO sections; rendered paragraph-style CLI examples as code blocks in HTML; replaced the manpage page left nav with a grouped full manpage index plus documentation lane links instead of the handbook tree; clarified maintainer docs so the published root `index.html` is traceable to the portal contract/generator rather than local `docs/html/index.html`.
- Tests: added focused portal rendering coverage for deep output links, removed old same-lane shortcut labels, checked that the latest minor lane is not repeated as a separate version option, updated HTML generator tests for the current landing and handbook navigation behavior, covered roff `.SS` subsection rendering, collapsible root subcommand groups, manpage cross-links, SEE ALSO links, paragraph-style CLI example recovery, and the dedicated manpage lane nav in generated manpage HTML.
- Test Run: `make man`; `make html`; `python3 -m unittest -v python.tests.test_python_build_pages_site python.tests.test_python_generate_command_html`; `python3 -m unittest -v python.tests.test_python_generate_command_html python.tests.test_python_generate_manpages`; `make html-check`; `make man-check`; `python3 scripts/build_pages_site.py --output-dir /tmp/grafana-util-pages-test --include-dev`; `rg -n "latest/handbook/zh-TW/index.html|latest/commands/zh-TW/index.html|dev/handbook/zh-TW/index.html|dev/commands/zh-TW/index.html|latest/man/index.html|href=\"#outputs\"|先開啟任一版本線|Open a docs lane first|v0.10/index.html" /tmp/grafana-util-pages-test/index.html`; Playwright screenshots at 1366x768 and 1920x1080 with local Chrome; `git diff --check`.
- Impact: `docs/landing/{en,zh-TW}.md`, `scripts/templates/docs.css`, generated `docs/html/`, generated `docs/man/`, `scripts/docsite_html_roff.py`, `scripts/docsite_html_manpage_pages.py`, `scripts/generate_manpages.py`, `scripts/contracts/manpage-router.json`, `scripts/docsite_version_portal.py`, `scripts/build_pages_site.py`, `scripts/contracts/versioned-docs-portal.json`, `python/tests/test_python_build_pages_site.py`, `python/tests/test_python_generate_command_html.py`, `python/tests/test_python_generate_manpages.py`, `docs/internal/generated-docs-playbook.md`, `docs/internal/generated-docs-architecture.md`, and AI trace docs.
- Rollback/Risk: docs navigation and generated HTML/manpage presentation only; lane HTML generation remains on the shared docs generator. Rollback would restore ambiguous portal links, duplicated latest minor version choices, the longer local landing page, table-like manpage command lists, long root subcommand summaries, and non-clickable manpage references.
- Follow-up: none.

## 2026-04-13 - Split Rust facade and CLI-args hotspots
- Summary: split several Rust maintainability hotspots into focused modules while keeping command paths, flags, output contracts, and public runner behavior unchanged; added a read-only maintainability reporter for oversized Rust files and re-export-heavy module roots.
- Tests: preserved existing Rust coverage and added Python coverage for the new reporter.
- Test Run: `cargo fmt --manifest-path rust/Cargo.toml --all`; `cargo fmt --manifest-path rust/Cargo.toml --all --check`; `cd rust && cargo test --quiet --lib`; `python -m unittest -q python.tests.test_python_rust_maintainability_report`; `python3 scripts/rust_maintainability_report.py --root rust/src --source-line-limit 2000 --test-line-limit 3000 --reexport-line-limit 100`.
- Impact: `rust/src/dashboard/command_runner.rs`, new `rust/src/dashboard/run_{list,inspect}.rs`, `rust/src/access/mod.rs`, new `rust/src/access/{dispatch,auth_materialize}.rs`, `rust/src/datasource.rs`, new datasource helper modules, split `rust/src/sync/cli_args*.rs`, `scripts/rust_maintainability_report.py`, Python reporter tests, `docs/overview-rust.md`, and AI trace docs.
- Rollback/Risk: behavior-preserving module-boundary refactor; rollback would collapse helper modules back into the original large facades. Future feature work should extend the new focused modules instead of re-growing root facades.
- Follow-up: run the maintainability reporter periodically and decide later whether to wire it into a quality target with project-specific thresholds.
