# Overview Architecture

Maintainer note for `grafana-util overview`.

This file is the source of truth for the Rust `overview` design intent:
- which module owns which responsibility
- how data moves from staged artifacts to output
- how to extend the system without turning it into AI-shaped sprawl

It is not an operator guide. Keep command examples and user-facing behavior in
`README.md` and the user guides.

## Purpose

`grafana-util overview` is a staged-artifact aggregator with one thin live
entrypoint.

It does not fetch live state by default, and it should not invent a second
parallel analysis pipeline. Its job is to:
- load already-staged export or preflight artifacts
- assemble one stable overview document
- project that document into text, JSON, and interactive views
- expose a conservative staged `projectStatus` layer for cross-domain triage
- optionally hand live reads through to the shared `project-status live` path
  without owning live derivation

The maintained mental model is:

`OverviewArgs -> artifacts -> overview document -> text/json/tui`

That single path should stay easy to trace in code.

## Module Boundaries

- `rust/src/overview.rs`
  - Owns CLI args, stable top-level types, thin wrappers, and output-mode dispatch.
  - This is the entrypoint and orchestration layer.
  - It should stay readable from top to bottom.

- `rust/src/overview_artifacts.rs`
  - Owns staged input loading, validation, and `OverviewArtifact` construction.
  - This is where new artifact kinds or new input sources should be added first.

- `rust/src/overview_document.rs`
  - Owns `OverviewDocument` assembly, staged `projectStatus` derivation, and text rendering.
  - This is where cross-domain status logic belongs.

- `rust/src/overview_sections.rs`
  - Owns section, view, and item projection for JSON/TUI consumers.
  - This is a presentation-model layer, not an input-loading layer.

- `rust/src/overview_support.rs`
  - Owns shared JSON/file helpers and common normalization helpers used by sibling modules.
  - Keep this low-level and boring.

- `rust/src/overview_tui.rs`
  - Owns interactive browsing only.
  - It must not become the owner of contract logic, staged interpretation, or document mutation.

## Data Flow

The intended flow is linear:

1. `OverviewArgs`
2. `build_overview_artifacts`
3. `build_overview_document`
4. `render_overview_text` or JSON serialization or TUI

For the live convenience entrypoint, the intended flow is also linear:

1. `OverviewLiveArgs`
2. `run_project_status_live`
3. shared project-status text / JSON / TUI

Important design rule:
- summary fields, `projectStatus`, and section/view projections are derived from staged artifacts
- renderers should display the document, not reinterpret the source artifacts
- TUI should browse the document, not rebuild it

Current stable artifact families are:
- dashboard export
- datasource export
- alert export
- access export bundles
- sync summary
- bundle preflight
- promotion preflight

Those artifact summaries are the traceable contract surface. If a domain status
changes, maintainers should be able to point at the staged summary keys or
explicit blocker rows that caused it.

## Extension Rules

- Add or change staged input loading in `overview_artifacts.rs` first.
- Add or change cross-domain status derivation in `overview_document.rs`.
- Add or change visible section/view projection in `overview_sections.rs`.
- Keep schema additions additive unless a documented migration is required.
- Keep `projectStatus` conservative and source-attributable.
- Prefer explicit summary keys or explicit blocker rows over inferred heuristics.
- If a field is for maintainers, prefer metadata like `reasonCode`, `sourceKinds`, and `signalKeys` over free-form prose.

When adding a new domain or artifact:
1. add the artifact loader
2. add document-level summary or status derivation
3. add section/view projection if the domain needs workbench support
4. update focused tests in `rust/src/overview_rust_tests.rs`

## Anti-Patterns To Avoid

- Do not split modules just because a file is large.
  - Split only when the boundary matches a stable responsibility in the data flow.

- Do not put file loading or validation into `overview_document.rs` or `overview_tui.rs`.

- Do not let the TUI own schema or staged contract logic.

- Do not introduce hidden cross-module state, callback-style orchestration, or ad hoc parsing in render paths.

- Do not create many tiny helper modules with unclear ownership.
  - The goal is maintainable boundaries, not maximal fragmentation.

- Do not weaken the artifact-kind whitelist or stable JSON shape without updating downstream tests and maintainer docs in the same change.

## Maintenance Intent

The overview stack is intentionally split for long-term maintenance, not for
cosmetic modularity.

The design goal is:
- a maintainer can start in `overview.rs`
- follow one clear call path
- find loading, document derivation, and projection logic in predictable places
- understand why the system emitted a given status without reverse-engineering AI-generated indirection

If a change makes that harder, the change is probably wrong even if the code
still compiles.
