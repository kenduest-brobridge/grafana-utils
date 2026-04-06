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

## [0.9.1] - 2026-04-07

### Highlights

- `main` now includes the Rust quality-gate cleanup so the CI lint path stays green again.
- Public install and getting-started examples were bumped to `0.9.1` so the pinned-release guidance matches the tagged release.
- The release metadata and generated docs were refreshed after the version bump.

### Changed

- `VERSION`, package manifests, install examples, and getting-started docs now point at `0.9.1`.
- Generated manpage and HTML docs were regenerated for the `0.9.1` line.

### Fixed

- The `main` branch quality gate no longer fails on the clippy warnings that were blocking the release lane.

## [0.9.0] - 2026-04-07

### Highlights

- The public README got a cleaner operator-first introduction, with a shorter feature summary and more concrete examples for profiles, connection methods, and dashboard export/import paths.
- Release metadata moved to `0.9.0` across the version file, package manifests, install examples, and generated docs so install and docs links stay aligned with the tagged release.
- GitHub Pages generation now skips incomplete historical release snapshots instead of failing the entire docs site build when a release line does not have every generated command page.

### Added

- README and getting-started examples now show `profile add` usage for reusable connection and secret defaults.
- Public docs now call out the common dashboard `raw`, `prompt`, and `provisioning` paths more clearly.

### Changed

- README examples now use `0.9.0` for pinned install instructions and release references.
- `VERSION`, `python/pyproject.toml`, `rust/Cargo.toml`, `rust/Cargo.lock`, and the generated manpage and HTML docs were refreshed for the `0.9.0` line.
- The public docs and generated docs were regenerated after the version bump so source docs and rendered outputs match.

### Fixed

- GitHub Pages assembly no longer aborts on incomplete historical version snapshots during docs-site generation.
- The changelog, install help, and generated docs now reflect the same release version.

## [0.8.0] - 2026-04-05

### Highlights

- Public docs now explain value more clearly through `Before / After`,
  success criteria, and failure checks across the README, handbook, and
  high-value command pages.
- The generated HTML docs now have a cleaner navigation hierarchy, improved
  entry labels, and better browser reading flow for handbook, command, and
  maintainer pages.
- Automation-facing output contracts were tightened so JSON-producing flows
  are easier to consume in CI and release workflows.

### Added

- New public command-reference coverage for `dashboard impact`, including the
  generated HTML and manpage surfaces.
- New validation coverage for public-doc evidence sections and release-driven
  generated-manpage updates.

### Changed

- `README.md` and `README.zh-TW.md` now read more like GitHub entry pages,
  with clearer install/auth setup, more purposeful workflow examples, and less
  duplicated handbook-style routing.
- High-value handbook and command pages now consistently explain who a page is
  for, when to use the workflow, what success looks like, and what to check
  first when it fails.
- Generated docs navigation and page chrome were refined so handbook,
  command-reference, and maintainer entry pages are easier to scan and use.
- Release metadata and install examples now consistently point at `0.8.0`
  across the version file, package metadata, install help, and getting-started
  examples.

### Fixed

- Generated manpages no longer misinterpret evidence headings such as
  `Before / After` or `Failure checks` as bogus subcommands.
- Release/version updates now regenerate manpage output without tripping the
  AI workflow drift guard when the only source change is the release version.
- Generated command/manpage outputs were refreshed so the browser docs and
  manpage lane match the current source docs and CLI help.

## [0.7.3] - 2026-04-03

### Highlights

- Command and handbook documentation entrypoints now expose the full command
  and subcommand surface more consistently, so discoverability issues like
  `dashboard screenshot` being hard to find are reduced across README, indexes,
  and handbook pages.
- GitHub Pages docs now support the multi-version site layout introduced on the
  docs side, while `dev` branch pushes validate the site build without trying
  to deploy through the protected Pages environment.
- Release metadata and generated docs are aligned again for the `0.7.3` line,
  including versioned manpage HTML mirrors and refreshed generated output.

### Added

- Versioned GitHub Pages site assembly for handbook, command reference, and
  manpage HTML output, including `/latest/`, `/dev/`, and release-lane paths.
- HTML mirrors for generated manpages so the published docs site can expose the
  manpage content in browser-readable form.

### Changed

- Command index and handbook navigation now enumerate command/subcommand
  entrypoints more completely in both English and Traditional Chinese.
- `dev` branch Docs Pages runs now stop after the build artifact on CI, while
  `main` remains the deploy path for the protected `github-pages` environment.
- The main branch release line now reports `0.7.3` across canonical version
  metadata and generated manpage output.

### Fixed

- The install script quality checks now match the current flavor-aware release
  archive naming and no longer fail under plain `sh` due to helper definition
  order.
- Rust quality gates are green again after addressing the clippy violations in
  dashboard raw-to-prompt logging and OS-backed profile secret storage.

## [0.7.2] - 2026-04-03

### Highlights

- The documentation entry flow now starts more cleanly by language, so English
  and Traditional Chinese handbook/reference paths no longer feel mixed
  together at the front door.
- The Traditional Chinese operator docs were refined toward more natural
  Taiwan-oriented wording for handbook, onboarding, and command-reference use.
- README badges now point at the active `kenduest-brobridge/grafana-utils`
  repository and use a tag-based version badge that matches the published tags.

### Changed

- Handbook and command-reference index pages now present language switching
  before deeper reading paths.
- The generated HTML landing page now separates English and zh-TW entrypoints
  before handbook, command-reference, and role-specific navigation.
- Several zh-TW handbook pages now use more natural operator-facing wording for
  onboarding, governance, and status-review concepts.

### Fixed

- README badges no longer point at the old `kendlee/grafana-utils` repository.
- The English command reference index no longer lists
  `dashboard raw-to-prompt` twice.

## [0.7.1] - 2026-04-03

### Highlights

- The published HTML docs site is now enabled on GitHub Pages for the
  `kenduest-brobridge/grafana-utils` repository and linked from the README.
- The README landing pages were rewritten in a more professional,
  operator-facing tone for SRE, sysadmin, and platform-maintainer audiences.
- The Traditional Chinese README wording was refined toward Taiwan usage rather
  than literal machine-style translation.

### Added

- GitHub Pages enablement in the docs deployment workflow so the docs site can
  self-bootstrap on repositories that have not enabled Pages yet.

### Changed

- README and README.zh-TW now point to the correct published docs URL:
  - `https://kenduest-brobridge.github.io/grafana-util/`
- The English README now presents the tool as an operational and
  administration CLI rather than as a marketing-style landing page.
- The Traditional Chinese README now uses more natural Taiwan-oriented wording
  across the landing sections, workflow headings, and documentation routing.

### Fixed

- The previous published docs URL in the README pointed at an old
  `kendlee.github.io` address and did not resolve for the current repository.

## [0.7.0] - 2026-04-03

### Highlights

- Dashboard migration now includes a dedicated `raw-to-prompt` workflow for
  converting ordinary Grafana dashboard JSON into UI-importable prompt JSON.
- The operator documentation now has a generated reference surface alongside
  the handbook, including man pages, local HTML output, and a clearer
  maintainer-oriented docs map.
- Live connection profiles now support multiple secret-storage modes, so
  repeated URL and credential handling can move out of ad hoc command lines.

### Added

- New dashboard migration command:
  - `grafana-util dashboard raw-to-prompt`
- New profile secret-storage modes in the Rust implementation:
  - `file`
  - `os`
  - `encrypted-file`
- New generated documentation surfaces:
  - `docs/man/*.1`
  - `docs/html/`
  - `make man`
  - `make man-check`
  - `make html`
  - `make html-check`
- New maintainer/internal docs for generated-doc architecture, playbooks,
  maintainer quickstart, role maps, and profile secret-storage design.
- New browser-enabled Rust build lane for release artifacts that need the
  optional browser feature.

### Changed

- `raw-to-prompt` now preserves the historical prompt semantics used by the
  existing exported dashboard bundles, including datasource-variable and
  prompt-placeholder edge cases.
- The operator handbook, command reference, generated HTML, and man pages are
  now aligned around the same command-doc source layer instead of drifting as
  separate handwritten outputs.
- Auth and example guidance now prefer `--profile` and basic-auth/profile
  workflows before token-first examples, with clearer notes around multi-org
  and scope limitations.
- Maintainer entry docs now route more explicitly by task and role, including
  first-entry guidance for new maintainers and AI/code agents.

### Fixed

- `cargo clippy --all-targets -- -D warnings` is clean again after addressing
  the profile CLI enum layout and secret-store lint issues.
- Dashboard prompt migration now matches the checked historical prompt bundle
  semantics used for compatibility validation, rather than producing only a
  superficially similar prompt document.

### Migration Notes

- If you receive a plain Grafana dashboard export and need a UI-importable
  prompt file, use `grafana-util dashboard raw-to-prompt` instead of trying to
  feed that JSON directly into the staged replay/import workflow.
- If you maintain local docs or release docs, treat `docs/commands/*` and
  `docs/user-guide/*` as the editable source layers; `docs/man/*.1` and
  `docs/html/` are generated artifacts.
- If you rely on repeated live connection flags, prefer moving them into
  profiles before the `0.7.0` line lands more broadly in release workflows.

## [0.6.3] - 2026-04-02

### Highlights

- The user guide now works as a bilingual handbook instead of a mixed set of
  oversized files and partial entry pages.
- The Traditional Chinese guide now follows the same chapter-based structure
  as the English handbook, so operators can move between languages without
  losing the reading order, workflow boundaries, or chapter layout.

### Changed

- `docs/user-guide/zh-TW/` is now the primary Traditional Chinese user-guide
  location.
- The English handbook chapters were expanded with per-page navigation,
  clearer table-based summaries, and more explicit output and interactive-mode
  guidance.
- The Traditional Chinese handbook now includes matching chapters for getting
  started, reference, dashboards, datasources, alerts, access, project-wide
  status, and scenarios.
- `docs/user-guide-TW.md` now remains as a compatibility bridge for older
  links while directing readers into the new handbook structure.
- `README.md`, `README.zh-TW.md`, and `docs/DEVELOPER.md` now point to the
  maintained handbook paths.

### Notes

- This release remains documentation-focused. It improves how operators find,
  read, and follow the maintained workflows; it does not change the CLI's live
  Grafana behavior.

## [0.6.2] - 2026-04-02

### Highlights

- The English operator guide moved from one oversized file into a handbook
  layout with focused chapters for getting started, reference, dashboards,
  datasources, alerts, access, project-wide status, and task-oriented
  scenarios.
- Release-facing entry docs now point readers directly at the handbook, so
  the first-run path is clearer and the older single-file guide no longer has
  to carry the full operator manual by itself.

### Changed

- `docs/user-guide/en/` is now the primary English user-guide location.
- `docs/user-guide.md` now remains as a compatibility bridge for older links
  while directing readers into the handbook structure.
- The English handbook now includes validated live-output excerpts for version
  checks, project status, dashboard inventory, and datasource inventory.

### Notes

- This release is documentation-focused. It does not change the Grafana API
  behavior of the CLI; it changes how the operator manual is organized and how
  readers discover the maintained workflows.

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

## [0.2.20] - 2026-03-23

### Highlights

- Dashboard workflows expanded further around browse/governance-style flows.
- Sync/preflight and datasource-secret handling became much more explicit.
- Operator and maintainer docs were refreshed alongside the larger CLI growth.

### Notes

- This was one of the largest `0.2.x` releases and acted as the handoff point
  into the later `0.3.x` Rust-first line.

## [0.2.19] - 2026-03-18

### Highlights

- Release/version handling was refreshed.
- Sync-related release preparation and staging behavior were tightened.

## [0.2.18] - 2026-03-17

### Highlights

- Dashboard workflows expanded noticeably, especially around governance and
  inspection-related behavior.
- Sync/preflight and access/alert support both grew in scope.

### Notes

- This release significantly broadened the operator surface before the later
  `0.2.19` and `0.2.20` cleanup/release-prep steps.

## [0.2.17] - 2026-03-17

### Highlights

- Dashboard workflows were broadened again, with more practical review and
  maintenance-oriented behavior.
- Public docs and release/build polish were refreshed at the same time.

## [0.2.16] - 2026-03-16

### Highlights

- Sync/preflight behavior improved substantially.
- Early interactive/TUI-style review flows were added.

## [0.2.15] - 2026-03-16

### Highlights

- Release/build handling was refreshed.

## [0.2.14] - 2026-03-16

### Highlights

- Release packaging and version metadata were adjusted for the next point
  release step.

## [0.2.13] - 2026-03-16

### Highlights

- Release/build handling was refreshed.

## [0.2.12] - 2026-03-16

### Highlights

- Operator and maintainer docs were reorganized.
- Release/build handling was refreshed during the same pass.

## [0.2.11] - 2026-03-16

### Highlights

- Release/build handling was refreshed.

## [0.2.10] - 2026-03-16

### Highlights

- The project took a major step forward in dashboard, sync, access, and alert
  operator coverage.
- This release is the main pivot point where the CLI started to feel like a
  broader multi-surface operator tool instead of a smaller utility set.

## [0.2.8] - 2026-03-15

### Highlights

- Operator/maintainer docs and release handling were refreshed together.

## [0.2.7] - 2026-03-15

### Highlights

- This release mostly captured published tree cleanup and consolidation rather
  than a new operator-facing feature cluster.

## [0.2.6] - 2026-03-15

### Highlights

- Release/build handling was refreshed.

## [0.2.5] - 2026-03-15

### Highlights

- Release/build handling was refreshed.

## [0.2.4] - 2026-03-15

### Highlights

- Release/build handling was refreshed.

## [0.2.3] - 2026-03-15

### Highlights

- Release/build handling and docs were both refreshed.

## [0.2.2] - 2026-03-15

### Highlights

- This release primarily captured published tree cleanup without a strong new
  operator-facing feature area.

## [0.2.1] - 2026-03-15

### Highlights

- Docs and release/build handling were refreshed.
- Early status/overview-facing guidance became clearer.

## [0.2.0] - 2026-03-15

### Highlights

- Baseline `0.2.x` release line for the early mixed Rust/Python utility era.

### Notes

- The later `0.2.x` point releases expanded this baseline into broader
  dashboard, sync, datasource, access, and alert operator workflows.
