# Unit Test Inventory

This is a maintainer-only summary of the validation surfaces kept in the repository.

## Python test suites

- `python/tests/test_python_dashboard_cli.py`
- `python/tests/test_python_dashboard_inspection_cli.py`
- `python/tests/test_python_datasource_cli.py`
- `python/tests/test_python_alert_cli.py`
- `python/tests/test_python_access_cli.py`
- `python/tests/test_python_packaging.py`

## Rust test suites

- `rust/src/dashboard/rust_tests.rs`
- `rust/src/datasource_rust_tests.rs`
- `rust/src/alert_rust_tests.rs`
- `rust/src/access_rust_tests.rs`
- `rust/src/sync/*_rust_tests.rs`

## Common validation commands

- `PYTHONPATH=python python3 -m unittest -v`
- `cd rust && cargo test --quiet`
- `make quality-python`
- `make quality-rust`
- `make test`

## Usage

- Use the Python suites when checking parity, regressions, or legacy workflow behavior.
- Use the Rust suites for the maintained runtime and release-blocking validation.
- When a feature spans both implementations, update both the Python and Rust references here if the test entrypoints change materially.
