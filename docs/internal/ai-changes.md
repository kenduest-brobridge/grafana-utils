# ai-changes.md

Current AI change log only.

- Older detailed history moved to [`archive/ai-changes-archive-2026-03-24.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-changes-archive-2026-03-24.md).
- Detailed 2026-03-27 entries moved to [`archive/ai-changes-archive-2026-03-27.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-changes-archive-2026-03-27.md).
- Detailed 2026-03-28 task notes were condensed into [`archive/ai-changes-archive-2026-03-28.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-changes-archive-2026-03-28.md).
- Keep this file limited to the latest active architecture and maintenance changes.
- Detailed 2026-03-29 through 2026-03-31 entries moved to [`archive/ai-changes-archive-2026-03-31.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-changes-archive-2026-03-31.md).

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
