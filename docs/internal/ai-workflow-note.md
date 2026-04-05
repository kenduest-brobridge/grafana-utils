# AI Workflow Note

Use this note when an AI agent is helping with repo maintenance work.

This is a repo-specific workflow note, not a generic position paper about AI.
It exists to keep AI-assisted changes aligned with the current source-of-truth,
contract, and validation rules already used in this repository.

## Why This Exists

The repo already has maintainer routing, contract docs, generated-doc rules,
and trace files. An AI agent should use those layers instead of inventing a
parallel workflow.

Use this note to answer:

- what the agent should treat as raw source versus maintained knowledge
- which updates should happen together in one patch
- what kinds of checks should run before the work is considered reviewable
- where human review still owns the final decision

## Repo-Shaped AI Model

Treat the repo in three layers:

- Raw sources:
  - runtime and parser code under `rust/src/`
  - legacy Python reference code under `python/grafana_utils/`
  - public docs under `README.md`, `README.zh-TW.md`, `docs/user-guide/`, and
    `docs/commands/`
  - tests, scripts, and the `Makefile`
- Maintained knowledge:
  - `docs/DEVELOPER.md`
  - `docs/internal/maintainer-quickstart.md`
  - `docs/internal/maintainer-role-map.md`
  - `docs/internal/contract-doc-map.md`
  - current internal architecture and policy docs under `docs/internal/`
- Workflow schema:
  - repo rules in `AGENTS.md`
  - maintainer routing and validation guidance in `docs/DEVELOPER.md`
  - contract split rules in `docs/internal/contract-doc-map.md`

The important rule is:

- raw sources hold the facts
- maintained knowledge explains how to navigate and apply those facts
- schema docs tell the agent how to work in this repo

Do not let AI-maintained notes become a second source of truth for runtime
behavior or compatibility rules.

## Core Workflow

### 1. Ingest

When new information enters the repo, the agent should update the right layer
instead of only editing the first file it touched.

Typical ingest cases:

- runtime behavior changed
- help text or CLI examples changed
- docs-generator behavior changed
- a stable schema or contract changed
- a maintainer-facing route or ownership boundary changed

Expected companion updates:

- runtime behavior change:
  - code, focused tests, and the public docs or help text that describe the
    behavior
- contract change:
  - code, focused tests, and the owning current contract doc in
    `docs/internal/`
- docs-generator change:
  - source docs or generator code, regenerated outputs when required, and the
    generated-docs architecture or playbook if the workflow changed
- maintainer-routing change:
  - the owning internal maintainer docs, not just one local note

### 2. Query

An agent should query the repo through the maintainer routing first.

Default read order:

1. `README.md`
2. `docs/DEVELOPER.md`
3. `docs/internal/maintainer-quickstart.md`
4. the owning code or policy files for the touched surface

The goal is not to read everything. The goal is to reach the current source of
truth quickly enough to make a safe change.

### 3. Lint

AI-assisted work should include a consistency pass before handoff.

Lint here means checking for:

- code/help/docs drift
- source/generated-doc drift
- summary/spec/trace drift
- parser behavior that no longer matches examples or command pages
- changes that touched a contract shape without updating the contract doc

The repo already has concrete lint-style checks:

- `make man-check`
- `make html-check`
- `make quality-ai-workflow`
- focused parser/help tests
- `cargo test` and Python unittest coverage
- `git diff --check`

## Source-Of-Truth Rules

Use these boundaries consistently:

- `rust/src/` is the maintained implementation surface
- `python/grafana_utils/` is legacy reference unless the task is explicitly in
  scope there
- `docs/user-guide/{en,zh-TW}/` is the handbook source layer
- `docs/commands/{en,zh-TW}/` is the command-reference source layer
- `docs/man/` and `docs/html/` are generated artifacts
- `docs/internal/*.md` should stay split by purpose:
  - summary in `docs/DEVELOPER.md`
  - spec in the dedicated current contract or architecture docs
  - trace in `docs/internal/ai-status.md` and `docs/internal/ai-changes.md`

If an AI agent edits a generated artifact without updating its source layer, the
change is incomplete unless the task is explicitly about generated output
debugging.

## Review-First Rule

This repo should treat prompt-style task briefs as an input to review, not as a
replacement for review.

Use the prompt or task brief to explain:

- what changed
- why the change is needed
- which source-of-truth files own the change
- which validations should pass

Still require review of:

- actual diffs
- affected tests
- regenerated docs when applicable
- contract changes and compatibility claims

Human review should approve meaning, compatibility, and scope. The agent can do
drafting, propagation, and bookkeeping, but should not silently redefine
contracts.

Review shape depends on the working mode:

- solo development:
  - `Task Brief -> Agent Execution -> Diff Review`
- collaborative development:
  - `Task Brief -> Agent Execution -> PR Review`

The invariant is the same in both modes:

- define the task clearly first
- inspect the actual diff and validation results before accepting the change

Use [`task-brief-template.md`](/Users/kendlee/work/grafana-utils/docs/internal/task-brief-template.md)
when you want a repo-shaped starting point for that brief.
If the work is tracked on GitHub, reuse the same fields through
`.github/ISSUE_TEMPLATE/ai-task-brief.md` or `.github/PULL_REQUEST_TEMPLATE.md`.

## Task Brief Shape

For agent-friendly work, prefer a short brief with these fields:

- goal
- touched surface
- constraints
- source-of-truth files
- expected companion updates
- validation commands

Example:

```text
Goal: add a new dashboard export flag for flat output naming
Touched surface: dashboard CLI + command docs
Constraints: preserve existing JSON contract; do not change import semantics
Source-of-truth files: rust/src/dashboard/, docs/commands/en/, docs/commands/zh-TW/
Expected companion updates: parser/help tests, command docs, regenerated man/html if docs text changes
Validation: cd rust && cargo test --quiet; make man-check; make html-check
```

## When The Agent Should Stop And Ask

The agent should not guess when:

- two current contract docs appear to disagree
- a change would redefine a stable field or compatibility rule
- the task conflicts with unrelated user changes already in the worktree
- generated docs and source docs disagree, but the intended truth source is not
  obvious
- the narrow validation path fails in unrelated parts of the repo and the risk
  is no longer local

## Minimal Working Loop

For most repo tasks, the safe loop is:

1. route to the current source-of-truth docs
2. inspect the owning code and focused tests
3. make the smallest coherent change
4. update the matching docs/spec/trace layer when required
5. run the narrowest relevant validation first
6. widen validation only if the change crosses subsystem boundaries

This keeps AI work useful without pretending the repo can be maintained from
free-form prompting alone.
