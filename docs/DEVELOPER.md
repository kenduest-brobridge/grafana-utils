# Developer Notes

This document is for maintainers. Keep `README.md` and the user guides operator-facing; keep Rust runtime notes, release ritual, and validation guidance here.

## Documentation Contract

- Keep `README.md`, `README.zh-TW.md`, `docs/user-guide.md`, and `docs/user-guide-TW.md` focused on the maintained user-facing `grafana-util` command surface.
- Keep older Python implementation notes in maintainer-only docs and internal reference pages only when they remain useful for historical context.
- When command behavior or parameter shapes change, update both user guides together.
- When maintainer validation or release behavior changes, update the relevant maintainer docs here instead of surfacing that detail in README unless operators need it.

## Repository Scope

### User-facing runtime

- `rust/src/cli.rs`: unified Rust entrypoint for namespaced command dispatch and `--help-full`.
- `rust/src/dashboard/`: dashboard export, import, diff, inspect, prompt-export, and screenshot workflows.
- `rust/src/datasource.rs`: datasource list, export, import, diff, add, modify, and delete workflows.
- `rust/src/alert.rs`: alerting export, import, diff, and shared alert document helpers.
- `rust/src/alert_list.rs`: alert list rendering and list command orchestration.
- `rust/src/access/`: access org, user, team, and service-account workflows plus shared renderers and request helpers.
- `rust/src/sync/`: staged sync bundle, preflight, review, and apply flows.

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
- `docs/core-python-call-hierarchy.md`: Python call graph reference for maintainers.
- `docs/unit-test-inventory.md`: test inventory reference for maintainers.
- `docs/internal/examples/`: maintainer-only demo scripts for intentionally unwired Python API flows.

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
