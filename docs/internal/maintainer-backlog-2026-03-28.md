# Maintainer Backlog

Date: 2026-03-28
Scope: Rust runtime only
Audience: Maintainers
Status: Active follow-up backlog derived from the current architecture review, roadmap, and AI trace docs

## Purpose

This file turns the current architecture and roadmap direction into a short
working backlog.

It is intentionally narrower than `project-roadmap.md` and more action-oriented
than `architecture-review-2026-03-27.md`.

## Phase Progress

- Phase 1 landed: dashboard inspect boundary cleanup now includes
  `inspect_output_report.rs`, `inspect_workbench_content.rs`, and
  `inspect_governance_render.rs`.
- Phase 1 partially landed: dashboard import now has a review-first interactive
  workbench with dry-run mode, context views, and an in-progress boundary split
  across `import_interactive.rs`, `import_interactive_render.rs`,
  `import_interactive_review.rs`, and `import_interactive_context.rs`.
- Phase 1 partially landed: datasource secret handling now has the first
  usable operator contract through import/mutation wiring and import dry-run
  `secretVisibility`.
- Phase 2 partially landed: promotion is now a staged review handoff rather
  than just a skeleton, but apply-side handoff work is still open.

## Now

### 1. Continue dashboard subsystem boundary cleanup

Why now:

- `dashboard` is now the clearest primary complexity center in the architecture
  review.
- More feature work will keep landing here unless ownership boundaries stay
  explicit after the recent inspect/report/governance splits.
- the current worktree has already started the next correct slice: splitting
  the interactive import workbench into state, render, review, and context
  seams instead of letting it become another monolithic dashboard facade.

Scope:

- separate inspect pipeline ownership from governance evaluation
- keep interactive workbench logic from bleeding into unrelated paths
- keep `dashboard import --interactive` moving toward a real subsystem instead
  of one hub module
- continue shrinking orchestration facades instead of only splitting helper
  files

Target areas:

- `rust/src/dashboard/`
- related dashboard tests

### 2. Deepen inspection and governance outputs

Why now:

- inspection remains one of the strongest differentiators in the roadmap
- dashboard dependency usage and orphan reporting already landed, so the next
  useful inspection work is broader governance depth and stronger operator
  visibility rather than reopening the finished dependency-report loop

Scope:

- add stronger governance and quality signals
- deepen blast-radius and stale-resource visibility
- prefer richer report outputs that still reuse the canonical inspection model

Target areas:

- dashboard inspect and governance modules

### 3. Continue selective crate-boundary cleanup

Why now:

- safe `pub(crate)` tightening already happened, but the remaining public
  surface still deserves review before more compatibility exposure accumulates
- this is no longer a first-pass visibility cleanup; it is a selective
  follow-through task

Scope:

- review the remaining public top-level modules and compatibility re-exports
- avoid widening the crate surface again for convenience-only helper paths
- keep contract modules explicit and implementation helpers less visible

Target files:

- `rust/src/lib.rs`
- any modules currently exported only for convenience

### 4. Preserve current sync and promotion contract discipline

Why now:

- `sync` and promotion improved materially, but the wins are recent and easy to
  regress.
- Follow-on work should not collapse staged/live and review/apply boundaries
  back into broad facade modules.

Scope:

- keep staged document ownership explicit
- keep promotion contract additions attached to promotion modules
- avoid adding new behavior directly into `sync` orchestration facades unless
  ownership is already clear

Target areas:

- `rust/src/sync/`

## Next

### 1. Extend datasource secret handling beyond the wired baseline

Why next:

- secret handling is now the clearest remaining adoption gap
- the first operator-facing contract is in place, but provider/backfill
  coverage and later-stage review or failure handling remain incomplete

Scope:

- keep import, mutation, and staged secret wording aligned
- make secret-missing and secret-loss cases explicit through later workflow
  stages, not only bundle-preflight
- add stronger reviewability for placeholder availability and missing-secret
  states
- evaluate provider-aware integration only where it remains explicit and
  reviewable

Target areas:

- `rust/src/datasource.rs`
- `rust/src/datasource_secret.rs`
- sync/apply integration points

### 2. Keep sync trustworthiness strong while promotion and secret flows grow

Why next:

- `sync` is no longer missing core staged structure, but it is still one of the
  clearest places where new behavior could collapse back into broad facades
- promotion and secret work now depend on keeping staged/live and review/apply
  seams explicit

Scope:

- preserve staged/live ownership
- preserve review/apply and promotion-module seams
- avoid routing new cross-resource logic into orchestration facades by default

Target areas:

- `rust/src/sync/`

## Later

### 1. Extend promotion from preflight into review/apply handoff

Why later:

- promotion is no longer missing; the staged review handoff is partially
  landed, but apply-side refinement still follows later
- current docs place dashboard boundaries and datasource secret wiring ahead of
  deeper promotion refinement

Scope:

- promotion review artifact
- resolved remap inventory
- warning vs blocking separation
- controlled handoff from promotion review into eventual apply

### 2. Expand promotion remap and prerequisite coverage

Why later:

- current promotion checks focus on folders and datasource references
- broader remap logic should build on the existing contract instead of widening
  too early

Scope:

- plugin prerequisites
- alert and contact-point prerequisites
- additional environment-specific rewrite visibility

### 3. Keep advanced analysis and packaging exploratory

Why later:

- these ideas are additive and not on the current correctness path
- they should only grow on top of the existing Rust analysis core

Scope:

- optional AI-assisted or rule-assisted analysis
- optional local packaging surfaces such as browser or WASM reuse

## Order Of Execution

If only a few slices move next, the recommended order is:

1. continue dashboard subsystem boundary cleanup
2. deepen inspection and governance outputs
3. continue selective crate-boundary cleanup
4. preserve sync and promotion contract discipline
5. extend datasource secret handling
5. continue promotion review/apply work later

## Non-Goals For This Backlog

- no Python parity work
- no attempt to replace Terraform or Grafana provisioning
- no SaaS or controller-style expansion
- no broadening of `sync` resource scope before trust and review surfaces are
  stronger
