# Repository Guidelines

## Project Structure & Module Organization

- `grafana_utils/dashboard_cli.py`: packaged dashboard export/import implementation.
- `grafana_utils/alert_cli.py`: packaged alerting resource export/import implementation.
- `grafana_utils/http_transport.py`: shared replaceable HTTP transport layer.
- `cmd/grafana-utils.py`: thin wrapper for running the dashboard CLI directly from the repo checkout.
- `cmd/grafana-alert-utils.py`: thin wrapper for running the alerting CLI directly from the repo checkout.
- `pyproject.toml`: package metadata and console-script entrypoints.
- `tests/`: unit tests for both entrypoints.
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
- `grafana-utils export -h`: show installed dashboard CLI help.
- `grafana-utils import -h`: show installed dashboard import help.
- `grafana-utils diff -h`: show dashboard diff help.
- `grafana-alert-utils -h`: show installed alerting CLI help and examples.
- `python3 cmd/grafana-utils.py export -h`: show dashboard CLI help.
- `python3 cmd/grafana-utils.py import -h`: show dashboard import help.
- `python3 cmd/grafana-utils.py diff -h`: show dashboard diff help.
- `python3 cmd/grafana-alert-utils.py -h`: show alerting CLI help and examples.
- `python3 -m unittest -v`: run the full test suite.
- `python3 -m unittest -v tests/test_python_alert_cli.py`: run alerting Python tests only.
- `python3 -m unittest -v tests/test_python_dashboard_cli.py`: run dashboard Python tests only.

Run the smallest relevant test target first, then the full suite when behavior changes span both tools.

## Coding Style & Naming Conventions

- Target Python syntax compatible with RHEL 8 environments; keep scripts parseable by Python 3.6 grammar.
- Use 4-space indentation and standard library modules unless a dependency is clearly justified.
- Prefer descriptive snake_case for functions, variables, and test names.
- Keep CLI help text concrete and operator-focused.
- Use `apply_patch` for edits; do not rewrite files with ad hoc scripts.

## Testing Guidelines

- Tests use `unittest`.
- Name Python test files `tests/test_python_*.py` and test methods `test_*`.
- Keep Rust unit tests in `rust/src/*_rust_tests.rs` when the filename needs to distinguish them from Python tests.
- Add or update tests for every user-visible behavior change.
- For CLI UX changes, test parser behavior or `format_help()` output directly.

## Commit & Pull Request Guidelines

- Follow the existing commit style: short imperative subject lines such as `Extend Grafana alert utility mapping and templates`.
- Group related code, tests, and doc updates in the same commit.
- PRs should describe the operator-facing change, validation run, and any Grafana version assumptions.

## Documentation Policy

- Put external usage in `README.md`.
- Put internal details, mappings, fallback rules, and maintenance notes in `DEVELOPER.md`.
- Update `docs/internal/ai-status.md` and `docs/internal/ai-changes.md` only for meaningful behavior or architecture changes.
