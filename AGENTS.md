# Repository Guidelines

## Repo Focus

- This repo is Rust-first. Treat `rust/src/` as the primary implementation and analysis surface, and prefer Rust for new work, code review, and architecture analysis unless the task is explicitly Python-specific.
- Python under `python/grafana_utils/` is the first-generation implementation and is now secondary; touch it only when required for necessary functionality, packaging/install behavior, or already-in-scope parity work.
- Never inspect Cargo build outputs under `rust/target` when performing code review or architecture analysis.

## Where To Look

- Start Rust work in `rust/src/`, with crate settings in `rust/Cargo.toml`.
- If Python context is required, start in `python/grafana_utils/`, `python/tests/`, and `python/pyproject.toml`.
- Maintainer routing and AI workflow guidance live in `docs/internal/maintainer-quickstart.md`, `docs/internal/ai-workflow-note.md`, and `docs/internal/task-brief-template.md`.
- Put operator-facing content in `README.md`, `README.zh-TW.md`, `docs/user-guide/`, and `docs/commands/`; put maintainer-only behavior and implementation tradeoffs in `docs/DEVELOPER.md`.
- Treat `docs/user-guide/{en,zh-TW}/` as handbook source, `docs/commands/{en,zh-TW}/` as command-reference source, and `docs/man/*.1` plus `docs/html/` as generated output.
- Update `scripts/contracts/command-surface.json` for public CLI path, legacy replacement, command-doc routing, or `--help-full` changes; update `scripts/contracts/docs-entrypoints.json` for landing quick command, jump-select, or handbook command-relationship changes.
- Update `docs/internal/ai-status.md` and `docs/internal/ai-changes.md` only for meaningful behavior or architecture changes.

## Working Rules

- Use `apply_patch` for file edits; do not rewrite tracked files with ad hoc scripts.
- Keep Python imports under `grafana_utils.*`.
- Target Python 3.9+ in touched Python code and prefer modern built-in generics.
- Use 4-space indentation and descriptive `snake_case` names.
- Keep CLI help and docs concrete and operator-focused.
- Before changing public CLI or doc surfaces, check the shared user journey: command family, docs layer, generated artifacts, tests, and extension path. Prefer shared taxonomy/rendering over per-command special cases, and treat cross-command inconsistency as an architecture issue to fix in the common layer when possible.
- When changing public CLI paths, help text, README snippets, handbook examples, or command docs, verify examples against `scripts/contracts/command-surface.json` and run `make quality-docs-surface`.
- Prefer the unified CLI shape in docs and examples:
  - `grafana-util dashboard ...`
  - `grafana-util alert ...`
  - `grafana-util access ...`
  - `grafana-util status ...`
  - `grafana-util config profile ...`
- Default Rust builds stay lean and do not include the `browser` feature unless the task explicitly targets the browser-enabled artifact lane.
- In Rust, use `///` only for public API surfaces and `//` for local implementation notes.
- Keep comments short and behavior-focused.
- When a screenshot or image task only needs text extraction, run OCR first with `tesseract <image> stdout -l chi_tra+eng`; if OCR quality is poor, preprocess with ImageMagick, for example `magick <image> -resize 200% -colorspace Gray <tmp-image>`.

## Model Routing

- Reserve highest-reasoning frontier runs, such as GPT-5.4 with xhigh/extreme reasoning, for changes that cross CLI behavior, artifact contracts, generated docs, tests, and real export compatibility at the same time.
- Use that heavier model tier for architecture decisions where a wrong assumption can ripple across command surface, schema contracts, workspace/review flows, or raw/prompt/provisioning lane semantics.
- Prefer normal GPT-5.4 reasoning or worker models for single-file Rust fixes, focused test cleanup, docs edits, generated-doc refreshes, and other narrow tasks with clear local context.
- When using workers, split by disjoint write scope and keep the main agent responsible for final integration, contract review, and validation.

## Validation And Commits

- Run the smallest relevant test target first, then broaden if the change crosses subsystems.
- Canonical validation commands: `cd rust && cargo test --quiet`, `cd python && PYTHONPATH=. poetry run python -m unittest -v tests`, and `make test`.
- Drift and generated-doc checks: `make quality-ai-workflow`, `make quality-docs-surface`, `make man-check`, and `make html-check`.
- Add or update tests for every user-visible behavior change.
- For CLI UX changes, test parser behavior or `format_help()` output directly.
- When changing handbook, command-reference, or docs-generator behavior, regenerate with `make man` and `make html` instead of editing generated output directly; when touching generated docs or maintainer/contract/architecture workflow docs, run `make quality-ai-workflow` or an equivalent narrow check.
- Default commit message format:
  - first line is a short imperative title with a type prefix such as `feature:`, `bugfix:`, `refactor:`, `docs:`, or `test:`
  - blank line
  - 2-4 flat `- ...` bullets with concrete code, test, or doc changes
