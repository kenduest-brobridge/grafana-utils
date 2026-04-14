# Project Surface Boundaries

Maintainer note for the current high-level project surfaces.

This file keeps the operator-facing names, internal runtime names, and
near-term ownership targets in one place. Keep operator examples in
`README.md` and the user guides.

## Public Surface

The maintained operator model is:

- `status overview`
  - human-first project entrypoint
  - reads staged artifacts by default
  - may hand live reads through to the shared `status live` path
- `status`
  - canonical staged/live readiness surface
  - should own shared project-level status assembly
- `workspace`
  - review-first staged change workflow
  - owns summary, bundle, preflight, plan, review, apply intent, and audit

## Conceptual Surface

- `Change`
  - architecture name for the staged change lifecycle
  - use in maintainer notes, docs, and design discussion when the command name is not the point

## Naming Boundary

- Public names are the `grafana-util` command names shown by `rust/src/cli/mod.rs`,
  `README.md`, and the user guides.
- Internal module or contract names may remain narrower or older than the
  public names when they describe implementation slices rather than the
  operator surface.
- `workspace` is the public command surface for staged change workflows.
- `Change` is the conceptual surface name for that workflow when discussing
  architecture, review flow, or ownership.
- `sync` is the internal runtime namespace and staged-document family used for
  compatibility, fixtures, and implementation details.
- `project-status` is now an internal architecture/file name behind the public
  `status` surface.
- Legacy Python module names remain maintainer-only reference and are not part
  of the current operator story.

## Default And Migration Rules

- Default to `workspace` in public docs, help text, examples, and user-facing
  descriptions.
- Use `Change` only when the topic is the staged change lifecycle as an
  architecture concept.
- Keep `sync` in internal runtime, JSON kinds, fixtures, and compatibility
  references only.
- When replacing stale copy, migrate `change` command wording to `workspace`
  unless the text is explicitly describing the conceptual surface or a legacy
  contract.

## Current Vs Target Ownership

| Area | Current state | Target state |
| --- | --- | --- |
| `status overview` staged path | owns staged artifact loading plus overview document projection | keep overview-specific projection separate from shared status aggregation |
| `status` staged path | owns shared staged status assembly directly and reuses overview artifact loading | keep shared staged aggregation under `status` ownership |
| `status` live path | shared live runtime already feeds `status live` and `status overview live` | keep shared live runtime ownership in `status` |
| `workspace` surface | public command name is `workspace`; internal runtime and JSON kinds may still use `sync` naming for compatibility | keep public/internal split explicit until or unless a future contract migration is planned |

## Current Maintainer Rule

- Add new project-wide signals as domain-owned producers first.
- Feed those signals into shared `status` aggregation second.
- Let `status overview` consume the shared status result plus its own project
  snapshot views.
- Do not make `overview` the long-term owner of staged status semantics.
- Do not make `workspace` a generic inventory or status surface.

## Immediate Follow-Up

- Keep public docs on `status overview` / `status` / `workspace` vocabulary
  only.
- Use `Change` only as the conceptual name for the staged change lifecycle.
- Make any remaining `sync` or `project-status` mentions in current docs
  clearly internal, historical, or compatibility-related.
- Keep `project_status_command.rs` focused on args, dispatch, shared rendering,
  and client/header helpers now that shared staged and live status assembly
  both live outside the command module.
- Keep `project_status_support.rs` limited to shared client/header support for
  the `status` live path instead of letting command-surface concerns drift into
  it.
