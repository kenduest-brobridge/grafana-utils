# Project Status Architecture

Maintainer mini-spec for project-wide progress and status visibility across the
Rust mainline.

This file defines the architecture above any single command or TUI surface. It
exists so the project can support a real "whole-project overview" without
turning `grafana-util overview` into the accidental owner of every status rule.

It is not an operator guide. Keep command usage in `README.md` and user guides.

For the current execution map of which domain producers already exist and which
ones are still missing, see `docs/internal/project-status-producer-gap-list.md`.

## Purpose

The project needs one shared status model that can answer:

- what domains exist
- what state each domain is currently in
- which blockers matter most
- what the next useful action is
- whether the answer comes from staged artifacts or live Grafana state

That model must be reusable by:

- CLI text output
- JSON output
- `overview`
- future project-home TUI surfaces
- future domain handoff screens

## Design Rule

Do not treat `overview` as the project-status architecture.

`overview` is one consumer and one entrypoint. The architecture must stay
broader than that so other surfaces can reuse the same status producers and
contracts.

## Layers

The intended stack is:

1. domain status producers
2. shared project-status contract
3. presentation consumers

The maintained mental model is:

`domain source -> domain status document -> project status document -> text/json/tui`

## Layer 1: Domain Status Producers

Each maintained Rust domain should expose a stable status-producing path instead
of forcing project-level views to reinterpret ad hoc command output.

Current target domains:

- dashboard
- datasource
- alert
- access
- sync
- promotion

### Domain Producer Requirements

Each producer should emit a typed or stable JSON-ready document with:

- `scope`
- `mode`
- `summary`
- `blockers`
- `warnings`
- `nextActions`
- `signalKeys`
- `sourceKind`
- `freshness`

The exact field names can evolve, but the semantics should stay consistent.

### Domain Producer Ownership

- `dashboard`
  - source contracts should build on inspect summary, governance, dependency,
    import readiness, and later live inspect surfaces.
- `datasource`
  - source contracts should build on export inventory, secret-reference
    readiness, mutation review, and provider/placeholder checks.
- `alert`
  - source contracts should build on export root summary, asset linkage, and
    later promotion or preflight readiness.
- `access`
  - source contracts should build on export bundle summaries first, then later
    drift/import readiness where supported.
- `sync`
  - source contracts should build on sync summary, staged plan/audit state, and
    review/apply blockers.
- `promotion`
  - source contracts should build on promotion preflight and later review/apply
    handoff status.

## Layer 2: Shared Project-Status Contract

The project-level contract should aggregate domain producers without becoming a
parallel analysis engine.

The shared contract should answer:

- overall status
- per-domain status
- top blockers across domains
- warnings across domains
- recommended next actions
- source attribution
- freshness / staleness
- staged vs live scope

### Minimum Contract Shape

The project-level document should have these logical sections:

- `scope`
  - `staged`, `live`, or `mixed`
- `overall`
  - status, severity, domain counts, blocker counts
- `domains[]`
  - one row per domain with summary, blockers, warnings, next actions, and
    source attribution
- `topBlockers[]`
  - cross-domain blocker rows ranked for operator triage
- `nextActions[]`
  - project-level recommended actions derived conservatively from domain status
- `freshness`
  - source timestamps or staleness flags where available

### Status Semantics

Use conservative shared states:

- `ready`
- `partial`
- `blocked`
- `unknown`

If more precision is needed, add a separate severity or reason layer instead of
multiplying status words.

### Source Attribution

Every project-level status decision must remain source-attributable through:

- `sourceKinds`
- `signalKeys`
- explicit blocker rows
- explicit freshness metadata

Project-level aggregation must not invent opaque conclusions that cannot be
traced back to domain evidence.

## Layer 3: Presentation Consumers

Consumers must display the project-status contract. They must not own status
derivation rules.

Initial consumers:

- `overview` text output
- `overview` JSON output
- `overview` interactive workbench
- `project-status staged`
- `project-status live`
- `project-status` interactive workbench

Future consumers:

- domain handoff panes
- live status surfaces

## Staged And Live Must Stay Distinct

The project must support both, but they are not the same thing.

### Staged Status

Staged status answers:

- what artifacts exist
- what preflight or review state is known
- whether planned promotion or sync workflows are blocked
- whether the repository is ready for the next workflow step

### Live Status

Live status answers:

- what Grafana currently contains
- whether live state appears healthy, risky, stale, or drifted
- whether live dependencies or governance signals need attention

### Rule

Do not blur staged and live results into one undocumented heuristic.

If a surface combines them, it must say so explicitly through `scope` and
source metadata.

## TUI Model

The user-facing TUI direction should not start at an artifact browser. It
should start at a project home.

### Required TUI Navigation Model

1. `Project Home`
   - overall status
   - top blockers
   - domain ranking
   - recommended next actions
   - freshness state

2. `Domain Drill-Down`
   - one domain at a time
   - domain status summary
   - domain blockers and warnings
   - the strongest relevant supporting views

3. `Action Handoff`
   - explain what to run or review next
   - hand the user off to the right domain command or workbench

### TUI Design Rule

The first screen should answer:

- where the operator is
- what is blocked
- what to do next

The current `Sections / Views / Items / Details` model is useful as a browser,
but it should sit behind a project-home landing view if the goal is
project-wide operator progress.

## Current Gap Assessment

### Already Present

- artifact-driven `overview`
- additive staged `projectStatus`
- interactive overview workbench
- shared TUI shell direction
- strong dashboard inspect and sync preflight/report surfaces

### Still Missing

- a project-level status contract that is explicitly broader than `overview`
- dedicated domain status producers for every maintained domain
- top-blocker and next-action ranking across domains
- freshness/staleness semantics
- a separate live-status path
- a project-home TUI surface above the current overview browser

## Recommended Implementation Order

1. define stable domain status producer shapes
2. align `overview` project-status with that shared contract instead of growing a
   one-off shape
3. add project-level `topBlockers` and `nextActions`
4. add freshness/staleness metadata
5. add a separate live-status path
6. add a project-home TUI that hands off into domain workbenches

## Non-Goals

- turning `overview` into a new all-in-one business-logic hub
- re-parsing raw domain artifacts in every renderer
- mixing staged and live semantics without explicit scope
- building a separate UI-only schema that drifts away from the typed/status
  contract
- adding more panes before the project-home and handoff model exists

## Maintenance Rule

When future work claims to improve project-wide overview support, check whether
it strengthens:

- domain producer clarity
- shared project-status contract quality
- source attribution
- staged/live separation
- project-home to domain-handoff flow

If it only makes `overview` larger, it is probably the wrong change.
