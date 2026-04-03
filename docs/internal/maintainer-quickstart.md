# Maintainer Quickstart

Use this page when you just entered the repo and need to understand the current
shape fast enough to make a safe change.

This is an orientation page, not a full spec. It should tell you where to look
first, what the maintained surfaces are, and which files own the thing you are
about to edit.

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
   - public product shape and operator-facing workflows
2. `docs/DEVELOPER.md`
   - maintainer routing by concern
3. `rust/src/cli.rs`
   - public CLI topology and namespace wiring
4. `docs/internal/maintainer-role-map.md`
   - routing by maintainer concern
5. `docs/internal/contract-doc-map.md`
   - where stable contracts and policy docs live
6. `Makefile`
   - supported validation and generation entrypoints
7. `docs/internal/README.md`
   - current internal-doc inventory

Then branch by task:

- runtime change: open the owning Rust module under `rust/src/`
- docs-generator change: open `docs/internal/generated-docs-architecture.md`
- secret/profile change: open `docs/internal/profile-secret-storage-architecture.md`
- build or release change: open `python/pyproject.toml`, `rust/Cargo.toml`,
  `Makefile`, and `scripts/`

## Repo Surface

Current repo reality:

- supported implementation surface:
  - `rust/src/`
- legacy reference surface:
  - `python/grafana_utils/`
- operator docs:
  - `README.md`, `README.zh-TW.md`, `docs/user-guide/`
- command-reference docs:
  - `docs/commands/`
- generated artifacts:
  - `docs/man/`
  - `docs/html/`

Do not treat every directory as equally important. Rust is the maintained
product surface. Python is useful reference material, but it is not the default
place to land a user-facing fix.

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

Treat these as generated output, not source:

- `docs/man/*.1`
- `docs/html/`

Fix the source Markdown or the generator unless the task is explicitly about
debugging generated output in place.

## Choose Your Lane

If the task is:

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
- change/status/overview behavior:
  - start with `rust/src/sync/` and the related internal architecture docs
- handbook or command docs:
  - start with `docs/user-guide/` or `docs/commands/`
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

## Fast Validation Defaults

When you are still orienting, prefer the narrowest non-destructive checks:

```bash
make help
make man-check
make html-check
cd rust && cargo test --quiet
cd python && PYTHONPATH=. poetry run python -m unittest -v tests
```

For generated docs work:

```bash
make man
make html
```

For local output review:

```bash
man ./docs/man/grafana-util.1
open ./docs/html/index.html
```

On Linux, replace `open` with `xdg-open`.

## Docs Map

Use these pages for the matching concern:

- `docs/DEVELOPER.md`
  - top-level maintainer router
- `docs/internal/README.md`
  - inventory of current internal docs
- `docs/internal/maintainer-role-map.md`
  - maintainer routing by role
- `docs/internal/contract-doc-map.md`
  - current contract/policy entrypoint
- `docs/internal/generated-docs-architecture.md`
  - generated-doc system design
- `docs/internal/generated-docs-playbook.md`
  - generated-doc maintenance tasks
- `docs/internal/profile-secret-storage-architecture.md`
  - secret backend model and platform rules

## Repo-Specific Gotchas

- `change`, `status`, and `overview` are related surfaces, but they are not
  interchangeable. Read the current architecture notes before collapsing or
  renaming them.
- Handbook content and command-reference content are separate source layers.
  Do not merge them into one doc family just because they cross-link.
- Generated artifacts should not become the only place a change is made.
- `docs/internal/ai-status.md` and `docs/internal/ai-changes.md` are trace
  files, not long-form design docs.

## Do Not Use This Page For

Do not let this page turn into:

- a duplicate of `docs/DEVELOPER.md`
- a contract or schema spec
- a generated-doc design note
- a change log or backlog
- a giant file tree dump with no opinionated reading order

For day-to-day maintenance, keep facades thin, prefer repo-owned typed envelopes when a workflow already owns the shape, and use comments only where they add signal about ownership or non-obvious behavior.

If you need one of those, jump to the owning doc instead of expanding this one.
