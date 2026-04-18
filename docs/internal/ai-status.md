# ai-status.md

Current AI-maintained status only.

- Older trace history moved to [`archive/ai-status-archive-2026-03-24.md`](docs/internal/archive/ai-status-archive-2026-03-24.md).
- Detailed 2026-03-27 entries moved to [`archive/ai-status-archive-2026-03-27.md`](docs/internal/archive/ai-status-archive-2026-03-27.md).
- Detailed 2026-03-28 task notes were condensed into [`archive/ai-status-archive-2026-03-28.md`](docs/internal/archive/ai-status-archive-2026-03-28.md).
- Detailed 2026-03-29 through 2026-03-31 entries moved to [`archive/ai-status-archive-2026-03-31.md`](docs/internal/archive/ai-status-archive-2026-03-31.md).
- Detailed 2026-04-01 through 2026-04-12 entries moved to [`archive/ai-status-archive-2026-04-12.md`](docs/internal/archive/ai-status-archive-2026-04-12.md).
- Keep this file short and current. Additive historical detail belongs in `docs/internal/archive/`.
- Older entries moved to [`ai-status-archive-2026-04-13.md`](docs/internal/archive/ai-status-archive-2026-04-13.md).
- Older entries moved to [`ai-status-archive-2026-04-14.md`](docs/internal/archive/ai-status-archive-2026-04-14.md).
- Older entries moved to [`ai-status-archive-2026-04-15.md`](docs/internal/archive/ai-status-archive-2026-04-15.md).
- Older entries moved to [`ai-status-archive-2026-04-16.md`](docs/internal/archive/ai-status-archive-2026-04-16.md).
- Older entries moved to [`ai-status-archive-2026-04-17.md`](docs/internal/archive/ai-status-archive-2026-04-17.md).
- Older entries moved to [`ai-status-archive-2026-04-18.md`](docs/internal/archive/ai-status-archive-2026-04-18.md).

## 2026-04-18 - Add dashboard plan multi-org routing
- State: Done
- Scope: Rust dashboard plan routing/model/tests, dashboard plan command docs, generated docs if needed, and AI trace docs. README files and Python implementation are out of scope.
- Baseline: `dashboard plan` can review one local dashboard export tree against one live Grafana org, but `--use-export-org`, `--only-org-id`, and `--create-missing-orgs` are parsed and then rejected as unsupported.
- Current Update: Added export-org routing for dashboard plan, including all-org scope discovery, Basic-auth org resolution, scoped live collection for matching orgs, and missing-org review rows.
- Result: Focused dashboard plan/parser tests, full Rust tests, clippy, formatting, docs generation, docs-surface, AI workflow, man/html checks, and whitespace checks pass.

## 2026-04-18 - Extend access plan resource coverage
- State: Done
- Scope: Rust access plan team/org/service-account slices, focused access tests, access plan docs, generated docs, and AI trace docs. README files and Python implementation are out of scope.
- Baseline: `access plan` initially reviewed user bundles only while team, org, service-account, and `all` selectors remained unsupported.
- Current Update: Added concrete `--resource org`, `--resource team`, and `--resource service-account` plan paths using the existing import/diff/live helpers. `--resource all` remains reserved for a later aggregate layer.
- Result: Focused access tests and clippy pass; broader validation is in progress.

## 2026-04-18 - Add review plans for access, dashboard, alert, and workspace
- State: Done
- Scope: Rust access/dashboard plan slices, alert plan contract fields, workspace preview normalized actions, focused tests, command docs, generated docs, and AI trace docs. README files and Python implementation are out of scope.
- Baseline: `datasource plan` introduced the reconcile-plan shape, while access/dashboard still lacked a plan command, alert plan rows did not consistently expose TUI-ready metadata, and workspace preview did not normalize legacy operations into a shared action/domain/blocker contract.
- Current Update: Added `access plan` for user bundles and `dashboard plan` for single-org dashboard exports, enriched alert plan rows with stable action/status/review fields while preserving existing apply compatibility, and normalized workspace preview output with `actions`, `domains`, and `blockedReasons` for future TUI review.
- Result: Focused access, dashboard, alert, and workspace preview tests pass; broader validation is in progress.

## 2026-04-18 - Add datasource reconcile plan
- State: Done
- Scope: Rust datasource plan command, plan model/rendering, focused datasource tests, command docs, generated docs, and AI trace docs. README files and Python implementation are out of scope.
- Baseline: `datasource diff` compares local bundles with live Grafana and reports `missing-remote` / `extra-remote`, while `datasource import --dry-run` previews create/update for import records only. There is no dedicated review-first datasource reconcile plan, no opt-in prune planning, and no TUI-ready action model.
- Current Update: Added `datasource plan` as a review-only reconcile surface with text/table/json output, opt-in prune planning, all-org export routing, safe field comparison, read-only blocking, and stable TUI-ready action IDs. `datasource diff` and import dry-run remain separate surfaces.
- Result: Focused datasource plan tests, datasource suite, clippy, formatting, docs-surface, and generated docs checks pass.

## 2026-04-18 - Repair legacy dashboard all-orgs root aggregates
- State: Done
- Scope: Rust dashboard export-layout repair, all-orgs export regression coverage, command docs, generated docs, and AI trace docs. README files, Python implementation, and live export behavior beyond regression coverage are out of scope.
- Baseline: older all-orgs export artifacts can contain valid `org_*` child exports while the root `index.json` and `export-metadata.json` only describe one org. `export-layout` repairs file layout and existing root index paths, but it does not rebuild missing all-orgs aggregate root data from child org indexes.
- Current Update: `dashboard convert export-layout` now rebuilds legacy all-orgs root `index.json` and root metadata from child `org_*/raw` and `org_*/prompt` indexes, even when no layout moves are possible because legacy folder identity is missing. The current exporter regression now asserts all-orgs root metadata uses all-orgs scope and does not carry a single root `org/orgId`.
- Result: Focused export-layout and all-orgs export tests pass; a copied legacy sample rebuilds root aggregate metadata to 138 dashboards across 2 orgs.

## 2026-04-18 - Align dashboard export/import with Grafana source
- State: Done
- Scope: Rust dashboard prompt export, raw-to-prompt fixture parity tests, import/publish target preflight evidence, dashboard command docs, generated docs, maintainer contract notes, and AI trace docs. README files, Python implementation, and dashboard v2 support are out of scope.
- Baseline: prompt export always emitted empty `__elements`, raw-to-prompt parity coverage did not include Grafana-source datasource/library/v2 fixture shapes, and dashboard import/publish did not surface or block provisioned target dashboard ownership before live overwrite.
- Current Update: Live dashboard export now fetches referenced library panel models into prompt `__elements` while offline raw-to-prompt remains warning-only. Raw-to-prompt tests now cover selected datasource variables, default datasource variables, special datasource refs, string datasource mappings, library panel warnings, and v2/k8s rejection. Dashboard import/publish dry-run surfaces target evidence and live apply blocks provisioned overwrite before POST.
- Result: Focused dashboard tests, full Rust tests, clippy, formatting, docs-surface, generated man/html checks, and whitespace checks pass.
