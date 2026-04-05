## Task Brief

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
- Which narrow checks passed first?
- Which broader checks were required because the change crossed subsystem boundaries?

Review Shape:
- Usually `PR Review`
- If this was prepared in solo mode first, note any earlier `Diff Review`

## Reviewer Notes

- Any contract, compatibility, or scope decisions reviewers should inspect closely
- Any known risks, follow-ups, or intentionally deferred work
