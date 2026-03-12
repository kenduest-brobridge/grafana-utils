# Repository Guidelines

## Project Structure & Module Organization

- `grafana_utils/dashboard_cli.py`: packaged dashboard implementation.
- `grafana_utils/alert_cli.py`: packaged alerting implementation.
- `grafana_utils/access_cli.py`: packaged access-management implementation.
- `grafana_utils/unified_cli.py`: unified Python CLI dispatcher.
- `grafana_utils/http_transport.py`: shared replaceable HTTP transport layer.
- `cmd/grafana-utils.py`: thin wrapper for running the unified CLI directly from the repo checkout.
- `pyproject.toml`: package metadata and console-script entrypoints.
- `rust/src/`: Rust implementation for dashboard, alerting, access, and unified dispatch.
- `tests/`: Python unit tests.
- `Makefile`: root shortcuts for Python wheel builds, Rust release builds, and test runs.
- `README.md`: GitHub-facing usage and operator examples.
- `DEVELOPER.md`: maintainer notes, internal behavior, and implementation tradeoffs.
- `docs/internal/ai-status.md` and `docs/internal/ai-changes.md`: internal change trace files for meaningful feature work.

Keep implementation code in `grafana_utils/` and keep `cmd/` wrappers thin unless a new workflow clearly deserves its own module.

## Build, Test, and Development Commands

- `python3 -m pip install .`: install the package into the active Python environment.
- `python3 -m pip install --user .`: install the package into the current user's Python environment.
- `python3 -m pip install '.[http2]'`: install the optional HTTP/2 transport dependencies on Python 3.8+.
- `make build-python`: build the Python wheel into `dist/`.
- `make build-rust`: build Rust release binaries into `rust/target/release/`.
- `make build`: build both the Python wheel and the Rust release binaries.
- `make test`: run both the Python and Rust test suites.
- `make test-rust-live`: start Docker Grafana and run the Rust live smoke test script.
- `grafana-utils -h`: show installed unified CLI help.
- `python3 cmd/grafana-utils.py -h`: show unified source-tree CLI help.
- `python3 cmd/grafana-utils.py dashboard list -h`: show dashboard list help.
- `python3 cmd/grafana-utils.py alert -h`: show alerting help.
- `python3 cmd/grafana-utils.py access user list -h`: show access-management help.
- `python3 -m unittest -v`: run the full test suite.
- `python3 -m unittest -v tests/test_python_alert_cli.py`: run alerting Python tests only.
- `python3 -m unittest -v tests/test_python_dashboard_cli.py`: run dashboard Python tests only.
- `python3 -m unittest -v tests/test_python_access_cli.py`: run access Python tests only.
- `cd rust && cargo test --quiet`: run the full Rust test suite.

Run the smallest relevant test target first, then the full suite when behavior changes span both tools.

## Coding Style & Naming Conventions

- Target Python syntax compatible with RHEL 8 environments; keep scripts parseable by Python 3.6 grammar.
- Use 4-space indentation and standard library modules unless a dependency is clearly justified.
- Prefer descriptive snake_case for functions, variables, and test names.
- Keep CLI help text concrete and operator-focused.
- Use `apply_patch` for edits; do not rewrite files with ad hoc scripts.
- Prefer the unified CLI shape in docs and examples:
  - `grafana-utils dashboard ...`
  - `grafana-utils alert ...`
  - `grafana-utils access ...`

## Testing Guidelines

- Tests use `unittest`.
- Name Python test files `tests/test_python_*.py` and test methods `test_*`.
- Keep Rust unit tests in `rust/src/*_rust_tests.rs` when the filename needs to distinguish them from Python tests.
- Add or update tests for every user-visible behavior change.
- For CLI UX changes, test parser behavior or `format_help()` output directly.

## Commit & Pull Request Guidelines

- Default commit message format for agents is:
  - first line: short imperative title
  - blank line
  - flat `- ...` sub-items with concrete details
- Prefer 2-4 detail bullets that describe the main code, test, or doc changes in the commit.
- Example:
  - `Split Rust dashboard module internals`
  - blank line
  - `- Extract dashboard CLI definitions, list rendering, and export orchestration into dedicated modules.`
  - `- Keep the existing crate::dashboard public API stable through re-exports.`
  - `- Record the refactor in maintainer docs and revalidate the full Rust suite.`
- Group related code, tests, and doc updates in the same commit.
- PRs should describe the operator-facing change, validation run, and any Grafana version assumptions.

## Documentation Policy

- Put external usage in `README.md`.
- Put internal details, mappings, fallback rules, and maintenance notes in `DEVELOPER.md`.
- Update `docs/internal/ai-status.md` and `docs/internal/ai-changes.md` only for meaningful behavior or architecture changes.
