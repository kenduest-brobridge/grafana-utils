# Internal Docs Index

`docs/internal/` now keeps only the maintainer docs that still act as current
entrypoints or stable architecture maps. Older plans, unwired scaffolds,
backlogs, market analysis, and progress snapshots have been moved into
`docs/internal/archive/`.

## Keep In The Root

- `ai-status.md`
  - current change trace and active maintainer notes
- `ai-changes.md`
  - current summarized change log for meaningful behavior or architecture work
- `overview-architecture.md`
  - source-of-truth maintainer map for `grafana-util overview`
- `project-status-architecture.md`
  - project-wide status-model architecture behind the public `status` surface
- `project-surface-boundaries.md`
  - current public-name, internal-name, and ownership map for `overview`,
    `status`, and `change`

## Inventory And Name Bridge

- Keep this index as the current inventory of maintainer-root docs, not as a history log.
- Use file names that bridge directly to the maintained concept or command name when possible.
- Keep one stable owner per entry so maintainers can tell whether a page is a trace, a map, or a status model.

- `ai-status.md` -> active trace and decision log
- `ai-changes.md` -> condensed change ledger for meaningful behavior or architecture work
- `overview-architecture.md` -> `grafana-util overview` map and extension rules
- `project-status-architecture.md` -> cross-domain status model behind the public `status` surface
- `project-surface-boundaries.md` -> public-name and internal-name bridge plus current-vs-target ownership
- `docs/DEVELOPER.md` -> maintainer policy, routing, and validation guidance

## Internal Examples

- `examples/datasource_live_mutation_api_example.py`
- `examples/datasource_live_mutation_safe_api_example.py`

## Archive Policy

- Move any unwired plan, dated execution note, backlog, proposal, or historical
  implementation scaffold into `archive/` unless it is still the current source
  of truth.
- Move dated architecture reviews and generated reference snapshots into
  `archive/` as well; keep only current maintainer entrypoints in the root.
- Keep core architecture docs in the root only when maintainers should still
  read them before editing code.
- Prefer consolidating small one-off maintainer references into
  `docs/DEVELOPER.md`, `docs/overview-rust.md`, or `docs/overview-python.md`
  instead of creating new standalone index pages.
