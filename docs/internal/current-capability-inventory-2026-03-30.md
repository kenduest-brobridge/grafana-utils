# Current Capability Inventory (2026-03-30)

Purpose:

- give maintainers one current snapshot of what the project can already do
- map the major command surfaces to practical operator use
- make clear which areas are mature enough to stop deepening for now

This note is intentionally current-only. It is not a backlog, plan queue, or
historical trace.

## Product Shape

The current project is a Rust-mainline operator toolkit for Grafana. The main
through-line is:

1. inventory live state
2. inspect or compare it
3. export artifacts for later review and analysis
4. preview import or sync changes before mutation
5. move toward review-first sync and promotion workflows

The Python tree remains in the repo as maintainer reference and compatibility
material, but the maintained operator surface is the Rust `grafana-util`
binary.

## Capability Matrix

### `dashboard`

- Primary use:
  - inventory dashboards, inspect dependency/governance/query usage, export,
    import, diff, delete, capture screenshots/PDFs
- Mature operator value:
  - dependency and blast-radius answers
  - governance and query review
  - staged import-readiness and folder-aware migration
- Main commands:
  - `grafana-util dashboard list`
  - `grafana-util dashboard inspect-export`
  - `grafana-util dashboard inspect-live`
  - `grafana-util dashboard import`
  - `grafana-util dashboard diff`
  - `grafana-util dashboard topology`
  - `grafana-util dashboard screenshot`
- Current status:
  - landed enough for the current pass
- Do not treat as next by default:
  - only reopen if a concrete consumer proves the current staged/live producer
    is missing a decision-critical signal

### `datasource`

- Primary use:
  - inventory, export/import/diff, dry-run review, and bounded live mutation
- Mature operator value:
  - staged/live trust for export/import/sync
  - org-aware replay and diff/drift visibility
  - secret/provider-adjacent readiness signals
- Main commands:
  - `grafana-util datasource list`
  - `grafana-util datasource export`
  - `grafana-util datasource import`
  - `grafana-util datasource diff`
- Current status:
  - landed enough for the current pass
- Do not treat as next by default:
  - only reopen if a concrete consumer proves the current staged/live producer
    is missing a decision-critical signal

### `alert`

- Primary use:
  - inventory plus export/import/diff for alert rules and related alerting
    resources
- Mature operator value:
  - alert asset bundle visibility
  - bounded readiness and prerequisite coverage
  - supporting-surface checks for policies, contact points, mute timings, and
    templates
- Main commands:
  - `grafana-util alert list`
  - `grafana-util alert export`
  - `grafana-util alert import`
  - `grafana-util alert diff`
- Current status:
  - landed enough for the current pass
- Do not treat as next by default:
  - keep follow-up presentation-only unless a concrete consumer gets blocked

### `access`

- Primary use:
  - inventory and lifecycle workflows for users, orgs, teams, and service
    accounts
- Mature operator value:
  - bounded export/import/diff support
  - CRUD and membership-aware workflows
  - enough staged/live status for the current operator intent
- Main commands:
  - `grafana-util access user ...`
  - `grafana-util access org ...`
  - `grafana-util access team ...`
  - `grafana-util access service-account ...`
- Current status:
  - landed enough for the current pass
- Do not treat as next by default:
  - do not keep polishing without a concrete downstream consumer need

### `sync`

- Primary use:
  - review-first staged sync planning, preflight, audit, and optional live
    apply/fetch paths
- Mature operator value:
  - deterministic staged documents
  - reviewable trust-chain evidence
  - bounded staged/live status around review/apply decisions
- Main commands:
  - `grafana-util sync summary`
  - `grafana-util sync plan`
  - `grafana-util sync review`
  - `grafana-util sync preflight`
  - `grafana-util sync assess-alerts`
- Current status:
  - landed enough for the current pass
- Do not treat as next by default:
  - only reopen if a concrete operator path still lacks decision-critical trust
    evidence

### `promotion`

- Primary use:
  - environment handoff and promotion preflight over staged documents
- Mature operator value:
  - review-first promotion checks
  - explicit handoff/apply-continuation evidence
  - safer fail-closed behavior before mutation
- Main surfaces:
  - promotion preflight and promotion-related status produced through the
    `sync` / `project-status` paths
- Current status:
  - landed enough for the current pass
- Do not treat as next by default:
  - only reopen if a concrete review/apply consumer is missing a
    decision-critical signal

## Project-Level Surfaces

### `overview`

- Purpose:
  - staged artifact aggregator and browser, plus a thin live convenience entry
- What it is good for:
  - one project-wide staged snapshot across dashboard, datasource, alert,
    access, sync, bundle-preflight, and promotion-preflight artifacts
  - an operator-friendly `overview live` entrypoint that hands off to the
    shared live project-status path
- Outputs:
  - text
  - JSON
  - interactive staged browser
  - interactive live project-home via the shared project-status path
- What it is not:
  - not the owner of project-status architecture
  - not a live-status engine or live rule owner
  - not a second domain workflow layer

### `project-status`

- Purpose:
  - canonical project-wide status surface
- What it is good for:
  - staged/live split
  - per-domain status
  - blockers, warnings, next actions, and freshness
  - shared status contract reused by CLI and TUI surfaces
- Outputs:
  - text
  - JSON
  - interactive project-home style surface
- What it is not:
  - not a domain rule owner
  - not a separate analysis engine
  - not another overview-like owner layer

## What Is Already Good Enough

For the current pass, these areas should be treated as stable enough:

- `dashboard`
- `datasource`
- `alert`
- `access`
- `sync`
- `promotion`

Meaning:

- keep them stable and readable
- keep tests aligned with real contracts
- reopen only from a real consumer gap

## Default Maintenance Mode

The default operating mode after this pass is:

- stability over new depth lanes
- consumer-driven reopen decisions only
- thin-consumer polish only at the project level

This means:

- do not assume another domain pass should start by default
- do not queue generic polish as if it were roadmap-critical work
- do not expand `overview` or `project-status` just because they are visible

Reopen work only when a concrete consumer can point to:

- the command or surface they are using
- the missing decision-critical signal
- why the current staged/live producer is insufficient

Without that evidence, the correct next step is maintenance, clarity, and
contract stability rather than deeper feature work.

## Allowed Near-Term Work

The project-level work that still makes sense without reopening a domain lane
is:

- documentation clarity and de-duplication
- help text alignment
- TUI handoff and presentation clarity
- contract-preserving consumer polish
- focused tests that protect existing typed outputs

## What Not To Expand Right Now

- do not make `overview` larger
- do not make `project-status` own more derivation logic
- do not reopen a domain lane because of generic polish alone
- do not turn exploratory analysis or alternate packaging into a default
  execution lane

## Practical Reading Order

If you are orienting quickly:

1. `README.md`
2. `docs/overview-rust.md`
3. this file
4. `docs/internal/current-execution-review-2026-03-29.md`
5. `docs/internal/project-roadmap.md`

If you need subsystem-specific intent:

- staged overview architecture:
  - `docs/internal/overview-architecture.md`
- shared project-status architecture:
  - `docs/internal/project-status-architecture.md`
