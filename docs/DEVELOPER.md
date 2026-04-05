# Developer Notes

This page is the maintainer entrypoint for the repo.

Use it to orient quickly, then jump to the narrower document that matches the kind of change you are making. Keep this file short, indexed by concern, and free of long contract detail that belongs in the dedicated internal docs.

## Start Here

If this is your first time entering the repo, open
`docs/internal/maintainer-quickstart.md` first.

If you are working on:

- runtime and CLI behavior:
  start with `rust/src/cli.rs`, the domain runtime modules under `rust/src/`, and the architecture walkthroughs in `docs/overview-rust.md` and `docs/internal/overview-architecture.md`
- command or handbook docs:
  start with `README.md`, `README.zh-TW.md`, `docs/user-guide/`, `docs/commands/`, and the generated-docs notes listed below
- generated `man` or HTML docs:
  start with `docs/internal/generated-docs-architecture.md` and `docs/internal/generated-docs-playbook.md`
- contract or schema changes:
  start with `docs/internal/contract-doc-map.md`
- validation, build, or release workflow:
  start with `Makefile` and the repo scripts under `scripts/`
- change history or current AI-maintained status:
  start with `docs/internal/ai-status.md` and `docs/internal/ai-changes.md`
- AI-assisted maintainer workflow:
  start with `docs/internal/ai-workflow-note.md`

If you prefer to route by maintainer persona instead of by subsystem, open `docs/internal/maintainer-role-map.md`.

## Repo Priorities

- Supported implementation surface: Rust under `rust/src/`
- Legacy reference surface: Python under `python/grafana_utils/`
- Operator-facing docs: `README.md`, `README.zh-TW.md`, `docs/user-guide/`
- Command-reference docs: `docs/commands/`
- Generated artifact docs: `docs/man/`, `docs/html/`

Treat the Rust runtime as the supported product surface. Keep the Python package as legacy maintainer reference material unless a task explicitly targets the Python lane.

## User-Facing CLI Surface

Current maintained public namespaces:

- `grafana-util dashboard`
- `grafana-util alert`
- `grafana-util access`
- `grafana-util change`
- `grafana-util status`
- `grafana-util overview`
- `grafana-util profile`
- `grafana-util snapshot`

Treat `rust/src/cli.rs` as the command-topology entrypoint and the domain facade modules as the runtime dispatch layer.

## Code Architecture Map

Primary code paths:

- `rust/src/cli.rs`
  - namespaced CLI dispatch and help routing
- `rust/src/cli_help.rs`
  - unified help rendering and example blocks kept out of `cli.rs`
- `rust/src/dashboard/`
  - dashboard export, import, diff, inspect, prompt-export, authoring, and screenshot workflows
- `rust/src/datasource.rs`
  - datasource list, export, import, diff, add, modify, and delete workflows
- `rust/src/alert.rs`
  - alerting export, import, diff, planning, apply, and shared alert helpers
- `rust/src/access/`
  - org, user, team, and service-account workflows plus shared helpers
- `rust/src/sync/`
  - internal runtime namespace behind the public `change` workflow
- `rust/src/sync/apply_contract.rs`
  - typed apply-intent envelope shared by local builders and live execution
- `python/grafana_utils/`
  - legacy Python reference implementation
- `python/tests/`
  - legacy Python regression coverage

Architecture walkthroughs:

- `docs/overview-rust.md`
- `docs/overview-python.md`
- `docs/internal/overview-architecture.md`

## Documentation Map

Use different docs for different purposes. Do not overload one file with every concern.

### Public Operator Docs

- `README.md`
- `README.zh-TW.md`
- `docs/user-guide/en/`
- `docs/user-guide/zh-TW/`

Use these for workflow, intent, examples, recommended order, and operator guidance.

### Command Reference Source

- `docs/commands/en/`
- `docs/commands/zh-TW/`

Use these as the source layer for per-command reference content. They are also the source for generated manpages and command-reference HTML.

### Generated Docs Design And Maintenance

- `docs/internal/generated-docs-architecture.md`
  - design, source-of-truth model, output tree, generator responsibilities
- `docs/internal/generated-docs-playbook.md`
  - step-by-step maintenance recipes for common docs-generator tasks

Generated artifact ownership:

- `scripts/generate_manpages.py`
  - owns `docs/man/*.1`
- `scripts/generate_command_html.py`
  - owns the HTML docs site rooted at `docs/html/index.html`

Regeneration commands:

```bash
# Purpose: Regeneration commands.
make man
make man-check
make html
make html-check
```

GitHub Pages deployment lives in `.github/workflows/docs-pages.yml`. It runs `make html`, uploads `docs/html/`, and publishes the site rooted at `docs/html/index.html`.

### Contract And Policy Docs

- `docs/internal/maintainer-quickstart.md`
  - first-entry routing for new maintainers and AI agents
- `docs/internal/ai-workflow-note.md`
  - repo-specific AI-assisted maintenance workflow
- `docs/internal/contract-doc-map.md`
- `docs/internal/export-root-output-layering-policy.md`
- `docs/internal/dashboard-export-root-contract.md`
- `docs/internal/datasource-masked-recovery-contract.md`
- `docs/internal/alert-access-contract-policy.md`
- `docs/internal/profile-secret-storage-architecture.md`
- `docs/internal/maintainer-role-map.md`

Use these for typed contract details, compatibility rules, stable field inventories, and policy boundaries. Do not duplicate that level of detail in this file.

### Trace And Change History

- `docs/internal/ai-status.md`
- `docs/internal/ai-changes.md`
- `docs/internal/archive/`

Use these for current trace and condensed history. Keep architecture or contract detail in the design/spec docs, not in the trace files.

## Validation And Build Guide

Use the repo-maintained commands first instead of ad hoc one-offs.

Primary entrypoint:

- `Makefile`

Common maintainer flows:

```bash
# Purpose: Validation And Build Guide.
make help
make test
make test-python
make test-rust
make man-check
make html-check
```

Release and artifact guidance:

- keep version bumps and release validation in the standard maintainer flow documented by the `Makefile`, repo scripts, and release notes
- for Linux artifact validation, build the release artifact first, then run that artifact inside a Linux container
- do not default to throwaway `cargo run` or broad ad hoc container commands unless you are debugging the build lane itself
- reuse fixed Docker image and container names for repo-maintained Linux validation flows so repeated runs replace the same runtime instead of leaving many short-lived containers behind

Browser-enabled build policy:

- the default Rust build should stay lean and omit the `browser` feature
- browser support is an explicit secondary build lane
- only the `*-browser` build targets and release assets should include `headless_chrome`

## High-Signal Project Rules

- Prefer updating Rust behavior and help text first; treat Python as legacy reference unless the task explicitly requires parity work there.
- Keep public usage guidance in `README.md`, `README.zh-TW.md`, and the user guides.
- Keep command-reference detail in `docs/commands/`.
- Keep generated docs logic narrow and documented through the generated-docs architecture and playbook.
- Keep typed contracts in the dedicated internal policy/spec docs.
- Prefer repo-owned typed envelopes over ad hoc maps when a workflow already owns the shape.
- Keep facades thin: `cli.rs` and domain `mod.rs` files should route, normalize, and re-export, not absorb downstream contract logic.
- Keep comments high-signal: explain ownership, invariants, and non-obvious behavior; do not narrate obvious control flow.
- Keep trace/history notes in `docs/internal/ai-status.md` and `docs/internal/ai-changes.md`.

## Maintainer Personas

- Runtime / CLI maintainer:
  start with `rust/src/cli.rs`, the domain modules under `rust/src/`, and `docs/internal/maintainer-role-map.md`.
- Docs / docs-generator maintainer:
  start with `docs/internal/maintainer-quickstart.md`, `docs/internal/generated-docs-architecture.md`, `docs/internal/generated-docs-playbook.md`, and `scripts/generate_command_html.py`.
- Contract / schema maintainer:
  start with `docs/internal/maintainer-quickstart.md`, `docs/internal/contract-doc-map.md`, and the linked policy docs.
- Build / release maintainer:
  start with `docs/internal/maintainer-quickstart.md`, `Makefile`, `scripts/`, and `docs/internal/maintainer-role-map.md`.

## Quick Routing Table

If the task is:

- CLI topology or help routing:
  `rust/src/cli.rs`
- dashboard lane behavior:
  `rust/src/dashboard/`
- datasource lane behavior:
  `rust/src/datasource.rs`
- alert lane behavior:
  `rust/src/alert.rs`
- access lane behavior:
  `rust/src/access/`
- change/status/overview runtime behavior:
  `rust/src/sync/` and the related project-status/overview modules
- generated docs behavior:
  `scripts/generate_manpages.py`, `scripts/generate_command_html.py`, and the generated-docs internal docs
- build, validation, and release commands:
  `Makefile` and `scripts/`
- contract boundaries:
  `docs/internal/contract-doc-map.md`

## Maintenance Standard

When you make a meaningful architecture, contract, or generated-docs workflow change:

- update the narrow spec/design doc first
- update this maintainer map only if the entrypoint or routing changed
- update `docs/internal/ai-status.md` and `docs/internal/ai-changes.md` when the repo policy says the change is meaningful enough to trace
- for docs-only updates, validate with `git diff --check` and the narrow docs checks that match the touched files

For the current summary/spec/trace split, start with [`docs/internal/contract-doc-map.md`](/Users/kendlee/work/grafana-utils/docs/internal/contract-doc-map.md).
