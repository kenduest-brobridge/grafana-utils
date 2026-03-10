# Repository Guidelines

## Project Structure & Module Organization

- `cmd/grafana-utils.py`: dashboard export/import CLI.
- `cmd/grafana-alert-utils.py`: alerting resource export/import CLI.
- `tests/`: unit tests for both entrypoints.
- `README.md`: GitHub-facing usage and operator examples.
- `DEVELOPER.md`: maintainer notes, internal behavior, and implementation tradeoffs.
- `docs/internal/ai-status.md` and `docs/internal/ai-changes.md`: internal change trace files for meaningful feature work.

Keep new code in the existing `cmd/` Python CLIs unless a new workflow clearly deserves its own script.

## Build, Test, and Development Commands

- `python3 cmd/grafana-utils.py export -h`: show dashboard CLI help.
- `python3 cmd/grafana-utils.py import -h`: show dashboard import help.
- `python3 cmd/grafana-alert-utils.py -h`: show alerting CLI help and examples.
- `python3 -m unittest -v`: run the full test suite.
- `python3 -m unittest -v tests/test_grafana_alert_utils.py`: run alerting tests only.
- `python3 -m unittest -v tests/test_dump_grafana_dashboards.py`: run dashboard tests only.

Run the smallest relevant test target first, then the full suite when behavior changes span both tools.

## Coding Style & Naming Conventions

- Target Python syntax compatible with RHEL 8 environments; keep scripts parseable by Python 3.6 grammar.
- Use 4-space indentation and standard library modules unless a dependency is clearly justified.
- Prefer descriptive snake_case for functions, variables, and test names.
- Keep CLI help text concrete and operator-focused.
- Use `apply_patch` for edits; do not rewrite files with ad hoc scripts.

## Testing Guidelines

- Tests use `unittest`.
- Name test files `tests/test_*.py` and test methods `test_*`.
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
