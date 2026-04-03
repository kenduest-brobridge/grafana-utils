# Project Roadmap

Date: 2026-03-15
Source: Derived from `docs/internal/project-value-assessment.md`, `TODO.md`, and current repo state.

## Purpose

This roadmap exists to keep the project moving in a coherent direction.

It is not a raw backlog dump. It is a prioritization document for deciding what to do next, what to defer, and what outcomes should define success.

## Positioning

The project should continue to position itself as:

- a Grafana migration, inspection, diff, and governance CLI
- a practical bridge between imperative Grafana operations and safer reviewable workflows

The project should not drift into:

- a generic all-in-one Grafana platform
- a full replacement for Terraform or native provisioning
- a browser-heavy product that eclipses the CLI's core operator value

## Planning Principles

- Prefer features that improve migration safety, inspection depth, and governance value.
- Prefer work that compounds the current Python + Rust architecture instead of fragmenting it.
- Keep Python and Rust behavior aligned through shared contracts, fixtures, and docs where the public workflow is shared.
- Treat secret handling, dry-run accuracy, and explicit preflight checks as first-class safety requirements.
- Add higher-level workflows only when they clearly reduce manual Grafana maintenance for real operators.

## Current State

Current strengths:

- Python and Rust both expose meaningful dashboard, datasource, alert, and access command surfaces.
- export / import / diff / inspect workflows already exist and are useful in real operator scenarios.
- inspection and query-analysis capabilities already provide more governance value than a basic backup/restore tool.
- datasource import/export now has an explicit normalized contract rather than loose best-effort replay.

Current constraints:

- dashboard and inspection workflows are still the main complexity center.
- Python and Rust dual maintenance increases coordination cost.
- environment-to-environment promotion still needs more explicit workflow support.
- datasource secret handling is intentionally conservative and does not yet integrate with external secret stores.

## Roadmap Overview

### Phase 1: Finish Inspection And Dependency Governance

Target outcome:

- operators can answer dependency, blast-radius, and query-governance questions directly from CLI output without custom scripts

Priority items:

- reduce Python/Rust inspect-live and inspect-export drift by keeping one stable summary/report schema
- deepen datasource usage and orphan-detection reporting
- add a first-class resource dependency graph export such as `graph --output-format dot|svg|json`
- refactor query extraction behind datasource-type-specific analyzers so Prometheus, Loki, Flux/Influx, SQL, and future families can evolve independently
- add management-friendly report rendering such as static HTML inspection output when it reuses the same canonical report model

Why this phase comes first:

- inspection is already one of the repo's strongest differentiators
- dependency visibility and governance reporting directly strengthen the current value proposition without forcing a broader platform scope

Definition of done for this phase:

- operators can see dashboard-to-datasource dependency summaries and common blast radius views from built-in commands
- report modes stay aligned across Python and Rust where they represent the same public contract
- richer report outputs reuse the same underlying inspection data instead of creating parallel ad hoc pipelines

Explicit non-goals for this phase:

- no attempt to parse every possible query language exhaustively
- no separate web product or always-on service layer

### Phase 2: Add Environment Promotion And Preflight Safety

Target outcome:

- promoting dashboards, alerts, and datasource state across environments becomes safer, more explicit, and less manual

Priority items:

- add a first-class promotion workflow such as `promote --from <env> --to <env>` around the existing export/import/diff foundations
- expand import preflight for datasources, plugins, alert/contact references, library panels, and other common prerequisites
- improve UID/name remapping support for datasource and dashboard promotion cases
- keep dry-run and diff outputs trustworthy enough for team review before mutation
- evaluate bundle/package workflows that snapshot dashboards, alerts, datasource inventory, and metadata together when that clearly improves portability

Why this phase matters:

- environment promotion is one of the most common high-value operator workflows after basic export/import exists
- better preflight prevents bad writes before they reach production Grafana

Definition of done for this phase:

- operators can promote common Grafana resources between environments with explicit rewrite/preflight visibility
- promotion flows can detect common blockers before mutation starts
- cross-environment remapping rules are documented and testable rather than hidden in ad hoc scripts

Explicit non-goals for this phase:

- no general deployment orchestrator
- no attempt to own every environment-management concern outside Grafana resource migration

Competitor-informed direction:

`grafana-backup-tool` is a useful reference for backup productization, but it should inform this phase selectively rather than redefine the repo as a generic backup utility.

Necessary or strong-reference additions:

- keep investing in cross-environment datasource and dashboard remap support because backup-style raw restore alone does not solve target-environment datasource selection
- evaluate one reviewable bundle/package format that groups dashboards, alerts, datasources, and metadata into one portable artifact while preserving current dry-run and diff guardrails
- make promotion/preflight outputs easy to use from scheduled jobs and CI so operators can treat migration bundles as operational artifacts rather than ad hoc export folders
- add stronger bundle metadata and validation so exported state can be verified before restore or promotion starts

Possible directions, but not required:

- add a dedicated `backup` or `restore` command family only if it clearly reuses the existing export/import/diff contracts instead of creating a second parallel workflow model
- add remote artifact storage targets such as S3, GCS, or Azure Blob only after the local bundle contract is stable
- add retention-oriented backup lifecycle features only if the project intentionally chooses to serve recurring disaster-recovery jobs in addition to migration/governance workflows
- add extra archival coverage for resources such as library elements, annotations, snapshots, or dashboard version history only when they materially improve migration or review value

Guardrail:

- do not trade away multi-org routing, dry-run trustworthiness, or reviewable remap/preflight behavior just to mimic a simpler save/restore backup experience

### Phase 3: Introduce GitOps-Oriented Declarative Sync

Target outcome:

- the repo supports a constrained declarative sync workflow that uses Git-managed state as the review surface while preserving the project's safety-first CLI behavior

Priority items:

- expand the shipped `grafana-util sync` surface beyond the current staged/local-first baseline
- define a narrow supported state model for dashboards, datasources, folders, and selected alert resources
- require reviewable plan/dry-run output before live mutation
- decide which live fetch and live apply bridges belong in the stable sync contract versus staying explicitly limited
- keep sync semantics compatible with existing normalized export formats wherever practical
- document where declarative sync complements rather than replaces Terraform/native Grafana provisioning

Why this phase is later:

- this is strategically valuable, but it should be built on top of already-safe promotion and preflight primitives
- a GitOps surface without clear constraints would risk turning the repo into a vague platform

Definition of done for this phase:

- operators can use one explicit sync workflow for a supported subset of Grafana state without dropping back to ad hoc scripts
- sync results are reviewable, predictable, and fail closed on unsupported ambiguity
- the feature reuses existing contracts instead of inventing a second incompatible resource model

Explicit non-goals for this phase:

- no claim to replace full Terraform-style resource management
- no broad always-reconcile controller or daemon

### Phase 4: Strengthen Secret Handling For Datasource And Access Workflows

Target outcome:

- sensitive values are handled more safely during datasource and access operations without weakening the repo's explicit operator controls

Priority items:

- evaluate external secret provider integration for datasource import workflows
- support placeholder-based secret references in reviewed config or bundle inputs
- preserve the current fail-closed behavior for unsupported secret-bearing datasource mutations
- keep password/token file and prompt-based flows aligned across Python and Rust

Why this phase matters:

- secret handling remains one of the main blockers for safer datasource lifecycle automation
- better secret injection makes promotion and declarative sync materially more usable in real environments

Definition of done for this phase:

- datasource workflows can reference secrets without encouraging plaintext storage in exported artifacts
- unsafe secret-loss cases remain explicit and blocked by default
- the repo still keeps a narrow, auditable secret contract instead of smuggling opaque blobs through exports

Explicit non-goals for this phase:

- no secret-management abstraction that hides risk behind silent magic
- no promise to round-trip every datasource vendor's secure settings automatically

### Phase 5: Explore Assisted Analysis And Local Runtime Extensions

Target outcome:

- the project can optionally offer higher-level analysis surfaces without making them a core requirement for safe CLI use

Priority items:

- evaluate AI-assisted query analysis or `inspect --ai-fix` style suggestions only on top of the existing analyzer outputs
- keep any assisted recommendations explainable and optional rather than silently mutating resources
- explore Rust-to-WASM packaging only if it cleanly reuses the existing Rust analysis core and stays local/offline-first

Why this phase is last:

- these ideas can be valuable, but they are additive multipliers rather than core blockers
- the foundation should first be strong in deterministic migration, governance, promotion, and secret safety

Definition of done for this phase:

- assisted analysis is strictly optional and grounded in existing report models
- any local browser/WASM packaging reuses the Rust core instead of forking logic into a separate implementation

Explicit non-goals for this phase:

- no dependency on an online AI service for baseline CLI correctness
- no separate SaaS direction

## Cross-Cutting Work

These items should continue across phases instead of waiting for one specific milestone:

- keep CI aligned with local quality commands
- keep Python and Rust help text, contracts, and fixtures synchronized
- reduce oversized orchestration modules before they become the default place for every new feature
- update maintainer docs when behavior or architecture changes materially
- resist scope creep that does not reinforce migration, inspection, diff, promotion, or governance value

## Backup Roadmap Guidance

When considering backup-oriented work inspired by `grafana-backup-tool`, use this filter:

- necessary or strong-reference work should strengthen portable bundles, preflight validation, remap support, scheduled-job ergonomics, and artifact verification
- optional work may include cloud backup destinations, retention policies, extra archival resource types, or a dedicated `backup/restore` UX layer
- reject work that mostly increases backup marketing surface area without improving the repo's core migration, diff, governance, or safety model

## Priority Order Right Now

If only a small number of items can be advanced next, the recommended order is:

1. finish inspection/dependency reporting and shared report contracts
2. add environment promotion and stronger preflight checks
3. design a constrained GitOps/declarative sync workflow
4. integrate safer secret-reference handling for datasource workflows
5. keep assisted analysis and WASM packaging exploratory until the core workflow layers are stable

## Success Metrics

The roadmap is working if these become true:

- dashboard and alert migrations require less manual repair between environments
- inspection reports replace one-off operator scripts for common governance and blast-radius questions
- promotion and sync workflows stay reviewable and fail closed when the target state is ambiguous
- datasource secret handling is safer without weakening explicit operator control
- Python and Rust stay aligned without frequent parity regressions

## Bottom Line

The project should grow by going deeper on its strongest use cases, then carefully layering higher-level automation on top.

The best direction is:

- stronger inspection and dependency visibility
- safer environment promotion
- constrained GitOps-style reconciliation
- clearer secret handling
- stable cross-language behavior
