# ai-status.md

Current AI-maintained status only.

- Older trace history moved to [`archive/ai-status-archive-2026-03-24.md`](docs/internal/archive/ai-status-archive-2026-03-24.md).
- Detailed 2026-03-27 entries moved to [`archive/ai-status-archive-2026-03-27.md`](docs/internal/archive/ai-status-archive-2026-03-27.md).
- Detailed 2026-03-28 task notes were condensed into [`archive/ai-status-archive-2026-03-28.md`](docs/internal/archive/ai-status-archive-2026-03-28.md).
- Detailed 2026-03-29 through 2026-03-31 entries moved to [`archive/ai-status-archive-2026-03-31.md`](docs/internal/archive/ai-status-archive-2026-03-31.md).
- Detailed 2026-04-01 through 2026-04-12 entries moved to [`archive/ai-status-archive-2026-04-12.md`](docs/internal/archive/ai-status-archive-2026-04-12.md).
- Keep this file short and current. Additive historical detail belongs in `docs/internal/archive/`.
- Older entries moved to [`ai-status-archive-2026-04-13.md`](docs/internal/archive/ai-status-archive-2026-04-13.md).
- Older entries moved to [`ai-status-archive-2026-04-14.md`](docs/internal/archive/ai-status-archive-2026-04-14.md).
- Older entries moved to [`ai-status-archive-2026-04-15.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-04-15.md).

## 2026-04-15 - Start Rust TODO maintainability pass
- State: In Progress
- Scope: conservative-boundary TODO execution across Rust tests, access TUI input, and status producer model. Python implementation and README files are out of scope.
- Baseline: `todo.md` records the Rust-first maintainability backlog and requires responsibility-based splits instead of line-count-only splits.
- Current Update: workers are being assigned disjoint TODO slices with conservative write scopes and focused validation.
- Result: In progress.

## 2026-04-15 - Improve Rust maintainability priorities
- State: Done
- Scope: Rust feature matrix policy, dashboard review source/model boundaries, dashboard orchestration/test splitting, output contract depth checks, and validation. Python implementation and README files are out of scope.
- Baseline: Rust default/browser gates pass, but no-default feature behavior is not a declared release surface; several Rust modules and test files remain large; dashboard review source concepts are shared implicitly across summary/dependencies/policy/impact; output contract validation covers root fields and golden fixtures but not nested shape depth.
- Current Update: Added a repo-owned Rust feature matrix check that documents default/browser as supported surfaces and keeps `--no-default-features` out of release claims; split dashboard inspect input resolution and topology rendering; moved the shared dashboard source resolver to a typed `review_source` model for export-tree, saved-artifact, and live review inputs; split datasource render/parser/payload tests; and extended output contract validation with nested `requiredPaths` and `pathTypes`.
- Result: Focused Rust tests, feature-matrix checks, output contract checks, architecture guardrails, generated docs checks, and whitespace checks pass for the changed surfaces. README files were left untouched.

## 2026-04-15 - Consolidate Rust review workflow contracts
- State: Done
- Scope: Rust dashboard summary/dependency naming, dashboard command dispatch boundaries, public JSON output contract fixtures, docs evidence checks, generated docs, and validation.
- Baseline: public docs and CLI use `dashboard summary` / `dashboard dependencies`, but Rust internals still carry `Analyze` names; dashboard dispatch duplicates summary/history handling across client-owned and top-level paths; output contract registry covers only core sync/datasource fixtures; docs evidence tests contained stale page/path expectations.
- Current Update: renamed Rust dashboard summary internals away from `Analyze`/`dashboard analyze`, split dashboard dispatch into focused summary/export/live helpers, expanded output contracts for dashboard review artifacts, refreshed source/generated docs evidence sections, and added guardrails that reject removed public dashboard analysis paths outside archive/trace contexts.
- Result: Rust tests, clippy, docs surface checks, output contract checks, generated docs checks, AI workflow checks, feature `browser` check, command smoke checks, and whitespace checks pass. README files were left untouched.

## 2026-04-14 - Tighten review-first workflow contracts
- State: Done
- Scope: public/internal workspace naming policy, core output contract registry, README positioning, generated handbook HTML, access browse loading boundaries, dashboard impact traversal, and validation.
- Baseline: maintainer docs still described `change` as a public command surface while user-facing docs and CLI expose `workspace`; core JSON output contracts were covered by scattered Rust/Python tests but did not have a small central registry of golden fixture expectations. Access user browsing also kept data loading in the input handler, and dashboard impact traversal did not have a reusable reference graph model.
- Current Update: clarified `workspace` as the public command surface, `Change` as the conceptual lifecycle name, and `sync` as internal/compatibility vocabulary; added an output contract registry with golden fixtures plus `make quality-output-contracts`; refreshed README opening copy and public naming checks; split user browse row loading into a focused module; added an internal reference graph used by dashboard impact reachability.
- Result: Rust tests, contract checks, docs-surface validation, HTML drift validation, AI workflow validation, and whitespace checks pass for the changed surfaces.

## 2026-04-13 - Reorganize Rust command modules
- State: Done
- Scope: Rust source module layout for command/subcommand directories, layered shared infrastructure, crate module wiring, maintainer docs, and Rust validation.
- Baseline: several command families still lived as root-level prefixed files under `rust/src/`, while shared transport/output/TUI helpers also lived as root singletons.
- Current Update: moved command families under `rust/src/commands/`, moved unified CLI internals under `rust/src/cli/`, split command-agnostic helpers under `rust/src/common/`, and moved Grafana transport/API integration under `rust/src/grafana/`. `lib.rs` keeps the public crate module names stable through explicit `#[path]` declarations.
- Result: Rust tests and formatting pass; public CLI behavior and docs contracts were not intentionally changed.

## 2026-04-13 - Add user browse membership removal
- State: Done
- Scope: access user browser team-membership rows, membership removal confirmation, user browse delete dialog consistency, and focused Rust regressions.
- Baseline: `access user browse` could expand a user to show team membership rows, but those rows were read-only. Operators had to switch to `access team browse` to remove a user from a team, and the user/team delete previews were still rendered inside the right facts pane instead of as confirmation dialogs.
- Current Update: expanded user team rows now preserve Grafana team ids, `r` and team-row `d` open a `Remove membership` confirmation dialog, and `y` removes the selected user from that team through `/api/teams/{team_id}/members/{user_id}` before refreshing back to the parent user. User delete and team delete/remove confirmations now render as centered dialogs.
- Result: team membership removal is available from both team-first and user-first browse flows without deleting the user account or the team.

## 2026-04-13 - Add team browse membership actions
- State: Done
- Scope: access team browser member-row actions, shared team browse footer/dialog presentation, focused Rust regressions, and worker-assisted implementation review.
- Baseline: selecting a team member row in `access team browse` could show membership detail, but `e` only told users to select a team row and there was no direct way from the member row to remove that relationship or change team-admin state. Team browse also still owned local footer/control and dialog presentation code while user browse had moved to shared TUI shell helpers.
- Current Update: member rows now keep user-owned fields read-only and direct account edits to `access user browse`; `r` and member-row `d` open a confirmation dialog before removing the selected team membership through the existing team modify flow; `a` grants or revokes team-admin state through the existing membership update path. Team-row `d` opens the whole-team delete confirmation dialog. Team browse footer controls now use the shared control grid/height helpers, and team edit/search/delete overlays use the shared dialog shell.
- Result: team browse can manage team/member relationships without pretending to edit user profile fields, and the browser presentation is closer to the shared TUI treatment already used by user browse.
