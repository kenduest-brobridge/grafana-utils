# ai-status.md

Current AI-maintained status only.

- Older trace history moved to [`archive/ai-status-archive-2026-03-24.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-03-24.md).
- Detailed 2026-03-27 entries moved to [`archive/ai-status-archive-2026-03-27.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-03-27.md).
- Detailed 2026-03-28 task notes were condensed into [`archive/ai-status-archive-2026-03-28.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-03-28.md).
- Detailed 2026-03-29 through 2026-03-31 entries moved to [`archive/ai-status-archive-2026-03-31.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-03-31.md).
- Detailed 2026-04-01 through 2026-04-12 entries moved to [`archive/ai-status-archive-2026-04-12.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-04-12.md).
- Keep this file short and current. Additive historical detail belongs in `docs/internal/archive/`.

## 2026-04-12 - Add AI trace maintenance tool
- State: Done
- Scope: `scripts/ai_trace.py`, `scripts/check_ai_workflow.py`, Python tests, and AI trace docs.
- Baseline: AI trace files require manual entry insertion, size control, and archive movement; `quality-ai-workflow` only checks whether trace files were touched for meaningful internal docs changes.
- Current Update: added a structured AI trace helper with `add`, `compact`, and `check-size` commands, then wired trace length checks into the existing workflow gate.
- Result: AI trace files can now be updated and compacted through one helper instead of manual Markdown movement; `quality-ai-workflow` now fails when current trace files exceed the configured active-entry limits.

## 2026-04-12 - Add flat CLI help inventory
- State: Done
- Scope: unified help routing, CLI help tests, command-surface contract, command reference index docs, and AI trace docs.
- Baseline: grouped `--help` and supported `--help-full` paths exist, but no root-level flat inventory lists every public command path with purpose text.
- Current Update: added `grafana-util --help-flat` as a pre-parse help path that renders visible Clap command paths with group/command kind and purpose.
- Result: root flat help now lists public command paths across status, export, dashboard, datasource, alert, access, workspace, and config with operator-facing purpose text; access leaf command purposes no longer leak Args struct documentation.

## 2026-04-12 - Infer unique long option prefixes
- State: Done
- Scope: `rust/src/cli.rs`, `rust/src/access/cli_defs.rs`, CLI parser tests, and AI trace docs.
- Baseline: unique-prefix matching worked for subcommands, but long options such as `--all-o` only produced a suggestion for `--all-orgs` instead of resolving the unique match.
- Current Update: enabled Clap unique long-argument inference on the unified root parser and access parser, with tests for inferred unique prefixes and rejected ambiguous prefixes.
- Result: `grafana-util access user list --all-o --tab` now parses as `--all-orgs --table`; ambiguous or invalid long prefixes still stay on Clap's error path.

## 2026-04-12 - Show org users in list table output
- State: Done
- Scope: `rust/src/access/org.rs`, `rust/src/access/org_workflows.rs`, access Rust tests, and AI trace docs.
- Baseline: `grafana-util access org list --with-users --table` fetched `/api/orgs/{id}/users` and updated `userCount`, but table and CSV output still rendered only `ID`, `NAME`, and `USER_COUNT`, so operator-visible user names were hidden unless JSON/YAML was used.
- Current Update: added shared org list headers/row helpers so `--with-users` adds user summaries to text, table, and CSV output while default org list output stays unchanged.
- Result: `grafana-util access org list --with-users --table` now includes a `USERS` column with labels such as `alice(Admin); bob(Viewer)`.

## 2026-04-12 - Remove legacy CLI compatibility
- State: Done
- Scope: `rust/src/bin/grafana-util.rs`, `rust/src/cli.rs`, `rust/src/cli_help.rs`, `rust/src/cli_help_examples.rs`, `rust/src/cli_help/grouped_specs.rs`, `rust/src/cli_rust_tests.rs`, `scripts/check_docs_surface.py`, `scripts/contracts/command-surface.json`, command-reference docs, and generated-doc source contracts.
- Baseline: the binary carried a legacy pre-check that intercepted removed roots and emitted replacement hints; `cli.rs` still defined unused old alert grouping schema; the docs surface contract and checker still allowed legacy replacement mappings.
- Current Update: removed the legacy pre-check, deleted the legacy hint module, removed unused old alert grouping schema, removed legacy replacement support from the docs-surface contract/checker, kept alert short help on real flat commands only, and made colored contextual help highlight option entries, inline `--flag` references, and example captions.
- Result: removed command paths now follow the normal Clap rejection path, public command docs/contracts no longer preserve a compatibility mapping for old roots, and CLI help keeps arguments plus example captions visibly highlighted in colored output.

## 2026-04-12 - Re-scope Developer Guide as a maintainer landing page
- State: Done
- Scope: `docs/DEVELOPER.md`, `docs/internal/maintainer-quickstart.md`, `docs/internal/ai-workflow-note.md`, `docs/internal/ai-change-closure-rules.md`, `docs/internal/task-brief-template.md`, `docs/internal/README.md`, plus the repo-maintained AI trace files required by the maintainer-doc workflow gate.
- Current Update: rewrote `docs/DEVELOPER.md` from an oversized mixed router/policy page into a shorter maintainer landing page; tightened `maintainer-quickstart` into the first-entry reading-order and source-of-truth map; moved stable closure rules into a dedicated `ai-change-closure-rules.md`; and routed both the maintainer docs and the AI workflow note to that stable closure file.
- Result: the maintainer entrypoint is now closer to its intended role, the quickstart no longer competes with it as a second guide, and future maintainer-routing changes have both a reusable closure contract and visible router links that reduce dropped updates across maintainer docs.
