# ai-changes.md

## 2026-03-12 - Add Developer Grafana Sample-Data Seed Script
- Summary: Added `scripts/seed-grafana-sample-data.sh` plus `make seed-grafana-sample-data`, `make destroy-grafana-sample-data`, and `make reset-grafana-all-data` so developers can seed, clean up, or aggressively reset a running Grafana with a stable manual-testing dataset instead of recreating sample orgs and dashboards ad hoc during interactive sessions. The script seeds fixed datasource, folder, and dashboard uids and uses overwrite or lookup flows so it can be rerun safely, while destroy mode removes only the known sample resources and reset mode clears the broader repo-relevant test surface in a disposable instance.
- Tests: Added shell-level validation for the new script and Make help output.
- Test Run: `bash -n scripts/seed-grafana-sample-data.sh`; `bash ./scripts/seed-grafana-sample-data.sh --help`; `make help`
- Reason: Repeated live testing was depending on hand-created sample orgs, subfolders, and dashboards, which made manual verification slower and less reproducible.
- Validation: Verified the script help text documents seed, destroy, and reset behavior and that the repo exposes all three workflows through dedicated Make targets.
- Impact: `scripts/seed-grafana-sample-data.sh`, `Makefile`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: Low. The feature is additive and opt-in, though it assumes admin Basic auth to seed orgs and dashboards into a running Grafana.

## 2026-03-12 - Add Prompted Basic-Auth Password Support
- Summary: Added `--prompt-password` to the Python dashboard, alerting, and access CLIs plus the Rust dashboard and alerting binaries. The new flag reads the Grafana Basic-auth password from a non-echoed terminal prompt instead of requiring `--basic-password` on the command line.
- Tests: Extended the Python dashboard, alert, and access suites plus the Rust auth and parser tests to cover parser support, prompted password resolution, and rejection of unsafe flag combinations.
- Test Run: `python3 -m unittest -v tests/test_python_dashboard_cli.py tests/test_python_alert_cli.py tests/test_python_access_cli.py`; `cd rust && cargo test --quiet`
- Reason: Operators wanted a safer Basic-auth workflow that avoids leaking Grafana passwords into shell history and process listings.
- Validation: Verified the prompt-aware auth paths in Python and Rust accept `--basic-user ... --prompt-password`, reject mixing prompt mode with token auth or explicit `--basic-password`, and keep the existing environment fallback behavior for non-prompted auth.
- Impact: `grafana_utils/dashboard_cli.py`, `grafana_utils/alert_cli.py`, `grafana_utils/access_cli.py`, `tests/test_python_dashboard_cli.py`, `tests/test_python_alert_cli.py`, `tests/test_python_access_cli.py`, `rust/Cargo.toml`, `rust/src/common.rs`, `rust/src/common_rust_tests.rs`, `rust/src/dashboard.rs`, `rust/src/dashboard_rust_tests.rs`, `rust/src/alert.rs`, `rust/src/alert_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: Low. The change is additive, but interactive prompting assumes a terminal is available when operators opt into `--prompt-password`.

## 2026-03-12 - Add Platform-Specific Rust Build Paths
- Summary: Added explicit Make and script entrypoints for macOS Apple Silicon and Linux `amd64` Rust release builds. `make build-rust-macos-arm64` copies native Apple Silicon binaries into `dist/macos-arm64/`, `make build-rust-linux-amd64` uses Docker to build `x86_64-unknown-linux-gnu` binaries into `dist/linux-amd64/`, and `make build-rust-linux-amd64-zig` uses local `zig` and `cargo-zigbuild` for the same Linux target without Docker.
- Tests: Added shell-level validation for all build scripts, checked `make help`, live-ran the Docker-backed Linux `amd64` build, and live-ran the non-Docker zig-based Linux `amd64` build.
- Test Run: `bash -n scripts/build-rust-macos-arm64.sh`; `bash -n scripts/build-rust-linux-amd64.sh`; `bash -n scripts/build-rust-linux-amd64-zig.sh`; `make help`; `RUST_IMAGE=rust:bookworm make build-rust-linux-amd64`; `. \"$HOME/.cargo/env\" && cd rust && cargo zigbuild --release --target x86_64-unknown-linux-gnu`
- Reason: Operators needed one obvious Makefile surface for producing both native Mac M1 binaries and Linux `amd64` release artifacts from the same repo.
- Validation: Verified the Linux Docker build completed successfully and produced ELF `x86-64` binaries in `dist/linux-amd64/`. Verified the non-Docker zig path also produced Linux `x86-64` ELF binaries under `rust/target/x86_64-unknown-linux-gnu/release/`. The macOS Apple Silicon path is a thin native wrapper around `cargo build --release` that copies the host-built binaries into `dist/macos-arm64/`.
- Impact: `Makefile`, `scripts/build-rust-macos-arm64.sh`, `scripts/build-rust-linux-amd64.sh`, `scripts/build-rust-linux-amd64-zig.sh`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: Low to moderate. The new paths are additive, but the Linux build depends on Docker and the chosen Rust image, while the macOS path intentionally only supports Apple Silicon hosts.
- Follow-up: If we need Intel macOS artifacts later, add a separate `build-rust-macos-amd64` path instead of overloading the Apple Silicon target.

## 2026-03-12 - Update Dashboard Help Examples And Local Default URL
- Summary: Changed the Python and Rust dashboard CLI default URL from `http://127.0.0.1:3000` to `http://localhost:3000`, added local Basic-auth examples plus token examples to the real `-h` output, and refreshed the public and maintainer docs to match the local-first operator flow.
- Tests: Extended `tests/test_python_dashboard_cli.py` and `rust/src/dashboard_rust_tests.rs` to assert the updated top-level and export help examples, then rechecked the actual help output for both implementations.
- Test Run: `python3 -m unittest -v tests/test_python_dashboard_cli.py`; `cd rust && cargo test dashboard --quiet`; `python3 cmd/grafana-utils.py -h`; `python3 cmd/grafana-utils.py export-dashboard -h`; `./rust/target/release/grafana-utils -h`
- Reason: Operators asked for the actual shipped help output to show username/password usage and a local URL by default instead of only token-based remote examples.
- Validation: Verified the Python top-level help, Python `export-dashboard -h`, Rust source-tree help, and the rebuilt Rust release binary help all show `http://localhost:3000` plus `--basic-user` and `--basic-password` examples.
- Impact: `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: Low. The change is operator-facing documentation and default-url polish, though any tests or wrappers that hardcoded the old default URL string needed to be updated.
- Follow-up: Consider normalizing the alerting and access README examples to `http://localhost:3000` too if we want one local-first documentation style across the repo.

## 2026-03-12 - Add Dashboard Multi-Org Export
- Summary: Extended both the Python and Rust `export-dashboard` commands with `--org-id` and `--all-orgs`. `--org-id` exports one explicit Grafana org, while `--all-orgs` enumerates visible orgs and exports each org into its own `org_<id>_<name>/` subtree. Both paths are Basic-auth-only and keep aggregate root-level variant indexes for the overall export root.
- Tests: Extended the focused Python and Rust dashboard suites with parser coverage, auth validation, explicit-org export tests, and multi-org export tests that verify scoped requests and org-separated output paths.
- Test Run: `python3 -m unittest -v tests/test_python_dashboard_cli.py`; `cd rust && cargo test dashboard --quiet`
- Reason: Operators asked for export-side org selection after `list-dashboard` gained `--org-id` and `--all-orgs`, because current-org-only export was too limiting in multi-org Grafana setups.
- Validation: Live-checked both CLIs against Docker Grafana `12.4.1`. `export-dashboard --org-id 2` exported only `org-two-main`, and `export-dashboard --all-orgs` wrote separate `org_1_Main_Org/...` and `org_2_Org_Two/...` trees plus a root `raw/index.json` that referenced the aggregate export.
- Impact: `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: Moderate. The feature is additive, but it depends on server-admin-style org enumeration and org switching, so token-auth exports remain current-org-only. Multi-org export also changes on-disk layout intentionally to avoid collisions, which downstream tooling should understand.
- Follow-up: Decide whether `import-dashboard` or `diff` should ever grow matching multi-org options, or whether multi-org export should remain archival and inspection focused.

## 2026-03-12 - Add Dashboard Multi-Org Listing
- Summary: Extended both the Python and Rust `list-dashboard` commands with `--org-id` and `--all-orgs`. `--org-id` switches listing to one explicit Grafana org, while `--all-orgs` enumerates visible orgs and aggregates dashboard output across them. Both paths keep the existing per-dashboard `org` and `orgId` metadata and are intentionally Basic-auth-only.
- Tests: Extended the focused Python and Rust dashboard suites with parser coverage, auth validation, and request-scoping tests for explicit-org and all-org listing.
- Test Run: `python3 -m unittest -v tests/test_python_dashboard_cli.py`; `cd rust && cargo test dashboard --quiet`
- Reason: Operators asked for the same org-switching capability Grafana exposes in the UI so dashboard listing can inspect another org or all visible orgs without manually changing session context first.
- Validation: Live-checked the Python CLI against Docker Grafana `12.4.1` by creating org `2`, seeding dashboard `org-two-main`, and verifying `list-dashboard --all-orgs --json` returned dashboards from both org `1` and org `2`.
- Impact: `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: Moderate. The feature is additive, but it depends on Grafana org-enumeration and org-switch behavior that are server-admin/Basic-auth workflows. Operators using token auth remain limited to the current org context.
- Follow-up: Completed in the next dashboard export change; `export-dashboard` now supports `--org-id` and `--all-orgs`.

## 2026-03-12 - Add Dashboard Datasource Listing Command
- Summary: Added `list-data-sources` to both the Python and Rust dashboard CLIs so operators can inspect the live Grafana datasource catalog directly. The new command reuses the existing `/api/datasources` client path and supports compact text output plus `--table`, `--csv`, and `--json` for datasource fields `uid`, `name`, `type`, `url`, and `isDefault`.
- Tests: Extended the focused Python and Rust dashboard suites with parser coverage, conflicting-output-mode validation, datasource renderer assertions, and command-path tests for the new datasource listing flow.
- Test Run: `python3 -m unittest -v tests/test_python_dashboard_cli.py`; `cd rust && cargo test dashboard --quiet`
- Reason: Operators asked for a first-class datasource listing command instead of inferring datasource usage only indirectly through dashboards or raw API calls.
- Validation: Verified the new command shape and output renderers in both implementations. The new output remains read-only and does not change existing dashboard export, import, diff, or dashboard-list behavior.
- Impact: `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: Low. The command is additive and reuses the existing datasource list API path, but consumers should treat the output schema as distinct from the dashboard list schema.
- Follow-up: Optional live Docker validation for `list-data-sources` if we want a checked-in smoke path for datasource listing similar to the existing dashboard and access live checks.

## 2026-03-12 - Rename Dashboard CLI Subcommands
- Summary: Renamed the dashboard CLI subcommands from `export`, `list`, and `import` to `export-dashboard`, `list-dashboard`, and `import-dashboard` in both the Python and Rust implementations. The CLI help, parser tests, and public/maintainer docs now use the explicit dashboard-prefixed names consistently, while `diff` remains unchanged.
- Tests: Extended focused Python and Rust dashboard parser/help coverage to assert the new subcommand names and to reject the old `list` name.
- Test Run: `python3 -m unittest -v tests/test_python_dashboard_cli.py`; `cd rust && cargo test dashboard --quiet`
- Reason: Operators asked for more explicit dashboard command names so the main `grafana-utils` surface reads clearly next to the alerting and access-management CLIs.
- Validation: Verified the renamed subcommands parse correctly in both implementations, updated the help examples to advertise the new names, and confirmed the focused dashboard suites still pass after the rename.
- Impact: `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: Moderate. The behavior is straightforward, but it is a breaking CLI rename for operators or automation that still invoke the old `export`, `list`, or `import` dashboard subcommands.
- Follow-up: Decide whether to add temporary aliases for backward compatibility or keep the rename strict.

## 2026-03-12 - Add Dashboard List Org Metadata
- Summary: Extended both the Python and Rust dashboard `list-dashboard` subcommands to fetch the current Grafana organization once from `GET /api/org` and include `org` and `orgId` in compact text output plus table, CSV, and JSON renderers. The change applies to plain `list-dashboard` and `list-dashboard --with-sources`, so source metadata now sits alongside explicit org metadata in all list formats.
- Tests: Extended `tests/test_python_dashboard_cli.py` and `rust/src/dashboard_rust_tests.rs` with current-org attachment coverage plus text, table, CSV, and JSON output assertions that include `org` and `orgId`.
- Test Run: `python3 -m unittest -v tests/test_python_dashboard_cli.py`; `cd rust && cargo test dashboard --quiet`
- Reason: Operators asked for dashboard list output to show the Grafana organization explicitly because the same host can expose multiple org contexts and the previous list output had no direct org identifier.
- Validation: Verified the Python renderer and CLI path locally, then live-checked the new list output against Docker Grafana `12.4.1` after seeding nested folders, dashboards, and datasources. The live output now shows the expected `org` and `orgId` fields alongside folder path and optional datasource metadata.
- Impact: `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: Low. The change is additive and fetches `GET /api/org` once per list run, but list consumers that assumed an exact older CSV or JSON schema will need to tolerate the new `org` and `orgId` fields.
- Follow-up: Add a checked-in live smoke script for dashboard `list --with-sources` if we want repeatable end-to-end coverage for the datasource and org metadata together.

## 2026-03-12 - Add Dashboard List Datasource Display
- Summary: Extended both the Python and Rust dashboard `list-dashboard` subcommands with `--with-sources`, an opt-in mode that fetches each dashboard payload and resolves datasource references into datasource names for display. The extra data now appears in compact text output and in table, CSV, and JSON output as a `sources` field or column. CSV output also includes best-effort datasource UID collection in a `sourceUids` column.
- Tests: Added parser/help coverage plus Python and Rust list rendering tests for the new `sources` field and datasource-resolution helpers.
- Test Run: `python3 -m unittest -v tests/test_python_dashboard_cli.py`; `python3 -m unittest -v`; `python3 cmd/grafana-utils.py list-dashboard -h`; `cd rust && cargo test dashboard --quiet`; `cd rust && cargo run --quiet --bin grafana-utils -- list-dashboard -h`
- Validation: Verified that plain `list` output stays unchanged unless `--with-sources` is passed, and that `--with-sources` shows resolved datasource names consistently across Python and Rust.
- Impact: `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: Low to moderate. The new behavior is opt-in, but it adds extra API calls and best-effort datasource resolution for dashboards that use aliases, UIDs, or unusual placeholder patterns.
- Follow-up: Optional live Docker validation for `list --with-sources` if we want end-to-end confirmation against a real Grafana datasource catalog.

## 2026-03-12 - Add Python Access Live Smoke Test
- Summary: Added `scripts/test-python-access-live-grafana.sh`, a Docker-backed smoke test for the Python access CLI, plus a `make test-access-live` target. The script starts Grafana, bootstraps an API token, and exercises the current Python access-management surface across user, team, and service-account workflows.
- Tests: Added shell-script coverage via `bash -n` and wired the script into the documented validation surface.
- Test Run: `bash -n scripts/test-python-access-live-grafana.sh`
- Validation: The script is designed to validate `user add`, `user modify`, `user delete` in both supported scopes, `team add`, `team list`, `team modify`, `service-account add`, `service-account token add`, and `service-account list` against Docker Grafana `12.4.1`.
- Impact: `scripts/test-python-access-live-grafana.sh`, `Makefile`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: Low to moderate. The new script is opt-in and isolated to local Docker validation, but like the Rust smoke test it depends on Docker daemon access and current Grafana API behavior.
- Follow-up: Run the script live on this machine when Docker access is available and, if it stays stable, consider adding one make target that runs both Rust and access live smoke tests together.

## 2026-03-12 - Add Access Utility Team Add
- Summary: Added Python `grafana-access-utils team add` support, including parser/help wiring, Grafana team creation through the org-scoped API, optional initial member/admin seeding, and aligned public/maintainer docs. The command creates the team first, then reuses the existing exact org-user resolution and guarded membership/admin update flow so initial admins are applied consistently with `team modify`.
- Tests: Extended `tests/test_python_access_cli.py` with parser/help coverage plus create-flow tests for initial members/admins and the failure path when an initial user cannot be resolved.
- Test Run: `python3 -m unittest -v tests/test_python_access_cli.py`; `python3 -m unittest -v`; `python3 cmd/grafana-access-utils.py team -h`; `python3 cmd/grafana-access-utils.py team add -h`
- Reason: `TODO.md` kept `team add` as the next unfinished team lifecycle command after `user delete`, and the access CLI needed a real create path before only `team delete` and the `group` alias remained.
- Validation: Verified Python 3.6 syntax compatibility and parser behavior, confirmed `team add -h` exposes the expected auth and argument surface, and exercised the create flow in unit tests including seeded member/admin updates.
- Impact: `grafana_utils/access_cli.py`, `tests/test_python_access_cli.py`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`, `TODO.md`
- Rollback/Risk: Low to moderate. The new command is additive, but seeded admin assignment still depends on the same org-user identity resolution and team-member update semantics as `team modify`, so Grafana version differences in those APIs remain the main compatibility risk.
- Follow-up: Continue with the remaining access-management plan: `team delete` and the `group` alias.

## 2026-03-12 - Add Access Utility User Delete
- Summary: Added Python `grafana-access-utils user delete` support, including parser/help wiring, explicit confirmation with `--yes`, exact target selection by id/login/email, and aligned public/maintainer docs. The command supports global deletion through the admin API and org-scoped removal through the org user API, with auth rules that match those two paths.
- Tests: Extended `tests/test_python_access_cli.py` with parser coverage, help coverage, confirmation/auth validation, and behavior tests for both global delete and org-scoped removal.
- Test Run: `python3 -m unittest -v tests/test_python_access_cli.py`; `python3 -m unittest -v`; `python3 cmd/grafana-access-utils.py user -h`; `python3 cmd/grafana-access-utils.py user delete -h`
- Reason: `TODO.md` listed `user delete` as the next unfinished user-lifecycle step after `user modify`, and the CLI needed an explicit destructive path with confirmation before moving on to the remaining team and alias commands.
- Validation: Verified Python 3.6 syntax compatibility and parser behavior, then ran Docker-backed Grafana `12.4.1` smoke tests that deleted one user globally with Basic auth and removed another user through the org-scoped path with a token. In the default single-org Grafana setup used for validation, the org-scoped removal also left the user absent from global listing afterward.
- Impact: `grafana_utils/access_cli.py`, `tests/test_python_access_cli.py`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`, `TODO.md`
- Rollback/Risk: Moderate. The new command is destructive by design, though it requires explicit confirmation and was live-validated on Grafana `12.4.1`. The global and org delete paths have different auth/permission models, and org-scoped behavior may vary in multi-org setups compared with the single-org validation environment.
- Follow-up: Continue with the remaining access-management plan: `team add`, `team delete`, and the `group` alias.

## 2026-03-12 - Add Access Utility User Modify
- Summary: Added Python `grafana-access-utils user modify` support, including parser/help wiring, exact target selection by id/login/email, explicit setters for login/email/name/password/org role/Grafana-admin state, and aligned public/maintainer docs. The command is Basic-auth-only and splits updates across the appropriate Grafana user, admin-password, org-role, and permission APIs.
- Tests: Extended `tests/test_python_access_cli.py` with parser coverage, Basic-auth-only help/validation coverage, modify-argument validation, and behavior tests for full-field updates and `--user-id` targeting.
- Test Run: `python3 -m unittest -v tests/test_python_access_cli.py`; `python3 -m unittest -v`; `python3 cmd/grafana-access-utils.py user -h`; `python3 cmd/grafana-access-utils.py user modify -h`
- Reason: `TODO.md` listed `user modify` as the next user-lifecycle step after `user add`, and the CLI needed a safe explicit update path before moving on to deletion and the remaining team/group commands.
- Validation: Verified Python 3.6 syntax compatibility and parser behavior, then ran a Docker-backed Grafana `12.4.1` smoke test that created a user, updated login/email/name/password/org role/Grafana-admin state, and verified the result through both global and org-scoped user listing. As with the earlier user flows, Grafana’s global list API reflected Grafana-admin state while the org-scoped list API reflected org role.
- Impact: `grafana_utils/access_cli.py`, `tests/test_python_access_cli.py`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`, `TODO.md`
- Rollback/Risk: Low to moderate. The new command is additive and live-validated on Grafana `12.4.1`, but it depends on multiple server-admin/global endpoints with slightly different visibility of user fields, so operator verification still benefits from checking both global and org-scoped list output when changing both org role and Grafana-admin state in one run.
- Follow-up: Continue with the remaining access-management plan: `user delete`, `team add`, `team delete`, and the `group` alias.

## 2026-03-11 - Add Access Utility Team Modify
- Summary: Added Python `grafana-access-utils team modify` support, including parser/help wiring, exact team lookup by id or name, exact user lookup by login or email, member add/remove operations, admin add/remove operations, and aligned public/maintainer docs. The command uses direct member add/delete calls for member changes and the documented bulk update payload for admin changes after reading current member permission metadata.
- Tests: Extended `tests/test_python_access_cli.py` with parser coverage, modify-argument validation, member add/remove behavior, admin bulk-update behavior, and the failure path when admin metadata is unavailable.
- Test Run: `python3 -m unittest -v tests/test_python_access_cli.py`; `python3 -m unittest -v`; `python3 cmd/grafana-access-utils.py team -h`; `python3 cmd/grafana-access-utils.py team modify -h`
- Reason: `TODO.md` put `team modify` next after `team list`, and the repo needed the first mutating team-membership workflow before moving on to user/team delete or the `group` alias surface.
- Validation: Verified Python 3.6 syntax compatibility and parser behavior, then ran Docker-backed Grafana `12.4.1` smoke tests that created users and a team, exercised member add/remove with Basic auth, exercised admin promote/demote with Basic auth, and exercised member add with a service-account token. Live Grafana returned team-member `permission` metadata (`4` for admin, `0` for member), which the command now uses to preserve existing admin assignments during bulk admin updates.
- Impact: `grafana_utils/access_cli.py`, `tests/test_python_access_cli.py`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`, `TODO.md`
- Rollback/Risk: Moderate. The new command is additive and live-validated on Grafana `12.4.1`, but admin updates depend on current team-member responses exposing permission/admin metadata; when that metadata is missing, the command fails instead of guessing and risking an overwrite of current admin assignments.
- Follow-up: Continue with the remaining access-management plan: `user modify`, `user delete`, `team add`, `team delete`, and the `group` alias.

## 2026-03-11 - Add Access Utility User Add
- Summary: Added Python `grafana-access-utils user add` support, including parser/help wiring, Basic-auth-only validation, Grafana admin API user creation, optional org-role and Grafana-admin follow-up updates, and aligned public/maintainer docs. The command now distinguishes Grafana auth `--basic-password` from the new user’s required `--password` cleanly in both parsing and help output.
- Tests: Extended `tests/test_python_access_cli.py` with parser/help coverage, auth-validation coverage, explicit-basic-over-env-token coverage, and create-flow tests for user creation plus follow-up role/permission calls.
- Test Run: `python3 -m unittest -v tests/test_python_access_cli.py`; `python3 -m unittest -v`; `python3 cmd/grafana-access-utils.py user -h`; `python3 cmd/grafana-access-utils.py user add -h`
- Reason: `TODO.md` called out `user add` as one of the next access-management lifecycle steps, and the CLI needed a real server-admin create path before moving on to user modification and deletion.
- Validation: Verified Python 3.6 syntax compatibility, corrected an argparse destination collision between auth password and the new-user password flag, ensured explicit CLI Basic auth wins over an ambient token env var, and ran a Docker-backed Grafana `12.4.1` smoke test that created a user and verified it through both global and org-scoped listing.
- Impact: `grafana_utils/access_cli.py`, `tests/test_python_access_cli.py`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`, `TODO.md`
- Rollback/Risk: Low to moderate. The new command is additive, but Grafana user-creation and follow-up admin/org APIs are Basic-auth server-admin workflows and may expose different fields across Grafana versions; live validation showed the global list API does not echo org role even after the org-role update, while org-scoped listing does.
- Follow-up: Continue with the remaining access-management plan: `team modify`, `user modify`, `user delete`, and the remaining team/group mutating workflow.

## 2026-03-11 - Add Access Utility Team List
- Summary: Added Python `grafana-access-utils team list` support, including parser/help wiring, Grafana team search and member lookup client calls, normalization/rendering for text/table/CSV/JSON output, and aligned public/maintainer docs. The command is org-scoped and follows the same auth model as the other org-scoped access commands.
- Tests: Extended `tests/test_python_access_cli.py` with parser/help coverage plus team row/build/render tests.
- Test Run: `python3 -m unittest -v tests/test_python_access_cli.py`; `python3 -m unittest -v`; `python3 cmd/grafana-access-utils.py team -h`; `python3 cmd/grafana-access-utils.py team list -h`
- Reason: `TODO.md` identified `team list` as the next access-management slice after `user list`, and the repo needed the first team-oriented command before moving on to mutating team membership workflows.
- Validation: Verified Python 3.6 syntax compatibility, incomplete-command help behavior, and the new team list output surfaces. Also fixed a pagination issue during review so team listing now iterates server pages before local filtering/pagination.
- Impact: `grafana_utils/access_cli.py`, `tests/test_python_access_cli.py`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`, `TODO.md`
- Rollback/Risk: Low to moderate. The new command is additive, but Grafana team search payloads can vary by version; member lookup remains best-effort and depends on the org-scoped team APIs returning the expected fields.
- Follow-up: Continue with the remaining access-management plan: `team modify`, `user add`, and the rest of the team/group mutating workflow.

## 2026-03-11 - Add Access Utility User List
- Summary: Added a new Python `grafana-access-utils` command with an initial access-management surface covering `user list`, `service-account list`, `service-account add`, and `service-account token add`. The first cut introduces `grafana_utils/access_cli.py`, the console-script entrypoint, a thin `cmd/grafana-access-utils.py` wrapper, packaging coverage, dedicated access-CLI unit tests, and public/maintainer docs that scope the feature to Python only for now.
- Tests: Added `tests/test_python_access_cli.py` for parser coverage, auth validation, filtering, pagination, and rendering behavior, and extended packaging coverage for the new console script.
- Test Run: `python3 -m unittest -v tests/test_python_access_cli.py tests/test_python_packaging.py` (pass); `python3 -m unittest -v` (pass)
- Validation: `python3 cmd/grafana-access-utils.py user list -h` now documents org/global scope, auth options, and output modes, and the service-account subcommands are documented alongside it. The implementation enforces the intended auth split: org-scoped user listing may use token or Basic auth, global user listing and `--with-teams` require Basic auth, and the service-account commands are org-scoped and may use token or Basic auth.
- Impact: `grafana_utils/access_cli.py`, `tests/test_python_access_cli.py`, `pyproject.toml`, `cmd/grafana-access-utils.py`, `tests/test_python_packaging.py`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: Moderate scope risk. This is the first access-management surface in the repo and still covers only part of the planned user/team lifecycle; full user add/modify/delete and team/group operations remain future work.
- Follow-up: Add `team list`, then extend user and service-account operations further once the remaining auth and permission boundaries are encoded explicitly.

## 2026-03-11 - Remove Python Dependency From Rust Live Smoke Test
- Summary: Updated the Docker-backed Rust Grafana smoke script so its token bootstrap path parses JSON with `jq` instead of calling `python3`, and replaced the last Perl-based in-place JSON edit with a `jq` temp-file rewrite. The script no longer checks for Python or Perl at startup and now requires `jq` explicitly.
- Tests: Reused the existing smoke script validation path after the helper change.
- Test Run: `bash -n scripts/test-rust-live-grafana.sh` (pass); `./scripts/test-rust-live-grafana.sh` (pass)
- Validation: The Docker-backed Grafana smoke test still created a token, rewrote the exported contact point, and completed successfully after replacing both the Python and Perl JSON helpers with `jq`.
- Impact: `scripts/test-rust-live-grafana.sh`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: Low risk. The change only affects test-script dependency handling; failure would be limited to token extraction during live smoke validation.
- Follow-up: None.

## 2026-03-11 - Clarify Rust CLI Help Text
- Summary: Added operator-facing help text to the Rust dashboard and alerting CLIs so `-h` and `--help` explain common flags such as auth, TLS verification, `--flat`, dry-run, diff, and import/export directory behavior. The dashboard Rust CLI now also includes top-level usage examples in help output.
- Tests: Added Rust help-output coverage for dashboard export help, dashboard top-level help examples, and alert help text for `--flat`.
- Test Run: `cd rust && cargo test --quiet` (pass)
- Validation: `cargo run --quiet --bin grafana-utils -- export -h` now explains that `--flat` writes dashboard files directly into the export variant directory instead of per-folder subdirectories; `cargo run --quiet --bin grafana-alert-utils -- -h` now explains that alert `--flat` writes resource files directly into their resource directories instead of nested subdirectories.
- Impact: `rust/src/dashboard.rs`, `rust/src/alert.rs`, `rust/src/dashboard_rust_tests.rs`, `rust/src/alert_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: Low risk. This changes CLI help text only and adds tests around the displayed wording.
- Follow-up: If more operator questions come up, mirror the same level of detail into Rust subcommand examples beyond the current dashboard top-level help.

## 2026-03-11 - Add Preferred Auth Flag Aliases
- Summary: Updated both Python CLIs to prefer `--token`, `--basic-user`, and `--basic-password` while still accepting the older `--api-token`, `--username`, and `--password` spellings. The auth resolver now fails early when operators mix token and Basic-auth flags or provide only one side of the Basic-auth pair, instead of silently preferring one mode.
- Tests: Added parser and auth-validation coverage in both Python CLI suites for the preferred aliases, token-only auth, Basic-auth success, mixed-auth rejection, and partial Basic-auth rejection.
- Test Run: `python3 -m unittest -v tests/test_python_dashboard_cli.py` (pass); `python3 -m unittest -v tests/test_python_alert_cli.py` (pass); `python3 -m unittest -v` (pass)
- Validation: README authentication examples now show the preferred flags and explicitly document the env-var fallback plus the rule that one command should use either token auth or Basic auth, not both.
- Impact: `grafana_utils/dashboard_cli.py`, `grafana_utils/alert_cli.py`, `tests/test_python_dashboard_cli.py`, `tests/test_python_alert_cli.py`, `README.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: Low to moderate operator-facing risk. Legacy auth flags still work, but commands that previously passed both token and Basic-auth inputs together will now fail fast and must choose one auth mode explicitly.
- Follow-up: Mirror the preferred auth flag names and validation rules in the Rust CLIs if cross-language parity becomes a requirement.

## 2026-03-11 - Add Dashboard List Subcommand
- Summary: Added a new read-only `list` subcommand to both the Python and Rust dashboard CLIs so operators can inspect live dashboard summaries without writing export files. Both implementations reuse the existing `/api/search` pagination helper, resolve folder tree path from `GET /api/folders/{uid}` when `folderUid` is present, and support compact text output plus `--table`, `--csv`, and `--json` rendering with `uid`, `name`, `folder`, `folderUid`, and `path`.
- Tests: Updated dashboard test coverage in both implementations to cover parser support for the new `list` mode, stable summary-line formatting, and list behavior against mocked `/api/search` results.
- Test Run: `python3 -m unittest -v tests/test_python_dashboard_cli.py` (pass); `cd rust && cargo test dashboard` (pass); `python3 -m unittest -v` (pass); `cd rust && cargo test` (pass)
- Validation: README, Traditional Chinese README, maintainer notes, and repo instructions now include the new `grafana-utils list` / `python3 cmd/grafana-utils.py list` entrypoints plus the available `--table`, `--csv`, and `--json` output modes. The command is read-only and does not change existing export/import/diff behavior.
- Impact: `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `README.zh-TW.md`, `DEVELOPER.md`, `AGENTS.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: Low operator-facing risk. The new mode only exposes an internal listing capability already used by export, and it does not alter any import/export payload handling.
- Follow-up: None.

## 2026-03-11 - Add Docker-Backed Rust Grafana Smoke Test
- Summary: Added a repeatable Docker-backed live validation path for the Rust CLIs with `scripts/test-rust-live-grafana.sh` and the root `make test-rust-live` shortcut. The script builds the Rust binaries, starts a temporary Grafana container, seeds a Prometheus datasource, a dashboard, and a webhook contact point, then validates dashboard export/import/diff/dry-run and alerting export/import/diff/dry-run against the live instance.
- Tests: Added Rust unit coverage for the alerting template-list null case so the client now matches Python behavior when Grafana returns JSON `null` from `/api/v1/provisioning/templates`.
- Test Run: `bash -n scripts/test-rust-live-grafana.sh` (pass); `cd rust && cargo test alert` (pass); `make test-rust-live` (pass); `cd rust && cargo test` (pass)
- Validation: The live smoke test passed against `grafana/grafana:12.4.1` on an auto-assigned localhost port, confirmed prompt export generated datasource `__inputs`, confirmed dashboard diff detected live drift, confirmed dashboard import restored a deleted dashboard, confirmed alert diff detected a changed exported contact point, and confirmed alert dry-run/update import reconciled that drift.
- Impact: `scripts/test-rust-live-grafana.sh`, `Makefile`, `README.md`, `README.zh-TW.md`, `DEVELOPER.md`, `AGENTS.md`, `rust/src/alert.rs`, `rust/src/alert_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: Low to moderate workflow risk. The new smoke test depends on Docker availability and local daemon access, but it is opt-in and pinned to a known Grafana version by default. The Rust null-handling fix reduces runtime failure risk against real Grafana alerting exports.
- Follow-up: If a future CI system is added, the new smoke test can be reused as the Docker-based Rust integration step instead of rebuilding this flow from scratch.

## 2026-03-11 - Add Versioned Export Schema, Dry-Run, and Diff Workflows
- Summary: Extended the Python dashboard and alerting CLIs so they can validate export schema versioning, preview import behavior safely, and compare local exports against live Grafana state before writing changes. Dashboard exports now write `export-metadata.json` manifests for the root and variant directories, the dashboard CLI now exposes `diff` as a first-class subcommand, and both Python CLIs now support non-mutating import `--dry-run`. The alerting tool-owned format now carries `schemaVersion` alongside the older `apiVersion`, import still accepts legacy tool documents without `schemaVersion`, and alerting diff now prints unified diffs for changed resources.
- Tests: Expanded Python CLI coverage around parser support for new dry-run and diff flags, schema-version validation, export manifest/index markers, dry-run non-mutation behavior, and unified diff output for changed dashboard and alert-rule payloads.
- Test Run: `python3 -m unittest -v tests/test_python_dashboard_cli.py` (pass); `python3 -m unittest -v tests/test_python_alert_cli.py` (pass); `python3 -m unittest -v` (pass)
- Validation: README, Traditional Chinese README, maintainer notes, and repo instructions were updated so operators can discover the new `diff` / `--diff-dir` workflows, understand the role of `export-metadata.json`, and know that nonzero exit status now signals drift when diff finds differences.
- Impact: `grafana_utils/dashboard_cli.py`, `grafana_utils/alert_cli.py`, `tests/test_python_dashboard_cli.py`, `tests/test_python_alert_cli.py`, `README.md`, `README.zh-TW.md`, `DEVELOPER.md`, `AGENTS.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: Moderate operator-facing risk because the new checks intentionally reject unsupported export schema versions and diff returns exit code `1` when drift exists. Existing legacy alerting exports remain importable, which reduces migration risk.
- Follow-up: Port the same schema-version, dry-run, and diff operator workflows into the Rust CLIs so the two implementations stay aligned.

## 2026-03-11 - Distinguish Python and Rust Test File Names
- Summary: Renamed the Python test modules so their filenames explicitly carry the implementation marker, and moved the Rust unit tests into dedicated `*_rust_tests.rs` files instead of keeping them inline inside production modules. The Python test files are now `test_python_dashboard_cli.py`, `test_python_alert_cli.py`, and `test_python_packaging.py`.
- Tests: No new behavior tests were added. Validation focused on test discovery and compile-time wiring after the file moves.
- Test Run: `python3 -m unittest -v` (pass); `cd rust && /opt/homebrew/bin/cargo test` (pass)
- Validation: Maintainer-facing docs were updated so targeted test commands and naming guidance now match the new Python and Rust test filenames.
- Impact: `tests/test_python_dashboard_cli.py`, `tests/test_python_alert_cli.py`, `tests/test_python_packaging.py`, `rust/src/common.rs`, `rust/src/http.rs`, `rust/src/alert.rs`, `rust/src/dashboard.rs`, `rust/src/common_rust_tests.rs`, `rust/src/http_rust_tests.rs`, `rust/src/alert_rust_tests.rs`, `rust/src/dashboard_rust_tests.rs`, `DEVELOPER.md`, `AGENTS.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: Low structural risk. The change affects only test-file layout and discovery, not production behavior.
- Follow-up: None.

## 2026-03-11 - Add Unified Build Makefile
- Summary: Added a root `Makefile` so the repo has one consistent command surface for building both implementations. The new targets cover Python wheel builds, Rust release builds, and aggregate `build` / `test` entrypoints.
- Tests: Validation is by executing the new `Makefile` targets directly instead of adding unit tests for shell behavior.
- Test Run: `make help` (pass); `make build-python` (pass); `make build-rust` (pass)
- Validation: README, Traditional Chinese README, maintainer notes, and repo instructions were updated to document the new `make` targets and where their build artifacts land. `make build-python` produced `dist/grafana_utils-0.1.0-py3-none-any.whl`, and `make build-rust` produced `rust/target/release/grafana-utils` plus `rust/target/release/grafana-alert-utils`.
- Impact: `Makefile`, `.gitignore`, `README.md`, `README.zh-TW.md`, `DEVELOPER.md`, `AGENTS.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: Low workflow risk. The new file adds convenience commands only and does not replace the existing direct `pip` or `cargo` build paths.
- Follow-up: None.

## 2026-03-11 - Rename Dashboard Export Variant Flags
- Summary: Renamed the dashboard export suppression flags in both implementations from `--without-raw` and `--without-prompt` to `--without-dashboard-raw` and `--without-dashboard-prompt`. The Python parser fields and Rust `ExportArgs` fields now use the dashboard-specific names as well, and the error text for disabling both export variants was updated to match.
- Tests: Updated the dashboard CLI unittest coverage to parse the renamed flags and to keep the invalid "disable both variants" path covered. Existing Rust dashboard tests continued to validate the export flow with the renamed `ExportArgs` fields.
- Test Run: `python3 -m unittest -v tests/test_dump_grafana_dashboards.py` (pass); `cd rust && /opt/homebrew/bin/cargo test dashboard` (pass); `python3 -m unittest -v` (pass); `cd rust && /opt/homebrew/bin/cargo test` (pass)
- Validation: README examples and option tables in both English and Traditional Chinese were updated to use the new flag names so the public documentation matches the Python and Rust CLI behavior.
- Impact: `grafana_utils/dashboard_cli.py`, `rust/src/dashboard.rs`, `tests/test_dump_grafana_dashboards.py`, `README.md`, `README.zh-TW.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Rollback/Risk: Low operator-facing rename risk because the change is limited to CLI flag names and matching internal field names. Existing scripts that still pass the old shorter flags will stop parsing until they are updated.
- Follow-up: None.

## 2026-03-11 - Port Grafana HTTP and API Flows Into Rust
- Summary: Added a shared Rust JSON HTTP client in `rust/src/http.rs` and moved the Rust crate beyond helper-only scaffolding. The dashboard Rust path now performs real raw and prompt-style dashboard export/import against Grafana APIs, including datasource placeholder rewriting for prompt exports, and the alerting Rust path now performs real export/import for rules, contact points, mute timings, notification policies, and templates.
- Tests: Expanded Rust module tests around the live-flow orchestration. The dashboard Rust tests now cover raw export/index writing, prompt-export datasource input generation from direct datasource refs and datasource template variables, dependent templating-variable datasource rewrites, and import request dispatch. The Rust crate test suite continues to cover helper-level alerting document normalization and path handling.
- Test Run: `cd rust && /opt/homebrew/bin/cargo test dashboard::tests::build_external_export_document_creates_input_from_datasource_template_variable -- --nocapture` (pass); `cd rust && /opt/homebrew/bin/cargo test dashboard` (pass); `/opt/homebrew/bin/cargo test` (pass); `python3 -m unittest -v` (pass)
- Validation: The Rust crate now compiles and its unit tests pass with the shared `reqwest`-based transport, including the prompt-export datasource rewrite path that previously required Python parity work. The Python implementation remains intact and its full unittest suite still passes, so the existing shipped behavior was not regressed while the Rust runtime path reached prompt-export parity.
- Impact: `rust/Cargo.toml`, `rust/Cargo.lock`, `rust/src/common.rs`, `rust/src/http.rs`, `rust/src/dashboard.rs`, `rust/src/alert.rs`, `docs/internal/ai-status.md`
- Rollback/Risk: Moderate architecture risk because the Rust crate now contains real API logic instead of only helper scaffolding. The main remaining risk is operational parity validation against a live Grafana instance before switching the shipped runtime entrypoints from Python to Rust.
- Follow-up: Validate the Rust binaries end-to-end against a live Grafana instance and then decide whether to switch packaging/runtime entrypoints from Python to Rust.

## 2026-03-11 - Add Rust Rewrite Scaffold for Grafana Utilities
- Summary: Added an isolated `rust/` crate as the starting point for rewriting the Grafana utilities in Rust. The new crate includes shared auth/path helpers in `rust/src/common.rs`, a dashboard-oriented module in `rust/src/dashboard.rs`, an alerting-oriented module in `rust/src/alert.rs`, and Rust binary entrypoints in `rust/src/bin/grafana-utils.rs` and `rust/src/bin/grafana-alert-utils.rs`.
- Tests: Added Rust unit tests inside the new modules for helper-level behavior such as auth resolution, path sanitization, path building, file discovery, and alerting document normalization. No Python tests needed changes because the shipping Python package remains the active implementation.
- Test Run: `python3 -m unittest -v` (pass); `/opt/homebrew/bin/cargo test` (pass)
- Validation: The existing Python test suite still passes after adding the Rust scaffold, so the current shipped CLI behavior is unchanged. The new Rust crate also compiles and its helper-level unit tests pass locally. The Rust crate is intentionally isolated so future porting can proceed incrementally without breaking the Python package before the network flows are ready.
- Impact: `rust/Cargo.toml`, `rust/Cargo.lock`, `rust/src/lib.rs`, `rust/src/common.rs`, `rust/src/dashboard.rs`, `rust/src/alert.rs`, `rust/src/bin/grafana-utils.rs`, `rust/src/bin/grafana-alert-utils.rs`, `docs/internal/ai-status.md`
- Rollback/Risk: Low runtime risk because the Python implementation still handles real operator workflows. The main risk is maintenance overhead while two implementations exist in parallel until the Rust HTTP and Grafana API flows are completed and validated.
- Follow-up: Port the HTTP transport and Grafana client flows into Rust, wire binary commands to the new network logic, and switch packaging/runtime entrypoints only after Rust validation is available locally.

## 2026-03-11 - Package Grafana Utilities for Installation
- Summary: Restructured the project into an installable `grafana_utils` package, moved the dashboard, alerting, and shared transport implementations under that package, and kept `cmd/grafana-utils.py` plus `cmd/grafana-alert-utils.py` as thin wrappers for direct source-tree usage. Added `pyproject.toml` with console-script entrypoints, base `requests` dependency, and an optional `http2` extra for `httpx[http2]` on Python 3.8+.
- Tests: Updated both CLI test modules to import the packaged modules while still parsing the `cmd/` wrappers for Python 3.6 syntax compatibility. Added `tests/test_packaging.py` to cover `pyproject.toml`, console-script declarations, and base dependency metadata.
- Test Run: `python3 -m unittest -v tests/test_dump_grafana_dashboards.py` (pass); `python3 -m unittest -v tests/test_grafana_alert_utils.py` (pass); `python3 -m unittest -v tests/test_packaging.py` (pass); `python3 -m unittest -v` (pass); `python3 -m pip install --no-deps --no-build-isolation --target /tmp/grafana-utils-install .` (install succeeded; local `pyenv` rehash hook returned a permissions warning after install)
- Validation: Public docs now describe `python3 -m pip install .`, `python3 -m pip install --user .`, and `python3 -m pip install '.[http2]'`, while maintainer docs describe the packaged layout and the thin `cmd/` wrappers. The package can be imported from an isolated `/tmp` install target and exposes version `0.1.0`.
- Impact: `pyproject.toml`, `grafana_utils/__init__.py`, `grafana_utils/dashboard_cli.py`, `grafana_utils/alert_cli.py`, `grafana_utils/http_transport.py`, `cmd/grafana-utils.py`, `cmd/grafana-alert-utils.py`, `tests/test_dump_grafana_dashboards.py`, `tests/test_grafana_alert_utils.py`, `tests/test_packaging.py`, `README.md`, `README.zh-TW.md`, `DEVELOPER.md`, `AGENTS.md`, `docs/internal/ai-status.md`
- Rollback/Risk: Moderate layout-change risk because imports and test loading now depend on the package structure instead of `cmd/` holding the main implementation. The thin wrappers preserve direct checkout usage, but external automation that imports by old filesystem path will need to follow the new package modules.
- Follow-up: If operator environments need fully offline installation, document a wheel-build workflow and pinned dependency constraints for the target Python baseline.

## 2026-03-11 - Enable Persistent Grafana HTTP Connections
- Summary: Upgraded the shared transport adapters so HTTP connections are reused instead of recreated for every request. The `requests` adapter now uses a persistent `requests.Session`, the `httpx` adapter now uses a persistent `httpx.Client`, and the default `auto` selector now prefers `httpx` only when HTTP/2 support is actually available at runtime. Otherwise it falls back to the pooled `requests` transport.
- Tests: Updated transport tests in both CLI test modules so the default-transport expectation follows the runtime capability helpers instead of assuming a hard-coded transport choice. Added direct assertions that the HTTP/2 capability helper returns a boolean.
- Test Run: `python3 -m unittest -v tests/test_dump_grafana_dashboards.py` (pass); `python3 -m unittest -v tests/test_grafana_alert_utils.py` (pass); `python3 -m unittest -v` (pass)
- Validation: Local full-suite unit tests passed after switching the transport implementations to persistent clients. In this environment `h2` is not installed, so the default runtime path uses keep-alive `requests.Session`; the code will automatically prefer HTTP/2-capable `httpx` in environments where `h2` is available.
- Impact: `cmd/grafana_http_transport.py`, `tests/test_dump_grafana_dashboards.py`, `tests/test_grafana_alert_utils.py`, `docs/internal/ai-status.md`
- Rollback/Risk: Low to moderate runtime-behavior risk because pooled clients can change how connections are reused across multiple requests. The transport API and client behavior remain the same from the caller's perspective, and tests passed after the change.
- Follow-up: If operator control is needed later, expose the `auto` / `requests` / `httpx` selection through CLI flags or environment variables.

## 2026-03-11 - Make Grafana HTTP Transport Replaceable
- Summary: Replaced the hard-wired `urllib` transport in both CLI tools with a shared transport module, `cmd/grafana_http_transport.py`. The new architecture introduces `RequestsJsonHttpTransport` and `HttpxJsonHttpTransport`, a small transport factory, and constructor injection so `GrafanaClient` and `GrafanaAlertClient` can use any compatible JSON transport implementation.
- Tests: Updated both test modules to load the shared transport module, verify Python 3.6 syntax parsing for it, verify both `requests` and `httpx` transport adapters build, and exercise the new injected-transport seam in the clients.
- Test Run: `python3 -m unittest -v tests/test_dump_grafana_dashboards.py` (pass); `python3 -m unittest -v tests/test_grafana_alert_utils.py` (pass); `python3 -m unittest -v` (pass)
- Validation: Local full-suite unit tests passed after removing the embedded `urllib` request logic from both clients. Transport-specific behavior is now isolated in the shared adapter module, while Grafana-specific error handling remains in the domain clients.
- Impact: `cmd/grafana_http_transport.py`, `cmd/grafana-utils.py`, `cmd/grafana-alert-utils.py`, `tests/test_dump_grafana_dashboards.py`, `tests/test_grafana_alert_utils.py`, `docs/internal/ai-status.md`
- Rollback/Risk: Moderate refactor risk because all network access now passes through the shared adapter layer. Existing tests passed, but any environment missing `requests` at runtime would now fail until the dependency is installed or an alternate transport is injected explicitly.
- Follow-up: If operators need runtime selection later, expose the transport choice through CLI flags or environment variables instead of changing the client classes again.

## 2026-03-11 - Refactor Grafana CLI Readability
- Summary: Refactored `cmd/grafana-utils.py` and `cmd/grafana-alert-utils.py` for human readability without changing behavior. The dashboard CLI now uses smaller helpers for dashboard object extraction, datasource lookup and normalization, template-variable rewrite steps, and export index construction. The alerting CLI now uses smaller helpers for linked-dashboard mapping, per-resource export handling, and per-kind import dispatch.
- Tests: No new tests were needed because the refactor preserved behavior. Existing coverage was used to validate the structural changes.
- Test Run: `python3 -m unittest -v` (pass)
- Validation: Local full-suite unit tests passed after the refactor. The resulting top-level flows are shorter and easier to scan, with behavior-sensitive logic moved into named helpers.
- Impact: `cmd/grafana-utils.py`, `cmd/grafana-alert-utils.py`, `docs/internal/ai-status.md`
- Rollback/Risk: Low to moderate risk because logic moved across helper boundaries, but no contracts or CLI behavior were intentionally changed and the existing test suite passed after the refactor.
- Follow-up: If readability needs more work later, the next candidates are normalizing repeated JSON write patterns and grouping the client API methods by resource family.

## 2026-03-11 - Move Grafana CLIs Into cmd
- Summary: Moved the dashboard and alerting CLI entrypoints from the repository root into `cmd/`, updated the scripts' embedded help/output strings to reflect the new invocation paths, and refreshed public and maintainer docs to use `python3 cmd/grafana-utils.py ...` and `python3 cmd/grafana-alert-utils.py ...`.
- Tests: Updated `tests/test_dump_grafana_dashboards.py` and `tests/test_grafana_alert_utils.py` to load the scripts from `cmd/`, and added `tests/__init__.py` so default `unittest` discovery reaches both modules.
- Test Run: `python3 -m unittest -v tests/test_dump_grafana_dashboards.py` (pass); `python3 -m unittest -v tests/test_grafana_alert_utils.py` (pass); `python3 -m unittest -v` (pass)
- Validation: Local unit tests passed after the move. `README.md`, `README.zh-TW.md`, `DEVELOPER.md`, and `AGENTS.md` now point at the new `cmd/` entrypoints, and the documented full-suite test command now discovers all tests.
- Impact: `cmd/grafana-utils.py`, `cmd/grafana-alert-utils.py`, `tests/test_dump_grafana_dashboards.py`, `tests/test_grafana_alert_utils.py`, `tests/__init__.py`, `README.md`, `README.zh-TW.md`, `DEVELOPER.md`, `AGENTS.md`, `docs/internal/ai-status.md`
- Rollback/Risk: Moderate path-change risk for anyone invoking the old root-level scripts directly. The docs and tests now consistently point at `cmd/`, but external automation will need the same path update.
- Follow-up: If backward compatibility is required later, add thin root-level wrapper scripts instead of moving the implementation back out of `cmd/`.

## 2026-03-10 - Extend Grafana Alerting Resource Coverage
- Summary: Extended `grafana-alert-utils.py` beyond rules, contact points, mute timings, and notification policies by adding notification template export/import support, explicit dashboard UID and panel ID mapping files for linked alert rules, and richer linked-dashboard metadata capture during export. Template import now uses the template name as the stable identity, fetches the current template version before `PUT` updates, and tolerates Grafana returning `null` from the template list endpoint when no templates exist.
- Tests: Expanded `tests/test_grafana_alert_utils.py` to cover template export documents, template import payload validation, template create/update conflict handling, parser support for the new mapping flags, empty template list handling, and linked alert-rule rewrite behavior when dashboard and panel maps are provided.
- Test Run: `python3 -m unittest -v` (pass)
- Validation: Local unit tests passed for the full project test target. README was updated to document `alerts/raw/templates/`, `--dashboard-uid-map`, `--panel-id-map`, and template update behavior.
- Impact: `grafana-alert-utils.py`, `tests/test_grafana_alert_utils.py`, `README.md`, `docs/internal/ai-status.md`
- Rollback/Risk: Moderate risk is limited to the standalone alert CLI. Template updates still depend on Grafana's provisioning API behavior and linked-rule automatic fallback only rewrites dashboard UID unless a panel map is supplied.
- Follow-up: If environments rely on template groups or panel IDs that are regenerated during dashboard migration, add an optional live validation flow against Grafana 9.x/10.x in addition to the current unit coverage.

## 2026-03-10 - Rename Grafana Dashboard Export Flag
- Summary: Renamed the dashboard export CLI flag from `--output-dir` to `--export-dir` in `grafana-utils.py`. The change updates the parser, the parsed argument name, the help text, and the dashboard README examples so export mode reads clearly next to the explicit `import` subcommand.
- Tests: Updated the dashboard CLI parse test to assert the default `export_dir` value and reran the dashboard test suite.
- Test Run: `python3 -m unittest tests.test_dump_grafana_dashboards` (pass)
- Validation: Local dashboard unit tests passed and the dashboard CLI help now shows `--export-dir` under the `export` subcommand.
- Impact: `grafana-utils.py`, `tests/test_dump_grafana_dashboards.py`, `README.md`, `docs/internal/ai-status.md`
- Rollback/Risk: Low to moderate operator-facing change because older `--output-dir` dashboard export invocations will no longer parse. The rename makes export intent more explicit.
- Follow-up: None.

## 2026-03-10 - Add Grafana Dashboard Import and Export Subcommands
- Summary: Changed `grafana-utils.py` so dashboard mode selection is explicit at the CLI level. The script now requires `export` or `import` subcommands, and export-only and import-only options live on separate subparsers instead of being mixed together on one parser.
- Tests: Updated the dashboard CLI tests to cover required subcommand selection, export defaults, import parsing, and the export validation path under the new command layout.
- Test Run: `python3 -m unittest tests.test_dump_grafana_dashboards` (pass)
- Validation: Local dashboard unit tests passed and README examples were updated to use `python3 grafana-utils.py export ...` and `python3 grafana-utils.py import ...`.
- Impact: `grafana-utils.py`, `tests/test_dump_grafana_dashboards.py`, `README.md`, `docs/internal/ai-status.md`
- Rollback/Risk: Moderate operator-facing change because old invocations without subcommands will now fail argument parsing. The benefit is that import and export intent is explicit.
- Follow-up: If backward compatibility is needed later, add a deliberate legacy shim rather than returning to implicit mode inference.

## 2026-03-10 - Change Grafana Default Server URL
- Summary: Changed the default Grafana base URL in both utilities from a hardcoded remote host to `http://127.0.0.1:3000`. Updated README examples and added direct unit tests so the new default is locked in.
- Tests: Added parse-args assertions for the default URL in both dashboard and alert utility test suites.
- Test Run: `python3 -m unittest tests.test_dump_grafana_dashboards tests.test_grafana_alert_utils` (pass)
- Validation: Local unit tests passed and README examples now match the CLI defaults.
- Impact: `grafana-utils.py`, `grafana-alert-utils.py`, `tests/test_dump_grafana_dashboards.py`, `tests/test_grafana_alert_utils.py`, `README.md`, `docs/internal/ai-status.md`
- Rollback/Risk: Low risk. This only changes the CLI default target; explicit `--url` values still override it.
- Follow-up: None.

## 2026-03-10 - Make Grafana Utilities RHEL 8 Python Compatible
- Summary: Reworked type annotations in both Grafana utility scripts so they no longer depend on Python 3.9+ built-in generics or Python 3.10+ union syntax. Removed `from __future__ import annotations` and converted signatures and local annotations to `typing.List`, `typing.Dict`, `typing.Optional`, `typing.Set`, and `typing.Tuple`. Latest change: documented RHEL 8+ support in the README and turned the Python 3.6 syntax parse check into permanent unit tests for both entrypoints.
- Tests: Reused the existing dashboard and alerting `unittest` suites to confirm the syntax-only compatibility refactor did not change behavior. Added parser-level unit tests that validate both scripts with `ast.parse(..., feature_version=(3, 6))`.
- Test Run: `python3 -m unittest -v` (pass)
- Validation: Local unit tests passed, both scripts parsed successfully as Python 3.6 grammar, and the README now states RHEL 8+ support explicitly.
- Impact: `grafana-utils.py`, `grafana-alert-utils.py`, `docs/internal/ai-status.md`
- Rollback/Risk: Low risk. This is a syntax-compatibility refactor only; behavior should remain unchanged.
- Follow-up: If RHEL 8 deployment uses a stricter runtime baseline than Python 3.6, validate the full CLI workflows there against the target Grafana instance.

## 2026-03-10 - Add Grafana Alerting Utility
- Summary: Expanded the standalone CLI, `grafana-alert-utils.py`, from rule-only backup/restore into a broader Grafana alerting utility. Export now writes a tool-owned JSON format under `alerts/raw/` with separate subdirectories for rules, contact points, mute timings, and notification policies. Import reads that same format and uses the Grafana alerting provisioning API to create or update rules/contact points/mute timings and to apply the notification policy tree. Latest change: alert-rule exports now capture linked dashboard metadata when a rule carries `__dashboardUid__` / `__panelId__`, and import now repairs `__dashboardUid__` automatically when the source dashboard UID is missing on the target Grafana but a unique dashboard match exists by exported title, folder title, and slug.
- Tests: Added `unittest` coverage for alert CLI argument parsing, auth handling, SSL behavior, per-resource path generation, export-root rejection on import, server-managed field stripping for all supported resource kinds, import payload validation, provisioning-export rejection, resource kind detection, export file/index generation across all resource types, create/update dispatch for rules/contact points/mute timings, policy import safety checks, linked-dashboard metadata preservation, and dashboard-UID fallback rewrite behavior for linked alert rules.
- Test Run: `python3 -m unittest -v` (pass)
- Reason: Unit tests cover the code paths locally, and live validation was performed against a temporary Docker Grafana instance rather than the user's Grafana environment.
- Validation: `python3 -m unittest -v`; updated `README.md`; live Docker validation against Grafana 12.4.1 on `http://127.0.0.1:33000` by creating folder `codex-alert-folder`, alert rule `afflu1oeeir5sd`, contact point `codex-webhook`, mute timing `codex-mute`, and a notification policy tree pointing at them; exporting all resources with `grafana-alert-utils.py`; resetting Grafana state; importing from `/tmp/grafana-alert-export-v2/raw`; and confirming the recreated resources preserved the rule UID, folder UID, rule group, contact point UID, mute timing name, and policy references. Additional live validation created a dashboard-linked alert rule with `__dashboardUid__=\"source-dashboard-uid\"` and `__panelId__=\"7\"`, exported it, deleted the source dashboard, created a same-title same-folder replacement dashboard with UID `target-dashboard-uid`, imported the alert backup, and confirmed the rule annotations were rewritten to `__dashboardUid__=\"target-dashboard-uid\"` while preserving `__panelId__=\"7\"`.
- Impact: `grafana-alert-utils.py`, `tests/test_grafana_alert_utils.py`, `README.md`, `docs/internal/ai-status.md`
- Rollback/Risk: Low to moderate risk. The new tool is isolated from `grafana-utils.py`, but import still depends on the target Grafana having any referenced folders and other alerting dependencies available or being restored in the same import set.
- Follow-up: If needed later, extend the separate alert CLI to cover message templates and other remaining Grafana alerting resources without folding that logic into the dashboard utility.

## 2026-03-10 - Export Grafana Dashboards
- Summary: Added a standalone Python utility to export Grafana dashboards by UID into local JSON files, extended it with import support for recursively loading exported dashboard JSON back into Grafana, and added datasource-prompt export behavior that now follows the import-critical pattern from the provided `1-prompt.json`. Current architecture writes both `dashboards/raw/` and `dashboards/prompt/` by default, with `raw/` intended for preserved-UID/API-safe imports and `prompt/` intended for Grafana web imports that ask for datasource mapping. Latest change: added `--without-raw` and `--without-prompt` so one export run can still be selective when needed, while rejecting the invalid case where both are disabled.
- Tests: Added `unittest` coverage for auth handling, CLI SSL behavior, dual export variant directory layout, variant suppression flags, rejection of disabling all export variants, path generation, pagination, overwrite protection, import file discovery, rejection of the combined export root, import payload shaping, preserved-uid web-import export shape, website-import placeholder export behavior, generic datasource input generation, datasource placeholder object rewriting, conversion of typed datasource variables into import placeholders, creation of import placeholders from datasource template variables, synthesized datasource template variables for single-type dashboards, passthrough handling for untyped datasource variables, passthrough handling for Grafana built-in datasource aliases, resolution of datasource references expressed as plain-string UIDs, and datasource type alias fallback.
- Test Run: `python3 -m unittest -v` (pass)
- Reason: Live Grafana export was not run because this turn did not include usable credentials or a network execution request against the target instance.
- Validation: `python3 -m unittest -v`; updated `README.md`
- Impact: `grafana-utils.py`, `tests/test_dump_grafana_dashboards.py`
- Rollback/Risk: Low risk. Revert by deleting the new utility and test files. Website-import exports with `__inputs` are meant for Grafana’s web UI and are not accepted by the script’s API import mode.
- Follow-up: Run one export and confirm `dashboards/raw/` and `dashboards/prompt/` are both populated, then use `dashboards/raw/` for API imports and `dashboards/prompt/` for Grafana web imports.
