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
- `generated-docs-architecture.md`
  - design map for the Markdown-to-manpage and Markdown-to-HTML pipeline
- `generated-docs-playbook.md`
  - task-oriented maintainer recipes for common generated-docs changes
- `ai-workflow-note.md`
  - repo-specific workflow note for AI-assisted maintenance
- `ai-change-closure-rules.md`
  - stable companion-update, source-of-truth, and maintainer-routing closure rules for AI-assisted changes
- `task-brief-template.md`
  - reusable task brief shape for solo or collaborative AI-assisted work
- `zh-tw-style-guide.md`
  - zh-TW terminology, tone, and product-name translation rules for docs review
- `maintainer-quickstart.md`
  - first-entry route for new maintainers and AI agents
- `../scripts/contracts/command-surface.json`
  - machine-readable CLI/docs synchronization contract for public paths,
    legacy replacements, command-doc routing, and `--help-full` / `--help-flat` support
- `../scripts/contracts/output-contracts.json`
  - machine-readable registry of core JSON output contract fixtures and
    required root fields
- `../scripts/check_output_contracts.py`
  - executable validation entrypoint for the output contract registry and
    fixture set
- `profile-secret-storage-architecture.md`
  - profile secret backend model, platform support, and maintainer rules
- `rust-architecture-guardrails.md`
  - Rust layer boundaries, split thresholds, hotspot order, and architecture-lint usage
- `docs-architecture-guardrails.md`
  - handbook/manual, command docs, generated docs, internal docs, and trace docs boundaries
- `maintainer-role-map.md`
  - maintainer routing by concern: runtime, docs, contracts, and release flow
- `overview-architecture.md`
  - legacy overview maintainer map behind the public `grafana-util status overview` surface
- `project-status-architecture.md`
  - project-wide status-model architecture behind the public `status` surface
- `project-surface-boundaries.md`
  - current public-name, internal-name, and ownership map for `overview`,
    `status`, `workspace`, and `Change`

## Inventory And Name Bridge

- Keep this index as the current inventory of maintainer-root docs, not as a history log.
- Use file names that bridge directly to the maintained concept or command name when possible.
- Keep one stable owner per entry so maintainers can tell whether a page is a trace, a map, or a status model.

- `ai-status.md` -> active trace and decision log
- `ai-changes.md` -> condensed change ledger for meaningful behavior or architecture work
- `generated-docs-architecture.md` -> source/output model, generator split, and generated-docs design rules
- `generated-docs-playbook.md` -> maintainer cookbook for adding docs pages, locales, outputs, and manpage coverage
- `ai-workflow-note.md` -> repo-shaped AI workflow, source-of-truth boundaries, and agent task-brief expectations
- `ai-change-closure-rules.md` -> stable closure rules for companion updates, maintainer-router sync, source-of-truth boundaries, and stop-and-ask conditions
- `task-brief-template.md` -> minimal task handoff template for chat prompts, issues, or PR descriptions
- `.github/ISSUE_TEMPLATE/ai-task-brief.md` -> GitHub issue form for the same task brief fields
- `.github/PULL_REQUEST_TEMPLATE.md` -> GitHub PR template for review-time task context
- `zh-tw-style-guide.md` -> review rules for Taiwan-facing Traditional Chinese docs and product-object naming
- `maintainer-quickstart.md` -> first-entry reading order, source-of-truth map, task routing, and safe validation commands
- `../scripts/contracts/command-surface.json` -> machine-readable CLI/docs synchronization contract for public paths, legacy replacements, routing, and `--help-full` / `--help-flat` support
- `../scripts/contracts/output-contracts.json` -> machine-readable registry of core JSON output fixtures and root contract checks
- `../scripts/check_output_contracts.py` -> repo-owned validation entrypoint for the output contract registry and fixture set
- `../scripts/contracts/docs-entrypoints.json` -> machine-readable docs-entry/navigation contract for landing quick commands, jump-select entries, and handbook sidebar command shortcuts
- `profile-secret-storage-architecture.md` -> profile secret modes, macOS/Linux backend behavior, and secret-resolution design rules
- `rust-architecture-guardrails.md` -> Rust layer boundaries, split thresholds, hotspot order, and lint expectations
- `docs-architecture-guardrails.md` -> handbook/manual, command docs, generated docs, internal docs, and trace docs boundaries
- `maintainer-role-map.md` -> maintainer persona entrypoint and validation map by concern
- `overview-architecture.md` -> legacy overview map behind `grafana-util status overview` and extension rules
- `project-status-architecture.md` -> cross-domain status model behind the public `status` surface
- `project-surface-boundaries.md` -> public-name and internal-name bridge plus current-vs-target ownership for `status`, `workspace`, and `Change`
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
