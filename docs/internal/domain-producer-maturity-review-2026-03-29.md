# Domain Producer Maturity Review

Date: 2026-03-29
Scope: Rust mainline only
Audience: Maintainers

This note stays current and bounded for the current pass:

- which domain-owned producers are already stable enough to stop deepening
- which domains still justify one more bounded round
- what practical role `overview` and `project-status` should keep

Use this with:

- `docs/internal/project-status-producer-gap-list.md`
- `docs/internal/next-phase-execution-plan-2026-03-29.md`
- `docs/internal/project-status-architecture.md`

## Outcome Key

- `stop`
  - good enough for the original product intent; do not keep polishing now
- `not-worth-digging-now`
  - possible depth exists, but it should not be a current priority

## Domain Outcomes

### Dashboard

Recommendation: `stop`

Why it is already good enough:

- the staged producer is already the real dashboard status contract
- it carries blocker rows, governance warnings, and bounded import-readiness evidence
- the live path stays cheap and conservative instead of growing more fetch-heavy heuristics

Residual gap:

- richer governance exemplars
- stronger import-readiness evidence

Why it should stop now:

- the current staged/live producers already answer the original inspection and readiness questions well enough
- further work here is now more likely to improve evidence polish than to change operator decisions

What not to do:

- do not move dashboard status logic back into `overview`
- do not add another dashboard summary layer

### Datasource

Recommendation: `stop`

Why it is already good enough:

- the staged/live split is stable and source-attributable
- staged status already covers inventory, defaults, diff/import-preview, and secret-reference-adjacent signals
- live status already covers bounded inventory, metadata, org-scope, and provider/secret surfaces

Residual gap:

- sharper diff/drift severity
- secret-reference readiness
- mutation/import readiness

Why it should stop now:

- the current staged/live split already answers the main export/import/sync trust questions
- further work here is now more likely to sharpen wording and evidence density than to change the go/no-go path

What not to do:

- do not create a second datasource summary surface
- do not pretend live warnings are full provider resolution

### Alert

Recommendation: `stop`

Why it is already good enough:

- staged export-summary status already exists
- live status already covers rule-linkage and policy gating conservatively
- blocked-by-blockers handling and basic coverage warnings are enough for the current product intent

Residual gap:

- mostly thin consumer presentation consistency, not more alert-owned derivation

Why it should stop now:

- the remaining work is small enough to treat as stop-if-done after the bounded pass
- more alert polishing would mostly add detail without changing decisions

What not to do:

- do not re-derive alert readiness outside the alert-owned producer path
- do not turn alert into a second summary surface

### Access

Recommendation: `stop`

Why it is already good enough:

- staged bundle presence/counts are already usable
- live list-surface status, partial handling, and review/drift signals already cover the bounded operator intent
- unreadable-scope handling and fallback behavior are already in place

Residual gap:

- org-family drift/review coverage is still thin

Why it should stop now:

- that remaining gap is smaller than the still-open work in dashboard, datasource, alert, sync, and promotion
- more access polishing would likely become diminishing-return detail work

### Sync

Recommendation: `stop`

Why it is already good enough:

- it already has the strongest staged document family
- staged and live rows are explicit and conservative
- blocker/warning reporting and next-action guidance are already operator-usable

Residual gap:

- `plan` and `audit` are not yet first-class in sync status
- apply-readiness vs review-readiness still needs stronger separation
- live sync still leans mainly on staged summary + bundle-preflight handoff

Why it should stop now:

- the producer now preserves bounded provider, placeholder, and alert-artifact review evidence instead of dropping it
- the remaining work is evidence-density polish, not a missing go/no-go path

What not to do:

- do not widen sync facades
- do not blur staged and live readiness into one heuristic

### Promotion

Recommendation: `stop`

Why it is already good enough:

- staged preflight status is already stable and explicit
- handoff and apply-continuation warnings already keep promotion review-first
- blocker and next-action output are already useful

Residual gap:

- resolved remap inventory
- stronger apply-continuation state
- richer live evidence beyond summary/mapping/availability handoff

Why it should stop now:

- handoff and continuation evidence now stay explicit enough for the current review-first intent
- the remaining work is better treated as follow-up only if a concrete promotion consumer is blocked

What not to do:

- do not turn promotion into a separate orchestration layer
- do not move promotion semantics back into shared consumers

## Consumer Boundaries

### `overview`

Keep it as:

- staged artifact aggregator
- staged overview document builder
- thin staged text / JSON / TUI browser

Do not expand it into:

- live-status owner
- reusable project-status architecture
- workflow-policy owner

### `project-status`

Keep it as:

- canonical project-wide status surface
- staged/live split
- thin top-level rendering and aggregation over the shared contract

Do not expand it into:

- domain-specific workflow owner
- another analysis engine
- another overview-like surface

## Only Consumer-Level Work Still Worth Doing

- presentation-only project-home / handoff polish that stays contract-driven
- clearer display of existing `scope`, `overall`, `domains`, `topBlockers`, `nextActions`, and `freshness`

Do not prioritize:

- more derivation logic in consumers
- another consumer surface
- another top-level status command

## Current Execution Choice

For this pass, keep no domain depth lane active by default.

Treat all six domain-owned producers as stop-for-now, and keep `overview` /
`project-status` thin.
