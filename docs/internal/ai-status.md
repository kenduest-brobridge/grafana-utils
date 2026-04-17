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
- Older entries moved to [`ai-status-archive-2026-04-15.md`](docs/internal/archive/ai-status-archive-2026-04-15.md).
- Older entries moved to [`ai-status-archive-2026-04-16.md`](docs/internal/archive/ai-status-archive-2026-04-16.md).

## 2026-04-16 - Add dashboard export layout repair
- State: Done
- Scope: Rust dashboard convert CLI surface, local export-layout repair planner/executor, dashboard export index metadata, focused parser/runtime/export tests, command docs/generated docs, command-surface contracts, and AI trace docs. README files, Python implementation, and provisioning layout repair are out of scope.
- Baseline: older dashboard exports can have correct `raw/folders.json` metadata while `raw/` and `prompt/` files are flattened under the leaf `folderTitle`; raw dashboard JSON may also omit `meta.folderUid`, so repair needs stable artifact metadata instead of relying on the dashboard payload alone.
- Current Update: Added `dashboard convert export-layout` with dry-run planning, copy-mode repair, in-place repair with backups, raw/prompt variant selection, folder inventory lookup, updated index paths, report-style summary text/table/csv output with `--show-operations` per-dashboard details, `extraFiles` reporting for unindexed files, case-insensitive/canonical path handling, and index-level `folderUid`/`folderPath` metadata for new exports.
- Result: Focused export-layout/dashboard export index tests, full Rust tests, clippy, docs surface, generated-doc checks, AI workflow, formatting, and whitespace checks pass. Real export smoke reports move=220, same=92, blocked=0, extra=0.

## 2026-04-16 - Mirror dashboard export folder paths
- State: Done
- Scope: Rust dashboard export path assembly, focused dashboard export tests, dashboard export command docs/help, and AI trace docs. README files, Python implementation, and provisioning layout are out of scope.
- Baseline: non-flat `dashboard export` writes raw and prompt files under the dashboard leaf `folderTitle`, even when Grafana reports a nested folder path through folder inventory. This can flatten distinct `Platform / Infra` and `Apps / Infra` folders into the same `Infra/` export directory.
- Current Update: Raw and prompt export paths now use the collected folder inventory full path when available; `--flat` and provisioning layout continue to use their previous behavior.
- Result: Focused dashboard export tests, full Rust tests, clippy, generated docs, docs-surface checks, formatting, AI workflow, and whitespace checks pass.

## 2026-04-15 - Make help-flat terminal-safe
- State: Done
- Scope: Rust `--help-flat` rendering and focused help regression tests. Runtime command behavior, public docs, README files, and Python implementation are out of scope.
- Baseline: `grafana-util --help-flat` renders a padded table with long purpose text, so narrow terminals wrap rows and make the flat command inventory hard to scan.
- Current Update: Changed the flat inventory to one public command path per line and removed KIND/PURPOSE columns so terminal output stays readable; detailed purpose and flags remain available through `<COMMAND> --help`.
- Result: Focused help tests and formatting pass; manual `--help-flat` smoke output now shows one command path per line without columns or ellipses.

## 2026-04-15 - Advertise help-flat in root help
- State: Done
- Scope: Rust grouped root/domain help, focused help regression tests, and command-surface contract metadata. Runtime command behavior, README files, and Python implementation are out of scope.
- Baseline: `grafana-util --help-flat` renders the flat public command inventory, but `grafana-util --help` only advertised `--help-full`. Follow-up review also found supported `access --help-full`, `workspace --help-full`, `workspace --help-schema`, and `dashboard summary --help-full` paths were not discoverable from their adjacent grouped help, while the command-surface contract incorrectly listed unsupported `dashboard --help-full`.
- Current Update: Added the missing help hints to root/dashboard/access/workspace grouped help, extended focused help regressions, and removed unsupported `dashboard` from `help_full_supported`.
- Result: Focused help tests, docs-surface contract checks, formatting, and CLI smoke checks pass.

## 2026-04-15 - Add Grafana instance metadata to status live
- State: Done
- Scope: Rust `status live` document assembly, focused status-live tests, operator docs, generated docs, and AI trace docs. Python implementation is out of scope.
- Baseline: `status live` returns project/domain readiness fields but does not read `/api/health`, so JSON/YAML output cannot show Grafana instance fields such as version, commit, or database health.
- Current Update: Added an additive `discovery.instance` section populated from `GET /api/health`; failed health reads are recorded as non-blocking discovery metadata instead of changing domain readiness.
- Result: Focused status tests, full Rust tests, clippy, docs surface, generated-doc checks, AI workflow, and whitespace checks pass. Source command/reference docs and generated man/html docs now describe the live instance metadata shape.

## 2026-04-15 - Fix stale dashboard command references
- State: Done
- Scope: Public docs and maintainer guidance that still present removed dashboard command names or stale inspect-artifact wording. Python implementation is out of scope.
- Baseline: `grafana-util dashboard analyze` and `grafana-util dashboard inspect-export` are rejected by the CLI, but README and maintainer docs still mention them as command paths.
- Current Update: Replaced README dependency examples with `dashboard summary`, updated policy/about artifact wording to dashboard summary JSON artifacts, refreshed generated man/html docs, and clarified maintainer docs that `inspect` is an internal artifact flow rather than a public dashboard command.
- Result: CLI smoke checks confirm `dashboard analyze` and `dashboard inspect-export` are rejected while `dashboard summary` is accepted. Docs, generated-doc, Rust help, AI workflow, and whitespace checks pass.
