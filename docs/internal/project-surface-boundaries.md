# Project Surface Boundaries

Maintainer note for the current high-level project surfaces.

This file keeps the operator-facing names, internal runtime names, and
near-term ownership targets in one place. Keep operator examples in
`README.md` and the user guides.

## Public Surface

The maintained operator model is:

- `overview`
  - human-first project entrypoint
  - reads staged artifacts by default
  - may hand live reads through to the shared `status live` path
- `status`
  - canonical staged/live readiness surface
  - should own shared project-level status assembly
- `change`
  - review-first staged change workflow
  - owns summary, bundle, preflight, plan, review, apply intent, and audit

## Naming Boundary

- Public names are the `grafana-util` command names shown by `rust/src/cli.rs`,
  `README.md`, and the user guides.
- Internal module or contract names may remain narrower or older than the
  public names when they describe implementation slices rather than the
  operator surface.
- `sync` is now an internal runtime namespace and staged-document family behind
  the public `change` surface.
- `project-status` is now an internal architecture/file name behind the public
  `status` surface.
- Legacy Python module names remain maintainer-only reference and are not part
  of the current operator story.

## Current Vs Target Ownership

| Area | Current state | Target state |
| --- | --- | --- |
| `overview` staged path | owns staged artifact loading plus overview document projection | keep overview-specific projection separate from shared status aggregation |
| `status` staged path | owns shared staged status assembly directly and reuses overview artifact loading | keep shared staged aggregation under `status` ownership |
| `status` live path | shared live runtime already feeds `status live` and `overview live` | keep shared live runtime ownership in `status` |
| `change` surface | public command name is `change`, but internal runtime and JSON kinds still use `sync` naming | keep public/internal split explicit until or unless a future contract migration is planned |

## Current Maintainer Rule

- Add new project-wide signals as domain-owned producers first.
- Feed those signals into shared `status` aggregation second.
- Let `overview` consume the shared status result plus its own project snapshot
  views.
- Do not make `overview` the long-term owner of staged status semantics.
- Do not make `change` a generic inventory or status surface.

## Immediate Follow-Up

- Keep public docs on `overview` / `status` / `change` vocabulary only.
- Make any remaining `sync` or `project-status` mentions in current docs
  clearly internal or historical.
- Keep `project_status_command.rs` focused on args, dispatch, shared rendering,
  and client/header helpers now that shared staged and live status assembly
  both live outside the command module.
- Keep `project_status_support.rs` limited to shared client/header support for
  the `status` live path instead of letting command-surface concerns drift into
  it.
