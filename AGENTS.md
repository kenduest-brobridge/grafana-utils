# Repository Guidelines

## Project Structure & Module Organization

- `python/grafana_utils/dashboard_cli.py`: packaged dashboard implementation.
- `python/grafana_utils/alert_cli.py`: packaged alerting implementation.
- `python/grafana_utils/access_cli.py`: packaged access-management implementation.
- `python/grafana_utils/access/parser.py`: Python access CLI argparse definitions.
- `python/grafana_utils/access/workflows.py`: Python access user, org, team, and service-account workflows.
- `python/grafana_utils/unified_cli.py`: unified Python CLI dispatcher.
- `python/grafana_utils/http_transport.py`: shared replaceable HTTP transport layer.
- `python/grafana_utils/__main__.py`: source-tree module entrypoint for running the unified CLI directly from the repo checkout.
- `pyproject.toml`: package metadata and console-script entrypoints.
- `rust/src/`: Rust implementation for dashboard, alerting, access, and unified dispatch.
- `rust/src/access_org.rs`: Rust access org list/add/modify/delete/export/import implementation.
- `python/tests/`: Python unit tests.
- `Makefile`: root shortcuts for Python wheel builds, Rust release builds, and test runs.
- `README.md`: GitHub-facing usage and operator examples.
- `docs/DEVELOPER.md`: maintainer notes, internal behavior, and implementation tradeoffs.
- `docs/internal/ai-status.md` and `docs/internal/ai-changes.md`: internal change trace files for meaningful feature work.

Keep Python implementation and source-tree entrypoints under `python/grafana_utils/` so the repo uses one clear parent path for Python code. Python imports remain `grafana_utils.*`.

## Build, Test, and Development Commands

- `poetry install --with dev`: create the standard Python development environment for this repo.
- `PYTHONPATH=python poetry run python -m unittest -v`: run the full Python test suite from the Poetry-managed environment.
- `PYTHONPATH=python poetry run python -m unittest -v python/python/tests/test_python_alert_cli.py`: run alerting Python tests only from the Poetry-managed environment.
- `PYTHONPATH=python poetry run python -m unittest -v python/python/tests/test_python_dashboard_cli.py`: run dashboard Python tests only from the Poetry-managed environment.
- `PYTHONPATH=python poetry run python -m unittest -v python/python/tests/test_python_access_cli.py`: run access Python tests only from the Poetry-managed environment.
- `python3 -m pip install .`: install the package into the active Python environment.
- `python3 -m pip install --user .`: install the package into the current user's Python environment.
- `python3 -m pip install '.[http2]'`: install the optional HTTP/2 transport dependencies on Python 3.9+.
- `make build-python`: build the Python wheel and sdist into `dist/`.
- `make build-rust`: build Rust release binaries into `rust/target/release/`.
- `make build`: build both the Python wheel and the Rust release binaries.
- `make test`: run both the Python and Rust test suites.
- `make test-rust-live`: start Docker Grafana and run the Rust live smoke test script.
- `PYTHONPATH=python python3 -m unittest -v`: run the full test suite.
- `PYTHONPATH=python python3 -m unittest -v python/python/tests/test_python_alert_cli.py`: run alerting Python tests only.
- `PYTHONPATH=python python3 -m unittest -v python/python/tests/test_python_dashboard_cli.py`: run dashboard Python tests only.
- `PYTHONPATH=python python3 -m unittest -v python/python/tests/test_python_access_cli.py`: run access Python tests only.
- `cd rust && cargo test --quiet`: run the full Rust test suite.

Use Poetry-first commands for Python development and test execution. Keep the `pip install` commands for packaged-install validation, local release checks, or environments that intentionally skip Poetry.

Run the smallest relevant test target first, then the full suite when behavior changes span both tools.

For external command usage and operator examples, prefer `README.md`, `README.zh-TW.md`, `docs/user-guide.md`, and `docs/user-guide-TW.md` instead of expanding usage examples here.

## Versioning And Release Policy

- Treat `dev` as the preview branch and `main` as the release branch.
- Day-to-day work on `dev` does not need a mandatory `.devN` / `-dev.N` package version.
- `dev` and `main` may carry the same plain checked-in package version between releases to reduce merge noise.
- Use preview versions such as `X.Y.Z.devN` / `X.Y.Z-dev.N` only when a branch-specific preview artifact is intentionally needed.
- On `main`, formal release tags and release artifacts must use plain release versions `X.Y.Z` with no dev suffix.
- Formal releases must use Git tags in the form `vX.Y.Z`, created from `main`.
- Release tags must match the plain release version already present in both `pyproject.toml` and `rust/Cargo.toml`.
- Preview GitLab artifacts come from the `dev` and `main` branches; release GitLab artifacts come only from `vX.Y.Z` tags.
- When changing versions, update Python and Rust package metadata together and keep the branch/tag policy above consistent.

## Coding Style & Naming Conventions

- Target Python 3.9+ syntax and runtime behavior. Prefer Python 3.9 built-in generics in touched code and do not preserve Python 3.6-era syntax constraints.
- Use 4-space indentation and standard library modules unless a dependency is clearly justified.
- Prefer descriptive snake_case for functions, variables, and test names.
- Keep CLI help text concrete and operator-focused.
- Use `apply_patch` for edits; do not rewrite files with ad hoc scripts.
- Prefer the unified CLI shape in docs and examples:
  - `grafana-util dashboard ...`
  - `grafana-util alert ...`
  - `grafana-util access ...`

## Commenting Requirements

- Use Python `#` comments for implementation notes and `"""` docstrings only when they help external-facing readers understand function/module intent.
- In Rust:
  - Use `///` only for public API surfaces (or items you want in `rustdoc`) and place them immediately above the item declaration.
  - Use `//` inside private function bodies for local logic notes.
  - Do not place `///` inside function bodies.
- Keep comments short and behavior-focused so maintainers and agents can trace decisions quickly.

## Testing Guidelines

- Tests use `unittest`.
- Name Python test files `python/python/tests/test_python_*.py` and test methods `test_*`.
- Keep Rust unit tests in `rust/src/*_rust_tests.rs` when the filename needs to distinguish them from Python tests.
- Add or update tests for every user-visible behavior change.
- For CLI UX changes, test parser behavior or `format_help()` output directly.

## Agent Routing

- Planner tasks should use `gpt-5.4`.
- General worker tasks should default to `gpt-5.4-mini` when the implementation slice is clear, bounded, and local.
- Use `gpt-5.4-mini` for clear, narrow, execution-heavy tasks where the main risk is implementation throughput, not reasoning depth.
- Use `gpt-5.4` with `medium` reasoning for bounded tasks that still require meaningful judgment, such as non-trivial coding, targeted debugging, design refinement inside known architecture, or moderate cross-file coordination.
- Use `gpt-5.4` with `high` reasoning for work that is open-ended, cross-cutting, assumption-heavy, or sensitive to edge cases, correctness, or operational risk.
- Reserve ultra-high reasoning only for rare, high-impact decisions under heavy ambiguity, deep architectural coupling, or unusually high correctness pressure.
- Do not force smaller models onto work where the main risk is choosing the wrong approach rather than writing the code itself.
- Validation tasks should use `gpt-5.4` with `high` reasoning.
- Bulk or repetitive tasks should prefer `gpt-5.4-mini`.

## Worker Delegation

- Write a short mini-spec before delegating: goal, owned files, non-goals, acceptance criteria, and focused validation commands.
- Delegate subagent work only after the spec is concrete enough that ownership, constraints, and success conditions are explicit.
- Give workers only the minimum context needed for the assigned slice; do not pass full thread history by default.
- Pass only the minimum context needed for correctness. Do not forward broad background material, large unrelated diffs, or the full conversation by default.
- Keep architecture, schema/contract changes, risky runtime wiring, migrations, and cross-language parity decisions on the main agent unless a stronger worker is clearly justified.
- Keep the main agent responsible for architecture, shared invariants, migrations, contract changes, and other high-risk integration points unless delegation is clearly safer or more efficient.
- Choose worker model and reasoning level by ambiguity, coupling, verification burden, and failure cost, not by task label alone.
- Prefer one worker per disjoint write scope. Avoid overlapping ownership unless the work is intentionally sequential.
- Provide explicit ownership so each worker knows exactly which files, modules, or questions it owns.
- Prefer smaller workers for bounded implementation slices when the spec is complete and the write scope is local.
- Good worker slices in this repo are local module work, CLI/parser/help updates, renderer or TUI slices, focused tests, and repetitive repo tasks.
- Poor worker slices in this repo are broad Rust/Python parity efforts, sync/apply semantics across resource kinds, and contract redesign without a settled main-agent plan.
- Reuse an existing worker only when its current context is still relevant and the ownership boundary remains clear.
- Close workers promptly when their task is complete, superseded, blocked beyond usefulness, or no longer needed. Do not leave idle workers running.
- Review worker output before integrating it into the main line of work, especially for changes that affect shared contracts, runtime behavior, or long-term maintainability.
- The main agent owns final architecture decisions, cross-slice integration, final validation, and commits.

## Commit & Pull Request Guidelines

- Default commit message format for agents is:
  - first line: short imperative title with a type prefix such as `feature:`, `bugfix:`, `refactor:`, `docs:`, or `test:`
  - blank line
  - flat `- ...` sub-items with concrete details
  - do not insert empty blank lines between detail bullets
- Prefer 2-4 detail bullets that describe the main code, test, or doc changes in the commit.
- Example:
  - `refactor: split Rust dashboard module internals`
  - blank line
  - `- Extract dashboard CLI definitions, list rendering, and export orchestration into dedicated modules.`
  - `- Keep the existing crate::dashboard public API stable through re-exports.`
  - `- Record the refactor in maintainer docs and revalidate the full Rust suite.`
- Group related code, tests, and doc updates in the same commit.
- PRs should describe the operator-facing change, validation run, and any Grafana version assumptions.

## Documentation Policy

- Put external usage in `README.md`.
- Put internal details, mappings, fallback rules, and maintenance notes in `docs/DEVELOPER.md`.
- Update `docs/internal/ai-status.md` and `docs/internal/ai-changes.md` only for meaningful behavior or architecture changes.
- When updating `docs/user-guide.md` or `docs/user-guide-TW.md`, prefer real command lines and output excerpts captured from a local Docker Grafana run over illustrative placeholders. If a documented example claims to be validated, it should match an actually executed local live-smoke path and mention the Grafana version when that context matters.
