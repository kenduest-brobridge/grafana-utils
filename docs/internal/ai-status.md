# ai-status.md

Current AI-maintained status only.

- Older trace history moved to [`archive/ai-status-archive-2026-03-24.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-03-24.md).
- Detailed 2026-03-27 entries moved to [`archive/ai-status-archive-2026-03-27.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-03-27.md).
- Detailed 2026-03-28 task notes were condensed into [`archive/ai-status-archive-2026-03-28.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-03-28.md).
- Detailed 2026-03-29 through 2026-03-31 entries moved to [`archive/ai-status-archive-2026-03-31.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-03-31.md).
- Detailed 2026-04-01 through 2026-04-12 entries moved to [`archive/ai-status-archive-2026-04-12.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-04-12.md).
- Keep this file short and current. Additive historical detail belongs in `docs/internal/archive/`.
- Older entries moved to [`ai-status-archive-2026-04-13.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-04-13.md).

## 2026-04-13 - Add shell completion command
- State: Done
- Scope: Rust unified CLI command surface, completion rendering, parser/render tests, command-reference docs, README snippets, generated man/html output, and command-surface contracts.
- Baseline: the CLI had no shell completion generator, and any future completion support would need a clear source of truth to avoid drifting from Clap command definitions.
- Current Update: added `grafana-util completion bash|zsh`, backed by `clap_complete` and generated from `CliArgs::command()` only; documented install snippets for Bash and Zsh.
- Result: Bash and Zsh completion scripts can be generated from the current binary without connecting to Grafana or reading profile/auth state.

## 2026-04-13 - Type Rust machine-output contract builders
- State: Done
- Scope: snapshot review warnings, sync source bundle, sync bundle preflight, and sync promotion preflight output assembly.
- Baseline: several machine-readable Rust outputs still assembled stable document structures with inline `serde_json::json!` or manual `Map` construction, leaving field names and summary shapes mostly constrained by tests and reviewer discipline.
- Current Update: replaced selected top-level document and warning builders with module-local `Serialize` DTOs/helpers while leaving nested resource `Value` payloads intact where they represent external Grafana or staged resource documents.
- Result: public JSON fields and behavior are unchanged; focused no-run targets for snapshot, sync source bundle, bundle preflight, and promotion preflight pass locally.

## 2026-04-13 - Split Rust snapshot/import/live-status hotspots
- State: Done
- Scope: Rust snapshot CLI/review document assembly, dashboard import lookup helpers, access live project-status helpers, and dashboard inspect CLI definition modules.
- Current Update: split `snapshot.rs` into CLI definitions, review count/warning rules, lane loading, and typed review-document serialization; split dashboard import lookup into cache, org lookup, and folder/inventory helpers; kept worker-produced access live-status and dashboard inspect CLI splits integrated with the current dev branch.
- Result: behavior and public command contracts are unchanged; full `cd rust && cargo test --quiet` passes with 1463 passed / 1 ignored in the main lib suite plus integration targets, and `cargo fmt --manifest-path rust/Cargo.toml --all --check` passes.
- Follow-up: `scripts/rust_maintainability_report.py --root rust/src` still flags larger untouched files, led by datasource project-status/live-status, `project_status_live_runtime.rs`, `snapshot_support.rs`, `profile_config.rs`, dashboard browse/export/import/project-status/topology surfaces, sync preflight modules, and large Rust test files.

## 2026-04-13 - Ignore credentials in Grafana base URLs
- State: Done
- Scope: Rust profile/env/CLI connection URL resolution and focused connection-setting tests.
- Baseline: `GRAFANA_URL`, `--url`, or profile `url` values containing URL userinfo were treated as plain base URLs instead of producing an explicit operator-facing error.
- Current Update: added a shared URL userinfo sanitizer after connection URL precedence is resolved, with a stderr warning that explicit Basic auth flags, Basic auth environment variables, or profile credentials should be used instead.
- Result: Grafana base URLs that include username or password continue through the original auth flow with URL credentials stripped and ignored; focused Rust tests and narrow formatting checks pass.

## 2026-04-13 - Split Rust facade and CLI-args hotspots
- State: Done
- Scope: dashboard reusable runners, access dispatch/auth materialization, datasource local-list/diff/import-export support helpers, sync CLI args modules, Rust maintainability reporter, and Rust maintainer architecture notes.
- Current Update: moved dashboard list/inspect reusable execution out of `command_runner`, split access routing and auth materialization out of `access/mod.rs`, split datasource local list/diff rendering and import/export IO/org routing out of root facades, split sync CLI argument definitions by command family, and added a read-only oversized-file/re-export reporter.
- Result: public CLI behavior and output contracts are unchanged; `cargo test --quiet --lib` passes with 1461 passed / 1 ignored, and the new Python maintainability reporter tests pass.

## 2026-04-13 - Fix docs portal, landing, and manpage HTML navigation
- State: Done
- Scope: GitHub Pages version portal generator, portal copy contract, local landing source/CSS, generated HTML/manpages, manpage router contract, manpage HTML renderer/CSS, Pages assembly script, focused script tests, and generated-docs maintainer notes.
- Baseline: the published Pages root portal was generated outside `docs/html/`, but maintainer docs still described Pages as publishing the local `docs/html/` tree; portal output shortcuts also pointed readers back to the same lane index instead of specific handbook, command, or manpage pages.
- Current Update: deep-linked portal output shortcuts by lane and locale, removed latest minor duplication from version choices, shortened the local landing page into recommended starts/common jobs/complete reference, widened the landing layout for 1366 and 1920 widths, moved root manpage router copy into a JSON contract, shortened root subcommand listings to purpose-only summaries, rendered manpage `.SS` subsections as HTML subheadings, changed manpage definition lists from a wide two-column grid to readable stacked entries, converted the root manpage subcommand index into collapsible linked groups, linked manpage references in index and SEE ALSO sections, rendered stray paragraph-style CLI examples as code blocks in HTML, replaced the handbook tree on manpage pages with a grouped full manpage index plus documentation lane links, and documented that the Pages root portal is generated by `scripts/docsite_version_portal.py` while lane pages still use the shared HTML generator.
- Result: Pages root navigation now distinguishes latest/dev lanes and direct output types; the local `docs/html/index.html` has a clearer, shorter 1080p-friendly layout; generated manpage HTML no longer reads like a broken table for long command descriptions and its manpage references are clickable; maintainers can find the local source for the published root `index.html`.
