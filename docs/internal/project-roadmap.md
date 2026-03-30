# Project Roadmap

Date: 2026-03-29
Scope: Rust runtime roadmap for the maintained `grafana-util` CLI. Python notes remain supporting context only.
Source: Derived from current repository state, maintainer docs, and product-direction review.

## Purpose

This roadmap defines what should come next for the Rust mainline and what should stay out of scope.

It is a prioritization document, not a raw feature backlog.

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

- Deepen the current Rust operator workflows instead of widening scope indiscriminately.
- Treat migration safety, dry-run accuracy, and preflight clarity as first-class requirements.
- Prefer typed contracts and reviewable documents over implicit mutation logic.
- Split complex Rust modules before they become maintenance bottlenecks.

## Current State

- The Rust CLI is the maintained mainline and already exposes `dashboard`, `alert`, `access`, `datasource`, and `sync` surfaces.
- Dashboard inspection/import, sync promotion, and datasource secret handling are the strongest active workflow areas.
- The shared Rust TUI shell grammar now keeps the main console surfaces visually aligned.
- The remaining work is mostly about trust, boundary cleanup, and making existing flows easier to review.

## Near-Term Focus

The next phase should stay narrower, not broader:

1. keep the landed Rust mainline stable and readable
2. reopen a domain lane only when a concrete consumer proves a missing decision-critical signal
3. keep sync/promotion review-first and document-driven if reopened
4. keep `overview` / `project-status` practical but thin
5. treat docs/help/TUI clarity as the default next work when no domain lane is justified

## Roadmap Overview

### Inspection And Dependency Governance

Status:

- landed enough for the current pass; targeted output gaps remain follow-up only

Target outcome:

- operators can answer dependency, blast-radius, governance, and dashboard quality questions directly from built-in CLI outputs

Current posture:

- keep the current dashboard and datasource producers stable
- reopen this lane only if a real downstream consumer proves the current staged/live status is missing decision-critical evidence
- avoid turning follow-up output polish into another active roadmap lane

### Environment Promotion And Preflight Safety

Status:

- landed enough for the current pass; promotion remains review-first and explicit, with bounded follow-up only

Target outcome:

- promoting dashboards, alerts, and datasource state between environments becomes safer, more explicit, and less manual

Current posture:

- keep promotion status attached to promotion modules
- reopen this lane only if a concrete review/apply consumer is missing a decision-critical signal
- avoid broadening promotion into a second orchestration layer

### Sync Trust And Secret Handling

Status:

- landed enough for the current pass; staged/live trust surfaces exist and remaining depth is bounded follow-up only

Target outcome:

- the `grafana-util sync` workflow is reviewable, predictable, and trusted enough for constrained Git-managed operations

Current posture:

- preserve review-first staged/live ownership
- reopen this lane only if a concrete operator path still lacks decision-critical trust evidence
- do not broaden sync scope before a concrete consumer forces it

### Exploratory Analysis And Packaging

Status:

- mostly open, with only limited rule-assisted analysis already visible in the existing governance path

Target outcome:

- the project can explore higher-level analysis surfaces without making them part of the core correctness path

Priority items:

- evaluate optional AI-assisted or rule-assisted query review only on top of existing analyzer outputs
- keep any recommendations explainable and optional rather than silently mutating resources
- explore Rust-to-WASM or local browser packaging only if it cleanly reuses the Rust analysis core

## Cross-Cutting Work

- keep CI aligned with local quality commands and release gates
- keep CLI help, output contracts, and maintainer docs aligned with real behavior
- add tests for every user-visible workflow change, especially contract and preflight behavior
- keep domain outputs typed and reviewable instead of relying on implicit CLI conventions
- resist scope creep that does not reinforce migration, inspection, diff, promotion, sync, or governance value
- protect the current owner boundaries so `overview` and `project-status` stay thin consumers

## Recommended Priority Order Right Now

If only a small number of items can advance next, the recommended order is:

1. keep current domain-owned producers stable
2. reopen only consumer-proven missing signals
3. keep `overview` / `project-status` presentation-only and contract-driven
4. keep advanced analysis and alternate packaging exploratory
5. spend unclaimed effort on clarity, contract protection, and maintainability rather than new depth

## Success Metrics

The roadmap is working if these become true:

- operators can see dependency and governance answers without external scripts
- promotion flows can fail closed before mutation when prerequisites are missing
- sync plans explain their blockers instead of hiding them behind generic status text
- datasource secret references remain explicit, reviewable, and auditable
- high-complexity Rust modules are gradually split instead of expanding without bound
- maintainers can explain the current active lane policy without reopening closed producer lanes by default
