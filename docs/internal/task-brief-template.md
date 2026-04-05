# Task Brief Template

Use this template when you want to hand a repo task to an AI agent.

Copy it into:

- a chat prompt
- an issue body
- a PR description
- a local scratch note

Repo-owned copies also live in:

- `.github/ISSUE_TEMPLATE/ai-task-brief.md`
- `.github/PULL_REQUEST_TEMPLATE.md`

Keep it short. The goal is to make the task precise enough that the agent can
find the right source-of-truth files and run the right validation.

## Template

```text
Goal:
- What should change?

Touched Surface:
- Which subsystem, command family, docs lane, or build lane is in scope?

Constraints:
- What must not change?
- What compatibility or workflow rule must stay intact?

Source-Of-Truth Files:
- Which code, docs, or contract files own this change?

Expected Companion Updates:
- Which docs, tests, generated outputs, spec docs, or trace docs should change too?

Validation:
- Which narrow checks should pass first?
- Which broader checks are required if the change crosses subsystem boundaries?

Review Shape:
- Solo: diff review
- Collaborative: PR review
```

## Example

```text
Goal:
- Add one dashboard export flag for flat output naming.

Touched Surface:
- dashboard CLI
- command reference docs

Constraints:
- Keep existing raw export contract intact.
- Do not change dashboard import semantics.

Source-Of-Truth Files:
- rust/src/dashboard/
- docs/commands/en/dashboard-export.md
- docs/commands/zh-TW/dashboard-export.md

Expected Companion Updates:
- focused parser/help tests
- command docs
- regenerated man/html output if source docs changed

Validation:
- cd rust && cargo test --quiet
- make man-check
- make html-check

Review Shape:
- Solo: diff review
```
