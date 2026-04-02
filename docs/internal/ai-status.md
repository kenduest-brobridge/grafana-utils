# ai-status.md

Current AI-maintained status only.

- Older trace history moved to [`archive/ai-status-archive-2026-03-24.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-03-24.md).
- Detailed 2026-03-27 entries moved to [`archive/ai-status-archive-2026-03-27.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-03-27.md).
- Detailed 2026-03-28 task notes were condensed into [`archive/ai-status-archive-2026-03-28.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-03-28.md).
- Keep this file short and current. Additive historical detail belongs in `docs/internal/archive/`.
- Detailed 2026-03-29 through 2026-03-31 entries moved to [`archive/ai-status-archive-2026-03-31.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-03-31.md).

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
