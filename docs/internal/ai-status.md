# ai-status.md

## 2026-03-12 - Task: Add Developer Grafana Sample-Data Seed Script
- State: Done
- Scope: `scripts/seed-grafana-sample-data.sh`, `Makefile`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Developer live testing was relying on one-off manual API calls to create sample datasources, folders, dashboards, and extra orgs. That made repeated verification of `list-dashboard`, `export-dashboard`, and `list-data-sources` less reproducible.
- Current Update: Added `make seed-grafana-sample-data`, `make destroy-grafana-sample-data`, and a dedicated shell script that seeds or removes a running Grafana test dataset with stable sample orgs, datasources, folders, and dashboards using fixed ids and overwrite-friendly upserts.
- Result: Developers now have repo-owned setup and cleanup commands for rebuilding the same manual test dataset instead of repeating ad hoc setup steps during local Grafana testing.

## 2026-03-12 - Task: Add Prompted Basic-Auth Password Support
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `grafana_utils/alert_cli.py`, `grafana_utils/access_cli.py`, `tests/test_python_dashboard_cli.py`, `tests/test_python_alert_cli.py`, `tests/test_python_access_cli.py`, `rust/Cargo.toml`, `rust/src/common.rs`, `rust/src/common_rust_tests.rs`, `rust/src/dashboard.rs`, `rust/src/dashboard_rust_tests.rs`, `rust/src/alert.rs`, `rust/src/alert_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The CLIs only supported token auth, explicit `--basic-password`, or environment fallback password input. Operators who wanted Basic auth had to expose the password through shell history, process arguments, or environment variables.
- Current Update: Added `--prompt-password` everywhere Basic auth is supported, wired it into the shared Python and Rust auth resolvers, and added validation that rejects mixing prompt mode with token auth or explicit `--basic-password`.
- Result: Operators can now run Basic-auth commands with `--basic-user ... --prompt-password` and enter the password securely without echo while keeping the existing token and environment-based auth paths.

## 2026-03-12 - Task: Add Platform-Specific Rust Build Paths
- State: Done
- Scope: `Makefile`, `scripts/build-rust-macos-arm64.sh`, `scripts/build-rust-linux-amd64.sh`, `scripts/build-rust-linux-amd64-zig.sh`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The repo could build native Rust release binaries on the current host only, but there was no explicit platform-targeted release workflow. In particular, macOS Apple Silicon and Linux `amd64` outputs did not have named Make targets or stable artifact directories.
- Current Update: Added `make build-rust-macos-arm64` for native Apple Silicon builds into `dist/macos-arm64/`, `make build-rust-linux-amd64` for Docker-based Linux `amd64` builds into `dist/linux-amd64/`, and `make build-rust-linux-amd64-zig` for non-Docker Linux `amd64` builds using local `zig`.
- Result: Operators on macOS now have explicit repo-owned release paths for native Apple Silicon binaries plus Linux `amd64` binaries through either Docker or local zig.

## 2026-03-12 - Task: Update Dashboard Help Examples And Local Default URL
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The dashboard CLI still defaulted to `http://127.0.0.1:3000`, and the real `-h` output either lacked examples entirely or only showed token-based remote examples. That made first-run local usage harder, especially for operators using Basic auth.
- Current Update: Changed the dashboard CLI default URL to `http://localhost:3000`, updated Python and Rust help output to show local Basic-auth examples plus token examples, and refreshed the public and maintainer docs to match the new local-first help text.
- Result: The shipped Python and Rust dashboard CLIs now guide operators toward a working local Grafana flow directly from `-h`, while still documenting token auth when needed.

## 2026-03-12 - Task: Add Dashboard Multi-Org Export
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `export-dashboard` only operated in the current Grafana org context. Operators could not export one explicit org or aggregate exports across all visible orgs, even after `list-dashboard` gained org selection support.
- Current Update: Added `--org-id` and `--all-orgs` to Python and Rust `export-dashboard`. Both paths are Basic-auth-only. Explicit-org export reuses the existing layout, while multi-org export writes `org_<id>_<name>/raw/...` and `org_<id>_<name>/prompt/...` trees plus aggregate root-level variant indexes so cross-org dashboards do not overwrite each other.
- Result: Operators can now export dashboards from one chosen org or every visible org without manually switching Grafana org context first.

## 2026-03-12 - Task: Add Dashboard Multi-Org Listing
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `list-dashboard` already exposed current-org metadata in each row, but it still only listed dashboards in the current request org context. Operators could not point the command at another org or aggregate dashboards across all visible orgs from one run.
- Current Update: Added `--org-id` and `--all-orgs` to Python and Rust `list-dashboard`. The command now accepts one explicit org override or enumerates `/api/orgs` and aggregates dashboard results across all visible orgs. Both paths are Basic-auth-only and preserve the existing `org` and `orgId` output fields for every listed dashboard.
- Result: Operators can now inspect one chosen Grafana org or all visible orgs from a single `list-dashboard` run instead of being limited to the auth context's current org.

## 2026-03-12 - Task: Add Dashboard Datasource Listing Command
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The dashboard CLI could list dashboards and could fetch the datasource catalog internally, but there was no dedicated operator command to inspect Grafana data sources directly with table, CSV, or JSON output.
- Current Update: Added `list-data-sources` in both Python and Rust, reusing the existing datasource list API path and adding compact text, `--table`, `--csv`, and `--json` renderers for `uid`, `name`, `type`, `url`, and `isDefault`.
- Result: Operators can now inspect live Grafana data sources directly from `grafana-utils` without exporting dashboards or reading raw API responses.

## 2026-03-12 - Task: Rename Dashboard CLI Subcommands
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The dashboard CLI exposed short subcommand names `export`, `list`, and `import`, while the repo now also contains separate alerting and access CLIs. The shorter names made the dashboard actions look inconsistent next to the more explicit access subcommands and left room for ambiguity when reading docs quickly.
- Current Update: Renamed the dashboard CLI subcommands to `export-dashboard`, `list-dashboard`, and `import-dashboard` in both Python and Rust, updated focused parser/help coverage, and refreshed public and maintainer docs to use the new names consistently.
- Result: Dashboard operations now read explicitly at the CLI boundary, and both Python and Rust `grafana-utils` help/output surfaces match the renamed operator workflow.

## 2026-03-12 - Task: Add Dashboard List Org Metadata
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The dashboard `list` subcommand already showed folder and datasource context, but operators still could not see which Grafana organization the current authenticated view belonged to in text, table, CSV, or JSON output.
- Current Update: Added one current-org fetch through `GET /api/org` in both Python and Rust dashboard list paths, attached `org` and `orgId` to every listed dashboard summary, and extended the renderer/tests so compact text, table, CSV, and JSON output all include those fields alongside the existing folder and optional datasource metadata.
- Result: Operators can now tell which Grafana org produced a given dashboard list result without guessing from the base URL or credentials, and machine-readable list consumers now receive stable `org` and `orgId` fields in both Python and Rust.

## 2026-03-12 - Task: Add Dashboard List Datasource Display
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The dashboard `list` subcommand already showed `uid`, `name`, `folder`, `folderUid`, and resolved folder path, but it could not show which datasource names each dashboard used.
- Current Update: Added an opt-in `--with-sources` flag to both Python and Rust dashboard list paths. When enabled, the command fetches the datasource catalog and each dashboard payload, resolves datasource references into display names, and appends those names to text, table, CSV, and JSON output. CSV output also carries a best-effort `sourceUids` column.
- Result: Operators can now inspect dashboard datasource usage directly from `grafana-utils list-dashboard --with-sources` without exporting dashboard files, while plain `list-dashboard` remains unchanged and cheaper. CSV consumers can also capture concrete datasource UIDs when Grafana exposed them.

## 2026-03-12 - Task: Add Python Access Live Smoke Test
- State: Done
- Scope: `scripts/test-python-access-live-grafana.sh`, `Makefile`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Python access CLI had live Docker validation recorded in docs, but there was no checked-in script to reproduce those user, team, and service-account workflows end to end.
- Current Update: Added a Docker-backed smoke script for the Python access CLI and a `make test-access-live` target. The script starts Grafana, bootstraps a token, then validates user add/modify/delete, team add/list/modify, and service-account add/token/list flows with the auth modes each command expects.
- Result: The repo now has a repeatable live validation path for the Python access CLI instead of relying only on ad hoc one-off Docker checks.

## 2026-03-12 - Task: Add Access Utility Team Add
- State: Done
- Scope: `grafana_utils/access_cli.py`, `tests/test_python_access_cli.py`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`, `TODO.md`
- Baseline: The Python access CLI already covered `team list` and `team modify`, but `TODO.md` still listed `team add` as one of the remaining team-lifecycle gaps.
- Current Update: Added `grafana-access-utils team add` with parser/help wiring, Grafana team creation through the org-scoped team API, optional initial `--member` and `--admin` seeding, and aligned public and maintainer docs. The command creates the team first, then reuses the existing exact org-user resolution and safe membership/admin update flow.
- Result: The Python access CLI now covers `team add` alongside the existing user, team-list, team-modify, and service-account workflows, leaving only `team delete` plus the `group` alias in the team/group backlog.

## 2026-03-11 - Task: Add Access Utility User List
- State: Done
- Scope: `grafana_utils/access_cli.py`, `tests/test_python_access_cli.py`, `pyproject.toml`, `cmd/grafana-access-utils.py`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The repo currently has dashboard and alerting CLIs only. `TODO.md` defines a future `grafana-access-utils` command shape, but there is no packaged script, wrapper, or public documentation for access-management workflows yet.
- Current Update: Added `grafana_utils/access_cli.py` with an initial Python access-management surface that now covers `user list` plus `service-account list`, `service-account add`, and `service-account token add`. Packaging wiring, focused unit coverage, and public/maintainer docs now describe the access CLI as Python-only for this first cut. The auth split is explicit: org-scoped user listing may use token or Basic auth, global user listing requires Basic auth, and the service-account commands are org-scoped and may use token or Basic auth.
- Result: The repo now ships a first Python access-management CLI surface for user listing and service-account creation flows, with focused tests plus a full Python suite pass confirming the new command does not regress the existing dashboard and alerting tools.

## 2026-03-11 - Task: Add Access Utility Team List
- State: Done
- Scope: `grafana_utils/access_cli.py`, `tests/test_python_access_cli.py`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Python access CLI already supports `user list` plus initial service-account commands, but `TODO.md` still lists all `team` operations as not started and the public docs say no `team` command exists yet.
- Current Update: Added a read-only `grafana-access-utils team list` command with org-scoped team search, optional member lookup, standard `--table|--csv|--json` output modes, and incomplete-command help for `grafana-access-utils team`. Public and maintainer docs now include the command and its auth expectations.
- Result: The Python access CLI now covers `user list`, `team list`, and the initial service-account workflows, with targeted and full Python test suite passes confirming the new command surface.

## 2026-03-11 - Task: Add Access Utility User Add
- State: Done
- Scope: `grafana_utils/access_cli.py`, `tests/test_python_access_cli.py`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Python access CLI already supports `user list`, `team list`, and the initial service-account commands, but it still cannot create Grafana users even though `TODO.md` calls out `user add` as one of the next lifecycle steps.
- Current Update: Added `grafana-access-utils user add` as a Basic-auth server-admin workflow that creates Grafana users through the admin API, supports optional org-role and Grafana-admin follow-up updates, and avoids the `--basic-password` versus new-user `--password` flag collision by separating the internal parser destinations and help text.
- Result: The Python access CLI now covers `user list`, `user add`, `team list`, and the initial service-account workflows, with targeted tests, the full Python suite, and a Docker-backed Grafana `12.4.1` smoke test confirming the new command path.

## 2026-03-11 - Task: Add Access Utility Team Modify
- State: Done
- Scope: `grafana_utils/access_cli.py`, `tests/test_python_access_cli.py`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Python access CLI can now list teams, but it still cannot add or remove team members or admins even though `TODO.md` puts `team modify` next in the planned access-management sequence.
- Current Update: Added `grafana-access-utils team modify` with `--team-id` or exact `--name` targeting, add/remove member actions, add/remove admin actions, and text or `--json` output. The command resolves users by exact login or email, uses org-scoped team APIs, and preserves admin changes safely by reading current member permission metadata before issuing the bulk admin update payload.
- Result: The Python access CLI now covers `user list`, `user add`, `team list`, `team modify`, and the initial service-account workflows, with targeted tests, the full Python suite, and Docker-backed Grafana `12.4.1` smoke tests confirming member and admin modification flows with both Basic auth and token auth.

## 2026-03-12 - Task: Add Access Utility User Modify
- State: Done
- Scope: `grafana_utils/access_cli.py`, `tests/test_python_access_cli.py`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Python access CLI can now create users and modify teams, but it still cannot update an existing user's identity fields, password, org role, or Grafana-admin state even though `TODO.md` lists `user modify` as the next user-lifecycle step.
- Current Update: Added `grafana-access-utils user modify` with id, login, or email targeting; explicit setters for login, email, name, password, org role, and Grafana-admin state; and text or `--json` output. The command is Basic-auth-only, updates profile fields and password through the global/admin user APIs, and reuses the existing org-role and permission update paths for role changes.
- Result: The Python access CLI now covers `user list`, `user add`, `user modify`, `team list`, `team modify`, and the initial service-account workflows, with targeted tests, the full Python suite, and a Docker-backed Grafana `12.4.1` smoke test confirming the update path.

## 2026-03-12 - Task: Add Access Utility User Delete
- State: Done
- Scope: `grafana_utils/access_cli.py`, `tests/test_python_access_cli.py`, `README.md`, `DEVELOPER.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Python access CLI can now create and modify users, but it still cannot remove users even though `TODO.md` keeps `user delete` as the next unfinished user-lifecycle step.
- Current Update: Added `grafana-access-utils user delete` with id, login, or email targeting; `--scope org|global`; required `--yes` confirmation; and text or `--json` output. Global deletion uses the admin delete API and requires Basic auth, while org-scoped removal uses the org user API and works with token or Basic auth.
- Result: The Python access CLI now covers `user list`, `user add`, `user modify`, `user delete`, `team list`, `team modify`, and the initial service-account workflows, with targeted tests, the full Python suite, and Docker-backed Grafana `12.4.1` smoke tests confirming both global delete and org-scoped removal flows.

## 2026-03-11 - Task: Remove Python Dependency From Rust Live Smoke Test
- State: Done
- Scope: `scripts/test-rust-live-grafana.sh`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Rust Docker smoke script required `python3` only to extract simple JSON fields while creating a Grafana API token.
- Current Update: Replaced the JSON field helper with `jq`, removed the explicit `python3` prerequisite from the script, replaced the last Perl-based in-place JSON rewrite with a `jq` temp-file rewrite, and now check for `jq` at startup.
- Result: The Rust live smoke test no longer depends on Python or Perl and now keeps its runtime requirements to Docker, curl, and `jq`.

## 2026-03-11 - Task: Clarify Rust CLI Help Text
- State: Done
- Scope: `rust/src/dashboard.rs`, `rust/src/alert.rs`, `rust/src/dashboard_rust_tests.rs`, `rust/src/alert_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Rust `-h` and `--help` output listed many flags without operator-facing explanations, so switches like `--flat` were hard to understand from the CLI alone.
- Current Update: Added explicit clap help text for common auth/TLS flags plus dashboard and alerting mode flags, and added help-output tests that assert the Rust help explains flat export layout and includes examples.
- Result: `grafana-utils export-dashboard -h` and `grafana-alert-utils -h` now explain what options do instead of only showing their names, reducing the need to cross-reference README or Python help for common workflows.

## 2026-03-11 - Task: Add Preferred Auth Flag Aliases
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `grafana_utils/alert_cli.py`, `tests/test_python_dashboard_cli.py`, `tests/test_python_alert_cli.py`, `README.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Python dashboard and alerting CLIs only advertise `--api-token`, `--username`, and `--password`, even though the auth TODO now prefers `--token`, `--basic-user`, and `--basic-password`. Mixed token and Basic-auth input also resolves implicitly instead of failing early.
- Current Update: Added preferred CLI aliases for token and Basic auth in both Python CLIs while keeping the legacy flag names accepted, updated help text to advertise the preferred flags, and tightened `resolve_auth` so mixed token plus Basic input and partial Basic-auth input fail with clear operator-facing errors.
- Result: Operators can now use `--token`, `--basic-user`, and `--basic-password` consistently across both Python CLIs, while older flag names still parse. `python3 -m unittest -v tests/test_python_dashboard_cli.py`, `python3 -m unittest -v tests/test_python_alert_cli.py`, and `python3 -m unittest -v` all pass after the auth validation change.

## 2026-03-11 - Task: Add Dashboard List Subcommand
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `tests/test_python_dashboard_cli.py`, `rust/src/dashboard.rs`, `rust/src/dashboard_rust_tests.rs`, `README.md`, `README.zh-TW.md`, `DEVELOPER.md`, `AGENTS.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The dashboard CLIs currently expose `export`, `import`, and `diff`, but there is no standalone operator command for listing dashboards without writing export files. The underlying `/api/search` lookup already exists only as an internal export helper.
- Current Update: Added a new explicit `list` subcommand in both Python and Rust dashboard CLIs, reusing the existing `/api/search` pagination path and enriching summaries with folder tree path from `GET /api/folders/{uid}` when `folderUid` is present. The command now supports compact text output, `--table`, `--csv`, and `--json`, with tests covering parser support, machine-readable renderers, table formatting, and folder hierarchy resolution.
- Result: Operators can now run `grafana-utils list` to inspect live dashboard summaries without exporting files first, and choose human-readable or machine-readable output with `--table`, `--csv`, or `--json`. The output fields are `uid`, `name`, `folder`, `folderUid`, and resolved folder tree path. Both `python3 -m unittest -v tests/test_python_dashboard_cli.py` and `cd rust && cargo test dashboard` pass, and the full Python and Rust test suites still pass after the new list formatting work.

## 2026-03-11 - Task: Add Docker-Backed Rust Grafana Smoke Test
- State: Done
- Scope: `scripts/test-rust-live-grafana.sh`, `Makefile`, `README.md`, `README.zh-TW.md`, `DEVELOPER.md`, `AGENTS.md`, `rust/src/alert.rs`, `rust/src/alert_rust_tests.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Rust CLIs already have unit coverage, but the repo has no repeatable live Grafana validation path for the Rust export/import/diff/dry-run workflows. Manual Docker validation knowledge is scattered, and the Rust alerting client still rejects Grafana template-list responses when the API returns JSON `null`.
- Current Update: Added `scripts/test-rust-live-grafana.sh` plus `make test-rust-live` to start a temporary Grafana Docker container, seed a datasource/dashboard/contact point, and exercise Rust dashboard export/import/diff/dry-run plus Rust alerting export/import/diff/dry-run. The script now defaults to pinned image `grafana/grafana:12.4.1`, auto-selects a free localhost port when `GRAFANA_PORT` is unset, and cleans up the container automatically. Also fixed the Rust alerting template-list path so `GET /api/v1/provisioning/templates` returning JSON `null` is treated as an empty list, matching the Python behavior.
- Result: `make test-rust-live` now passes locally against a temporary Docker Grafana instance, and `cd rust && cargo test` still passes after the Rust alerting null-handling fix. Maintainer and public docs now point at the live smoke-test entrypoint and its overrides.

## 2026-03-11 - Task: Add Versioned Export Schema, Dry-Run, and Diff Workflows
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `grafana_utils/alert_cli.py`, `tests/test_python_dashboard_cli.py`, `tests/test_python_alert_cli.py`, `README.md`, `README.zh-TW.md`, `DEVELOPER.md`, `AGENTS.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Python CLIs can export and import Grafana dashboards and alerting resources, but there is no versioned export schema marker for dashboards, no dry-run path to preview import behavior safely, and no built-in diff workflow to compare local exports against live Grafana state.
- Current Update: Added versioned export metadata for dashboard exports and extended alerting tool documents/root indexes with `schemaVersion`, while keeping older alerting `apiVersion`-only tool docs importable. Added non-mutating import `--dry-run` behavior for both CLIs, added dashboard `diff` as an explicit subcommand, and added alerting `--diff-dir` to compare exported files with live Grafana resources. Both diff paths now print unified diffs for changed documents.
- Result: Operators can validate export shape compatibility, preview create/update behavior safely, and compare local exports against Grafana before applying changes. The focused Python dashboard and alerting suites plus the full Python suite pass with the new workflows.

## 2026-03-11 - Task: Distinguish Python and Rust Test File Names
- State: Done
- Scope: `tests/test_python_dashboard_cli.py`, `tests/test_python_alert_cli.py`, `tests/test_python_packaging.py`, `rust/src/common.rs`, `rust/src/http.rs`, `rust/src/alert.rs`, `rust/src/dashboard.rs`, `rust/src/common_rust_tests.rs`, `rust/src/http_rust_tests.rs`, `rust/src/alert_rust_tests.rs`, `rust/src/dashboard_rust_tests.rs`, `DEVELOPER.md`, `AGENTS.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Python tests are named generically under `tests/test_*.py`, while Rust unit tests are inline inside implementation files. That makes it hard to distinguish Python and Rust test files by filename alone.
- Current Update: Renamed the Python test files to `test_python_*`, moved the Rust unit tests into dedicated `*_rust_tests.rs` files loaded from their parent modules, and updated maintainer docs to use the new test names and layout.
- Result: Python and Rust test files are now distinguishable by filename, and both `python3 -m unittest -v` and `cd rust && cargo test` still pass with the new layout.

## 2026-03-11 - Task: Add Unified Build Makefile
- State: Done
- Scope: `Makefile`, `.gitignore`, `README.md`, `README.zh-TW.md`, `DEVELOPER.md`, `AGENTS.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The repo supports Python packaging and a separate Rust crate, but build commands are split across `pip` and `cargo` examples in the docs. There is no single root command surface for building the Python wheel and Rust release binaries together.
- Current Update: Added a root `Makefile` with Python, Rust, aggregate build, and aggregate test targets. Updated the English and Traditional Chinese README files plus maintainer docs to document those commands, and extended `.gitignore` for Python build outputs created by `make build-python`.
- Result: `make help`, `make build-python`, and `make build-rust` all pass locally. The Python target writes the wheel to `dist/`, and the Rust target produces release binaries under `rust/target/release/`.

## 2026-03-11 - Task: Rename Dashboard Export Variant Flags
- State: Done
- Scope: `grafana_utils/dashboard_cli.py`, `rust/src/dashboard.rs`, `tests/test_dump_grafana_dashboards.py`, `README.md`, `README.zh-TW.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Both the packaged Python dashboard CLI and the Rust dashboard CLI expose short export-suppression flags, `--without-raw` and `--without-prompt`, with matching internal field names. The current docs and tests also use those shorter names.
- Current Update: Renamed the public export flags to `--without-dashboard-raw` and `--without-dashboard-prompt` in both implementations, renamed the corresponding Python namespace attributes and Rust struct fields, updated the rejection error text for disabling both variants, and refreshed the dashboard tests plus English and Traditional Chinese README examples.
- Result: The Python and Rust dashboard CLIs now use the longer dashboard-specific variant flag names consistently, and the focused dashboard unittest suite plus the full Rust and Python test suites pass with the new flag names.

## 2026-03-11 - Task: Port Grafana HTTP and API Flows Into Rust
- State: Done
- Scope: `rust/Cargo.toml`, `rust/Cargo.lock`, `rust/src/lib.rs`, `rust/src/common.rs`, `rust/src/http.rs`, `rust/src/dashboard.rs`, `rust/src/alert.rs`, `rust/src/bin/grafana-utils.rs`, `rust/src/bin/grafana-alert-utils.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The Rust crate can parse CLI arguments and normalize dashboard and alerting documents, but the actual Grafana HTTP client and live export/import flows are still stubbed with explicit not-implemented errors.
- Current Update: Added a shared Rust JSON HTTP client on top of `reqwest`, wired real dashboard raw export/import flows into `rust/src/dashboard.rs`, and wired real alerting export/import flows into `rust/src/alert.rs` for rules, contact points, mute timings, policies, and templates. The Rust alerting path now also includes linked-dashboard metadata export plus import-time dashboard UID repair logic. The remaining dashboard gap, prompt-export datasource rewrite, is now ported as well, including datasource-template-variable input generation and dependent-variable placeholder rewrites.
- Result: The Rust crate now executes the real Grafana HTTP/API flows and can produce both raw and prompt-style dashboard exports instead of relying on Python for datasource rewrite parity. `/opt/homebrew/bin/cargo test` passes, the targeted dashboard Rust tests pass, and the existing Python `python3 -m unittest -v` suite still passes.

## 2026-03-11 - Task: Add Rust Rewrite Scaffold for Grafana Utilities
- State: Done
- Scope: `rust/Cargo.toml`, `rust/Cargo.lock`, `rust/src/lib.rs`, `rust/src/common.rs`, `rust/src/dashboard.rs`, `rust/src/alert.rs`, `rust/src/bin/grafana-utils.rs`, `rust/src/bin/grafana-alert-utils.rs`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The repo ships only Python implementations. There is no Rust crate, no Rust CLI entrypoints, and no shared Rust model for dashboard or alerting document normalization.
- Current Update: Added an isolated `rust/` crate with shared auth and path helpers, a first-pass dashboard module, a first-pass alerting module, and Rust binary entrypoints for `grafana-utils` and `grafana-alert-utils`. The Rust port currently covers CLI parsing, auth/header resolution, path-building helpers, file discovery, and dashboard/alerting document normalization helpers, while the live HTTP flows still return explicit not-implemented errors.
- Result: The repository now contains a concrete Rust rewrite scaffold that can be extended incrementally without disturbing the shipping Python package. Existing Python tests still pass, and the new Rust crate now passes `cargo test` after the Rust toolchain was installed locally.

## 2026-03-11 - Task: Package Grafana Utilities for Installation
- State: Done
- Scope: `pyproject.toml`, `grafana_utils/__init__.py`, `grafana_utils/dashboard_cli.py`, `grafana_utils/alert_cli.py`, `grafana_utils/http_transport.py`, `cmd/grafana-utils.py`, `cmd/grafana-alert-utils.py`, `tests/test_dump_grafana_dashboards.py`, `tests/test_grafana_alert_utils.py`, `tests/test_packaging.py`, `README.md`, `README.zh-TW.md`, `DEVELOPER.md`, `AGENTS.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The repo runs from source, but it is not structured as an installable Python package. The implementation lives under `cmd/`, there is no packaging metadata, and there are no console entry points for global or per-user installs on other systems.
- Current Update: Moved the implementation into the `grafana_utils/` package, kept `cmd/` as thin source-tree wrappers, added `pyproject.toml` with console scripts for `grafana-utils` and `grafana-alert-utils`, and updated the English and Traditional Chinese docs plus maintainer guidance to cover normal, `--user`, and optional HTTP/2 installs. Packaging validation now includes package metadata tests and an isolated local `pip install --target` run.
- Result: The repo now supports installation as a Python package for either system/global environments or user-local environments while preserving direct repo execution through `cmd/`. Targeted tests and the full unittest suite passed. Local package installation also succeeded into `/tmp` with `--no-build-isolation`; a post-install `pyenv` rehash hook reported a local permissions warning after the install completed.

## 2026-03-11 - Task: Enable Persistent Grafana HTTP Connections
- State: Done
- Scope: `cmd/grafana_http_transport.py`, `tests/test_dump_grafana_dashboards.py`, `tests/test_grafana_alert_utils.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The shared transport abstraction exists, but both transport adapters still issue one-shot requests. That means no deliberate connection reuse, and HTTP/2 is not attempted even when the runtime could support it.
- Current Update: Changed the `requests` transport to use a persistent `requests.Session`, changed the `httpx` transport to use a persistent `httpx.Client`, and added automatic HTTP/2 enablement for `httpx` only when the runtime has `h2` support available. The default transport selector now uses `auto`, which prefers HTTP/2-capable `httpx` when possible and otherwise falls back to `requests` keep-alive sessions.
- Result: Grafana HTTP requests now reuse connections by default, and HTTP/2 is enabled automatically only in environments that can actually negotiate it. Full unit tests still pass after the transport behavior change.

## 2026-03-11 - Task: Make Grafana HTTP Transport Replaceable
- State: Done
- Scope: `cmd/grafana_http_transport.py`, `cmd/grafana-utils.py`, `cmd/grafana-alert-utils.py`, `tests/test_dump_grafana_dashboards.py`, `tests/test_grafana_alert_utils.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Both CLI tools embed `urllib` request handling directly inside their Grafana client classes. That makes the HTTP implementation fixed, mixes transport concerns into the resource clients, and leaves no clean seam for swapping `requests`, `httpx`, or a test transport.
- Current Update: Added a shared replaceable JSON transport module with `RequestsJsonHttpTransport` and `HttpxJsonHttpTransport`, changed both CLI clients to depend on an injected transport object, and kept `requests` as the default transport selected by the client constructors. Updated tests to load the shared transport module, verify both transport adapters build successfully, and exercise the new injected-transport seam directly.
- Result: The Grafana dashboard and alerting clients now use a replaceable transport architecture instead of hard-wired `urllib` calls. Full unit tests pass, and both CLIs can now switch HTTP engines by swapping the transport implementation rather than rewriting client logic.

## 2026-03-11 - Task: Refactor Grafana CLI Readability
- State: Done
- Scope: `cmd/grafana-utils.py`, `cmd/grafana-alert-utils.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Both CLI modules are functionally covered by tests, but several import/export and API-normalization flows are long enough that humans need to read multiple branches at once to understand them. The current structure leans on comments and helper names, but key paths such as datasource rewriting and alert import dispatch still need cleaner decomposition.
- Current Update: Refactored the dashboard CLI by splitting datasource resolution, templating rewrite, and export index generation into smaller helpers. Refactored the alerting CLI by splitting linked-dashboard repair, export document generation, and per-resource import handling into clearer dispatcher-style helpers with smaller units of work.
- Result: Both CLIs now read more like orchestration code with named helper steps instead of large inline branches. Full unit tests still pass, so the refactor changed structure and readability without changing behavior.

## 2026-03-11 - Task: Move Grafana CLIs Into cmd
- State: Done
- Scope: `cmd/grafana-utils.py`, `cmd/grafana-alert-utils.py`, `tests/test_dump_grafana_dashboards.py`, `tests/test_grafana_alert_utils.py`, `tests/__init__.py`, `README.md`, `README.zh-TW.md`, `DEVELOPER.md`, `AGENTS.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Both CLI entrypoints currently live at the repository root as `grafana-utils.py` and `grafana-alert-utils.py`. Unit tests import those scripts by direct filesystem path, and the public and maintainer docs still show root-level invocation examples.
- Current Update: Moved both CLI entrypoints into `cmd/`, updated the path-sensitive test loaders and CLI help strings, refreshed the English and Traditional Chinese docs plus maintainer guidance to use `python3 cmd/...`, and added `tests/__init__.py` so the documented `python3 -m unittest -v` command discovers the suite.
- Result: The repository now keeps both CLIs under `cmd/` without changing their behavior, unit tests load the new file locations correctly, and both targeted test runs plus the full unittest command pass from the repo root.

## 2026-03-10 - Task: Extend Grafana Alerting Resource Coverage
- State: Done
- Scope: `grafana-alert-utils.py`, `tests/test_grafana_alert_utils.py`, `README.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `grafana-alert-utils.py` already exports and imports alert rules, contact points, mute timings, and notification policies, and it can repair linked alert-rule dashboard UIDs by matching exported dashboard metadata. It does not yet cover notification templates, manual dashboard UID maps, or panel ID maps.
- Current Update: Added notification template export/import support, including version-aware template updates on `--replace-existing` and empty-list handling when Grafana returns `null`. Added `--dashboard-uid-map` and `--panel-id-map` so linked alert rules can be remapped explicitly during import before the existing metadata fallback logic runs. Exported linked-dashboard metadata now also captures panel title and panel type when available, and the README now documents the new alerting resource scope and mapping-file usage.
- Result: The standalone alert CLI now covers templates in addition to the existing alerting resources, supports operator-provided dashboard and panel remapping files for linked rules, and keeps the older dashboard-title/folder/slug fallback for cases where no explicit map is provided.

## 2026-03-10 - Task: Rename Grafana Dashboard Export Flag
- State: Done
- Scope: `grafana-utils.py`, `tests/test_dump_grafana_dashboards.py`, `README.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: The dashboard export subcommand uses `--output-dir`, which is generic enough to be confused with import behavior now that the CLI has explicit import and export modes.
- Current Update: Renamed the dashboard export flag to `--export-dir`, updated the parsed attribute and help text, and changed dashboard README examples and tests to use the more explicit export-only name.
- Result: The dashboard CLI now uses `--export-dir` for export mode, which better matches the subcommand and reduces mode confusion.

## 2026-03-10 - Task: Add Grafana Dashboard Import and Export Subcommands
- State: Done
- Scope: `grafana-utils.py`, `tests/test_dump_grafana_dashboards.py`, `README.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: `grafana-utils.py` decides between export and import implicitly by checking whether `--import-dir` is present, so export-only and import-only flags live in the same top-level parser and can be confused.
- Current Update: Split the dashboard CLI into explicit `export` and `import` subcommands, moved mode-specific flags onto the matching subparser, and added maintainer comments in the parser setup explaining why the split exists. README examples now call the subcommands directly.
- Result: Operators must now choose import or export explicitly at the command line, which removes the ambiguous mode inference and makes misuse harder.

## 2026-03-10 - Task: Change Grafana Default Server URL
- State: Done
- Scope: `grafana-utils.py`, `grafana-alert-utils.py`, `tests/test_dump_grafana_dashboards.py`, `tests/test_grafana_alert_utils.py`, `README.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Both utilities default to `https://10.21.104.120`, which assumes a specific remote server instead of a local Grafana instance.
- Current Update: Changed both CLI defaults to `http://127.0.0.1:3000`, added unit tests that assert the new parse-time default, and updated README command examples to match.
- Result: Operators now get a local Grafana default out of the box and can still override it with `--url` when targeting another instance.

## 2026-03-10 - Task: Make Grafana Utilities RHEL 8 Python Compatible
- State: Done
- Scope: `grafana-utils.py`, `grafana-alert-utils.py`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Both utility scripts use `from __future__ import annotations`, PEP 585 built-in generics like `list[str]`, and PEP 604 unions like `str | None`, which Python 3.6 on RHEL 8 cannot parse.
- Current Update: Replaced those annotations with `typing` module equivalents such as `List[...]`, `Dict[...]`, `Optional[...]`, and `Tuple[...]`, removed the unsupported future import so both scripts remain parseable on Python 3.6 without changing behavior, added parser-level tests that validate both entrypoints against Python 3.6 grammar, and documented RHEL 8+ support in the README.
- Result: The dashboard and alerting utilities now avoid Python 3.9+/3.10+ annotation syntax, explicitly document RHEL 8+ support, and have automated syntax checks that keep them compatible with RHEL 8's default Python parser.

## 2026-03-10 - Task: Add Grafana Alerting Utility
- State: Done
- Scope: `grafana-alert-utils.py`, `tests/test_grafana_alert_utils.py`, `README.md`, `docs/internal/ai-status.md`, `docs/internal/ai-changes.md`
- Baseline: Alert rules are not supported. The workspace only has dashboard export/import tooling in `grafana-utils.py`.
- Current Update: Expanded the standalone alerting CLI so it now exports and imports four resource types under `alerts/raw/`: rules, contact points, mute timings, and notification policies. Import uses create by default, switches to update with `--replace-existing` for rules/contact points/mute timings, and always applies the notification policy tree with `PUT`. The current increment adds alert-rule linkage metadata export for `__dashboardUid__`/`__panelId__`, plus import-time fallback that rewrites missing dashboard UIDs by matching the target Grafana dashboard on exported title/folder/slug metadata. Validation now includes a live Docker scenario where a linked rule was exported from dashboard UID `source-dashboard-uid`, the source dashboard was deleted, a replacement dashboard with UID `target-dashboard-uid` but the same title/folder/slug was created, and alert import rewrote the rule linkage to the new dashboard UID automatically.
- Result: Grafana alerting backup/restore is now separated from `grafana-utils.py` and covers the core alerting resources needed for notifications. The tool rejects Grafana provisioning `/export` files for API import, documents the limitation, has dedicated unit tests, and now preserves or repairs panel-linked alert rules when dashboard UIDs differ across Grafana systems.

## 2026-03-10 - Task: Export Grafana Dashboards
- State: Done
- Scope: `grafana-utils.py`, `tests/test_dump_grafana_dashboards.py`
- Baseline: Workspace is empty and there is no existing Grafana export utility.
- Current Update: Added `--without-raw` and `--without-prompt` so operators can selectively suppress one export variant while keeping the dual-export default. The exporter now rejects disabling both at once.
- Result: The tool now supports both workflows: export both variants by default, or export only `raw/` or only `prompt/` when needed. API import still requires an explicit path and should point at `raw/`.
