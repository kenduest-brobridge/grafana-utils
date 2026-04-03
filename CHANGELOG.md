# Changelog

## 2026-03-16

- `401c088` refactor: group CLI help options by intent across commands
  - Grouped connection/auth flags under a dedicated `Connection and Auth` heading across Python and Rust help surfaces.
  - Split dashboard import output help into routing, behavior, safety, dry-run output, and progress/logging groups.
  - Updated help assertions in Python and Rust tests so grouped headings remain enforced.
  - Kept command behavior and existing examples unchanged while improving parser help readability.

- `6b6cde3` feature: add machine-readable Rust sync artifact schemas
  - Added canonical JSON Schema files for summary, plan, preflight, bundle-preflight, and apply-intent artifacts.
  - Added a machine-readable schema index manifest to enumerate all sync contracts.
  - Added Rust contract tests that validate metadata and required/const field behavior across generated artifacts.

- `623a3d7` feature: add canonical rust sync fixtures and artifact contract
  - Added canonical Rust sync demo and contract fixtures to reuse in both code and docs.
  - Refactored Rust sync tests to load canonical fixtures via `include_str!` and keep suite coverage stable.
  - Expanded staged sync contract detail with explicit scope/prune metadata and richer preflight/bundle-preflight summaries.

- `8d7adb5` docs: refresh stale internal status notes
  - Marked service-account snapshot status as done and removed the duplicate planned entry.
  - Updated GitOps sync internal notes to describe shipped staged CLI behavior.
  - Reframed the sync roadmap from “design from scratch” to expansion of the existing workflow.


## 2026-03-15

- `37f2db7` bugfix: complete rust cli import/workbench follow-up
  - Completed access import/update/export argument normalization and added runtime guards.
  - Extended dashboard inspect/report and datasource provider handling paths used by sync/import tooling.
  - Hardened sync and preflight failure handling and aligned test coverage.

- `da74501` bugfix: harden python quality gate path handling
  - Dynamically build quality paths so available tools/lists are checked against real checkout paths.
  - Removed duplicate parser test import that could break quality checks.

- `e497e8d` ci: avoid poetry root install in github actions
  - CI now uses `poetry install --with dev --no-root` and optional transport dependencies in a safer order.

- `f37e3e0` bugfix: add continue-on-error for alert and datasource import
  - Added `--continue-on-error` parser and runtime support for alert and datasource import.
  - Enabled per-item continuation during import while preserving non-zero exit behavior on failed items.
  - Extended parser and runtime tests for continue-on-error acceptance paths.

- `9f5ced6` ci: align python quality job with poetry workflow
  - Simplified GitHub Actions Python quality job to be Poetry-first and aligned with local quality gate.

- `4386e0e` bugfix: continue-on-error dashboard import
  - Centralized dashboard continue-on-error behavior in import execution and dry-run.
  - Added parser and runtime tests covering continue-on-error semantics and export-org item failures.

- `0c57d96` feature: expose python http transport selection
  - Added shared `--http-transport auto|requests|httpx` across all Python CLIs.
  - Threaded transport selection into Python Grafana clients and cloning paths.
  - Made explicit-httpx tests pass with and without optional `httpx` dependency installed.

- `586c749` bugfix: allow sync apply to continue on non-blocking errors
  - Added sync apply continue-on-error and added per-item status output for successful and failed actions.
  - Kept failed operations visible in a structured result stream while preserving non-zero failure summary exits.

- `8b72104` bugfix: add rust sync live apply execution
  - Implemented rust sync live apply execution for folder/dashboards/datasources.
  - Added parser coverage for `--execute-live` and folder-delete options.
  - Wired execute-live dispatch in CLI run path and validated in full Rust suite.

- `b6db8dd` feat: align rust sync cli with live and alert assess
  - Added optional live fetch support and new `assess-alerts` sync command path.
  - Extended parse model with optional live availability files and flattened auth arguments.
  - Added CLI regression coverage for `--fetch-live` and `assess-alerts`.

- `ad5cfb1` feature: add python batch error policy
  - Added shared abort/continue batch policy helper to datasource and access batch workflows.
  - Kept batch failures item-scoped while preserving fail-open/fail-closed behavior at the right layer.

- `17ec0dc` feature: add continue-on-error batch policy to dashboard and alert CLIs
  - Added `--error-policy` to dashboard and alert flows for item-level continuation.
  - Preserved strict pre-flight validation while allowing post-validation continuation.
  - Expanded parser and behavior tests for dashboard and alert batch error policy.

- `daa23ca` refactor: reduce CLI maintenance hotspots
  - Replaced Python inspection string-key wiring with explicit dependency/setting objects.
  - Centralized common dispatch paths while keeping compatibility wrappers in place.

- `c399a8f` docs: capture sync follow-up backlog
  - Captured sync follow-up technical backlog including sync round-trip, alert policy, and contract hardening.
  - Updated internal docs to keep planned sync work explicit and stable.

- `ac7cb2c` bugfix: allow parent-linked sync preflight lineage
  - Relaxed parent lineage checks where appropriate while keeping apply-side validation strong.
  - Removed stale no-parent lineage rejection from apply-time preflight checks.
  - Revalidated focused Rust sync suite after lineage contract adjustment.

- `f9d549f` feature: add rust sync preflight lineage metadata
  - Added optional trace IDs for preflight/bundle-preflight metadata and staged lineage checks.
  - Tightened staged apply validation for expected lineage metadata and step intent.
  - Added contract-focused Rust sync tests for lineage compatibility.

- `188b1d0` feature: carry trace ids through rust sync preflight
  - Added trace-id support across preflight surfaces and applied lineage metadata to emitted staged docs.
  - Rejected apply intent inputs when expected lineage metadata is missing.
  - Updated tests and trace docs around lineage-aware sync documents.

- `95e2510` feature: add sync workflows and scoped import extensions
  - Added planning/preflight/review/apply sync workflow framing and scoped import extension points.
  - Extended datasource, dashboard, alert, and access CLIs with new staged import and org-scoped behavior.
  - Added internal modules and user-guide notes for the staged sync operator path.

- `2abe613` build: add canonical version sync workflow
  - Added canonical version sync via VERSION file plus helper scripts and Make targets.
  - Synced Python and Rust metadata updates to one source of truth.
  - Added targets for version show/sync/dev/release and tag workflows.

- `134212e` bugfix: tighten rust sync staged lineage gating
  - Tightened staged lineage gating in sync review/apply and aligned 1/2/3 staged flow indexes.
  - Rejected inconsistent lineage or trace metadata for reviewed plan inputs before local apply.

- `50e469a` feature: expand Grafana sample data workflow
  - Expanded sample-data generation for users/teams/folders/datasources with read-only verify.
  - Added richer workflow coverage across multiple orgs and richer dashboard fixtures.

- `8b5ab08` feature: wire roadmap graph and promotion plan surfaces
  - Added graph output formats and promote-plan preflight command surfaces for roadmap workflows.
  - Added local command surfaces for dashboard promote plan and preflight plan previews.
  - Added tests for new inspect export path and roadmap workbench behavior.

- `a1bb2ec` feature: simplify inspect output selection
  - Added inspect `--view/--format/--layout` while preserving legacy output compatibility.
  - Kept old `--output-format` compatibility through argument normalization.
  - Added parser/help/runtime tests for inspect UX normalization.

- `20f44c0` docs: refresh internal roadmap and archive notes
  - Updated roadmap/archive docs and aligned names to current `grafana-util` wording.
  - Clarified historical command naming so legacy internal notes are not mistaken for current guidance.

- `6d96e07` docs: bump dev version to 0.2.0 preview
  - Updated Python and Rust dev version identifiers per branch policy.
  - Kept release policy expectations explicit in version metadata and release flow notes.

- `9683be4` docs: align shared CLI guides and help
  - Updated shared CLI guides to prefer unified command shape with clarified examples.
  - Expanded root datasource and subcommand guidance in help and user-facing docs.

- `dbbbed0` ci: enforce release version policy
  - Added CI checks for release-version format alignment across branches and tags.
  - Added policy enforcement for branch, dev, and tagged release version consistency.

## 2026-03-14

- `c0574cd` Block datasource UID-drift updates
  - Added stricter datasource import validation so name-matched records with conflicting UIDs are no longer updated implicitly.
  - Reduced the risk of mutating the wrong live datasource when export metadata and target instance state disagree.
  - Kept Python and Rust datasource behavior aligned for this guardrail.
  - Updated tests and docs to make the drift-handling rule explicit.

- `f3d471e` Add prompt-token auth flow
  - Added prompt-driven token input so operators can avoid passing Grafana tokens directly on the command line.
  - Extended auth resolution paths across the Python CLIs and matching Rust surfaces.
  - Improved secret-handling ergonomics without changing the existing explicit token flag behavior.
  - Updated CLI help and tests to cover the new prompt-based auth path.

- `4586b10` Add datasource import workflow
  - Added live datasource import support so exported datasource bundles can now be replayed back into Grafana.
  - Reused the normalized datasource contract for create, update, and dry-run review flows.
  - Expanded Python and Rust command coverage so datasource management now includes inventory, diff, and import.
  - Updated operator docs to describe the datasource reconciliation workflow.

- `744c024` Wire inspection governance reports
  - Added governance-style dashboard inspection reporting so exported analysis can highlight actionable risk findings.
  - Extended the inspection output model beyond raw extraction summaries into policy-oriented reporting.
  - Improved review workflows for dashboard estates with datasource and query hygiene concerns.
  - Added tests around the new governance report rendering paths.

- `54c9b86` Trim dashboard CLI wrappers and extend dry-run output
  - Removed more compatibility wrapper logic from the Python dashboard facade to keep orchestration code thinner.
  - Extended dashboard and datasource dry-run review output so import results are easier to scan.
  - Continued the ongoing split of helper ownership into dedicated workflow/runtime modules.
  - Preserved existing user-facing behavior while improving internal maintainability.

## 2026-03-13

- `a363416` Improve dashboard dry-run import output
  - Refined dashboard import dry-run output so operators can review planned actions more clearly before applying them.
  - Added better structured reporting for folder, action, and reconciliation context.
  - Improved import review without changing live mutation behavior.
  - Updated tests to lock in the revised dry-run output shape.

- `368b1e2` Add dashboard export inspection command
  - Added a first-class dashboard inspection command to analyze exported dashboard bundles.
  - Created a foundation for datasource usage summaries, query extraction, and governance checks over exported content.
  - Expanded the CLI beyond backup/restore into post-export analysis workflows.
  - Added supporting tests and docs for the new inspection surface.

- `2e078b1` Record raw datasource inventory in dashboard exports
  - Started writing datasource inventory metadata alongside dashboard export artifacts.
  - Made downstream diff, audit, and inspection workflows less dependent on re-reading every raw dashboard file.
  - Improved exported bundle completeness for governance and migration review.
  - Kept the export structure compatible with the broader dashboard workflow.

- `41dceaf` Add live dashboard inspection command
  - Extended inspection so operators can analyze dashboards directly from a live Grafana instance instead of only from exported files.
  - Reused the inspection reporting model across live and exported input sources.
  - Improved operational usability for teams that want fast read-only inspection without a prior export step.
  - Added tests to cover the live inspection entrypoint behavior.

- `31c40ae` Add datasource CLI and dashboard governance helpers
  - Added the initial datasource CLI surface and related governance helper plumbing.
  - Expanded the project scope from dashboards, alerts, and access into datasource administration.
  - Introduced shared helper paths needed for datasource inventory and follow-on import/diff work.
  - Updated docs and tests to reflect the new datasource domain.

## 2026-03-12

- `fce9af6` Add Grafana access utility team modification
  - Added team update support to the access CLI so operators can modify existing Grafana teams directly.
  - Expanded the access-management surface beyond listing and creation into lifecycle maintenance.
  - Updated tests to cover the new team mutation behavior and CLI handling.
  - Kept the Python and Rust access roadmap aligned around the same operator workflows.

- `af6161a` Add Grafana access utility user update and delete
  - Added user modification and deletion flows to the access CLI.
  - Completed more of the expected user lifecycle surface for Grafana access administration.
  - Improved the usefulness of the access tool for day-two operations, not just discovery and creation.
  - Added focused tests and doc updates for the new commands.

- `ae658c1` Consolidate Python and Rust CLIs under grafana-utils
  - Unified the Python and Rust command shape under the shared `grafana-utils` entrypoint.
  - Reduced fragmentation between separate binaries and moved the project toward one consistent CLI surface.
  - Updated packaging, docs, and tests to reflect the consolidated command model.
  - Improved long-term maintainability by aligning command discovery across implementations.

- `df43c39` Reshape alert CLI commands and remove legacy shim
  - Reworked alert command structure into clearer export/import/diff-oriented subcommands.
  - Removed older compatibility shims that were keeping the alert surface harder to reason about.
  - Improved consistency between alert workflows and the rest of the repository's command design.
  - Updated tests and docs to reflect the modernized alert CLI layout.

- `67fa1e4` Align dashboard prompt export with Grafana external export
  - Adjusted prompt-style dashboard exports so they better match Grafana's external export expectations.
  - Improved datasource placeholder labeling and portability for re-import workflows.
  - Reduced friction when using exported dashboards as migration-ready artifacts.
  - Updated tests and docs to capture the refined export contract.

## 2026-03-11

- `2972101` Add Grafana dry-run and diff workflows
  - Added versioned export metadata for dashboard export roots and variants.
  - Added dashboard `diff` as an explicit CLI subcommand.
  - Added alerting `--diff-dir` to compare local exports with live Grafana state.
  - Added non-mutating `--dry-run` import behavior to both Python CLIs.

- `56ac3d2` Reorganize docs and split Python Rust tests
  - Renamed the Python unittest files so their implementation type is obvious from the filename.
  - Standardized the Python test names as `test_python_dashboard_cli.py`, `test_python_alert_cli.py`, and `test_python_packaging.py`.
  - Moved Rust inline unit tests out of production modules and into dedicated `*_rust_tests.rs` files.
  - Updated module wiring so Rust test files are still compiled and discovered from their parent modules.
  - Refreshed maintainer docs and repo guidance so targeted test commands point at the new file names.
  - Preserved Python `unittest` discovery and Rust `cargo test` behavior after the layout change.

- `4991900` Add build Makefile for Python and Rust
  - Added a root `Makefile` to give the repository one shared build and test entrypoint.
  - Introduced `build-python`, `build-rust`, `build`, `test-python`, `test-rust`, `test`, and `help` targets.
  - Standardized where Python and Rust artifacts land so operators know where to find wheels and release binaries.
  - Updated README, Traditional Chinese README, maintainer docs, and repo instructions to document the new commands.
  - Extended `.gitignore` so Python build outputs created by the Makefile do not pollute the worktree.
  - Reduced the need to remember separate `pip` and `cargo` command sequences for common tasks.

- `54f44ba` Rename dashboard export variant flags
  - Renamed the dashboard raw export suppression flag from the shorter form to `--without-dashboard-raw`.
  - Renamed the prompt export suppression flag to `--without-dashboard-prompt` for the same reason.
  - Updated the Python dashboard CLI, the Rust dashboard CLI, and the matching test coverage together.
  - Refreshed operator docs so command examples no longer use the old short flag names.
  - Made the option names self-describing when read out of context in scripts or CI jobs.
  - Kept both implementations aligned so Python and Rust users see the same public flag surface.

- `257150f` Port Grafana API flows into Rust
  - Replaced the earlier Rust scaffolding and stubs with real Grafana HTTP request flows.
  - Added a shared Rust HTTP client layer and connected it to the dashboard and alerting modules.
  - Implemented dashboard raw export/import behavior in Rust, including live API fetching and write paths.
  - Implemented prompt export datasource rewrite behavior so Rust can generate Grafana-style import placeholders.
  - Implemented alerting export/import behavior for rules, contact points, mute timings, notification policies, and templates.
  - Added Rust-side linked-dashboard handling so alert rules can carry and repair dashboard linkage metadata.
  - Brought the Rust implementation much closer to Python parity instead of remaining helper-only code.

- `c4bcc7b` Package Grafana utilities for install
  - Added Python packaging metadata so the repository can be installed cleanly on other systems.
  - Created installable console scripts for `grafana-utils` and `grafana-alert-utils`.
  - Moved the real Python implementation into the `grafana_utils/` package instead of keeping logic only in top-level scripts.
  - Kept `cmd/` wrappers as thin source-tree launchers so repo-local execution still works during development.
  - Updated install, usage, and maintainer docs to explain package install paths and command names.
  - Made it possible to install into a global environment or a user-local Python environment with the normal `pip` flow.

- `79f9b7e` Make Grafana HTTP transport replaceable
  - Added a swappable transport abstraction for Grafana HTTP requests.
  - Enabled both `requests` and `httpx` transport implementations behind the same interface.
  - Kept client code focused on Grafana API behavior instead of library-specific HTTP details.
  - Added tests to verify transport selection and injection.

- `b4065b2` Refactor Grafana CLI readability
  - Split large Python flows into smaller helpers with clearer names.
  - Reduced nesting in import/export paths and separated orchestration from detail logic.
  - Made datasource rewrite and linked-dashboard repair flows easier to scan.
  - Preserved behavior while improving code readability for maintainers.

- `89299ba` Move Grafana CLIs into cmd
  - Moved the repo-run Python entrypoints into `cmd/`.
  - Updated unit tests to load the new command paths.
  - Refreshed documentation so repo usage examples point at `cmd/`.
  - Kept the implementation separate from the thin command wrappers.

- `20e1fff` Split Chinese README guide
  - Separated the English and Traditional Chinese READMEs into distinct files.
  - Added top-level navigation links between the two README variants.
  - Restored the English README as the main English-facing document.
  - Improved language-specific onboarding from the repository homepage.

- `464514d` Add Chinese README summary
  - Added a Traditional Chinese summary near the top of the project docs.
  - Clarified the repository purpose for Chinese readers at a glance.
  - Explained the difference between the dashboard and alerting tools in Chinese.
  - Improved README accessibility without changing code behavior.

- `0594a6d` Clarify README purpose
  - Reworked the README opening to explain what the repository is for.
  - Brought backup, migration, and re-import use cases to the top.
  - Clarified the split between dashboard and alerting workflows.
  - Reduced the need to infer project purpose from option sections alone.

- `e921e06` add LICENSE
  - Restored the repository `LICENSE` file from the main branch.
  - Reintroduced an explicit license file into the working tree.
  - Ensured the repository includes the expected legal metadata.

## 2026-03-10

- `06a71a9` Reorganize project docs and tests
  - Reworked documentation structure to separate operator-facing and maintainer-facing content.
  - Cleaned up test organization so it better matched the evolving project layout.
  - Refreshed internal tracking files to reflect the reorganized structure.
  - Improved navigation across the main project documents.

- `1e563b6` Clarify documentation policy
  - Defined where public usage docs should live versus internal notes.
  - Clarified the intended roles of README, DEVELOPER, and internal trace files.
  - Reduced ambiguity about where future project guidance should be added.
  - Established a clearer documentation maintenance boundary.

- `2ffd80a` Extend Grafana alert utility mapping and templates
  - Improved alert-rule remapping behavior for linked dashboards and panels.
  - Extended template handling so update paths preserve Grafana expectations.
  - Expanded alert import logic for more realistic migration scenarios.
  - Added coverage around the new alert mapping behavior.

- `ff80bb4` Add developer maintenance notes
  - Added `DEVELOPER.md` to capture internal architecture and maintenance guidance.
  - Documented important API usage and implementation tradeoffs for maintainers.
  - Created a stable place for operational notes that do not belong in the public README.
  - Improved long-term maintainability of the project documentation.

- `6b0bac9` Extend Grafana alerting backup compatibility
  - Broadened alerting backup and restore support across more resource shapes.
  - Improved compatibility between exported tool documents and import expectations.
  - Reduced failure cases when moving alerting resources between Grafana instances.
  - Strengthened the backup/restore story for alerting workflows.

- `b45ba4c` Add explicit dashboard subcommands and export-dir flag
  - Introduced explicit dashboard `export` and `import` subcommands.
  - Switched the dashboard export option name to `--export-dir`.
  - Reduced argument ambiguity by separating export-only and import-only flags.
  - Updated tests and docs to match the new command layout.

- `0a87701` Ignore local Grafana artifacts
  - Added ignore rules for local files and generated Grafana artifacts.
  - Reduced accidental commits of temporary or environment-specific output.
  - Improved cleanliness of the git working tree during local testing.

- `2fcf3ac` Refresh alerting utility documentation and status
  - Updated alerting usage examples and project status notes.
  - Synchronized internal change tracking with the current alerting capabilities.
  - Improved discoverability of supported alerting resource behavior.
  - Kept project docs aligned with the evolving alert utility.

- `dc2ee24` Make Grafana utilities RHEL 8 compatible and default to localhost
  - Reworked Python annotations so the code stays parseable on Python 3.6 syntax.
  - Locked the CLIs to a localhost Grafana default URL instead of a hardcoded remote target.
  - Added tests that validate Python 3.6 grammar compatibility.
  - Updated documentation to note RHEL 8 support expectations.

- `6d79b85` Refine alert utility change log
  - Improved the recorded history for recent alert utility work.
  - Made internal status notes clearer and more traceable.
  - Tightened the wording around what changed in alerting support.
  - Helped future maintenance by preserving better internal context.

- `c411ffd` Add Grafana alert rule utility
  - Added the standalone Python alerting CLI for Grafana resources.
  - Implemented export/import support for core alerting provisioning resources.
  - Added linked-dashboard metadata handling for alert-rule portability.
  - Added unit tests and documentation for the new alerting tool.

- `6637d4b` Document dashboard datasource export flow
  - Documented how prompt exports rewrite datasource references.
  - Explained the role of Grafana `__inputs` placeholders in prompt exports.
  - Made the dashboard export behavior easier to understand for operators.
  - Captured the datasource rewrite flow for future maintenance.

- `0a9db4c` Add Grafana dashboard export/import utility
  - Added the initial Python dashboard export/import CLI.
  - Established dual export variants under `raw/` and `prompt/`.
  - Added unit tests for export, import, auth, and datasource placeholder behavior.
  - Documented the basic dashboard backup and restore workflow.
