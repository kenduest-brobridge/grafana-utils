# Project Status Producer Gap List

Date: 2026-03-29
Scope: Rust mainline only
Audience: Maintainers

This file turns the current project-status mainline into an execution map for the bounded mainline follow-through pass.

No domain slice is now treated as the remaining bounded mainline follow-through
by default. Dashboard, datasource, alert, access, sync, and promotion are all
stop-for-now unless a concrete consumer gets blocked.

It answers:

- which domain status-like producers already exist
- what is still missing before the project supports real project-wide progress
- what the shortest safe next step is for each domain

Use this with `docs/internal/project-status-architecture.md`.

## Status Key

- `landed`
  - a stable or mostly stable status-producing surface already exists
- `partial`
  - useful summary or preflight surfaces exist, but they do not yet form a
    proper domain status producer
- `missing`
  - no usable domain status producer exists yet for project-wide aggregation

## Dashboard

### Current Producer State

Status: `landed` for staged status, `landed` for bounded live inspection/readiness status, `partial` for governance/import layering

Current usable sources:

- inspect summary document
- dependency and governance outputs
- governance gate result
- import dry-run and interactive review surfaces

Strongest current files:

- `rust/src/dashboard/inspect_summary.rs`
- `rust/src/dashboard/inspect_report.rs`
- `rust/src/dashboard/inspect_governance*.rs`
- `rust/src/dashboard_inspection_dependency_contract.rs`
- `rust/src/dashboard/governance_gate*.rs`

### What Already Works

- dashboard inspection can already expose query count, datasource usage,
  orphaned datasource count, mixed-datasource dashboards, governance findings,
  dependency usage, and blast-radius signals
- governance gate can already turn some inspect signals into enforceable policy

### Residual Gap

The staged producer and first live producer are landed, and the live path now carries a bounded import/dry-run readiness follow-up. Remaining depth is:

- richer governance exemplars
- import readiness

### Current Decision

Stop for now. Only reopen `dashboard/project_status.rs` if a real consumer
later proves the current producers are missing decision-critical staged or live
evidence.

## Datasource

### Current Producer State

Status: `landed` for staged inventory status, `landed` for deeper live inventory/readiness status, `partial` for diff/mutation layering

Current usable sources:

- export inventory root
- diff report
- import dry-run secret visibility
- provider and placeholder assessments used by sync/bundle-preflight
- live provider/placeholder readiness checks from the existing project-status live path

Strongest current files:

- `rust/src/datasource_export_support.rs`
- `rust/src/datasource_diff.rs`
- `rust/src/datasource_import_export.rs`
- `rust/src/datasource_secret.rs`
- `rust/src/datasource_provider.rs`

### What Already Works

- datasource export already has a stable inventory shape
- diff already classifies compare status
- secret placeholder handling is fail-closed and reviewable

### Residual Gap

The staged producer now covers inventory readiness, defaults, and stable source attribution. The live producer now adds bounded provider/placeholder readiness in the existing project-status path. Remaining depth is:

- secret-reference readiness
- diff/drift severity
- mutation/import readiness

### Current Decision

Stop for now. Only reopen the datasource producer path if a real consumer later
proves the current staged/live rows are missing decision-critical trust
evidence.

## Alert

### Current Producer State

Status: `landed` for staged export-summary status, `landed` for deeper live surface status, `stop-for-now` for further producer deepening

Current usable sources:

- export root index
- compare and diff documents
- import dry-run document
- bundle alert contract participation

Strongest current files:

- `rust/src/alert.rs`
- `rust/src/sync/bundle_alert_contracts.rs`

### What Already Works

- alert export already has a stable root/index shape
- the runtime already knows about rules, contact points, mute timings,
  templates, and policies

### Residual Gap

The remaining gap is small and mostly presentation-oriented:

- clearer consumer display of existing staged/live alert evidence
- no new alert-owned derivation unless a concrete consumer later proves it is missing

### Shortest Next Step

Do not reopen the alert producer path now. Keep alert status logic in the
alert-owned producer and only revisit it if an actual consumer is blocked by a
missing signal.

## Access

### Current Producer State

Status: `landed` for staged export-bundle status, `landed` for deeper live list-surface status, `partial` for import-review/drift layering

Current usable sources:

- export bundle summaries for users, teams, orgs, and service accounts
- user/team import dry-run documents
- live browse surfaces

Strongest current files:

- `rust/src/access/user_workflows.rs`
- `rust/src/access/team_import_export_diff.rs`
- `rust/src/access/mod.rs`

### What Already Works

- access export bundles already provide stable staged inventory roots
- user and team import paths already emit dry-run documents

### Gap

The staged producer now answers export presence and missing bundle kinds, but it still needs:

- import-review readiness
- drift/review surface coverage by resource family

### Shortest Next Step

Layer user/team import-review and later drift signals onto the access-owned
producer instead of rebuilding access status in project aggregation.

## Sync

### Current Producer State

Status: `landed` for staged status, `landed` for first staged-backed live status, `partial` for broader audit/plan/live layering

Current usable sources:

- sync summary
- sync plan
- sync apply intent
- sync audit
- sync preflight
- review/audit TUI surfaces

Strongest current files:

- `rust/src/sync/workbench.rs`
- `rust/src/sync/staged_documents.rs`
- `rust/src/sync/audit.rs`
- `rust/src/sync/preflight.rs`
- `rust/src/sync/cli.rs`

### What Already Works

- sync already has the strongest staged document family in the repo
- audit and preflight already express drift and blocking information
- review/apply separation is explicit

### Gap

The staged sync producer is landed, and live sync now has a conservative
staged-backed row. Remaining depth is now bounded follow-up only:

- sync plan/audit integration
- apply-readiness vs review-readiness distinction
- richer live evidence beyond staged summary and bundle-preflight handoff

### Shortest Next Step

Do not deepen by default. Reopen the sync-owned producer only if a concrete
consumer needs stronger review/apply evidence.

## Promotion

### Current Producer State

Status: `landed` for staged preflight status, `landed` for first staged-backed live status, `partial` for fuller handoff/apply layering

Current usable sources:

- promotion preflight document
- handoff summary
- bundle-preflight inheritance

Strongest current files:

- `rust/src/sync/promotion_preflight.rs`

### What Already Works

- promotion preflight already exposes remap checks, blocker counts, and a
  review-handoff summary

### Gap

The staged producer is landed for preflight readiness. Live promotion now has a
conservative staged-backed row, and remaining depth is now bounded follow-up
only:

- review readiness vs apply readiness
- resolved remap inventory
- controlled continuation after preflight
- richer live evidence beyond summary, mapping, and availability handoff

### Shortest Next Step

Keep promotion status attached to promotion modules and extend the landed
producer with handoff/apply state instead of routing promotion semantics back
through generic overview logic.

## Cross-Domain Gaps

These are the remaining pieces that still block true project-wide progress visibility:

1. live-status path is now landed, but several live domains still only expose
   bounded first-pass readiness and drift signals from the owning modules
2. staged freshness still uses conservative artifact-age heuristics only, and broader real-source timestamp freshness remains a bounded follow-up inside the existing project-status path
3. some domains still stop at first-pass staged/live summaries instead of
   deeper owned readiness inputs

## Recommended Build Order

1. keep landed staged producers stable across dashboard/datasource/alert/access/sync/promotion
2. deepen domain-owned producers with second-pass readiness inputs
3. deepen the landed live-status path with richer drift/readiness inputs where the
   domain already owns live knowledge
4. replace conservative staged freshness with richer source timestamps where available inside the existing project-status path
5. keep project-home TUI consuming shared status instead of owning derivation

## Architectural Guardrails

- Do not make `overview` the only owner of domain status logic.
- Do not let renderers rebuild domain status from raw artifacts.
- Do not invent one-off project heuristics that bypass domain evidence.
- Prefer additive typed status documents over more ad hoc summary rows.
- Keep staged and live producers separate even if they later aggregate into one
  project surface.
