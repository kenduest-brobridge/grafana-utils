# ai-status-archive-2026-04-15

## 2026-04-13 - Fix access user browse TUI layout
- State: Done
- Scope: access user browser detail navigation, shared TUI footer/dialog sizing/rendering, user browser footer control layout, and focused Rust regressions.
- Baseline: `access user browse` facts navigation counted fewer user fact rows than the right pane rendered, so Down/End could not reach the final rows. The user browser footer also allocated four terminal rows while rendering three control rows plus a status line inside a bordered block, causing clipping and visual misalignment. The edit/search overlays each owned their own centering and frame style instead of using a common TUI dialog surface.
- Current Update: corrected the user facts line count, added shared `tui_shell::footer_height`, `centered_fixed_rect`, `dialog_block`, and `render_dialog_shell` helpers, made footer controls clip instead of wrapping across rows, switched user browse footer controls to the shared grid alignment helper, and moved user edit/search overlays onto the shared dialog shell.
- Result: the facts pane can select the final user fact row, the footer has enough height for its controls/status without rows overwriting each other, and user browse overlays now share the same centered dialog frame and background treatment.
