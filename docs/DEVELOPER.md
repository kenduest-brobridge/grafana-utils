# Developer Notes

This document is for maintainers. Keep `README.md` and the user guides operator-facing; keep Rust runtime notes, release ritual, and validation guidance here.

## Documentation Contract

- Keep `README.md`, `README.zh-TW.md`, `docs/user-guide.md`, and `docs/user-guide-TW.md` focused on the maintained user-facing `grafana-util` command surface.
- Keep older Python implementation notes in maintainer-only docs and internal reference pages only when they remain useful for historical context.
- When command behavior or parameter shapes change, update both user guides together.
- When maintainer validation or release behavior changes, update the relevant maintainer docs here instead of surfacing that detail in README unless operators need it.

## Public Surface Map

- `grafana-util overview`
  - human-first project entrypoint
  - Rust owner: `rust/src/overview.rs` plus the `overview_*` modules
- `grafana-util status`
  - canonical staged/live readiness surface
  - Rust owner: `rust/src/project_status.rs` and `rust/src/project_status_command.rs`
- `grafana-util change`
  - review-first staged change workflow
  - Rust owner: `rust/src/sync/`
- `grafana-util dashboard`
  - Rust owner: `rust/src/dashboard/`
  - legacy Python reference: `python/grafana_utils/dashboard_cli.py`
- `grafana-util datasource`
  - Rust owner: `rust/src/datasource.rs`
  - legacy Python reference: `python/grafana_utils/datasource_cli.py`
- `grafana-util alert`
  - Rust owner: `rust/src/alert.rs` and alert helper modules
  - legacy Python reference: `python/grafana_utils/alert_cli.py`
- `grafana-util access`
  - Rust owner: `rust/src/access/`
  - legacy Python reference: `python/grafana_utils/access_cli.py`

See [`docs/internal/project-surface-boundaries.md`](/Users/kendlee/work/grafana-utils/docs/internal/project-surface-boundaries.md) for the current public-name versus internal-name map.

## Naming Boundary

- Public names are the command names shown by `rust/src/cli.rs`, README, and the user guides.
- Internal module names may stay narrower or older than the public names when they describe implementation slices rather than operator-facing behavior.
- Treat `overview` as the current staged-artifact aggregation surface.
- Treat `status` as the public surface and `project-status` as the current internal architecture/file name for the shared status model.
- Treat `change` as the public surface and `sync` as the current internal runtime/document namespace behind it.
- When current maintainer docs mention `sync` or `project-status`, label them as internal or historical terms rather than current public commands.

## Repository Scope

### User-facing runtime

- `rust/src/cli.rs`: unified Rust entrypoint for namespaced command dispatch and `--help-full`.
- `rust/src/dashboard/`: dashboard export, import, diff, inspect, prompt-export, and screenshot workflows.
- `rust/src/datasource.rs`: datasource list, export, import, diff, add, modify, and delete workflows.
- `rust/src/alert.rs`: alerting export, import, diff, and shared alert document helpers.
- `rust/src/alert_list.rs`: alert list rendering and list command orchestration.
- `rust/src/access/`: access org, user, team, and service-account workflows plus shared renderers and request helpers.
- `rust/src/sync/`: internal runtime namespace for the public `change` staged workflow.

### Legacy maintainer reference runtime

- `python/grafana_utils/unified_cli.py`: older unified Python dispatcher kept as maintainer-only reference.
- `python/grafana_utils/dashboard_cli.py`: Python dashboard facade.
- `python/grafana_utils/datasource_cli.py`: Python datasource facade.
- `python/grafana_utils/alert_cli.py`: Python alert facade.
- `python/grafana_utils/access_cli.py`: Python access facade.
- `python/grafana_utils/http_transport.py`: shared Python transport abstraction.
- `python/grafana_utils/dashboards/`, `python/grafana_utils/datasource/`, `python/grafana_utils/access/`, `python/grafana_utils/alerts/`: older Python workflow and helper modules kept for maintainer reference.
- `python/tests/`: older Python regression coverage retained for maintainers when needed.

### Build, scripts, and docs

- `Makefile`: maintainer shortcuts for build, test, lint, and version bump flows.
- `.github/workflows/ci.yml`: CI entrypoint that should stay aligned with local quality gates.
- `examples/`: user-facing example assets that public docs may reference directly.
- `scripts/check-python-quality.sh`: centralized Python validation gate.
- `scripts/check-rust-quality.sh`: centralized Rust validation gate.
- `scripts/set-version.sh`: shared version bump helper for `VERSION`, `pyproject.toml`, `rust/Cargo.toml`, and `rust/Cargo.lock`.
- `docs/overview-rust.md`: Rust architecture walkthrough.
- `docs/overview-python.md`: Python maintainer architecture walkthrough.
- `docs/internal/overview-architecture.md`: maintainer map for the staged `grafana-util overview` design, data flow, and extension rules.
- `docs/internal/examples/`: maintainer-only demo scripts for intentionally unwired Python API flows.

Do not reintroduce standalone `call-hierarchy` or `unit-test-inventory` pages
unless they become generated artifacts with a clear maintenance owner. Keep
that routing in the overview and developer guides instead.

## Domain Freeze Policy

- `dashboard`
  - default state: frozen for new capability work
  - allowed by default: bug fixes, parity fixes, focused tests, and doc/help corrections
- `datasource`
  - default state: frozen for net-new lifecycle work
  - allowed by default: correctness fixes and minimum viable parity repair
- `alert`
  - default state: bounded reopen only for reliability, compatibility, and UX clarity
  - do not widen feature surface without an explicit maintainer decision
- `access`
  - default state: bounded hardening only
  - focus on correctness, operator safety, and parser/help clarity rather than redesign

Any exception to the default freeze posture should be recorded in this file and
in the current internal trace files with a concrete workflow, owned files, and
validation plan.

## Rust Ownership Cues

- Treat `rust/src/cli.rs` as command-topology only: parser shape, namespaced routing, help rendering, and dispatch seams for tests.
- Treat domain facade files (`rust/src/dashboard/mod.rs`, `rust/src/access/mod.rs`, `rust/src/datasource.rs`, `rust/src/sync/mod.rs`, `rust/src/alert.rs`) as runtime entrypoints only: normalize args, build clients/requests, and route to owned helpers.
- Treat `*summary.rs`, `*report.rs`, `*contract*.rs`, and staged-document modules as typed contract boundaries. Change these first when the JSON or cross-module data shape changes.
- Treat `*render*.rs`, `*_tui.rs`, and `tui_shell.rs` as presentation layers. Change these first when only visible text, layout, or interaction chrome changes.
- Treat `*state.rs`, `*workbench*.rs`, and `*interactive*.rs` as state-machine or interactive-flow ownership. Change these first when keyboard flow, modal state, or review-pane behavior changes.
- Watch for intentional cross-module reuse:
  - datasource auth/client setup is shared from dashboard helpers
  - sync composes crate-private alert/datasource assessment helpers instead of redefining those checks in one large module
  - access keeps distinct auth/client rules for org-admin paths versus org-scoped user/team/service-account paths

## Shortest Modification Paths

- `dashboard inspect` contract changes: start in `rust/src/dashboard/mod.rs`, then split between `rust/src/dashboard/inspect.rs`, `rust/src/dashboard/inspect_query.rs`, `rust/src/dashboard/inspect_live.rs`, and `rust/src/dashboard/inspect_live_tui.rs`; typed summary/report boundaries live in `rust/src/dashboard/inspect_summary.rs` and `rust/src/dashboard/inspect_report.rs`.
- `dashboard inspect` test changes: keep parser/help coverage near the relevant `*_cli_defs.rs`, and keep contract regressions in `rust/src/dashboard/rust_tests.rs`.
- `dashboard import --interactive` changes: start in `rust/src/dashboard/mod.rs` to confirm the entrypoint, then choose `import_interactive_state.rs` for state/event flow, `import_interactive_review.rs` for live review/diff assembly, `import_interactive_loader.rs` for local artifact/context loading, and `import_interactive_render.rs` or `import_interactive_context.rs` for screen layout and pane content.
- `sync` contract changes: start in `rust/src/sync/mod.rs`, then route dispatch and helpers through `rust/src/sync/cli.rs`, `rust/src/sync/live.rs`, `rust/src/sync/json.rs`, `rust/src/sync/bundle_inputs.rs`, `rust/src/sync/staged_documents.rs`, and `rust/src/sync/workbench.rs`; `live.rs`, `staged_documents.rs`, and `workbench.rs` own the typed apply/live boundary.
- `sync` test changes: keep CLI and live regressions in `rust/src/sync/cli_rust_tests.rs` and `rust/src/sync/rust_tests.rs`.
- `access` auth scope, request-shape, or browse changes: start in `rust/src/access/mod.rs`; only then branch into `cli_defs.rs` for parser shape or `user.rs` / `team.rs` / `service_account.rs` / `org.rs` for resource-specific workflow logic.
- `datasource` import/export/diff or mutation changes: start in `rust/src/datasource.rs`, then move into `datasource_import_export.rs`, `datasource_diff.rs`, or `datasource_mutation_support.rs` depending on whether the change is contract, compare semantics, or live mutation payload/rendering.

## Version Workflow

- `dev` is the preview branch; `main` is the release branch.
- `VERSION` is the checked-in maintainer version source.
- Use `make print-version` to inspect the current checked-in version state across package metadata.
- Use `make sync-version` after editing `VERSION` manually.
- Use `make set-release-version VERSION=X.Y.Z` when preparing `main` for release.
- Use `make set-dev-version VERSION=X.Y.Z DEV_ITERATION=N` when moving `dev` to the next preview cycle.
- Preferred release ritual:

## Validation Map

Keep the validation entrypoints here instead of spreading them across multiple
small maintainer-only files.

### Python suites

- `python/tests/test_python_dashboard_cli.py`
- `python/tests/test_python_dashboard_inspection_cli.py`
- `python/tests/test_python_datasource_cli.py`
- `python/tests/test_python_alert_cli.py`
- `python/tests/test_python_access_cli.py`
- `python/tests/test_python_packaging.py`

### Rust suites

- `rust/src/dashboard/rust_tests.rs`
- `rust/src/datasource_rust_tests.rs`
- `rust/src/alert_rust_tests.rs`
- `rust/src/access_rust_tests.rs`
- `rust/src/sync/*_rust_tests.rs`

### Common commands

- `PYTHONPATH=python python3 -m unittest -v`
- `cd rust && cargo test --quiet`
- `make quality-python`
- `make quality-rust`
- `make test`

### Usage

- Use the Python suites when checking parity, regressions, or legacy workflow
  behavior.
- Use the Rust suites for the maintained runtime and release-blocking
  validation.
- When a feature spans both implementations, keep this section current instead
  of reintroducing a separate test-inventory page.
  - work on `dev`
  - merge `dev` into `main`
  - run `make set-release-version VERSION=X.Y.Z` on `main`
  - run `make test`
  - create tag `vX.Y.Z`
  - merge `main` back into `dev`
  - run `make set-dev-version VERSION=X.Y.$((Z+1)) DEV_ITERATION=1` or the intended next preview
- Treat the post-release `main -> dev` sync as required so CI, docs, scripts, and version metadata do not drift.

## Runtime Positioning

- The maintained operator entrypoint is `grafana-util`.
- The Rust binary is the primary user-facing runtime.
- The Python implementation remains in-repo only as legacy maintainer reference material.
- Keep user docs Rust-first and avoid treating Python internals as part of the supported operator story.

## Python Maintainer Notes

- Python remains useful for:
  - historical behavior lookup during refactors
  - old workflow reference when investigating regressions
  - selective maintainer validation when a legacy comparison is still useful
- Keep Python command examples inside maintainer docs only.
- Prefer `PYTHONPATH=python python3 -m unittest -v` for full Python validation.
- Keep Python version metadata aligned with Rust version metadata through the shared version bump flow.

## Quality Gates

- `make quality-python` runs the legacy Python validation lane when maintainers still need it.
- `make quality-rust` runs the Rust validation lane used by the maintained runtime.
- `make test` should remain the broad maintainer gate across the repository.
- `cargo clippy --all-targets -- -D warnings` is release-blocking in CI.
- Keep CI wired to shared scripts rather than duplicating logic in workflow YAML.

## Maintenance Rules

- Keep README and user guides free of Python installation or entrypoint guidance unless Python becomes a supported user distribution again.
- Keep internal Python docs available only as maintainer reference while those files still exist in-repo.
- Keep `examples/` limited to operator-facing sample assets; place unwired demos and maintainer-only prototypes under `docs/internal/`.
- If a workflow change affects operator behavior, update both user guides in the same change.
- If a maintainer validation or release rule changes, update this file and the relevant internal reference docs in the same change.
- Historical notes in `docs/internal/` are archival and may still mention older rollout context.
