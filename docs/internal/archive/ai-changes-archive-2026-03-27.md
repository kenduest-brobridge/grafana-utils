# ai-changes.md

Historical note:

- This archive captures the detailed AI-maintained change-log entries that were current through 2026-03-27.
- Keep the top-level `docs/internal/ai-changes.md` file limited to the latest active summaries.

## 2026-03-27 - Detailed Change Archive

The detailed entries previously kept in `docs/internal/ai-changes.md` on 2026-03-27 were archived here so the top-level file can stay short and current.

- Covered themes:
  - internal Rust refactors that preserved the public CLI and JSON contract surfaces
  - feature-specific test splits out of larger umbrella suites
  - validation notes for module splits and error-model cleanup
  - documentation maintenance around active architecture and cleanup work

- Representative archived changes:
  - split dashboard inspection query feature helpers
  - split datasource import/export helpers
  - split dashboard governance rule helpers
  - split access org import/export/diff helpers
  - split datasource routed import helpers
  - split dashboard CLI runtime helpers
  - split access user workflow helpers
  - gate sync shared TUI flows behind the `tui` feature
  - migrate selected dashboard and sync modules to the unified error model
  - earlier 2026-03-24 refactors for sync workbench, dashboard import, governance rules, and dashboard test splits

- Historical context:
  - the 2026-03-27 change log had become another rolling archive instead of a current-only summary
  - the active file now keeps only the current architecture direction and immediate follow-up
