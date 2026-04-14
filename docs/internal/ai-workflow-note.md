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
  - `scripts/contracts/command-surface.json`
  - current internal architecture and policy docs under `docs/internal/`
- Workflow schema:
  - repo rules in `AGENTS.md`
  - maintainer routing and validation guidance in `docs/DEVELOPER.md`
  - contract split rules in `docs/internal/contract-doc-map.md`

The important rule is:

- raw sources hold the facts
- maintained knowledge explains how to navigate and apply those facts
- schema docs tell the agent how to work in this repo

## Local Live Validation Baseline

When an AI agent needs a disposable local Grafana instance for live validation,
prefer the repo-owned Docker test port:

- `http://127.0.0.1:43011`

Use this as the default local live Grafana target for repo validation scripts,
manual smoke checks, and AI-assisted live testing unless the task explicitly
needs a different port.

Why this baseline exists:

- avoid colliding with a human-maintained Grafana already using `:3000`
- stay aligned with the repo's live-test script conventions
- keep AI-authored examples and local validation notes consistent

Practical rule:

- if the task says "use local Docker Grafana" and no port is specified, assume
  `43011` first
- only fall back to another port when `43011` is already in use or the task
  explicitly says otherwise

Do not let AI-maintained notes become a second source of truth for runtime
behavior or compatibility rules.

## Core Workflow

The working sequence stays:

1. ingest the change into the right owning layer
2. query the repo through the current maintainer routing
3. lint for code/help/docs/spec/generated-output drift
4. review the actual diff and validations before accepting the change

For stable closure rules around companion updates, maintainer routing sync,
source-of-truth boundaries, and stop-and-ask conditions, use
`docs/internal/ai-change-closure-rules.md`.

### Architecture Consistency Pass

Before editing architecture, facade, or large-file organization, do a short
consistency pass against the current guardrails and owning docs.

Read in this order when relevant:

1. `docs/internal/overview-architecture.md`
2. the current contract or policy doc for the touched surface
3. `docs/internal/generated-docs-architecture.md` if generated output is part
   of the change
4. `docs/internal/profile-secret-storage-architecture.md` for profile or secret
   work
5. `docs/internal/task-brief-template.md` to confirm the brief carries the
   right ownership and validation fields
6. `docs/internal/docs-architecture-guardrails.md` for handbook/manual,
   command-doc, generated-doc, internal-doc, and trace-doc boundaries
7. `scripts/contracts/command-surface.json` for public command-path, legacy
   replacement, routing, and `--help-full` / `--help-flat` changes

Use this pass to answer:

- is the change preserving the current layer boundary, or is it mixing two
  responsibilities into one file
- does a large file need to be split by responsibility before more behavior is
  added
- does the task brief already identify the owned layer, source of truth,
  contract impact, test strategy, and generated-doc impact
- will the docs update stay in the right layer, or is the manual drifting into
  command-reference detail
- does the command reference need the exact syntax first, with the manual only
  updating the stable user journey or decision table

If the answer is unclear, stop and resolve it against the owning docs before
editing code or docs.

## Query

An agent should query the repo through the maintainer routing first.

Default read order:

1. `README.md`
2. `docs/DEVELOPER.md`
3. `docs/internal/maintainer-quickstart.md`
4. the owning code or policy files for the touched surface

The goal is not to read everything. The goal is to reach the current source of
truth quickly enough to make a safe change.

## Lint

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

Use [`task-brief-template.md`](task-brief-template.md)
when you want a repo-shaped starting point for that brief.
If the work is tracked on GitHub, reuse the same fields through
`.github/ISSUE_TEMPLATE/ai-task-brief.md` or `.github/PULL_REQUEST_TEMPLATE.md`.

## Task Brief Shape

For agent-friendly work, prefer a short brief with these fields:

- goal
- touched surface
- constraints
- owned layer
- source-of-truth files
- contract impact
- expected companion updates
- test strategy
- generated docs impact
- validation commands

When the task crosses architecture or large-file boundaries, include the
architecture consistency pass in the brief instead of relying on ad hoc
judgment.

Example:

```text
Goal: add a new dashboard export flag for flat output naming
Touched surface: dashboard CLI + command docs
Constraints: preserve existing JSON contract; do not change import semantics
Source-of-truth files: rust/src/commands/dashboard/, docs/commands/en/, docs/commands/zh-TW/
Expected companion updates: parser/help tests, command docs, regenerated man/html if docs text changes
Validation: cd rust && cargo test --quiet; make man-check; make html-check
```

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
