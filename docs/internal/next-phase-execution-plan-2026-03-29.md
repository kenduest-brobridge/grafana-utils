# Next Phase Execution Plan

Date: 2026-03-29
Scope: Rust mainline only
Audience: Maintainers
Status: Current execution arrangement for the bounded producer-deepening pass

This file turns the current architecture, status-producer map, and backlog into
one short current-phase execution arrangement for the bounded mainline
follow-through pass.

It exists to keep the project on the original tool path:

- inventory and inspection
- live visibility
- output for later analysis
- export / import / sync / promotion workflows

It is not a product roadmap or a new architecture branch. It is the current
"what should move next" note for this pass.

## Design Guardrails

- Keep domain-owned staged/live status producers as the mainline contract.
- Keep `project-status` and `overview` as thin consumers of shared status, not
  owners of workflow rules.
- Keep sync and promotion review-first, staged-document-driven, and explicit.
- Prefer additive deepening of owned evidence over new top-level surfaces.
- Avoid broadening resource scope or introducing a second summary layer.

## Mainline Vs Bounded Follow-Up

### Mainline

These are now the real Rust mainline, not experiments:

- domain-owned staged status producers
- first-pass domain-owned live status producers
- shared `project-status` contract
- `project-status staged` / `project-status live`
- staged `overview` as a project-wide artifact aggregator and browser

### Bounded Follow-Up

These are important, but they are still bounded mainline follow-through inside
existing owners, not a new architecture branch:

- richer sync plan/audit/apply-readiness evidence
- richer promotion handoff/apply state
- broader real-source timestamp freshness
- thin consumer presentation consistency, if needed

## Priority Order

### 1. Dashboard

Dashboard is now stop-for-now unless a concrete consumer proves a missing
signal.

Why:

- it is still the largest feature surface
- it already has the strongest inspect/governance/dependency model
- its domain producer now exists and should be deepened instead of bypassed

Residual gap:

- richer governance exemplars
- stronger import-readiness evidence

Why it is not the active lane now:

- the current staged/live producers already answer the original inspection/import-readiness questions well enough for this pass

Do not:

- move dashboard status logic back into `overview`
- re-centralize dashboard TUI/import/inspect into one hub module

### 2. Datasource

Datasource is now stop-for-now unless a concrete consumer proves a missing
signal.

Why:

- the producer is already landed
- the repo already owns provider/placeholder and diff inputs
- this work is directly tied to export/import/sync trust

Residual gap:

- diff/drift severity
- secret-reference readiness
- mutation/import readiness

Why it is not the active lane now:

- the current staged/live producers already answer the main export/import/sync trust questions well enough for this pass

Do not:

- create a second datasource summary surface
- pretend live producer warnings are provider resolution

### 3. Sync And Promotion Trust Chain

Treat this as closed for the current pass unless a concrete consumer proves a
missing decision-critical signal.

Why:

- sync/promotion is still the highest-value review/apply trust path
- the remaining gap is evidence depth, not missing architecture

Residual follow-up only:

- `plan` and `audit` evidence could still become first-class in sync status
- apply gating should stay document-driven if reopened
- promotion could still grow stronger handoff/apply state if reopened

Do not:

- widen sync orchestration facades
- blur staged and live readiness into one heuristic
- broaden sync resource scope before trust surfaces are stronger

### 4. Alert

Alert is now stop-if-done after the bounded pass. Keep `project-status` /
`overview` thin, and do not reopen alert as a depth lane.

Do not:

- re-derive alert readiness outside the alert-owned producer path
- add another alert summary surface

## `overview` And `project-status`

Both are practical and should stay.

### `overview`

`overview` should stay the staged artifact aggregator and browser.

It should own:

- staged input loading
- staged overview document assembly
- staged text / JSON / TUI projection

It should not own:

- reusable project-status architecture
- live-status logic
- domain workflow policy
- alert-specific derivation

### `project-status`

`project-status` should stay the project-wide status surface.

It should own:

- staged/live mode split
- top-level status rendering
- aggregation handoff across the shared contract

It should not own:

- domain-specific workflow semantics
- a parallel analysis engine
- logic that belongs in dashboard/datasource/alert/access/sync/promotion

### Short-Term Arrangement

Keep the current arrangement:

- staged `project-status` can continue to reuse `overview` artifact/document
  assembly
- live `project-status` continues to consume domain-owned live producers

New work should go into domain producers and the shared contract first, not
into making either `overview` or `project-status` bigger.

## Not Now

Do not prioritize these next:

- new top-level status commands
- another overview-like surface
- broad cross-resource redesign
- Python parity work
- controller/SaaS-style expansion
- sync resource-scope expansion before review/apply trust is stronger

## Current Bounded Follow-Through

No domain-owned depth lane remains active by default for this pass.

- keep all current domain-owned producers stable
- reopen a lane only if a concrete consumer proves a missing decision-critical signal

Do not reopen `dashboard`, `datasource`, `alert`, `access`, `sync`, or
`promotion` unless a real consumer gap proves the current producers are
missing decision-critical evidence.
