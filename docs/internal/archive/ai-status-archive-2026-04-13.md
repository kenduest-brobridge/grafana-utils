# ai-status-archive-2026-04-13

## 2026-04-12 - Infer unique long option prefixes
- State: Done
- Scope: `rust/src/cli.rs`, `rust/src/access/cli_defs.rs`, CLI parser tests, and AI trace docs.
- Baseline: unique-prefix matching worked for subcommands, but long options such as `--all-o` only produced a suggestion for `--all-orgs` instead of resolving the unique match.
- Current Update: enabled Clap unique long-argument inference on the unified root parser and access parser, with tests for inferred unique prefixes and rejected ambiguous prefixes.
- Result: `grafana-util access user list --all-o --tab` now parses as `--all-orgs --table`; ambiguous or invalid long prefixes still stay on Clap's error path.

## 2026-04-12 - Add flat CLI help inventory
- State: Done
- Scope: unified help routing, CLI help tests, command-surface contract, command reference index docs, and AI trace docs.
- Baseline: grouped `--help` and supported `--help-full` paths exist, but no root-level flat inventory lists every public command path with purpose text.
- Current Update: added `grafana-util --help-flat` as a pre-parse help path that renders visible Clap command paths with group/command kind and purpose.
- Result: root flat help now lists public command paths across status, export, dashboard, datasource, alert, access, workspace, and config with operator-facing purpose text; access leaf command purposes no longer leak Args struct documentation.

## 2026-04-12 - Add AI trace maintenance tool
- State: Done
- Scope: `scripts/ai_trace.py`, `scripts/check_ai_workflow.py`, Python tests, and AI trace docs.
- Baseline: AI trace files require manual entry insertion, size control, and archive movement; `quality-ai-workflow` only checks whether trace files were touched for meaningful internal docs changes.
- Current Update: added a structured AI trace helper with `add`, `compact`, and `check-size` commands, then wired trace length checks into the existing workflow gate.
- Result: AI trace files can now be updated and compacted through one helper instead of manual Markdown movement; `quality-ai-workflow` now fails when current trace files exceed the configured active-entry limits.

## 2026-04-12 - Split snapshot review shaping and browser behavior
- State: Done
- Scope: `rust/src/snapshot_review.rs`, new `rust/src/snapshot_review_common.rs`, `rust/src/snapshot_review_render.rs`, `rust/src/snapshot_review_browser.rs`, `rust/src/snapshot_review_output.rs`, and snapshot review coverage in `rust/src/snapshot_rust_tests.rs`.
- Baseline: `snapshot_review.rs` still mixed text rendering, tabular shaping, browser item shaping, and interactive browser dispatch in one file.
- Current Update: split shared validation, text rendering, table/output shaping, and browser-specific behavior into separate helper modules; kept the public snapshot review entrypoints unchanged.
- Result: snapshot review responsibilities are now thinner and easier to extend; targeted Rust verification hit unrelated pre-existing `access` / `alert` compile errors in the current worktree, but no new `snapshot_review` errors remained.

## 2026-04-12 - Split unified CLI help routing helpers
- State: Done
- Scope: `rust/src/cli_help.rs`, `rust/src/cli_help/routing.rs`, new `rust/src/cli_help/*` helper modules, Rust CLI help tests, and AI trace docs.
- Baseline: `rust/src/cli_help/routing.rs` still mixes orchestration, flat help inventory rendering, contextual clap help shaping, option-heading inference, ANSI stripping, and inferred-subcommand normalization in one large file.
- Current Update: kept `routing.rs` as the orchestration layer, moved contextual clap help shaping plus inferred-heading logic into `cli_help/contextual.rs`, and moved flat inventory rendering into `cli_help/flat.rs` without changing unified help entrypoints.
- Result: unified help routing now has clearer seams between routing, contextual rendering, and flat inventory rendering; focused Rust help tests and `dashboard` help-full coverage still pass after the split.

## 2026-04-12 - Split Rust architecture hotspots and test modules
- State: Done
- Scope: `rust/src/alert.rs`, `rust/src/access/render.rs`, `rust/src/cli_help/routing.rs`, `rust/src/snapshot_review.rs`, and split Rust test modules for CLI, access, alert, dashboard help, and overview coverage.
- Current Update: Split large orchestration/render/test surfaces into focused helper modules and thin aggregators while preserving public command behavior and test contracts.
- Result: Focused Rust tests pass; `make quality-architecture` now reports 17 warnings, down from the pre-refactor 23, with remaining warnings limited to untouched hotspots and two existing brittle help-test files.

## 2026-04-13 - Improve public docs voice and hygiene
- State: Done
- Scope: README zh-TW, command reference docs, handbook source, generated HTML/man output, zh-TW style guide, and generated-docs playbook.
- Current Update: removed generated-looking example-comment labels, removed handbook emoji headings, tightened zh-TW product terminology, expanded reader-oriented handbook framing, added handbook workflow maps and task-first sections for alert, dashboard, datasource, access, and status/workspace, kept command maps out of handbook bodies, renamed the sidebar command map to command shortcuts, removed handbook chapter-count chrome, and pointed English handbook links at specific command pages.
- Result: public docs read less like generated summaries while keeping command reference pages lookup-oriented; core handbook pages now explain subcommand relationships, output interpretation, and next-step decisions before sending readers to exact flag references; generated HTML/man outputs are refreshed and docs surface checks pass.

## 2026-04-13 - Fix docs portal, landing, and manpage HTML navigation
- State: Done
- Scope: GitHub Pages version portal generator, portal copy contract, local landing source/CSS, generated HTML/manpages, manpage router contract, manpage HTML renderer/CSS, Pages assembly script, focused script tests, and generated-docs maintainer notes.
- Baseline: the published Pages root portal was generated outside `docs/html/`, but maintainer docs still described Pages as publishing the local `docs/html/` tree; portal output shortcuts also pointed readers back to the same lane index instead of specific handbook, command, or manpage pages.
- Current Update: deep-linked portal output shortcuts by lane and locale, removed latest minor duplication from version choices, shortened the local landing page into recommended starts/common jobs/complete reference, widened the landing layout for 1366 and 1920 widths, moved root manpage router copy into a JSON contract, shortened root subcommand listings to purpose-only summaries, rendered manpage `.SS` subsections as HTML subheadings, changed manpage definition lists from a wide two-column grid to readable stacked entries, converted the root manpage subcommand index into collapsible linked groups, linked manpage references in index and SEE ALSO sections, rendered stray paragraph-style CLI examples as code blocks in HTML, replaced the handbook tree on manpage pages with a grouped full manpage index plus documentation lane links, and documented that the Pages root portal is generated by `scripts/docsite_version_portal.py` while lane pages still use the shared HTML generator.
- Result: Pages root navigation now distinguishes latest/dev lanes and direct output types; the local `docs/html/index.html` has a clearer, shorter 1080p-friendly layout; generated manpage HTML no longer reads like a broken table for long command descriptions and its manpage references are clickable; maintainers can find the local source for the published root `index.html`.
