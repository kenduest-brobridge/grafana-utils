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
