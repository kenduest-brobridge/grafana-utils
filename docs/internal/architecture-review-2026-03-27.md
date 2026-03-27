# Architecture Review

Date: 2026-03-27
Scope: Rust runtime only
Audience: Maintainers
Status: Updated for current Rust mainline state on 2026-03-28

## Purpose

This document records an architecture-focused review of the maintained Rust
codebase.

It is intended to answer four questions:

- what the project is today
- where the current design is strong
- what looks risky or structurally weak
- what should be improved next

## Current Shape

The project is no longer just a collection of Grafana API wrappers.

The maintained Rust runtime already behaves like an operator-focused toolkit
with five major domain surfaces:

- `dashboard`
- `datasource`
- `alert`
- `access`
- `sync`

At a high level, the current structure is:

- `rust/src/cli.rs`: unified entrypoint, namespaced command topology, and top-level help flow
- `rust/src/access/`: user, org, team, and service-account lifecycle workflows
- `rust/src/dashboard/`: export, import, diff, inspect, governance, screenshot, topology, and interactive browse/workbench flows
- `rust/src/datasource.rs` plus helper modules: datasource inventory, import/export/diff, live mutation, and staged secret placeholder support
- `rust/src/alert.rs` plus helper modules: alert export/import/diff/list and related support paths
- `rust/src/sync/`: staged summary, plan, review, audit, preflight, promotion-preflight, bundle-preflight, and apply workflows
- `rust/src/common.rs` and `rust/src/http.rs`: shared error, auth, JSON, and transport foundations

Since the original review pass, several recommendations have already landed or
partially landed:

- dashboard inspect boundaries are clearer through `inspect_output_report.rs`, `inspect_governance_render.rs`, and `inspect_workbench_content.rs`
- `sync` has clearer staged-vs-live ownership boundaries and stronger staged wording
- promotion now has a mapping contract, handoff summary, review instruction, and help text that frames it as a staged review handoff
- datasource secret handling now has a staged placeholder contract, import and mutation wiring, and import dry-run `secretVisibility`
- unified CLI help/example ownership is less centralized than it was
- release/build policy is more explicit in CI and maintainer build paths

The project's strongest product direction is already visible:

- migration and replay
- dependency inspection and governance
- reviewable staged workflows instead of blind mutation
- safety-first operator flows

## Strengths

### Clear product direction

The repo has a stronger point of view than a generic Grafana utility.

It is clearly oriented toward:

- migration safety
- inspection depth
- governance visibility
- reviewable staged workflows

This is a meaningful differentiator compared with simple export/import tools.

### Broad functional coverage

The maintained Rust surface already covers a wide set of real operator tasks:

- resource inventory
- export/import/diff
- dashboard inspection
- staged sync planning and review
- datasource mutation
- datasource secret placeholder review and resolution
- promotion-preflight handoff
- access lifecycle management

This gives the project real platform value even before the next round of
refinement.

### Good testing posture

The Rust test surface is broad and active.

The repo also enforces a meaningful quality gate through:

- tests
- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`

That is a solid foundation for ongoing refactor work.

### Active modularization effort

The codebase is moving away from monolithic implementation files into smaller
subsystem-oriented modules.

That direction is correct and is already visible in current `dashboard`,
`sync`, and datasource secret handling work.

## Main Design Risks

### 1. `dashboard` is still the clearest complexity center

This remains the largest structural risk.

Recent modularization is real, not cosmetic. The inspect pipeline now has
clearer output, governance-render, and workbench-content ownership than it did
in the earlier review. Even so, `dashboard` still carries the highest feature
density and the most cross-cutting operator workflow logic.

The deeper risk is still responsibility coupling:

- command orchestration
- contract building
- render decisions
- live-vs-local workflow branching
- inspect/import/governance crossover

The current state is better than it was, but `dashboard` is still the primary
place where follow-on feature work can re-concentrate complexity.

### 2. Public crate boundaries are broader than necessary

`rust/src/lib.rs` still exports domain runtime modules, contract modules,
helpers, and compatibility re-exports from one crate surface.

That makes the crate play multiple roles at once:

- executable runtime library
- internal shared implementation
- compatibility layer

This is workable for now, but it still weakens API discipline and makes it
harder to tell which boundaries are truly public and which are just convenient
to expose.

### 3. Operator workflows are stronger than before, but some are still incomplete

This risk is no longer about missing primitives.

Promotion now has a real staged mapping contract and review handoff shape.
Datasource automation now has a real staged placeholder contract and live
resolution wiring. The unevenness is now in end-to-end completion rather than
in first-contract existence.

The main incomplete operator stories are:

- promotion beyond review handoff into fuller controlled apply continuation
- datasource secret workflows beyond placeholder review and inline resolution
- dashboard-side operator flows that still cross many subsystems

### 4. Secret handling is no longer unmodeled, but it is still an adoption gap

This area has advanced materially.

The project now has:

- staged datasource secret placeholder planning
- import and mutation wiring for placeholder resolution
- import dry-run `secretVisibility`
- sync wording aligned around staged placeholder availability

What remains weak is not the presence of a contract, but the completeness of
the surrounding workflow:

- provider-aware integration remains limited
- later-stage missing-secret and secret-loss handling is still conservative
- secret review surfaces are not yet as complete as sync/promotion review

## Design Smells To Watch

These are not all immediate defects, but they are worth tracking:

- too many public or re-exported modules for internal-only behavior
- facade modules that still know too much about downstream shape even after file splits
- dashboard-side workflow logic spreading across inspect/import/governance paths
- promotion growing by additive slices faster than review/apply continuation is finalized
- secret-handling semantics staying correct but becoming fragmented across dry-run, mutation, and staged review surfaces

## Three-Phase Improvement Plan

### Phase 1: Consolidate The Wins Already Landed

Status:

- largely landed

Goal:

- keep the recently improved dashboard, sync, promotion, and datasource-secret
  boundaries from drifting again

Recommended work:

- preserve the newer dashboard inspect subsystem splits instead of routing new behavior back into facade modules
- keep promotion help text, mapping contract, handoff summary, and review instruction aligned as one staged story
- keep datasource placeholder review and mutation semantics aligned as one contract
- keep public-vs-internal crate boundaries under review as helper modules continue to grow

Definition of success:

- recently improved areas stay stable under follow-on feature work
- new staged work keeps the same contract discipline as current sync and promotion paths
- maintainers can explain the current ownership boundaries without reconstructing them from many files

### Phase 2: Refactor By Stable Subsystem Boundaries

Status:

- partially landed

Goal:

- move from smaller files to clearer ownership boundaries

Recommended work:

- continue shrinking `dashboard` orchestration and facade ownership, especially where inspect, import, and governance still overlap
- keep workbench content, governance rendering, and report rendering attached to their current subsystem homes
- keep `sync` promotion work attached to staged contract ownership rather than spreading across unrelated modules
- avoid adding new features directly into orchestration facades unless the ownership boundary is already clear

Definition of success:

- `dashboard` no longer dominates the structural risk profile as clearly as it does today
- major domains have identifiable subsystem boundaries instead of only split helper files
- maintainers can predict where new work belongs without re-reading many modules

### Phase 3: Complete The Operator Stories

Status:

- partially landed, still incomplete

Goal:

- turn the strongest primitives into stronger end-to-end workflows

Recommended work:

- extend promotion from staged review handoff into a fuller controlled review/apply continuation
- strengthen rewrite, remap, and prerequisite visibility where promotion still depends on manual interpretation
- extend datasource secret handling beyond placeholder planning and inline resolution into clearer later-stage review and failure handling
- keep sync and operator review surfaces coherent instead of fragmenting across too many staged document types

Definition of success:

- users can move from inventory and export to promotion and review with fewer custom steps
- sync and promotion workflows are trusted because failure states are explicit and understandable
- datasource automation becomes more realistic for production use

## Recommended Feature Investment Order

### 1. Inspection and dependency governance

This remains the strongest differentiator.

The mainline architecture is already moving in the right direction here, so the
next work should deepen operator value without collapsing the new boundaries.

Priority additions:

- stronger dependency and governance reporting
- blast-radius and stale-resource visibility
- management-friendly static or HTML-style reports
- continued dashboard boundary cleanup where inspect/import/governance still overlap

### 2. Datasource secret handling

This is no longer a missing-contract problem. It is now a completeness problem.

Priority additions:

- stronger staged review for placeholder availability and missing-secret states
- clearer secret-loss and later-stage failure handling
- optional provider-aware integration that stays reviewable
- keeping import, mutation, and staged sync wording aligned

### 3. Environment promotion

Promotion is no longer just a future direction.

The project already has a staged mapping contract, a promotion handoff summary,
a review instruction, and help text that frames promotion-preflight as a
staged review handoff. The next work should extend that handoff cleanly rather
than redefining it.

Priority additions:

- fuller continuation from promotion review into eventual controlled apply
- stronger rewrite and prerequisite coverage
- clearer warning-vs-blocking separation
- keeping promotion aligned with sync trust and staged review semantics

### 4. Sync trustworthiness

`sync` has improved materially. The next sync work should stay selective, not
broad.

Priority additions:

- keep fail-closed behavior strong as promotion and other staged contracts grow
- avoid explainability regressions as new review documents are added
- preserve clear staged/live ownership boundaries
- add more sync depth only when it reinforces operator trust

### 5. Advanced assisted analysis

This should remain exploratory until the core workflow layers are stronger.

Priority additions:

- optional assisted query review
- explainable lint or recommendation output
- local packaging only if it reuses the Rust core cleanly

## Bottom Line

The project is already strong in breadth and direction.

The main challenge is no longer "add the first version" of these workflows.
Several of the most important contracts are now present. The challenge is:

- keeping implicit rules explicit
- preventing complexity from re-concentrating in `dashboard`
- finishing operator workflow continuity after the recent staged contract wins

The best next investments are now:

- deeper inspection and dependency governance
- completing datasource secret handling beyond its current staged placeholder baseline
- extending promotion beyond the current staged review handoff
- keeping sync trustworthy without letting its complexity re-concentrate
- continuing subsystem-boundary cleanup where current mainline work has already started to land
