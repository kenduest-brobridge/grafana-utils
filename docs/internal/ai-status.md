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

## 2026-04-13 - Fix access user browse TUI layout
- State: Done
- Scope: access user browser detail navigation, shared TUI footer/dialog sizing/rendering, user browser footer control layout, and focused Rust regressions.
- Baseline: `access user browse` facts navigation counted fewer user fact rows than the right pane rendered, so Down/End could not reach the final rows. The user browser footer also allocated four terminal rows while rendering three control rows plus a status line inside a bordered block, causing clipping and visual misalignment. The edit/search overlays each owned their own centering and frame style instead of using a common TUI dialog surface.
- Current Update: corrected the user facts line count, added shared `tui_shell::footer_height`, `centered_fixed_rect`, `dialog_block`, and `render_dialog_shell` helpers, made footer controls clip instead of wrapping across rows, switched user browse footer controls to the shared grid alignment helper, and moved user edit/search overlays onto the shared dialog shell.
- Result: the facts pane can select the final user fact row, the footer has enough height for its controls/status without rows overwriting each other, and user browse overlays now share the same centered dialog frame and background treatment.

## 2026-04-13 - Improve CLI help command emphasis
- State: Done
- Scope: Rust unified CLI grouped help footer, shared help color palette, CLI help colorization helpers, and focused help regressions.
- Baseline: root `grafana-util --help` ended with the vague label `First 3 commands:` and included completion setup as one of the first commands. Colored help also split terminal styling between Clap's `CLI_HELP_STYLES` and the custom help-example palette, so Clap-rendered `Commands:` entries such as `browse` stayed blue while custom `grafana-util ...` examples could be bright white.
- Current Update: changed the root quick-start footer and full-help section to `Suggested flow:`, aligned the suggested commands to version, read-only status, and profile setup, moved terminal help color ownership into `help_styles`, set Clap literal styling and custom command rendering to the same bright-white command treatment, and routed full command lines plus grouped `Usage:` command syntax through a shared CLI-command detector.
- Result: CLI help now presents a workflow-oriented footer, and command syntax/entries are highlighted consistently across Clap-generated help and custom Rust help renderers.
