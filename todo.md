# TODO

Current maintainer backlog for the Rust-first `grafana-util` project.

Scope rules:

- Treat `rust/src/` as the primary implementation surface.
- Ignore Python implementation unless packaging, install behavior, or explicit parity work requires it.
- Keep README changes out of this backlog unless a task explicitly targets public GitHub positioning.
- Prefer small grouped commits with focused validation.
- Use the conservative boundary policy below before starting any split.

## Current Baseline

- Branch is `dev`; keep new work grouped into focused Rust/test commits.
- GitHub Actions `rust-quality` is currently green on Rust 1.95 after the
  latest clippy compatibility pass.
- Default Rust build and `--features browser` are supported release surfaces.
- `--no-default-features` is explicitly not claimed as a supported release surface yet.
- Dashboard `summary` / `dependencies` naming and review-source model are now clearer.
- Output contracts have root and nested-path validation through `requiredFields`, `requiredPaths`, `pathTypes`, and golden fixtures.
- Remaining risk is mostly maintainability: oversized Rust tests, TUI input/render modules, live apply paths, and overlapping contract systems.

## Split Policy - Conservative Boundaries

Use this policy before implementing any TODO item in this file.

The goal is not to make every file small. The goal is to make each module
own one stable responsibility without turning the codebase into a maze of tiny
files.

Rules:

- Split by responsibility, not by line count alone.
- Keep the original file as a facade, routing point, or assembly point when that helps readability.
- Add at most 1-3 new modules per task unless splitting a test suite into obvious behavior groups.
- Do not extract a module unless its name describes a stable concept in the domain.
- Do not introduce `utils`, `helpers2`, `misc`, or similar catch-all modules.
- Prefer behavior-preserving moves before abstraction changes.
- Keep control flow readable from the parent file after the split.
- Avoid shared traits or generic envelopes until at least two or three domains have proven the same shape.

Pre-split checklist:

- What responsibility is being separated?
- Which file remains the facade after the split?
- Can a reviewer understand the workflow without opening every new file?
- Is the new module name domain-specific and stable?
- Does the split reduce mixed responsibility, or only reduce line count?
- Are fixtures/setup duplicated after the split?

Reject the split if the answer is only "the file is large." Large files are
acceptable when they own one clear responsibility and are easier to read in one
place.

## P0 - Dashboard Prompt External Export

### Align Prompt Export With Grafana UI Semantics

Status: partially done for classic prompt parity. Keep this item open only for
the remaining library panel live-model parity and any future dashboard v2
adapter work.

Problem:

Grafana's official source has two external dashboard export paths. The classic
exporter and the scene exporter agree that prompt output must not synthesize
datasource variables or treat a datasource variable `query` as an import input.
The newer scene exporter also preserves `$datasource` panel references while
mapping a datasource variable's current concrete datasource through a `DS_*`
input when that variable is used by panel or target datasource references.

Official source areas to keep using as behavior references:

- `/Users/kendlee/tmp/grafana/public/app/features/dashboard/components/DashExportModal/DashboardExporter.ts`
- `/Users/kendlee/tmp/grafana/public/app/features/dashboard-scene/scene/export/exporters.ts`
- `/Users/kendlee/tmp/grafana/public/app/features/manage-dashboards/import/utils/inputs.ts`
- `/Users/kendlee/tmp/grafana/pkg/services/dashboardimport/utils/dash_template_evaluator.go`
- `/Users/kendlee/tmp/grafana/pkg/services/dashboardimport/service/service.go`

Action:

- Keep concrete datasource references mapped to `__inputs` and `${DS_*}`.
- Keep datasource variable definitions as variables; do not convert the variable
  `query` into a datasource input.
- Preserve panel and target datasource references such as `$datasource`.
- When a used datasource variable has a concrete current value and datasource
  type, add the corresponding `DS_*` input and set the variable `current.value`
  to `${DS_*}`.
- Keep constant variables mapped through `VAR_*` inputs.
- Keep expression datasource import handling (`__expr__`) out of user-mapped
  datasource inputs.
- Reject dashboard v2 resource/spec input in raw-to-prompt until a dedicated
  adapter exists.
- Add later parity for library panel `__elements` live-model export and import
  input validation.

### Dashboard Source-Alignment Follow-ups

Keep these follow-ups separated from the classic prompt contract so the next
changes stay reviewable and do not blur lane boundaries.

- Add live library-panel `__elements` lookup only on the live export /
  import-handoff path. Keep local raw-to-prompt conversion warning-only when a
  referenced library panel model is missing.
- Keep prompt/export fixture parity anchored to Grafana source testdata for
  datasource variables, selected current datasource handling, library panels,
  and the classic-vs-v2 rejection boundary.
- Add dashboard import/publish preflight evidence for provisioned or managed
  dashboards before any live write. Surface ownership and provenance as target
  evidence instead of waiting for Grafana API failures.
- Keep dashboard v2 as a separate future adapter boundary. Continue rejecting
  v2-shaped input in the classic prompt lane rather than mixing it into
  `raw/`, `prompt/`, or provisioning behavior.
- Treat provisioning as a derived projection that can be compared later
  against Grafana file provisioning. Do not rebase the dashboard contract on
  provisioning as if it were the source of truth.
- Keep dashboard permissions adjacent to access evidence and access workflows,
  not as dashboard JSON fields or as an extension of the prompt export shape.
- Split large dashboard modules by responsibility, not by line count alone.
  Favor focused export planning, prompt conversion, live preflight, and
  provisioning projection boundaries over arbitrary file carving.

Validation:

- `cargo test --manifest-path rust/Cargo.toml --quiet raw_to_prompt`
- `cargo test --manifest-path rust/Cargo.toml --quiet`
- `cargo fmt --manifest-path rust/Cargo.toml --all --check`

## P0 - Test Surface Control

### Split Oversized Rust Test Files

Problem:

Several Rust test files are still too large to review or debug quickly. Keep
splitting only when a file mixes clearly separable behavior groups.

Current hotspots:

- `rust/src/commands/access/rust_tests.rs`
- `rust/src/commands/datasource/tests/payload.rs`
- `rust/src/commands/dashboard/rust_tests.rs`
- `rust/src/commands/snapshot/tests_review_rust_tests.rs`

Action:

- Split by behavior suite, not by arbitrary line count.
- Preserve existing test names when possible.
- Move shared fixture builders into local test support modules.
- Keep each split behavior-preserving.
- Avoid scattering one assertion family across many small files; each new test module should represent a real workflow or contract group.

Suggested order:

1. `datasource/tests/payload.rs`
2. `access/rust_tests.rs`
3. `dashboard/rust_tests.rs`
4. `snapshot/tests_review_rust_tests.rs`

Validation:

- `cargo fmt --manifest-path rust/Cargo.toml --all --check`
- Focused `cargo test --manifest-path rust/Cargo.toml --quiet <domain_or_test_filter>`
- `cargo test --manifest-path rust/Cargo.toml --quiet` when the split crosses module boundaries

## P1 - TUI Boundary Cleanup

### Split Access Team Browse Input

Problem:

`rust/src/commands/access/team_browse_input.rs` is still a dense TUI input surface. It mixes key handling, selection state, confirmation flow, mutation dispatch, refresh behavior, and error handling.

Action:

- Extract only the most stable focused boundary first. Candidate boundaries are:
  - action dispatch
  - confirmation dialogs
  - refresh/reload behavior
  - key handling
- Keep public behavior unchanged.
- Keep live mutation confirmation paths easy to review.
- Do not create all candidate modules in one pass unless each one removes a clearly mixed responsibility.

Validation:

- `cargo test --manifest-path rust/Cargo.toml --quiet team_browse`
- `cargo test --manifest-path rust/Cargo.toml --quiet access`
- `cargo fmt --manifest-path rust/Cargo.toml --all --check`

### Continue Dashboard Browse Render Split

Problem:

Dashboard browse/render is still large and UI-sensitive.

Hotspots:

- `rust/src/commands/dashboard/browse_support.rs`
- `rust/src/commands/dashboard/browse_render.rs`

Action:

- Extract row model helpers.
- Extract detail-pane rendering.
- Extract footer/action rendering.
- Separate live-tree rendering from local-export-tree rendering where practical.
- Keep the main render path readable from the current parent module; do not turn one render file into many single-widget files.

Validation:

- `cargo test --manifest-path rust/Cargo.toml --quiet dashboard_browse`
- `cargo test --manifest-path rust/Cargo.toml --quiet dashboard_cli`

## P1 - Status Producer Model

### Normalize Project Status Producers

Problem:

Status/project-status logic exists across dashboard, datasource, access, alert, sync, and `status overview`, but the producer contract is not fully unified.

Relevant areas:

- `rust/src/commands/dashboard/project_status.rs`
- `rust/src/commands/datasource/project_status/live.rs`
- `rust/src/commands/datasource/project_status/staged.rs`
- `rust/src/commands/access/project_status.rs`
- `rust/src/commands/status/live.rs`
- `rust/src/commands/status/overview/`

Action:

- Introduce a shared data shape before introducing a trait. Candidate names:
  - `StatusProducer`
  - `StatusReading`
  - `StatusWarning`
  - `StatusBlockedReason`
  - `StatusRecordCount`
- Keep `status overview` as a consumer/reporting surface, not an orchestration owner.
- Move domain-specific discovery and warnings into domain producers.
- Delay a shared trait until at least dashboard, datasource, and access prove the same producer interface.

Validation:

- `cargo test --manifest-path rust/Cargo.toml --quiet status`
- `cargo test --manifest-path rust/Cargo.toml --quiet project_status`
- `make quality-architecture`

## P2 - Live Apply Safety

### Split Sync Live Apply By Phase

Problem:

`rust/src/grafana/api/sync_live_apply.rs` is a high-risk live mutation path and remains large.

Action:

- Split by apply phase:
  - request builders
  - dependency ordering
  - apply execution
  - response normalization
  - error classification
- Keep API behavior unchanged.
- Add focused tests around ordering and error normalization if missing.
- Start with one phase boundary, then reassess. Do not split every phase in a single pass if the parent control flow becomes harder to follow.

Validation:

- `cargo test --manifest-path rust/Cargo.toml --quiet sync_live`
- `cargo test --manifest-path rust/Cargo.toml --quiet apply`
- `make quality-sync-rust`

### Standardize Mutation Review Envelopes

Problem:

Dashboard, datasource, access, alert, and workspace mutation flows each have review/dry-run/apply concepts, but envelopes are still domain-shaped.

Action:

- Introduce shared concepts:
  - `ReviewAction`
  - `ReviewRisk`
  - `ReviewRequest`
  - `ReviewApplyResult`
  - `ReviewBlockedReason`
- Keep domain-specific payloads behind a shared review wrapper.
- Avoid changing public JSON contracts until a migration path is defined.
- Start with one internal model or adapter. Do not force all domains to adopt the envelope in the first commit.

Validation:

- Domain-focused tests first.
- Full `cargo test --manifest-path rust/Cargo.toml --quiet` after shared envelope changes.
- `make quality-output-contracts` if JSON output changes.

## P2 - Contract Depth And Schema Governance

### Extend Output Contract Checker

Problem:

The output contract checker now supports nested required paths and path types, but it does not yet validate richer collection shape.

Action:

- Add support for:
  - `arrayItemTypes`
  - `minimumItems`
  - `enumValues`
  - wildcard-like paths such as `operations[*].kind`
- Apply first to dashboard and sync fixtures.
- Keep registry syntax simple enough to review in JSON.

Validation:

- `make quality-output-contracts`
- Negative probes for missing array items, wrong enum values, and wrong item types

### Reconcile Output Contracts And Schema Manifests

Problem:

There are two contract systems:

- `scripts/contracts/output-contracts.json`
- `schemas/manifests` plus `scripts/generate_schema_artifacts.py`

Action:

- Define ownership:
  - output contracts: runtime golden JSON artifacts and regression gates
  - schema manifests: published schema/help contract
- Promote only stable public artifacts from output contracts into schema manifests.
- Document promotion criteria in `docs/internal/contract-doc-map.md`.

Validation:

- `make quality-output-contracts`
- `make schema-check`
- `make quality-docs-surface`

## P2 - Dashboard Review Model Completion

### Wire Review Source Model Into Remaining Dashboard Paths

Problem:

`review_source.rs` now models export-tree, saved-artifact, and live review inputs for topology/impact/policy. Some dashboard summary/help/internal names still use inspection/analysis vocabulary where the concept is really review or summary.

Action:

- Audit dashboard modules for stale user-facing `analysis` wording.
- Keep true query analyzer internals as analyzer names.
- Route any remaining policy/topology/impact source resolution through `review_source`.
- Add tests around saved-artifact vs live/export source selection.
- Do not rename internal analyzer modules that really parse query language or query family behavior.

Validation:

- `cargo test --manifest-path rust/Cargo.toml --quiet topology`
- `cargo test --manifest-path rust/Cargo.toml --quiet governance_gate`
- `cargo test --manifest-path rust/Cargo.toml --quiet dashboard_cli_inspect_help`

## P3 - Docs And Generated Surface Discipline

### Add Docs Diff Classifier

Problem:

Generated man/html docs are correct but noisy. Small source/help changes can produce large diffs, making review harder.

Action:

- Add a classifier script that reports:
  - source docs changed
  - generated docs changed
  - command contract changed
  - public CLI changed
  - generated docs changed without source/contract reason
  - source docs changed but generated docs missing
- Integrate as a maintainer-only quality command.

Validation:

- `make man-check`
- `make html-check`
- `make quality-docs-surface`

### Keep Public Command Wording Consistent

Problem:

The project has intentionally moved away from stale `dashboard analyze` naming. Future command docs and help text can drift back unless wording stays guarded.

Action:

- Keep removed public paths in `scripts/contracts/command-surface.json`.
- Keep docs checks rejecting removed public paths outside archive/trace contexts.
- Prefer:
  - `dashboard summary` for live dashboard review
  - `dashboard dependencies` for local/export dependency review
  - `query analyzer` only for true internal analyzer code

Validation:

- `make quality-docs-surface`
- `make quality-ai-workflow`
- targeted `rg` search for removed public paths

## P3 - Feature Matrix Maturity

### Add Full Rust Feature Matrix Gate

Problem:

`make quality-rust-feature-matrix` now documents the supported feature surfaces, but it is mostly a policy/gate check rather than a full artifact capability check.

Action:

- Add optional deeper gate:
  - default check
  - browser check
  - explicit no-default expected-fail probe
  - TUI-gated module lint check
- Keep the fast gate cheap enough for normal development.

Validation:

- `make quality-rust-feature-matrix`
- optional `make quality-rust-feature-matrix-full`

## P3 - Product Surface Balance

### Keep Domain Maturity Balanced

Problem:

Dashboard tooling remains deeper than some other domains. That is useful, but the tool should not become dashboard-only in practice.

Action:

- For every new dashboard intelligence feature, check whether access, datasource, alert, or workspace needs a corresponding minimal contract.
- Prefer shared review/status/output infrastructure before adding another dashboard-only surface.
- Keep simple backup/export use cases low-friction.

Validation:

- `make quality-architecture`
- `make quality-docs-surface`
- domain-focused Rust tests

## General Guardrails

- Do not inspect or edit `rust/target`.
- Do not modify README unless the task explicitly targets GitHub-facing positioning.
- Do not touch Python implementation for these tasks.
- Do not perform mechanical line-count splits without the pre-split checklist.
- Prefer fewer, stronger modules over many tiny modules.
- Use grouped commits:
  - `refactor:` for behavior-preserving Rust splits
  - `test:` for contract/test coverage
  - `docs:` for maintainer docs and generated docs
  - `bugfix:` only for real behavior fixes
- For public CLI/help/docs changes, run:
  - `make quality-docs-surface`
  - `make man-check`
  - `make html-check`
- For output JSON changes, run:
  - `make quality-output-contracts`
- For broad Rust refactors, run:
  - `cargo fmt --manifest-path rust/Cargo.toml --all --check`
  - focused Rust tests
  - `cargo test --manifest-path rust/Cargo.toml --quiet`
  - `cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings`
