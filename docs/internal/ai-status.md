# ai-status.md

Current AI-maintained status only.

- Older trace history moved to [`archive/ai-status-archive-2026-03-24.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-03-24.md).
- Detailed 2026-03-27 entries moved to [`archive/ai-status-archive-2026-03-27.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-03-27.md).
- Detailed 2026-03-28 task notes were condensed into [`archive/ai-status-archive-2026-03-28.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-03-28.md).
- Keep this file short and current. Additive historical detail belongs in `docs/internal/archive/`.
- Detailed 2026-03-29 through 2026-03-31 entries moved to [`archive/ai-status-archive-2026-03-31.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-03-31.md).

## 2026-04-05 - Add repo-specific AI workflow note, drift checks, GitHub templates, and AGENTS routing
- State: Done
- Scope: `AGENTS.md`, `docs/internal/ai-workflow-note.md`, `docs/internal/task-brief-template.md`, `.github/ISSUE_TEMPLATE/ai-task-brief.md`, `.github/PULL_REQUEST_TEMPLATE.md`, `scripts/check_ai_workflow.py`, `python/tests/test_python_check_ai_workflow.py`, `Makefile`, `docs/DEVELOPER.md`, `docs/internal/maintainer-quickstart.md`, `docs/internal/README.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the repo already had strong maintainer routing, contract layering, generated-doc rules, and AI trace files, but it did not yet have one current note that translated those existing rules into a concrete workflow for AI-assisted maintenance. It also lacked a stable task brief shape, any repo-owned automated check for the most obvious source/generated/trace drift rules, and any GitHub-native place to reuse the same task fields during collaborative review.
- Current Update: added a dedicated internal workflow note that maps AI-assisted maintenance onto the repo's existing raw-source, maintained-knowledge, and workflow-schema layers. The note defines repo-specific `ingest`, `query`, and `lint` expectations, reinforces generated-versus-source boundaries, gives a minimal task-brief shape for agent work, and frames the final review step as `Diff Review` for solo work or `PR Review` for collaborative work. Added `task-brief-template.md` as a reusable handoff shape, mirrored the same fields into `.github/ISSUE_TEMPLATE/ai-task-brief.md` and `.github/PULL_REQUEST_TEMPLATE.md`, added `scripts/check_ai_workflow.py` as a lightweight drift checker for current-path changes, and wired the new AI workflow entrypoints and `make quality-ai-workflow` validation path into `AGENTS.md` so first-entry agents see the repo-specific workflow immediately.
- Result: the repo now has a written AI workflow, a small executable enforcement layer, GitHub-native templates that keep issue and PR context aligned with the same repo-specific task brief shape, and top-level agent routing that points new agents at the workflow instead of relying on discovery by accident.

## 2026-04-04 - Start template-backed HTML shell rendering
- State: Done
- Scope: `scripts/generate_command_html.py`, `scripts/templates/base.html.tmpl`, `scripts/templates/article_layout.html.tmpl`, `scripts/templates/page_header.html.tmpl`, `scripts/templates/right_sidebar.html.tmpl`, `python/tests/test_python_generate_command_html.py`, `python/tests/test_python_docgen_landing.py`, `docs/internal/generated-docs-architecture.md`, `docs/internal/generated-docs-playbook.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the HTML renderer had already separated some content metadata, but the shared page shell, article layout, header block, and right sidebar were still assembled inline as large Python strings inside `scripts/generate_command_html.py`. That kept layout work tightly coupled to renderer logic and made simple shell edits noisy.
- Current Update: added a minimal file-backed template layer under `scripts/templates/` and moved the shared shell markup, article layout, page header, and right sidebar into template files. `generate_command_html.py` now loads those templates, fills them with existing escaped view data, and keeps the same content contracts for handbook, landing, and command pages.
- Result: the renderer now has a clearer separation between view-model assembly and shared shell markup, while generated output stays in sync with the checked-in HTML tree and focused generator tests still pass.

## 2026-04-04 - Group handbook sidebar navigation by information architecture
- State: Done
- Scope: `scripts/docgen_handbook.py`, `scripts/generate_command_html.py`, `python/tests/test_python_generate_command_html.py`, `docs/internal/generated-docs-architecture.md`, `docs/internal/generated-docs-playbook.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the HTML sidebar rendered every handbook chapter from one flat `HANDBOOK_ORDER` sequence, then appended a second flat command list underneath. That mixed onboarding, role paths, asset guides, governance chapters, and command reference into one scan surface, which made the handbook feel fragmented even when individual pages were fine.
- Current Update: split handbook navigation concerns into two metadata layers. `HANDBOOK_ORDER` still owns linear reading order for previous/next, while new sidebar groups in `scripts/docgen_handbook.py` now define the actual information architecture shown in the HTML nav. The renderer now reads those groups and reduces command reference to a single hub entry instead of a second mini index.
- Result: handbook and command pages now share a grouped handbook sidebar that matches the handbook index IA more closely, while command docs are represented as one secondary entrypoint instead of a competing taxonomy. Validation passed with regenerated HTML plus focused landing and HTML generator tests.

## 2026-04-04 - Separate landing-page content from the HTML renderer
- State: Done
- Scope: `docs/landing/`, `scripts/docgen_landing.py`, `scripts/generate_command_html.py`, `python/tests/test_python_docgen_landing.py`, `python/tests/test_python_generate_command_html.py`, `docs/internal/generated-docs-architecture.md`, `docs/internal/generated-docs-playbook.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the generated homepage had already moved to a task-oriented layout, but the actual landing-page copy, task order, locale strings, and curated links were still hardcoded directly in `scripts/generate_command_html.py`. That made simple homepage content edits look like renderer changes and blurred the source-of-truth boundary.
- Current Update: added a dedicated `docs/landing/{en,zh-TW}.md` source layer plus `scripts/docgen_landing.py` to parse a fixed Markdown contract for hero copy, search copy, task sections, and maintainer links. The HTML generator now renders the landing page from those parsed structures, keeps only UI chrome and version metadata in Python, and still auto-selects `en` or `zh-TW` on first homepage load while preserving manual switching in local storage.
- Result: homepage content now lives in Markdown instead of hardcoded Python, the landing renderer is materially thinner, and landing-page maintenance follows the same metadata-vs-renderer split already used by the handbook generator.

## 2026-04-03 - Tighten dashboard raw-to-prompt semantic compatibility
- State: Done
- Scope: `rust/src/dashboard/prompt.rs`, `rust/src/dashboard/raw_to_prompt_rust_tests.rs`, `scripts/compare_prompt_semantics.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the first `dashboard raw-to-prompt` implementation produced usable prompt JSON, but when compared against a real historical export bundle it only matched 40 of 83 prompt dashboards semantically. The main drift was missing datasource template variables, treating generic `type: datasource` or `-- Mixed --` selectors incorrectly, and collapsing repeated same-family datasource requirements too aggressively.
- Current Update: updated the shared prompt builder so single-family dashboards preserve the Grafana-style datasource templating variable even when multiple prompt slots exist, generic/mixed datasource selectors now become prompt slots without rewriting builtin Grafana annotation selectors, and `__requires` now keeps one datasource entry per prompt slot instead of deduplicating by plugin family. Added focused regression tests for single-family templating, mixed-selector rewriting, and builtin Grafana annotation handling, plus a semantic compare script for replaying historical prompt bundles against generated output. The final historical edge case is now handled in the `raw-to-prompt` runtime itself: it records which panel-subtree datasource paths were placeholders in the raw dashboard and rewrites only those same prompt-output paths back to `$datasource`.
- Result: on the Pontus dashboard export sample, semantic compatibility improved from 40/83 prompt dashboards to 83/83 without modifying the source bundle.

## 2026-04-03 - Thin the unified CLI and type the sync apply intent envelope
- State: Done
- Scope: `rust/src/cli.rs`, `rust/src/cli_help.rs`, `rust/src/lib.rs`, `rust/src/sync/apply_contract.rs`, `rust/src/sync/apply_builder.rs`, `rust/src/sync/live.rs`, `rust/src/sync/live_apply.rs`, `rust/src/sync/workbench.rs`, `rust/src/alert_client.rs`, `rust/src/http.rs`, `rust/src/dashboard/export.rs`, `rust/src/dashboard/live.rs`, `rust/src/sync/preflight.rs`, `rust/src/sync/rust_tests.rs`, `rust/src/sync/live_rust_tests.rs`, `docs/DEVELOPER.md`, `docs/overview-rust.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `cli.rs` still mixed command topology with large help-rendering/example blocks, the sync apply-intent contract was still passed around mostly as ad hoc JSON, and several Rust files still carried low-signal comments that narrated signatures instead of explaining boundaries or invariants.
- Current Update: moved unified help rendering and long example blocks into `cli_help.rs`, introduced `sync/apply_contract.rs` as the typed apply-intent envelope shared by the local builder and live execution, kept the operation loader backward-compatible for existing review/render/live tests, and removed boilerplate comments from the touched Rust files. Maintainer docs now point to the new helper modules and restate the comment-signal / thin-facade rule alongside the concrete refactor.
- Result: the root CLI entrypoint is thinner, the sync apply-intent path now has a repo-owned typed contract instead of only loose JSON, comment noise is lower in the touched files, and the full Rust test suite still passes.

## 2026-04-03 - Add dashboard raw-to-prompt migration workflow
- State: Done
- Scope: `rust/src/dashboard/cli_defs_command.rs`, `rust/src/dashboard/cli_defs_shared.rs`, `rust/src/dashboard/raw_to_prompt.rs`, `rust/src/dashboard/raw_to_prompt_rust_tests.rs`, `rust/src/dashboard/mod.rs`, `rust/src/dashboard/test_support.rs`, `rust/src/cli.rs`, `rust/src/cli_help.rs`, `rust/src/cli_help_examples.rs`, `rust/src/cli_rust_tests.rs`, `rust/src/dashboard/dashboard_cli_parser_help_rust_tests.rs`, `README.md`, `README.zh-TW.md`, `docs/commands/en/*.md`, `docs/commands/zh-TW/*.md`, `docs/user-guide/en/dashboard.md`, `docs/user-guide/en/reference.md`, `docs/user-guide/zh-TW/dashboard.md`, `docs/user-guide/zh-TW/reference.md`, `docs/man/*.1`, `docs/html/**`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the dashboard surface could export a `prompt/` lane from live Grafana, but there was no offline migration path for ordinary dashboard JSON or `raw/` files. Operators had to hand-edit datasource prompts or misuse `dashboard import` for files that really belonged in the Grafana UI import flow.
- Current Update: added a dedicated `dashboard raw-to-prompt` command with explicit file/dir input modes, sibling/default output rules, `infer-family|exact|strict` datasource repair policy, summary/log output controls, optional live datasource lookup through `--profile` or direct live auth flags, and prompt-lane metadata/index writing for `raw/` directory conversions. The command docs, handbook, README, generated manpages, and generated HTML now all explain that `raw/` remains the API replay lane, `prompt/` is for Grafana UI import, and `raw-to-prompt` is the migration bridge between them.
- Result: the repo now has an operator-facing migration path for raw dashboard JSON to prompt JSON, plus optional live datasource augmentation when operators need to repair prompt files against a target Grafana inventory. Focused Rust tests and docs generation now pass for this slice.

## 2026-04-03 - Add maintainer quickstart for first-entry repo orientation
- State: Done
- Scope: `README.md`, `README.zh-TW.md`, `docs/DEVELOPER.md`, `docs/internal/README.md`, `docs/internal/maintainer-quickstart.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the repo already had several strong entrypoints, but a first-time AI agent or new maintainer still had to infer the right reading order from multiple files. There was no single short page that answered which files to open first, which layers are source of truth, which outputs are generated, and which safe validation commands to prefer while still orienting.
- Current Update: added a dedicated `maintainer-quickstart.md` under `docs/internal/` and linked it from README, the Traditional Chinese README, `docs/DEVELOPER.md`, and the internal docs index. The new page defines the first files to read, the maintained surfaces, source-of-truth boundaries, task routing, safe validation defaults, and repo-specific gotchas.
- Result: future AI agents and human maintainers now have one explicit first-stop orientation page instead of reconstructing the repo map from scattered entrypoints.

## 2026-04-03 - Document generated docs architecture for maintainers
- State: Done
- Scope: `docs/DEVELOPER.md`, `docs/internal/generated-docs-architecture.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the repo already had working Markdown-to-manpage and Markdown-to-HTML generators, but the maintainer-facing explanation was still scattered across script docstrings and short notes. Future maintainers would have had to infer source-of-truth rules, Markdown subset limits, locale asymmetry, and Pages deployment behavior from code.
- Current Update: added a dedicated internal design document for the generated docs system and linked it from `docs/DEVELOPER.md`. The new doc explains source layers, output trees, module responsibilities, supported Markdown subset, command/handbook schema expectations, test flow, and GitHub Pages deployment rules.
- Result: maintainers now have one explicit design/maintenance reference for the generated docs pipeline instead of reconstructing it from the generators.

## 2026-04-03 - Add generated docs maintainer playbook
- State: Done
- Scope: `docs/DEVELOPER.md`, `docs/internal/generated-docs-architecture.md`, `docs/internal/generated-docs-playbook.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the new generated-docs architecture note explained the system well, but it was still architecture-first. A maintainer adding a new command page, handbook chapter, namespace manpage, locale, or Pages-facing output file still had to translate the design into concrete repo steps by hand.
- Current Update: added a task-oriented playbook for the generated docs pipeline and linked it from both `docs/DEVELOPER.md` and the architecture note. The playbook covers the common change types, the exact files to edit, the generator hooks to update, and the standard validation loop.
- Result: maintainers now have both the design reference and an operational cookbook for common generated-docs changes.

## 2026-04-03 - Reorganize DEVELOPER.md as a maintainer routing map
- State: Done
- Scope: `docs/DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `docs/DEVELOPER.md` already contained the right pointers, but the content was still arranged more like a short note dump than a high-signal maintainer map. It mixed code architecture, generated-docs notes, contract pointers, and validation guidance without a strong routing structure.
- Current Update: reorganized `docs/DEVELOPER.md` into explicit maintainer sections: start-here routing, repo priorities, code architecture map, documentation map, validation/build guidance, project rules, and a quick routing table by task type.
- Result: maintainers can now navigate by concern instead of reading the whole page linearly to discover where to go next.

## 2026-04-03 - Tighten maintainer guidance for comment signal and facade thinning
- State: Done
- Scope: `docs/DEVELOPER.md`, `docs/overview-rust.md`, `docs/internal/maintainer-quickstart.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the maintainer docs already described the Rust routing layers, but they did not yet state the quality bar clearly enough for comment signal, repo-owned envelopes, or how thin the facades should stay.
- Current Update: added short maintainer guidance that prefers repo-owned typed envelopes over ad hoc shapes, keeps `cli.rs` and domain facades focused on routing/normalize/re-export, and treats comments as signal for ownership or non-obvious behavior rather than narration.
- Result: maintainers now have a concise policy for keeping the Rust surface thinner and the comments more useful without turning the docs into a longer design note.

## 2026-04-03 - Document profile secret storage across user and maintainer docs
- State: Done
- Scope: `README.md`, `README.zh-TW.md`, `docs/DEVELOPER.md`, `docs/internal/README.md`, `docs/internal/profile-secret-storage-architecture.md`, `docs/user-guide/en/reference.md`, `docs/user-guide/zh-TW/reference.md`, `docs/commands/en/profile.md`, `docs/commands/zh-TW/profile.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the repo already supported environment-backed secrets, OS secret storage, and encrypted secret files, but that guidance was scattered. Operators could see fragments in `profile` docs, and maintainers could infer the platform backends from Rust code, but there was no single complete explanation of what the modes are, why they exist, what platforms they support, or how to troubleshoot them.
- Current Update: added a dedicated internal secret-storage architecture note, linked it from the maintainer entrypoints, and expanded the user-facing reference/profile docs with complete mode descriptions, macOS/Linux OS-store notes, usage guidance, caveats, and troubleshooting.
- Result: both operators and maintainers now have a clear documented model for profile secret storage instead of piecing it together from examples and code.

## 2026-04-03 - Add role-based doc entrypoints for operators and maintainers
- State: Done
- Scope: `README.md`, `README.zh-TW.md`, `docs/user-guide/en/index.md`, `docs/user-guide/zh-TW/index.md`, `docs/user-guide/en/role-new-user.md`, `docs/user-guide/en/role-sre-ops.md`, `docs/user-guide/en/role-automation-ci.md`, `docs/user-guide/zh-TW/role-new-user.md`, `docs/user-guide/zh-TW/role-sre-ops.md`, `docs/user-guide/zh-TW/role-automation-ci.md`, `docs/DEVELOPER.md`, `docs/internal/README.md`, `docs/internal/maintainer-role-map.md`, `scripts/docgen_handbook.py`, `scripts/generate_command_html.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the docs already had substantial content, but the primary navigation was still file-family oriented. New users, SREs, automation owners, and maintainers had to infer their own reading order from README sections, handbook chapter lists, and internal indexes.
- Current Update: added dedicated public role-guide handbook pages in English and Traditional Chinese, added an internal maintainer-role map, updated handbook ordering to treat the public role pages as first-class chapters, and upgraded README, handbook indexes, and the generated HTML landing page to route readers by role as well as by document type.
- Result: the docs now support both content-type navigation and role-based navigation, which makes the full document set easier to approach without already knowing the repo’s file layout.

## 2026-04-03 - Split default and browser-enabled Rust release artifacts
- State: Done
- Scope: `rust/Cargo.toml`, `Makefile`, `scripts/build-rust-macos-arm64.sh`, `scripts/build-rust-linux-amd64.sh`, `scripts/build-rust-linux-amd64-zig.sh`, `scripts/validate-rust-linux-amd64-artifact.sh`, `scripts/install.sh`, `.github/workflows/ci.yml`, `docs/DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the default Rust feature set included `browser`, so every normal build and release artifact pulled in `headless_chrome`. That made cross-builds slower and heavier, while Linux validation kept getting tangled with the browser-enabled dependency set.
- Current Update: switched the default feature set to lean/TUI-only, added explicit browser-enabled build targets and release assets, aligned install/build/validation tooling to choose standard versus browser artifacts intentionally, and cleaned up browser-disabled screenshot warnings triggered by the new default feature policy.
- Result: standard builds now omit `headless_chrome`, browser support is shipped through explicit `*-browser` artifacts and CI jobs, and both the default and browser-enabled Rust compile paths validate cleanly.

## 2026-04-02 - Consolidate contract docs into summary/spec/trace layers
- State: Done
- Scope: `docs/DEVELOPER.md`, `docs/internal/contract-doc-map.md`, `docs/internal/export-root-output-layering-policy.md`, `docs/internal/dashboard-export-root-contract.md`, `docs/internal/datasource-masked-recovery-contract.md`, `docs/internal/alert-access-contract-policy.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: current contract guidance was split awkwardly across maintainer summary notes and trace files, which made navigation noisier and encouraged repeating the same detailed rules in multiple places.
- Current Update: created dedicated current spec docs for repo-level export-root policy, dashboard export-root, and datasource masked-recovery contracts; kept `docs/DEVELOPER.md` as the short summary layer; and aligned the AI trace files to stay trace-only.
- Result: maintainers now have one clear summary/spec/trace split for the active contract topics instead of overlapping note fragments.

## 2026-04-02 - Clarify export-root/output layering scope
- State: Done
- Scope: `docs/DEVELOPER.md`, `docs/internal/export-root-output-layering-policy.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the maintainer notes already documented dashboard and datasource export/projection boundaries, but they did not yet spell out the repo-level pattern clearly enough to prevent overgeneralizing it to every resource kind.
- Current Update: added the short repo-level policy that reserves the explicit export-root/output-layering pattern for `dashboard` and `datasource`, with the detailed domain rule now anchored in a dedicated policy doc.
- Result: maintainers now have one concise place to read where the pattern applies and one detailed current policy doc for the full rule.

## 2026-04-02 - Clarify alert/access contract boundaries
- State: Done
- Scope: `docs/DEVELOPER.md`, `docs/internal/alert-access-contract-policy.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the repo-level export-root note already kept `alert` and `access` outside the dashboard/datasource pattern, but it still left too much room for maintainers to infer that any root index or staged bundle set should automatically grow `scopeKind` or aggregate-root semantics.
- Current Update: moved the detailed requirements into a dedicated policy doc that defines the current `alert` and `access` contract types, promotion criteria, and documentation split between summary docs and trace docs.
- Result: the repo now has one stable requirements doc for this boundary instead of repeating the same policy text across multiple maintainer notes.

## 2026-04-02 - Formalize dashboard export-root contract
- State: Done
- Scope: `docs/DEVELOPER.md`, `docs/internal/dashboard-export-root-contract.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: dashboard runtime and help text already treated `raw/`, `provisioning/`, and combined roots as different staged contract shapes, but the maintainer docs did not yet define the dashboard root contract as explicitly as the datasource masked-recovery contract.
- Current Update: moved the detailed dashboard root-manifest, scope semantics, and output-layering rules into a dedicated current contract doc while leaving only the short summary in `docs/DEVELOPER.md`.
- Result: dashboard now has a stable spec doc that can be updated without turning the maintainer summary or trace files into duplicate contract inventories.

## 2026-04-02 - Close datasource masked-recovery bookkeeping
- State: Done
- Scope: `TODO.md`, `docs/DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the datasource masked-recovery export/import/inspect lane was already complete, but the active backlog and maintainer notes still read as if the work was open.
- Current Update: removed the datasource masked-recovery item from the active TODO backlog and recorded the current maintainer contract at a concise level: `datasources.json` stays the canonical replay/masked-recovery artifact, `provisioning/datasources.yaml` stays a projection, and inspect/output notes should keep the masked secret boundary intact.
- Result: the bookkeeping now matches the finished datasource contract and no longer advertises the lane as active work.

## 2026-04-02 - Formalize datasource masked-recovery schema policy
- State: Done
- Scope: `docs/DEVELOPER.md`, `docs/internal/datasource-masked-recovery-contract.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the datasource maintainer notes already described the masked-recovery lane, but the schema compatibility rules were still implicit instead of written down as a stable contract policy.
- Current Update: moved the detailed stable fields, additive-versus-breaking rules, and `schemaVersion` guidance into a dedicated current contract doc while leaving the short summary in `docs/DEVELOPER.md`.
- Result: maintainers now have one current datasource contract spec to read before making export/import or help-text changes.

## 2026-04-01 - Add repo-owned install script for release binaries
- State: Done
- Scope: `scripts/install.sh`, `python/tests/test_python_install_script.py`, `README.md`, `README.zh-TW.md`, `docs/user-guide.md`, `docs/user-guide-TW.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: operators could build from source or download release assets manually, but there was no supported one-line install path that fetched the right Rust binary and placed it into a common executable directory.
- Current Update: added a repo-owned POSIX install script that detects `linux-amd64` and `macos-arm64`, downloads the matching GitHub release archive, installs `grafana-util` into `/usr/local/bin` when writable or falls back to `~/.local/bin`, and supports explicit `BIN_DIR`, `VERSION`, `REPO`, and `ASSET_URL` overrides. Public English and Traditional Chinese docs now show the one-line `curl ... | sh` path plus the direct local-checkout fallback.
- Result: users now have a documented one-line installer for the maintained Rust binary without needing to compile from source or hand-place the executable.

## 2026-04-01 - Extend alert list output formats
- State: Blocked
- Scope: `rust/src/alert_cli_defs.rs`, `rust/src/alert_list.rs`, `rust/src/alert_rust_tests.rs`
- Baseline: alert list commands (`list-rules`, `list-contact-points`, `list-mute-timings`, `list-templates`) only normalized `table`, `csv`, and `json` flags, with table as the default. The runtime list renderer also only handled table/csv/json.
- Current Update: widened the list parser/output normalization to include `text` and `yaml`, updated the list help text examples, and added focused parser/rendering tests for the expanded output set. Focused `cargo test` validation is blocked by unrelated compile errors elsewhere in the crate.
- Result: code changes are in place, but the focused Rust test slice does not complete because the repository currently fails to compile in unrelated dashboard/datasource files.

## 2026-04-01 - Record baseline-five live defaults and dashboard review output inventory
- State: Done
- Scope: `rust/src/profile_config.rs`, `rust/src/profile_cli.rs`, `rust/src/cli.rs`, `rust/src/cli_help_examples.rs`, `rust/src/dashboard/cli_defs_shared.rs`, `rust/src/dashboard/dashboard_runtime.rs`, `rust/src/access/access_cli_shared.rs`, `rust/src/access/access_cli_runtime.rs`, `rust/src/alert_cli_defs.rs`, `rust/src/project_status_command.rs`, `rust/src/project_status_support.rs`, `rust/src/dashboard/authoring.rs`, `rust/src/dashboard/cli_defs_command.rs`, `rust/src/dashboard/dashboard_cli_parser_help_rust_tests.rs`, `rust/src/dashboard/authoring_rust_tests.rs`, `rust/src/dashboard/mod.rs`, `rust/src/cli_rust_tests.rs`, `docs/user-guide.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: the shared live connection baseline still required repeating URL/auth/TLS flags by hand, and the dashboard authoring lane had not yet been documented as a local-only specialization with an explicit output-mode inventory.
- Current Update: expanded the repo-local profile baseline across the five live surfaces on the baseline-five rule (`dashboard`, `datasource`, `access`, `alert`, and `status live`) so they now inherit named live defaults from `grafana-util.yaml` while preserving explicit CLI overrides and environment fallbacks. In the same wave, documented the dashboard-only authoring/review lane as intentionally specialized: `get` and `clone-live` create local drafts, `patch-file` and `publish` reuse the import pipeline, and `review` now makes its output coverage explicit across text, table, CSV, JSON, and YAML.
- Result: the Rust CLI now has a five-surface shared live baseline, while the dashboard authoring/review surfaces stay deliberately specialized instead of being folded into the shared live connection layer.
