# Maintainer Backlog

Date: 2026-03-29
Scope: Rust runtime only
Audience: Maintainers
Status: Current follow-up backlog derived from the current architecture review, roadmap, and AI trace docs

## Purpose

This file turns the current architecture and roadmap direction into a short working backlog.

It is intentionally narrower than `project-roadmap.md` and more action-oriented than `architecture-review-2026-03-27.md`.

## Phase Progress

- Phase 1 landed: dashboard inspect boundary cleanup and the current import/inspect TUI splits are in place.
- Phase 1 partially landed: dashboard import remains a review-first workbench with room for more boundary cleanup.
- Phase 2 partially landed: promotion is now a staged review handoff rather than just a skeleton.
- Phase 2 partially landed: datasource secret handling has a usable operator contract, but provider-aware follow-through is still open.

## Now

### 1. Keep landed domain producers stable

- Treat `dashboard`, `datasource`, `alert`, `access`, `sync`, and `promotion` as stop-for-now by default.
- Reopen a producer lane only when a concrete consumer proves a missing decision-critical signal.

### 2. Preserve sync and promotion contract discipline if reopened

- Keep staged document ownership explicit.
- Keep promotion contract additions attached to promotion modules.
- Avoid adding new behavior directly into sync orchestration facades unless ownership is already clear.

### 3. Keep consumers thin

- Keep `overview` as the staged artifact aggregator/browser.
- Keep `project-status` as the canonical project-wide status surface.
- Do not move domain semantics back into shared consumers.

### 4. Keep maintainer docs current

- Keep roadmap, gap-list, and execution notes aligned with current behavior.
- Prefer one current answer over multiple partially stale queue documents.

## Next

### 1. Reopen only consumer-proven gaps

- Prefer a concrete missing decision signal over speculative follow-up depth.
- Keep fixes inside the owning domain producer.

### 2. Keep trust and review semantics explicit

- Preserve staged/live ownership if sync or promotion must be reopened.
- Preserve review/apply and promotion-module seams.

## Later

### 1. Keep advanced analysis and packaging exploratory

- Grow optional AI-assisted or rule-assisted analysis only on top of existing report models.
- Keep any local packaging path reusing the Rust core instead of forking business logic.

## Order Of Execution

If only a few slices move next, the recommended order is:

1. keep current producers and consumers stable
2. preserve sync and promotion contract discipline if reopened
3. keep roadmap/docs/current notes aligned
4. treat any new deepening work as consumer-proven exceptions

## Non-Goals For This Backlog

- no Python parity work
- no attempt to replace Terraform or Grafana provisioning
- no SaaS or controller-style expansion
- no broadening of `sync` resource scope before trust and review surfaces are stronger
