# Project Roadmap

Date: 2026-03-27
Scope: Rust runtime roadmap for the maintained `grafana-util` CLI.
Source: Derived from current repository state, maintainer docs, and product-direction review.

## Purpose

This roadmap defines what should come next for the project and what should stay out of scope.

It is not a raw feature backlog. It is a prioritization document for keeping the project coherent as the Rust CLI grows.

## Positioning

The project should continue to position itself as:

- a Grafana migration, inspection, diff, and governance CLI
- a practical bridge between imperative Grafana operations and safer reviewable workflows
- a local-first operator tool that favors explicit plans, preflight checks, and fail-closed behavior

The project should not drift into:

- a generic all-in-one Grafana platform
- a full replacement for Terraform or native provisioning
- a browser-heavy product that eclipses the CLI's core operator value
- a SaaS-style control plane

## Planning Principles

- Prefer work that deepens the current Rust operator workflows instead of widening scope indiscriminately.
- Treat migration safety, dry-run accuracy, and preflight clarity as first-class requirements.
- Prefer typed contracts and reviewable documents over implicit mutation logic.
- Add higher-level automation only when it reduces real operator toil.
- Keep architecture pressure visible: complex domains should be split before they become maintenance bottlenecks.

## Current State

Current strengths:

- The Rust CLI already exposes meaningful `dashboard`, `alert`, `access`, `datasource`, and `sync` surfaces.
- Export, import, diff, inspect, preflight, and staged review flows already form a credible operator toolkit.
- Dashboard inspection and query-analysis features are already stronger than a basic backup/restore utility.
- The staged `sync` workflow has a strong product direction: explicit plan, review, and apply intent rather than blind mutation.
- Test coverage is broad across CLI topology, contract behavior, and domain workflows.

Current constraints:

- `dashboard` and `sync` remain the main complexity centers.
- Several large orchestration and contract modules will keep absorbing behavior unless they are actively split.
- Environment-to-environment promotion still lacks a first-class operator workflow.
- Datasource secret handling is intentionally conservative and still blocks broader automation adoption.
- The project has strong primitives, but some higher-level flows still feel like connected building blocks rather than one polished operator path.

## Next-Phase Strategy

The next phase should focus on going deeper, not broader.

The priority is to make the existing differentiators more complete and more trustworthy:

1. inspection and dependency visibility
2. safer environment promotion
3. more reviewable and trustworthy sync flows
4. explicit secret-reference handling
5. ongoing architecture simplification in high-complexity Rust modules

## Roadmap Overview

### Phase 1: Deepen Inspection And Dependency Governance

Status:

- largely landed, with targeted output gaps still open

Target outcome:

- operators can answer dependency, blast-radius, governance, and dashboard quality questions directly from built-in CLI outputs

Priority items:

- deepen datasource usage, orphan detection, and stale-reference reporting
- add first-class dependency graph export such as `graph --output-format dot|svg|json`
- strengthen datasource-family analyzers so Prometheus, Loki, Flux/Influx, SQL, and future families stay independently evolvable
- add management-friendly outputs such as static HTML inspection reports when they reuse the canonical report model
- add stronger governance and quality signals such as risky query traits, oversized dashboards, and variable-chain risk summaries

Why this phase comes first:

- inspection is already one of the project's strongest differentiators
- stronger dependency and governance reporting improves operator value without expanding product scope

Definition of done for this phase:

- operators can see dashboard-to-datasource dependency summaries and common blast-radius views without custom scripts
- richer reports reuse the same underlying inspection model instead of creating parallel ad hoc pipelines
- dependency and governance outputs are strong enough to support review meetings, migration planning, and cleanup decisions

Explicit non-goals for this phase:

- no attempt to parse every possible query language exhaustively
- no separate web product or always-on service layer

### Phase 2: Add First-Class Environment Promotion And Preflight Safety

Status:

- partially landed; staged preflight and review handoff exist, but controlled
  promotion continuation does not

Target outcome:

- promoting dashboards, alerts, and datasource state between environments becomes safer, more explicit, and less manual

Priority items:

- add a first-class promotion workflow around the existing export/import/diff foundations
- improve UID, folder, datasource, and naming remap support for cross-environment migration
- expand import preflight for datasources, plugins, alert/contact references, library panels, and other common prerequisites
- keep dry-run and diff outputs trustworthy enough for peer review before mutation
- evaluate bundle/package workflows that snapshot dashboards, alerts, datasource inventory, and migration metadata together when that improves portability

Why this phase matters:

- environment promotion is one of the highest-value workflows after export/import exists
- better preflight prevents bad writes before they reach production Grafana

Definition of done for this phase:

- operators can promote common Grafana resources between environments with explicit rewrite and preflight visibility
- promotion flows can detect common blockers before mutation starts
- cross-environment remapping rules are documented, reviewable, and testable

Explicit non-goals for this phase:

- no general deployment orchestrator
- no attempt to own every environment-management concern outside Grafana resource migration

### Phase 3: Make Sync More Trustworthy Before Making It Broader

Status:

- partially landed; staged review/apply/preflight exists, but the model remains
  intentionally constrained and still needs deeper trust surfaces

Target outcome:

- the `grafana-util sync` workflow is reviewable, predictable, and trusted enough for constrained Git-managed operations

Priority items:

- strengthen fail-closed handling for ambiguous or unsupported resource states
- improve plan explainability so blocked, candidate, and applyable states are easier to understand
- deepen preflight and audit coverage before expanding resource scope
- keep sync semantics compatible with existing normalized export formats where practical
- improve review surfaces so operators can validate intent before any live mutation path is enabled

Why this phase comes after promotion:

- sync becomes strategically valuable only when the lower-level migration and preflight primitives are already trustworthy
- broadening sync too early would increase scope faster than operator confidence

Definition of done for this phase:

- operators can declare a constrained subset of Grafana state in versioned files and reconcile drift through one explicit workflow
- sync plans fail closed on unsupported ambiguity and explain why
- apply flows remain review-first instead of silently becoming an imperative mutation shortcut

Explicit non-goals for this phase:

- no claim to replace full Terraform-style resource management
- no always-reconcile controller or daemon

### Phase 4: Formalize Secret Handling For Datasource Automation

Status:

- partially landed; placeholder planning, dry-run visibility, and staged
  blockers exist, but provider-backed execution and fuller review coherence do
  not

Target outcome:

- datasource workflows can reference secrets safely enough for real environment promotion and sync usage without weakening explicit operator control

Priority items:

- formalize placeholder-based secret references in reviewed config, bundle, or staged inputs
- evaluate external secret-provider integration on top of the staged contract
- preserve the current fail-closed behavior for unsupported secret-bearing mutations
- make secret-loss and secret-missing cases explicit in preflight and apply intent outputs

Why this phase matters:

- secret handling remains one of the main blockers for safer datasource lifecycle automation
- promotion and sync become materially more useful once secret references are explicit and reviewable

Definition of done for this phase:

- datasource workflows can reference secrets without encouraging plaintext storage in exported artifacts
- unsafe secret-loss cases remain explicit and blocked by default
- the project keeps a narrow and auditable secret contract instead of passing opaque blobs through exports

Explicit non-goals for this phase:

- no secret-management abstraction that hides risk behind silent magic
- no promise to round-trip every datasource vendor's secure settings automatically

### Phase 5: Keep Advanced Analysis And Packaging Exploratory

Status:

- mostly open, with only limited rule-assisted analysis already visible in the
  existing governance path

Target outcome:

- the project can explore higher-level analysis surfaces without making them part of the core correctness path

Priority items:

- evaluate optional AI-assisted or rule-assisted query review only on top of existing analyzer outputs
- keep any recommendations explainable and optional rather than silently mutating resources
- explore Rust-to-WASM or local browser packaging only if it cleanly reuses the Rust analysis core

Why this phase is later:

- these ideas are additive multipliers, not current blockers
- the core workflow layers should be stronger before optional higher-level extensions are expanded

Definition of done for this phase:

- assisted analysis remains optional and grounded in existing report models
- any local packaging path reuses the Rust core instead of forking business logic

Explicit non-goals for this phase:

- no dependency on an online AI service for baseline CLI correctness
- no SaaS direction

## Cross-Cutting Work

These items should continue across all phases:

- keep CI aligned with local quality commands and release gates
- reduce oversized Rust orchestration modules before they become default dumping grounds
- keep CLI help, output contracts, and maintainer docs aligned with real behavior
- add tests for every user-visible workflow change, especially contract and preflight behavior
- keep domain outputs typed and reviewable instead of relying on implicit CLI conventions
- resist scope creep that does not reinforce migration, inspection, diff, promotion, sync, or governance value

## Recommended Priority Order Right Now

If only a small number of items can advance next, the recommended order is:

1. deepen inspection and dependency reporting
2. add first-class promotion and stronger preflight checks
3. improve sync trustworthiness before broadening scope
4. formalize secret-reference handling for datasource workflows
5. keep advanced analysis and alternate packaging exploratory

## Success Metrics

The roadmap is working if these become true:

- dashboard and alert migrations require less manual repair between environments
- inspection outputs replace one-off operator scripts for common governance and blast-radius questions
- promotion and sync workflows stay reviewable and fail closed when target state is ambiguous
- datasource secret handling is safer without weakening explicit operator control
- high-complexity Rust modules are gradually split instead of expanding without bound

## Bottom Line

The project should grow by going deeper on its strongest operator use cases, then carefully layering higher-level automation on top.

The best direction for the next phase is:

- stronger inspection and dependency visibility
- safer environment promotion
- more trustworthy review-first sync workflows
- clearer secret handling
- sustained Rust architecture simplification
