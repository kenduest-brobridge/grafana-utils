# Repository Guidelines

## Repo Focus

- This repo is Rust-first. Treat `rust/src/` as the primary implementation and analysis surface.
- Python under `python/grafana_utils/` is the first-generation implementation and is now secondary.
- Prefer Rust for new work, code review, and architecture analysis unless the task is explicitly Python-specific.
- Touch Python only when required for necessary functionality, packaging/install behavior, or already-in-scope parity work.
- Never inspect Cargo build outputs under `rust/target` when performing code review or architecture analysis.

## Where To Look

- Start Rust work in `rust/src/`, with crate settings in `rust/Cargo.toml`.
- If Python context is required, start in `python/grafana_utils/`, `python/tests/`, and `python/pyproject.toml`.
- First-entry maintainer routing lives in `docs/internal/maintainer-quickstart.md`.
- Put external/operator usage in `README.md`, `README.zh-TW.md`, `docs/user-guide/`, and `docs/commands/`.
- Put maintainer behavior, internal mappings, and implementation tradeoffs in `docs/DEVELOPER.md`.
- Treat `docs/user-guide/{en,zh-TW}/` as the handbook source layer.
- Treat `docs/commands/{en,zh-TW}/` as the command-reference source layer.
- Treat `docs/man/*.1` and `docs/html/` as generated artifacts, not primary edit targets.
- Update `docs/internal/ai-status.md` and `docs/internal/ai-changes.md` only for meaningful behavior or architecture changes.

## Working Rules

- Use `apply_patch` for file edits; do not rewrite tracked files with ad hoc scripts.
- Keep Python imports under `grafana_utils.*`.
- Target Python 3.9+ in touched Python code and prefer modern built-in generics.
- Use 4-space indentation and descriptive `snake_case` names.
- Keep CLI help and docs concrete and operator-focused.
- Prefer the unified CLI shape in docs and examples:
  - `grafana-util dashboard ...`
  - `grafana-util alert ...`
  - `grafana-util access ...`
- Rust profile credentials now support `file`, `os`, and `encrypted-file` secret modes.
- OS-backed profile secret storage is supported on macOS and Linux only.
- Default Rust builds stay lean and do not include the `browser` feature unless the task explicitly targets the browser-enabled artifact lane.
- In Rust, use `///` only for public API surfaces and `//` for local implementation notes.
- Keep comments short and behavior-focused.

## Validation And Commits

- Run the smallest relevant test target first, then broaden if the change crosses subsystems.
- Canonical Rust test command: `cd rust && cargo test --quiet`
- Canonical Python test command: `cd python && PYTHONPATH=. poetry run python -m unittest -v tests`
- Combined validation entry point: `make test`
- Generated-doc validation entry points: `make man-check` and `make html-check`
- When changing handbook, command-reference, or docs-generator behavior, regenerate with `make man` and `make html` instead of editing generated output only.
- Add or update tests for every user-visible behavior change.
- For CLI UX changes, test parser behavior or `format_help()` output directly.
- Default commit message format:
  - first line is a short imperative title with a type prefix such as `feature:`, `bugfix:`, `refactor:`, `docs:`, or `test:`
  - blank line
  - 2-4 flat `- ...` bullets with concrete code, test, or doc changes
