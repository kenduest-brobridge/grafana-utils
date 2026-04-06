# TODO

This file tracks the active backlog only.

Rust mainline work is the priority here; Python entries only cover supporting packaging and tooling.

Completed items that were previously listed here now live in `docs/internal/todo-archive.md`.

## In Progress

- Python packaging, docs, and syntax-floor tests now target Python 3.9+, but optional formatter/lint/static-check coverage still depends on tool availability in the active environment.

## Next

- keep current Rust domain-owned producers stable by default
- reopen a domain lane only when a concrete consumer proves a missing decision-critical signal
- keep `overview` and `project-status` as thin consumers instead of adding more derivation logic there
- if a lane must reopen, prefer owner-module work inside `dashboard`, `datasource`, `alert`, `access`, `sync`, or `promotion`
- keep advanced analysis and packaging exploratory rather than making it a current execution lane
- clean repo workflow noise by keeping local scratch files, temp exports, and ad hoc notes out of normal review/commit paths
- evaluate streaming or lower-memory dashboard listing/export paths only if large-instance validation shows the current full-materialization approach is a real bottleneck
- evaluate semantic alert diff normalization for equivalent values such as duration aliases after the current structural diff behavior is otherwise stable
- keep dashboard `publish --watch` on the current polling implementation unless live validation shows a concrete missed-save, latency, or portability problem that justifies an event-based watcher

## Shared Access Parameters

Currently implemented:

- `--url`
- `--token`
- `--basic-user`
- `--basic-password`
- `--prompt-password`
- `--insecure`
- `--ca-cert`
- `--org-id`
- `--json`
- `--csv`
- `--table`

## Authentication Rules

Current implementation status:

- `user list --scope org`: token or Basic auth
- `user list --scope global`: Basic auth only
- `user list --with-teams`: Basic auth only
- `user add`: Basic auth only
- `user modify`: Basic auth only
- `user delete --scope global`: Basic auth only
- `user delete --scope org`: token or Basic auth
- `team list`: token or Basic auth
- `team add`: token or Basic auth
- `team modify`: token or Basic auth
- `team delete`: token or Basic auth
- `service-account list`: token or Basic auth
- `service-account add`: token or Basic auth
- `service-account token add`: token or Basic auth
- `service-account delete`: token or Basic auth
- `service-account token delete`: token or Basic auth

Rules to keep:

- if `--token` is provided, treat it as the primary authentication input unless the command explicitly requires Basic auth
- only require `--basic-user` and `--basic-password` for operations that truly need Basic auth
- reject mixed auth inputs unless the command has a specific, documented reason to support them
- keep prompted password support aligned with dashboard and alert auth behavior

## Priority Order

1. keep current domain-owned producers stable
2. reopen only consumer-proven missing signals
3. keep `overview` / `project-status` thin and contract-driven
4. clean repo workflow noise and local scratch artifacts
5. extend Rust bundle normalization beyond alert-rule specs only if a concrete consumer needs it
6. semantic alert diff normalization for equivalent values only after the current structural diff behavior is otherwise stable

## Architecture Follow-up

1. enforce the tool-not-platform boundary
- treat `overview` and `status` as consumer/reporting surfaces, not new orchestration owners
- require a concrete operator decision gap before adding another top-level surface or cross-domain workbench flow
- prefer tightening help, docs, and output clarity over adding another architectural layer

2. keep domain depth balanced before widening the surface
- do not keep deepening dashboard-only intelligence while `access` and other domains remain materially shallower
- define a minimum producer maturity bar for each domain before expanding cross-domain summary features again
- use consumer-driven reopen rules so domain work resumes only when a real operator decision is blocked

3. preserve a lightweight lane for simple backup use cases
- keep plain export/backup flows obvious and low-friction for users who only need JSON capture and restore
- avoid forcing review/workbench/governance-heavy flows onto the simplest inventory and backup paths
- make the heavier plan/diff/governance lane explicitly optional and operator-intent driven
