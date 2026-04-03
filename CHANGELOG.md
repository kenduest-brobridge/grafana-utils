# Changelog

This file is the fixed release-note source for `grafana-utils`.

It is intended to stay operator-facing:

- summarize user-visible changes by tagged release
- call out important migration notes
- avoid low-level internal refactor detail unless it changes behavior

Format rule going forward:

- add the next release at the top
- keep older tagged releases below
- use commit/tag history as the source of truth

## [0.6.1] - 2026-04-02

### Highlights

- The CLI now exposes a direct version check at the root through both
  `grafana-util --version` and `grafana-util version`.
- The release-facing documentation was rewritten to make the command areas,
  workflow boundaries, and staged contract rules easier to follow.

### Changed

- `README.md` and `README.zh-TW.md` now act more like product entry pages:
  they explain what the tool is for, how the major command areas fit
  together, and which staged workflow rules matter most.
- `docs/user-guide.md` and `docs/user-guide-TW.md` now tell operators to
  confirm the installed CLI version at the start of the workflow.

### Fixed

- GitHub Actions release and quality workflows now use Node 24 compatible
  versions of the official checkout, setup-python, upload-artifact, and
  download-artifact actions, avoiding the Node 20 deprecation warnings on
  current runners.

## [0.6.0] - 2026-04-02

### Highlights

- The Rust CLI grew from resource-specific commands into a more complete
  operator surface with `overview`, `status`, `change`, `snapshot`, and
  `profile` workflows.
- Dashboard and datasource staged contracts are now more explicit, especially
  around provisioning lanes, export roots, and replay/import boundaries.
- Alert management now supports a fuller desired-state authoring and
  review/apply workflow instead of only export/import style flows.

### Added

- New top-level project surfaces:
  - `grafana-util overview`
  - `grafana-util status`
  - `grafana-util change`
- New top-level snapshot workflow:
  - `grafana-util snapshot export`
  - `grafana-util snapshot review`
- New profile workflow for repo-local live connection defaults:
  - `grafana-util profile init`
  - `grafana-util profile list`
  - `grafana-util profile show`
- New dashboard authoring helpers:
  - `dashboard get`
  - `dashboard clone-live`
  - `dashboard patch-file`
  - `dashboard review`
  - `dashboard publish`
- New dashboard browser and delete workflows.
- New dashboard provisioning lane support across export/import/diff/validate,
  and inspect flows.
- New datasource provisioning lane support across export/import/diff and
  inspect flows.
- New datasource masked-recovery contract with placeholder-based secret
  recovery support.
- New alert desired-state management surfaces:
  - `alert init`
  - `alert add-rule`
  - `alert clone-rule`
  - `alert add-contact-point`
  - `alert set-route`
  - `alert preview-route`
  - `alert plan`
  - `alert apply`
  - `alert delete`
- Repo-owned install script for release binaries:
  - `scripts/install.sh`

### Changed

- Public project vocabulary is now centered on:
  - `overview` for human-first project entry
  - `status` for staged/live readiness
  - `change` for staged review/apply workflows
- The older `sync` and `project-status` names are now treated as internal
  runtime/architecture names rather than the preferred public surface.
- Dashboard staged exports are more explicit:
  - `raw/` is the canonical dashboard replay/export variant
  - `provisioning/` is a separate provisioning-oriented variant
- Datasource staged exports are more explicit:
  - `datasources.json` remains the canonical replay/import/diff contract
  - `provisioning/datasources.yaml` is a projection for Grafana provisioning,
    not the primary restore contract
- `overview` and `status` now consume domain-owned staged contracts more
  consistently instead of reinterpreting staged layouts ad hoc.
- Shared output handling is more consistent across commands, including broader
  text/table/csv/json/yaml coverage and color-aware JSON rendering.
- Live dashboard and datasource status reporting is more consistent with the
  staged contract boundaries, especially around multi-org and root-scoped
  inventory reads.

### Fixed

- Alert authoring round-trip behavior is more stable after apply by normalizing
  equivalent live payload shapes more conservatively.
- Datasource secret handling is more explicit and fail-closed when required
  recovery values are missing.
- Access and alert list/browse/runtime presentation now align better with the
  shared output and interactive shell behavior.
- Snapshot review wording and inventory behavior are clearer and more aligned
  with the actual staged review flow.

### Migration Notes

- If you were using older project-level naming, prefer:
  - `grafana-util change ...` instead of older `sync`-style public wording
  - `grafana-util status ...` instead of older `project-status` public wording
- For dashboard staged inputs, treat `raw/` and `provisioning/` as separate
  contracts rather than interchangeable path aliases.
- For datasource staged inputs, treat `datasources.json` as the canonical
  replay/import artifact and use provisioning YAML only for provisioning-style
  consumption.
- For live command defaults, `grafana-util.yaml` plus `--profile` is now the
  preferred path over repeating the same URL/auth/TLS flags in every command.

## [0.5.0] - 2026-03-27

### Highlights

- Dashboard browser and delete workflows became first-class Rust operator
  surfaces.
- Governance and browse-related dashboard analysis expanded beyond the earlier
  inspect-only baseline.

### Added

- Dashboard browser workflow for navigating exported/live dashboard inventory.
- Dashboard delete workflow with review-oriented operator behavior.
- Expanded governance and browse reporting around dashboard maintenance.

### Changed

- Rust dashboard operator workflows became more practical for day-to-day
  inventory, review, and cleanup work.

## [0.4.0] - 2026-03-25

### Highlights

- The project shifted from basic Rust command coverage to a more structured
  Rust operator workflow with split modules, clearer docs, and stronger
  support for staging, review, and governance.

### Added

- Wider operator examples and support-matrix guidance in the public docs.
- More explicit governance and browse workflow coverage in the Rust CLI.

### Changed

- Rust `dashboard`, `access`, and `sync` internals were split into clearer
  modules to support ongoing CLI growth.
- Maintainer docs and README entry points were reorganized to better reflect
  the Rust-first direction.

## [0.3.0] - 2026-03-24

### Highlights

- `grafana-utils` moved from a smaller mixed Python/Rust utility set toward a
  fuller Rust-first CLI with dashboard inspection, datasource workflows,
  access management, and sync-related staged artifacts.

### Added

- Dashboard inspect-export and inspect-live workflows.
- Datasource import/export and live admin workflows.
- Access user, team, org, and service-account workflows.
- Sync/preflight and staged artifact workflows.
- Unified `grafana-util` naming and packaging path.

### Changed

- The unified CLI name was normalized to `grafana-util`.
- Python packaging and repo layout were standardized around the current source
  tree.
- Dashboard export/import and inspection contracts became much more explicit.

## [0.2.x] - 2026-03-23 and earlier

### Summary

The `0.2.x` series was the formative line that established the base utility
set and the first Rust/Python shared operator story.

### Major Themes

- Initial dashboard export/list/import/diff workflows.
- Early alert export/import utility support.
- Initial access management commands.
- Packaging, HTTP transport, and installability improvements.
- Multi-org and dry-run support across several operator paths.
- Early documentation split between user-facing guides and maintainer notes.

### Notes

- This range includes many rapid point releases from `v0.2.0` through
  `v0.2.20`.
- The earlier history is better treated as one foundation series than as
  separate operator-facing release notes for each point tag.
