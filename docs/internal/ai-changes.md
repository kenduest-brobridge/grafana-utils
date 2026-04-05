# ai-changes.md

Current AI change log only.

- Older detailed history moved to [`archive/ai-changes-archive-2026-03-24.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-changes-archive-2026-03-24.md).
- Detailed 2026-03-27 entries moved to [`archive/ai-changes-archive-2026-03-27.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-changes-archive-2026-03-27.md).
- Detailed 2026-03-28 task notes were condensed into [`archive/ai-changes-archive-2026-03-28.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-changes-archive-2026-03-28.md).
- Keep this file limited to the latest active architecture and maintenance changes.
- Detailed 2026-03-29 through 2026-03-31 entries moved to [`archive/ai-changes-archive-2026-03-31.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-changes-archive-2026-03-31.md).

## 2026-04-05 - Add repo-specific AI workflow note, drift checks, GitHub templates, and AGENTS routing
- Summary: added `docs/internal/ai-workflow-note.md` plus `docs/internal/task-brief-template.md` as the current AI-assisted maintainer workflow layer. The note translates the repo's existing source-of-truth model into an AI workflow and keeps the final review step adaptable to solo or collaborative work. Added `scripts/check_ai_workflow.py` as a lightweight executable guard for a few high-signal drift rules: generated HTML must come with source or generator changes, generated manpages must come with English command-doc or generator changes, and meaningful maintainer/contract/architecture doc updates must also update both `docs/internal/ai-status.md` and `docs/internal/ai-changes.md`. Mirrored the same task-brief fields into `.github/ISSUE_TEMPLATE/ai-task-brief.md` and `.github/PULL_REQUEST_TEMPLATE.md` so collaborative issue and PR flows reuse the same structure instead of inventing a second schema. Exposed the check through `make quality-ai-workflow` and updated `AGENTS.md` so first-entry agents are routed to the workflow note, task brief template, and the new validation target.
- Tests: `python3 -m unittest -v python.tests.test_python_check_ai_workflow`; `python3 scripts/check_ai_workflow.py AGENTS.md docs/internal/ai-workflow-note.md docs/internal/task-brief-template.md scripts/check_ai_workflow.py python/tests/test_python_check_ai_workflow.py Makefile docs/internal/maintainer-quickstart.md docs/internal/README.md docs/internal/ai-status.md docs/internal/ai-changes.md .github/ISSUE_TEMPLATE/ai-task-brief.md .github/PULL_REQUEST_TEMPLATE.md`; `git diff --check`
- Impact: `AGENTS.md`, `docs/internal/ai-workflow-note.md`, `docs/internal/task-brief-template.md`, `.github/ISSUE_TEMPLATE/ai-task-brief.md`, `.github/PULL_REQUEST_TEMPLATE.md`, `scripts/check_ai_workflow.py`, `python/tests/test_python_check_ai_workflow.py`, `Makefile`, `docs/DEVELOPER.md`, `docs/internal/maintainer-quickstart.md`, `docs/internal/README.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: low to moderate. The new script is intentionally conservative, and the GitHub templates are additive, but future workflow changes should keep the local brief template, GitHub templates, and `AGENTS.md` routing aligned instead of letting them drift into parallel formats.
- Follow-up: if the team later wants stronger enforcement in CI, add the same `make quality-ai-workflow` gate to the appropriate GitHub Actions job instead of duplicating the rule logic in YAML.

## 2026-04-04 - Start template-backed HTML shell rendering
- Summary: introduced a minimal file-backed template layer for the generated docs shell. The shared outer page shell, article layout, page header, and right sidebar moved out of inline Python string assembly and into `scripts/templates/*.tmpl`, while `scripts/generate_command_html.py` now loads those templates and fills them with the same existing escaped view-model data. This keeps the current landing/handbook/command content contracts intact but makes shared layout work less entangled with renderer logic.
- Tests: reused the focused HTML generator and landing parser tests to confirm the template-backed renderer still produces the checked-in output tree.
- Test Run: `python3 scripts/generate_command_html.py --write`; `python3 scripts/generate_command_html.py --check`; `python3 -m unittest -v python.tests.test_python_generate_command_html python.tests.test_python_docgen_landing`
- Impact: `scripts/generate_command_html.py`, `scripts/templates/base.html.tmpl`, `scripts/templates/article_layout.html.tmpl`, `scripts/templates/page_header.html.tmpl`, `scripts/templates/right_sidebar.html.tmpl`, `docs/html/**`, `docs/internal/generated-docs-architecture.md`, `docs/internal/generated-docs-playbook.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: low to moderate. The main risk is accidental placeholder/template drift if future shell edits update the Python view model without updating the template files, but the existing checked-output test catches most of that quickly.
- Follow-up: the next logical split is CSS and theme/runtime JS, so `generate_command_html.py` can stop carrying the remaining large asset strings.

## 2026-04-04 - Group handbook sidebar navigation by information architecture
- Summary: separated handbook reading order from handbook sidebar structure. Added explicit sidebar groups in `scripts/docgen_handbook.py` so the HTML nav now reflects the handbook IA instead of flattening every chapter into one list. Updated the renderer to show grouped handbook sections and reduced command reference from a second flat namespace list to one command-docs hub entry, so the handbook sidebar stops mixing subject taxonomy and CLI inventory in the same visual layer.
- Tests: updated generated HTML nav assertions for grouped handbook sections and the single command-reference hub entry.
- Test Run: `python3 scripts/generate_command_html.py --write`; `python3 scripts/generate_command_html.py --check`; `python3 -m unittest -v python.tests.test_python_generate_command_html python.tests.test_python_docgen_landing`
- Impact: `scripts/docgen_handbook.py`, `scripts/generate_command_html.py`, `python/tests/test_python_generate_command_html.py`, `docs/internal/generated-docs-architecture.md`, `docs/internal/generated-docs-playbook.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: moderate. This changes sidebar wayfinding across handbook and command pages, so the main risk is users needing a short adjustment period or maintainers forgetting to keep `HANDBOOK_ORDER` and `HANDBOOK_NAV_GROUPS` aligned when adding chapters.
- Follow-up: if grouped sidebar navigation feels better after regeneration, consider mirroring the same grouping language in the homepage handbook sections so entrypoint IA and in-page navigation stay fully consistent.

## 2026-04-04 - Separate landing-page content from the HTML renderer
- Summary: moved the generated homepage content out of `scripts/generate_command_html.py` into a new public source layer under `docs/landing/{en,zh-TW}.md`. Added `scripts/docgen_landing.py` to parse a small landing-page Markdown contract for hero copy, search copy, task sections, inline reference links, and maintainer links, then rewired the HTML generator to render the landing page from that parsed structure instead of a hardcoded locale data table. The homepage still auto-selects `en` or `zh-TW` from browser language on first load and preserves manual switching in local storage, but the renderer now owns only layout, UI chrome, and build-time version links.
- Tests: added focused landing-parser coverage and updated generated-html coverage to assert the new landing source and locale-switch payload still render correctly.
- Test Run: `python3 -m unittest -v python.tests.test_python_docgen_landing python.tests.test_python_generate_command_html`; `python3 scripts/generate_command_html.py --write`; `python3 scripts/generate_command_html.py --check`
- Impact: `docs/landing/en.md`, `docs/landing/zh-TW.md`, `scripts/docgen_landing.py`, `scripts/generate_command_html.py`, `python/tests/test_python_docgen_landing.py`, `python/tests/test_python_generate_command_html.py`, `docs/html/**`, `docs/internal/generated-docs-architecture.md`, `docs/internal/generated-docs-playbook.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: low to moderate. The main risk is drift between the landing Markdown contract and the parser expectations, but that risk is contained to the homepage and now has direct parser tests. If needed, rollback can revert the landing source layer and parser together without touching handbook or command rendering.
- Follow-up: if the homepage later needs richer structured controls such as explicit search placeholder/button copy or task badges, extend the landing schema in `docgen_landing.py` rather than pushing those values back into `generate_command_html.py`.

## 2026-04-03 - Thin the unified CLI and type the sync apply intent envelope
- Summary: split unified help rendering and long example blocks out of `rust/src/cli.rs` into `rust/src/cli_help.rs` so the root CLI module stays focused on command topology and dispatch. Added `rust/src/sync/apply_contract.rs` as the typed apply-intent envelope shared by the local builder and live execution path, then kept `load_apply_intent_operations` backward-compatible so existing review/render/live callers can still consume lighter JSON documents. Also removed low-signal boilerplate comments from the touched Rust files and updated the maintainer docs to point at the new helper modules.
- Tests: updated sync apply-intent regression coverage and revalidated the full Rust suite after the refactor.
- Test Run: `cd rust && cargo fmt --check`; `cd rust && cargo test --quiet`
- Impact: `rust/src/cli.rs`, `rust/src/cli_help.rs`, `rust/src/lib.rs`, `rust/src/sync/apply_contract.rs`, `rust/src/sync/apply_builder.rs`, `rust/src/sync/live.rs`, `rust/src/sync/live_apply.rs`, `rust/src/sync/workbench.rs`, `rust/src/http.rs`, `rust/src/alert_client.rs`, `rust/src/dashboard/export.rs`, `rust/src/dashboard/live.rs`, `rust/src/sync/preflight.rs`, `rust/src/sync/rust_tests.rs`, `rust/src/sync/live_rust_tests.rs`, `docs/DEVELOPER.md`, `docs/overview-rust.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: moderate but contained to CLI help routing and sync apply-intent parsing. If a regression appears, roll back the `cli_help.rs` extraction or the `apply_contract.rs` introduction as isolated steps rather than reverting unrelated sync or dashboard logic.
- Follow-up: `cargo clippy --all-targets -- -D warnings` still fails on pre-existing issues in `rust/src/profile_cli_defs.rs` and `rust/src/profile_secret_store.rs`, outside this change scope.

## 2026-04-03 - Add dashboard raw-to-prompt migration workflow
- Summary: added a dedicated `grafana-util dashboard raw-to-prompt` surface for converting ordinary dashboard JSON or `raw/` lane files into Grafana UI prompt JSON with `__inputs`. The new runtime handles repeatable `--input-file`, `--input-dir`, sibling/default output rules, `--output-format`, `--log-file`, `--log-format`, `--color`, `--dry-run`, `infer-family|exact|strict` datasource resolution, and optional live datasource lookup through `--profile` or direct live auth flags. It also writes prompt-lane `index.json` plus `export-metadata.json` when converting a `raw/` directory tree and documents that `prompt/` is for Grafana UI import, not API import.
- Tests: added focused dashboard parser/runtime tests for the new command and regenerated the man/html docs to keep the public command references in sync.
- Test Run: `cd rust && CARGO_INCREMENTAL=0 cargo test raw_to_prompt --quiet`; `cargo fmt --manifest-path rust/Cargo.toml --all`; `cargo fmt --manifest-path rust/Cargo.toml --all --check`; `make man`; `make html`; `make man-check`; `make html-check`; `git diff --check`
- Impact: `rust/src/dashboard/cli_defs_command.rs`, `rust/src/dashboard/cli_defs_shared.rs`, `rust/src/dashboard/raw_to_prompt.rs`, `rust/src/dashboard/raw_to_prompt_rust_tests.rs`, `rust/src/dashboard/mod.rs`, `rust/src/dashboard/test_support.rs`, `rust/src/cli.rs`, `rust/src/cli_help.rs`, `rust/src/cli_help_examples.rs`, `rust/src/cli_rust_tests.rs`, `rust/src/dashboard/dashboard_cli_parser_help_rust_tests.rs`, `README.md`, `README.zh-TW.md`, `docs/commands/en/dashboard.md`, `docs/commands/en/dashboard-export.md`, `docs/commands/en/dashboard-import.md`, `docs/commands/en/dashboard-raw-to-prompt.md`, `docs/commands/en/index.md`, `docs/commands/zh-TW/dashboard.md`, `docs/commands/zh-TW/dashboard-export.md`, `docs/commands/zh-TW/dashboard-import.md`, `docs/commands/zh-TW/dashboard-raw-to-prompt.md`, `docs/commands/zh-TW/index.md`, `docs/user-guide/en/dashboard.md`, `docs/user-guide/en/reference.md`, `docs/user-guide/zh-TW/dashboard.md`, `docs/user-guide/zh-TW/reference.md`, `docs/man/*.1`, `docs/html/**`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: moderate. The main risk is datasource inference for ambiguous SQL/search/tracing families, which intentionally still requires better source metadata or an explicit `--datasource-map`. If runtime behavior regresses, roll back `rust/src/dashboard/raw_to_prompt.rs` and the dispatch wiring as one slice.
- Follow-up: broaden from the focused `raw_to_prompt` slice to full `cargo test --quiet` and `cargo clippy --all-targets -- -D warnings` once the remaining worktree changes are settled.

## 2026-04-03 - Add maintainer quickstart for first-entry repo orientation
- Summary: added a dedicated maintainer quickstart page so the next AI agent or new maintainer can enter the repo through one short route instead of bouncing between README, `docs/DEVELOPER.md`, generated-doc notes, and the internal docs index. The new page explains which files to open first, what the current maintained surfaces are, where source-of-truth layers live, which outputs are generated, how to route common task types, and which validation commands are safe to run first.
- Tests: Not run. Documentation-only update.
- Impact: `README.md`, `README.zh-TW.md`, `docs/DEVELOPER.md`, `docs/internal/README.md`, `docs/internal/maintainer-quickstart.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: low. This is an orientation-only docs change, but future maintainer-routing changes should update this page in the same patch so it stays more useful than a generic repo overview.
- Follow-up: none.

## 2026-04-03 - Document generated docs architecture for maintainers
- Summary: added a dedicated internal design document for the Markdown-to-manpage and Markdown-to-HTML pipeline so maintainers can understand the current source-of-truth model, generator split, supported Markdown subset, locale policy, cross-linking rules, test flow, and GitHub Pages deployment without reverse-engineering the scripts.
- Tests: Not run. Documentation-only update.
- Impact: `docs/DEVELOPER.md`, `docs/internal/generated-docs-architecture.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: low. This is a documentation-only architecture clarification, but future generator changes should keep this design note current so it remains more useful than the script docstrings alone.
- Follow-up: when the docs schema or generator split changes, update this design doc in the same patch instead of letting knowledge drift back into code-only comments.

## 2026-04-03 - Add generated docs maintainer playbook
- Summary: added a task-oriented playbook for the generated docs system so maintainers have direct recipes for the common changes: adding command pages, adding handbook chapters, wiring command-to-handbook links, adding namespace manpages, introducing locales, changing generated output inventory, and validating the result. Linked it from `docs/DEVELOPER.md` and the architecture note so the maintainer path is now design first, task cookbook second.
- Tests: Not run. Documentation-only update.
- Impact: `docs/DEVELOPER.md`, `docs/internal/generated-docs-architecture.md`, `docs/internal/generated-docs-playbook.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: low. This is documentation-only, but future generated-docs changes should update the playbook in the same patch so it stays operationally accurate.
- Follow-up: none.

## 2026-04-03 - Reorganize DEVELOPER.md as a maintainer routing map
- Summary: reshaped `docs/DEVELOPER.md` from a compact note page into a clearer maintainer router. The file now starts with task-based entry guidance, then splits code architecture, documentation layers, validation/build flow, project rules, and quick routing into separate sections so maintainers can jump directly to the right surface.
- Tests: Not run. Documentation-only update.
- Impact: `docs/DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: low. This is a structure-only docs update, but future edits should preserve the routing-first shape instead of drifting back into an undifferentiated note list.
- Follow-up: none.

## 2026-04-03 - Tighten maintainer guidance for comment signal and facade thinning
- Summary: added a short maintainer policy for the Rust layer that prefers repo-owned typed envelopes over ad hoc shapes, keeps facades thin, and uses comments only for ownership, invariants, or other non-obvious behavior. The same wording now appears in the Rust overview, the maintainer quickstart, and the maintainer summary so the guidance stays easy to find.
- Tests: Not run. Documentation-only update.
- Impact: `docs/DEVELOPER.md`, `docs/overview-rust.md`, `docs/internal/maintainer-quickstart.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: low. This is guidance-only documentation, but future maintainer docs should keep the comment-signal and facade-thinning advice aligned so the summary stays consistent.
- Follow-up: none.

## 2026-04-03 - Document profile secret storage across user and maintainer docs
- Summary: filled the secret-storage documentation gap by adding a dedicated internal architecture note for profile secret handling and expanding the user-facing handbook/reference docs. The docs now explain what each secret mode is, why it exists, when to use it, macOS and Linux backend behavior for `os` storage, the main limits, and common troubleshooting paths. README and maintainer indexes now also point to the new secret-storage note so the topic has a clear entrypoint.
- Tests: Not run. Documentation-only update.
- Impact: `README.md`, `README.zh-TW.md`, `docs/DEVELOPER.md`, `docs/internal/README.md`, `docs/internal/profile-secret-storage-architecture.md`, `docs/user-guide/en/reference.md`, `docs/user-guide/zh-TW/reference.md`, `docs/commands/en/profile.md`, `docs/commands/zh-TW/profile.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: low. This is documentation-only, but future changes to secret backends, path rules, or profile resolution should update these notes together so operator guidance stays aligned with implementation.
- Follow-up: if the repo later adds another secret backend or platform-specific behavior, extend this note before adding examples that assume the new mode exists.

## 2026-04-03 - Tighten dashboard raw-to-prompt semantic compatibility
- Summary: tightened the shared dashboard prompt builder so `dashboard raw-to-prompt` better matches historical prompt-lane semantics. Single-family dashboards now keep the Grafana-style datasource template variable even when several datasource slots exist, generic `type: datasource` / `-- Mixed --` selectors now become prompt slots without rewriting builtin Grafana annotation selectors, and `__requires` now preserves one datasource requirement per prompt slot instead of deduplicating by plugin family. Added a semantic compare helper to replay historical prompt bundles against regenerated output. The final historical edge case now uses a migrate-only post-processing step in `raw_to_prompt.rs`: after building prompt JSON, it rewrites only those panel-subtree datasource paths that were placeholders in the raw dashboard back to `$datasource`, which preserves the legacy panel/target placeholder mix without changing live export behavior.
- Tests: added focused raw-to-prompt regressions for single-family templating, mixed datasource selectors, builtin Grafana annotation selectors, and datasource-variable slot reuse; reran the focused Rust raw-to-prompt slice; and replayed the Pontus dashboard export sample through the compare script until the historical prompt bundle reached full semantic parity.
- Test Run: `cargo fmt --manifest-path rust/Cargo.toml --all`; `cd rust && CARGO_INCREMENTAL=0 cargo test raw_to_prompt --quiet`; `python3 ./scripts/compare_prompt_semantics.py --expected-root /Users/kendlee/Downloads/2/Pontus_20260312_extracted/grafana-prod-dashboard-20260312-1/dashboard --generated-root /tmp/pontus-raw-to-prompt-check-v8 --show-limit 20`
- Impact: `rust/src/dashboard/prompt.rs`, `rust/src/dashboard/raw_to_prompt_rust_tests.rs`, `scripts/compare_prompt_semantics.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: medium-low. The changes are isolated to prompt-lane generation semantics, but future prompt-builder work should re-run semantic comparisons against historical export bundles so datasource templating and mixed-selector behavior do not drift again.
- Follow-up: none.

## 2026-04-03 - Add role-based doc entrypoints for operators and maintainers
- Summary: upgraded the docs from file-family navigation to a hybrid model with role-based entrypoints. Added short public role pages for new users, SRE / operators, and automation / CI readers in English and Traditional Chinese, added a maintainer-role map under `docs/internal/`, inserted the role pages into handbook ordering, and updated README, handbook indexes, `docs/DEVELOPER.md`, `docs/internal/README.md`, and the generated HTML landing page to route by persona as well as by document type.
- Tests: regenerated the HTML docs site and ran the generated-doc determinism checks plus `git diff --check`.
- Impact: `README.md`, `README.zh-TW.md`, `docs/user-guide/en/index.md`, `docs/user-guide/zh-TW/index.md`, `docs/user-guide/en/role-new-user.md`, `docs/user-guide/en/role-sre-ops.md`, `docs/user-guide/en/role-automation-ci.md`, `docs/user-guide/zh-TW/role-new-user.md`, `docs/user-guide/zh-TW/role-sre-ops.md`, `docs/user-guide/zh-TW/role-automation-ci.md`, `docs/DEVELOPER.md`, `docs/internal/README.md`, `docs/internal/maintainer-role-map.md`, `scripts/docgen_handbook.py`, `scripts/generate_command_html.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: low. This is still docs-only, but future navigation changes should keep the role paths synchronized across README, handbook indexes, and the generated HTML landing page.
- Follow-up: after this lands, do a second-pass content review to find any role pages or entrypoint sections that feel too thin or purely ceremonial and enrich them with more concrete operator detail.

## 2026-04-03 - Split default and browser-enabled Rust release artifacts
- Summary: changed the Rust feature/build policy so the default artifact is lean and omits `browser`, while browser capture support now ships through explicit `*-browser` build targets and release assets. Updated the Makefile, build scripts, install script, Linux artifact validator, CI release jobs, and maintainer notes to keep the standard and browser-enabled artifacts separate.
- Tests: updated browser-disabled screenshot code to allow dead code cleanly when `browser` is off, then validated the standard and browser-enabled compile paths plus the feature-disabled screenshot behavior.
- Test Run: `bash -n scripts/build-rust-macos-arm64.sh scripts/build-rust-linux-amd64.sh scripts/build-rust-linux-amd64-zig.sh scripts/validate-rust-linux-amd64-artifact.sh scripts/install.sh`; `make help`; `cargo check --quiet --manifest-path rust/Cargo.toml`; `cargo check --quiet --manifest-path rust/Cargo.toml --features browser`; `cargo test --quiet --manifest-path rust/Cargo.toml capture_dashboard_screenshot_reports_missing_browser_support`; `git diff --check -- rust/Cargo.toml Makefile scripts/build-rust-macos-arm64.sh scripts/build-rust-linux-amd64.sh scripts/build-rust-linux-amd64-zig.sh scripts/validate-rust-linux-amd64-artifact.sh scripts/install.sh .github/workflows/ci.yml docs/DEVELOPER.md docs/internal/ai-status.md docs/internal/ai-changes.md rust/src/dashboard/screenshot.rs rust/src/dashboard/screenshot_runtime.rs rust/src/dashboard/screenshot_header.rs`
- Impact: `rust/Cargo.toml`, `Makefile`, `scripts/build-rust-macos-arm64.sh`, `scripts/build-rust-linux-amd64.sh`, `scripts/build-rust-linux-amd64-zig.sh`, `scripts/validate-rust-linux-amd64-artifact.sh`, `scripts/install.sh`, `.github/workflows/ci.yml`, `docs/DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: medium. This changes release artifact expectations and installer naming, so standard/browser archive names and feature flags must stay aligned across scripts and CI.
- Follow-up: update release-facing docs and installer usage examples so users can discover the new browser-enabled artifact flavor without reading maintainer notes.

## 2026-04-02 - Consolidate contract docs into summary/spec/trace layers
- Summary: reorganized the active contract documentation into three layers. `docs/DEVELOPER.md` now stays at short maintainer-summary level, dedicated `docs/internal/*` contract docs now hold current detailed requirements, and `ai-status.md` / `ai-changes.md` stay trace-oriented. Added a contract-doc map to make the navigation explicit.
- Tests: Not run. Documentation-only update.
- Impact: `docs/DEVELOPER.md`, `docs/internal/contract-doc-map.md`, `docs/internal/export-root-output-layering-policy.md`, `docs/internal/dashboard-export-root-contract.md`, `docs/internal/datasource-masked-recovery-contract.md`, `docs/internal/alert-access-contract-policy.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: low. This is a documentation-structure cleanup, but future contract work should keep using the same three-layer split instead of rebuilding overlapping note fragments.
- Follow-up: none.

## 2026-04-02 - Clarify export-root/output layering scope
- Summary: added a short maintainer note that reserves the explicit export-root/output-layering pattern for `dashboard` and `datasource`, with the detailed `alert` / `access` boundary rules now delegated to a dedicated policy doc instead of being repeated inline.
- Tests: Not run. Documentation-only update.
- Impact: `docs/DEVELOPER.md`, `docs/internal/export-root-output-layering-policy.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: low. This is a scope clarification only, but future docs should keep the dashboard/datasource split explicit rather than implying a shared helper across every resource kind.
- Follow-up: extend the same wording only when a new dashboard or datasource export/output variant needs it.

## 2026-04-02 - Clarify alert/access promotion criteria
- Summary: added a dedicated internal policy doc for the two non-export-root domains. That doc now owns the detailed `alert` / `access` contract types, promotion criteria, and documentation-guidance rules so maintainer summaries and trace files can stay short and point back to one current requirements source.
- Tests: Not run. Documentation-only update.
- Impact: `docs/DEVELOPER.md`, `docs/internal/alert-access-contract-policy.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: low. This is policy clarification only, but future alert/access work should either stay inside the current bundle/resource-tree contracts or explicitly promote the domain before adding root-contract vocabulary.
- Follow-up: none.

## 2026-04-02 - Formalize dashboard export-root contract
- Summary: moved the detailed dashboard export-root requirements into a dedicated current contract doc. `docs/DEVELOPER.md` now keeps only the short summary, while the dedicated spec owns the stable root-manifest fields, scope semantics, summary/output-layering rule, and compatibility guidance.
- Tests: Not run. Documentation-only update.
- Impact: `docs/DEVELOPER.md`, `docs/internal/dashboard-export-root-contract.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: low. This is policy documentation only, but future dashboard export or inspect work should keep the `raw/` vs `provisioning/` split and the summary/report layering aligned with the written contract.
- Follow-up: none.

## 2026-04-02 - Close datasource masked-recovery bookkeeping
- Summary: retired the datasource masked-recovery lane from the active backlog and added a concise maintainer note that keeps `datasources.json` as the canonical replay contract while treating `provisioning/datasources.yaml` as a projection. The docs also keep the secret boundary explicit so future inspect/output wording does not drift back toward plaintext `secureJsonData`.
- Tests: Not run. Documentation-only update.
- Impact: `TODO.md`, `docs/DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: low. This is bookkeeping only, but future datasource docs should keep the canonical/recovery/projection split aligned.
- Follow-up: none.

## 2026-04-02 - Formalize datasource masked-recovery schema policy
- Summary: moved the detailed datasource masked-recovery schema policy into a dedicated current contract doc. The spec now owns the stable root-manifest and record fields, projection rule, additive evolution rules, and `schemaVersion` guidance, while `docs/DEVELOPER.md` keeps only the summary.
- Tests: Not run. Documentation-only update.
- Impact: `docs/DEVELOPER.md`, `docs/internal/datasource-masked-recovery-contract.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: low. This is policy documentation only, but future schema changes should follow the documented additive-versus-breaking distinction instead of relying on ad hoc compatibility calls.
- Follow-up: none.

## 2026-04-01 - Add repo-owned install script for release binaries
- Summary: added a POSIX `scripts/install.sh` installer so operators can fetch the published Rust release binary with one command instead of manually opening release assets or compiling from source. The installer resolves the current platform, supports `linux-amd64` and `macos-arm64`, installs to `/usr/local/bin` when writable or `~/.local/bin` otherwise, and accepts explicit `BIN_DIR`, `VERSION`, `REPO`, and `ASSET_URL` overrides for pinned or test installs. Public English and Traditional Chinese docs now advertise the `curl ... | sh` path plus the local-checkout fallback.
- Tests: added a focused Python packaging-style test that verifies the release download contract in the script and exercises an offline install using a local `file://` tarball override.
- Test Run: `PYTHONPATH=python python3 -m unittest -v python/tests/test_python_packaging.py python/tests/test_python_install_script.py`
- Impact: `scripts/install.sh`, `python/tests/test_python_install_script.py`, `README.md`, `README.zh-TW.md`, `docs/user-guide.md`, `docs/user-guide-TW.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: low. The installer is additive and doc-driven, but release asset naming in CI and the installer script must stay aligned or the one-line install path will drift.
- Follow-up: if maintainers add more published release targets later, extend the platform map in `scripts/install.sh` and keep the docs examples unchanged.

## 2026-04-01 - Record baseline-five live defaults and dashboard review output inventory
- Summary: recorded the current Rust CLI split after the profile and dashboard-authoring waves. The shared live connection baseline now covers `dashboard`, `datasource`, `access`, `alert`, and `status live`, while dashboard `get`, `clone-live`, `patch-file`, `publish`, and `review` stay intentionally specialized. The dashboard review output contract is now explicit across text, table, CSV, JSON, and YAML, with text remaining the default.
- Tests: Not run. Documentation-only update.
- Impact: `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: low. This records the current contract split without changing runtime behavior, but future CLI help/docs should keep the baseline-five list and the dashboard-only lane names aligned with implementation.
- Follow-up: none.

## 2026-04-01 - Extend alert list output formats
- Summary: widened the four Rust alert list surfaces so they now normalize and render `text`, `table`, `csv`, `json`, and `yaml` output modes consistently. The list help examples now advertise text and YAML alongside the existing table/CSV/JSON paths, and the runtime renderer now emits YAML through the shared YAML helper while keeping table semantics intact.
- Tests: added focused parser coverage for all four list subcommands plus output-format normalization, and added a rendering regression that exercises text, CSV, JSON, and YAML output paths.
- Test Run: `cargo test --manifest-path rust/Cargo.toml --quiet alert -- --test-threads=1` failed during crate compilation before the alert list tests could run.
- Reason: the repository currently has unrelated compile failures in `src/dashboard/authoring.rs`, `src/dashboard/mod.rs`, and `src/datasource.rs`, so the focused alert test slice cannot complete cleanly in the current worktree.
- Impact: `rust/src/alert_cli_defs.rs`, `rust/src/alert_list.rs`, `rust/src/alert_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: low. The change is additive and scoped to list-only surfaces, but future alert CLI help should keep the output-mode examples aligned with the parser so text/YAML do not drift out of the advertised contract.
- Follow-up: none.
