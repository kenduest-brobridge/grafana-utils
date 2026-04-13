# Maintainer Quickstart

Use this page when you just entered the repo and need the fastest safe route
into the current code, docs, and policy surfaces.

This is an orientation page, not a full spec. It should tell you what to open
first, what the maintained surfaces are, and which narrower doc owns the next
decision.

## What This Page Is For

Read this first if you are:

- a new maintainer
- an AI agent entering the repo for the first time
- returning after enough time away that the current routing is no longer obvious

Do not use this page as the final source of truth for runtime contracts,
generated-doc design, or compatibility policy. It is the shortest path to those
documents, not a replacement for them.

## Start In 5 Minutes

Open these in order:

1. `README.md`
   - public product shape and operator-facing entrypoints
2. `docs/DEVELOPER.md`
   - short maintainer landing page by concern
3. `rust/src/cli.rs`
   - public CLI topology and namespace wiring
4. `docs/internal/maintainer-role-map.md`
   - routing by maintainer persona
5. `docs/internal/contract-doc-map.md`
   - where stable contracts and policy docs live
6. `Makefile`
   - supported validation and generation entrypoints
7. `scripts/contracts/command-surface.json`
   - machine-readable CLI/docs synchronization contract for public command
     paths, legacy replacements, docs routing, and `--help-full` / `--help-flat` support

Then branch by task:

- runtime change:
  - open the owning Rust module under `rust/src/`
- docs-generator change:
  - open `docs/internal/generated-docs-architecture.md`
- zh-TW doc copy review:
  - open `docs/internal/zh-tw-style-guide.md`
- secret/profile change:
  - open `docs/internal/profile-secret-storage-architecture.md`
- architecture boundary or large-module refactor:
  - open `docs/internal/rust-architecture-guardrails.md`
- handbook/manual boundary or docs split change:
  - open `docs/internal/docs-architecture-guardrails.md`
- AI-assisted workflow or agent task shaping:
  - open `docs/internal/ai-workflow-note.md` and
    `docs/internal/ai-change-closure-rules.md`
- task brief drafting for agent work:
  - open `docs/internal/task-brief-template.md`
- build or release change:
  - open `python/pyproject.toml`, `rust/Cargo.toml`, `Makefile`, and `scripts/`

## Repo Surface

Current repo reality:

- maintained implementation surface:
  - `rust/src/`
- legacy reference surface:
  - `python/grafana_utils/`
- public operator docs:
  - `README.md`, `README.zh-TW.md`, `docs/user-guide/`
- command-reference docs:
  - `docs/commands/`
- generated artifacts:
  - `docs/man/`
  - `docs/html/`

Rust is the maintained product surface. Python is useful reference material,
but it is not the default place to land a user-facing fix.

## Source Of Truth Map

Treat these as authoritative:

- CLI/runtime behavior:
  - `rust/src/`
- handbook content:
  - `docs/user-guide/{en,zh-TW}/`
- command-reference content:
  - `docs/commands/{en,zh-TW}/`
- contract and compatibility rules:
  - `docs/internal/*.md` routed by `docs/internal/contract-doc-map.md`
- generated-doc rules:
  - `docs/internal/generated-docs-architecture.md`
  - `docs/internal/generated-docs-playbook.md`
  - `docs/internal/zh-tw-style-guide.md`
- CLI/docs synchronization:
  - `scripts/contracts/command-surface.json`
  - `scripts/check_docs_surface.py`
  - `make quality-docs-surface`

Treat these as generated output, not source:

- `docs/man/*.1`
- `docs/html/`

Fix the source Markdown or the generator unless the task is explicitly about
debugging generated output in place.

## Choose Your Lane

If the task is mostly:

- CLI flags, parsing, help, or dispatch:
  - start with `rust/src/cli.rs`
- dashboard behavior:
  - start with `rust/src/dashboard/`
- datasource behavior:
  - start with `rust/src/datasource.rs`
- alert behavior:
  - start with `rust/src/alert.rs`
- access behavior:
  - start with `rust/src/access/`
- status, snapshot, resource, or workspace/change behavior:
  - start with `rust/src/sync/`
- handbook or command docs:
  - start with `docs/user-guide/` or `docs/commands/`, then validate command
    examples against `scripts/contracts/command-surface.json`
- generated HTML or manpages:
  - start with the generated-docs architecture/playbook and the generator
    scripts under `scripts/`
- file format, export schema, or compatibility policy:
  - start with `docs/internal/contract-doc-map.md`
- secret storage or profile resolution:
  - start with `docs/internal/profile-secret-storage-architecture.md`
- build, packaging, or release workflow:
  - start with `Makefile`, `scripts/`, `python/pyproject.toml`, and
    `rust/Cargo.toml`

## Maintainer Docs Sync Contract

Keep the maintainer routing docs intentionally split:

- `docs/DEVELOPER.md`
  - short maintainer landing page by concern
- `docs/internal/maintainer-quickstart.md`
  - first-entry reading order and source-of-truth map
- `docs/internal/maintainer-role-map.md`
  - routing by maintainer persona
- `docs/internal/contract-doc-map.md`
  - contract and policy routing

When the change is:

- a new maintainer entry route or repo priority shift:
  - update `docs/DEVELOPER.md` and this file
- a maintainer persona or ownership boundary shift:
  - update `docs/internal/maintainer-role-map.md` and any affected router
- a contract or policy doc move:
  - update `docs/internal/contract-doc-map.md` and every router that links to it
- an AI workflow or maintainer workflow rule change:
  - update `docs/internal/ai-workflow-note.md`, and update the routers only if
    the entry route changed

Do not update only one maintainer doc when the routing contract clearly spans
more than one of them.

## Fast Validation Defaults

When you are still orienting, prefer the narrowest non-destructive checks:

```bash
# Purpose: When you are still orienting, prefer the narrowest non-destructive checks.
make help
make quality-docs-surface
make quality-ai-workflow
make man-check
make html-check
cd rust && cargo test --quiet
cd python && PYTHONPATH=. poetry run python -m unittest -v tests
```

For generated docs work:

```bash
# Purpose: For generated docs work.
make man
make html
```

For installer work:

```bash
# Purpose: Build a local binary, pack a local archive, and verify install.sh
# without downloading from GitHub.
make test-installer-local
```

## Repo-Specific Gotchas

- handbook content and command-reference content are separate source layers
- generated docs stay derived artifacts, not a second source layer
- `docs/internal/ai-status.md` and `docs/internal/ai-changes.md` are trace
  files, not long-form design docs
- `change`, `status`, and `overview` are related surfaces, but they are not
  interchangeable
- when a command writes a persisted artifact, keep the on-disk output plain
  text and only duplicate stdout when `--also-stdout` is explicitly set

## Do Not Use This Page For

Do not let this page turn into:

- a duplicate of `docs/DEVELOPER.md`
- a contract or schema spec
- a generated-doc design note
- a change log or backlog
- a giant file tree dump with no opinionated reading order

If you need one of those, jump to the owning doc instead of expanding this one.
