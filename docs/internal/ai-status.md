# ai-status.md

Current AI-maintained status only.

- Older trace history moved to [`archive/ai-status-archive-2026-03-24.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-03-24.md).
- Detailed 2026-03-27 entries moved to [`archive/ai-status-archive-2026-03-27.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-03-27.md).
- Detailed 2026-03-28 task notes were condensed into [`archive/ai-status-archive-2026-03-28.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-03-28.md).
- Detailed 2026-03-29 through 2026-03-31 entries moved to [`archive/ai-status-archive-2026-03-31.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-03-31.md).
- Detailed 2026-04-01 through 2026-04-12 entries moved to [`archive/ai-status-archive-2026-04-12.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-04-12.md).
- Keep this file short and current. Additive historical detail belongs in `docs/internal/archive/`.
- Older entries moved to [`ai-status-archive-2026-04-13.md`](/Users/kendlee/work/grafana-utils/docs/internal/archive/ai-status-archive-2026-04-13.md).

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

## 2026-04-13 - Reduce sync maintainability hotspots
- State: Done
- Scope: sync bundle preflight, promotion preflight, workspace discovery rules, source-bundle input loading, Rust maintainability reporting, and architecture guardrail notes.
- Baseline: `sync/bundle_preflight.rs`, `sync/promotion_preflight.rs`, `sync/workspace_discovery.rs`, and `sync/bundle_inputs.rs` mixed document assembly, mapping/discovery rules, rendering, file loading, and normalization helpers in large files; the maintainability reporter listed only file-level findings, so domain-level sync growth was harder to see.
- Current Update: split bundle preflight assessments, promotion preflight checks/mapping/rendering, workspace discovery path rules, and source-bundle input loading into focused modules; converted alert artifact, promotion remap, alert export section, and alert sync-kind differences into small rule/spec structures instead of scattered per-case branches; added a shared source-bundle input pipeline and directory summaries in the maintainability reporter.
- Result: public CLI and JSON/text contracts are unchanged; focused sync tests, reporter tests, formatting, and static checks pass locally. The remaining sync hotspots are now other production/test domains rather than the preflight/discovery/bundle-input facades.

## 2026-04-13 - Add GitHub installer completion option
- State: Done
- Scope: GitHub install script, install README snippets, getting-started docs, completion command docs, generated HTML output, and installer coverage.
- Baseline: the GitHub install path installed only the binary; shell completion had to be installed manually in a separate README section after installation.
- Current Update: added `INSTALL_COMPLETION=auto|bash|zsh`, `COMPLETION_DIR`, and `--interactive` support to the installer; refactored the installer into maintainable helper stages; added `make test-installer-local` for GitHub-free local archive smoke testing; documented the correct GitHub pipe usage; and refreshed generated HTML docs.
- Result: users can opt in to installing Bash/Zsh completion from the just-installed binary, or run `sh -s -- --interactive` after the pipe to answer install-directory and completion prompts from the terminal. Maintainers can verify the release-style install path locally without downloading from GitHub.
