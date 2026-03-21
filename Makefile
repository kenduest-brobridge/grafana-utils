.PHONY: help print-version sync-version set-release-version set-dev-version poetry-install poetry-lock poetry-test poetry-quality-python build build-python build-rust build-rust-native build-rust-macos-arm64 build-rust-linux-amd64 build-rust-linux-amd64-zig seed-grafana-sample-data destroy-grafana-sample-data reset-grafana-all-data test test-python test-rust fmt-rust-check lint-rust quality quality-python quality-rust quality-alert-rust quality-sync-rust test-rust-live test-sync-live test-alert-live test-alert-live-artifact test-alert-live-replay test-access-live test-python-datasource-live test-datasource-live

PYTHON ?= python3
PIP ?= $(PYTHON) -m pip
POETRY ?= poetry
CARGO ?= cargo
RUST_DIR ?= rust
PYTHON_DIST_DIR ?= dist
DEV_ITERATION ?= 1

help:
	@printf '%s\n' \
		'Available targets:' \
		'  make print-version  Show VERSION plus Python/Rust package versions' \
		'  make sync-version   Sync pyproject.toml, rust/Cargo.toml, and rust/Cargo.lock from VERSION' \
		'  make set-release-version VERSION=X.Y.Z  Set VERSION and package metadata to a release version' \
		'  make set-dev-version VERSION=X.Y.Z DEV_ITERATION=N  Set VERSION and package metadata to a preview version' \
		'  make poetry-install  Install the Poetry-managed development environment' \
		'  make poetry-lock   Refresh poetry.lock from pyproject.toml' \
		'  make poetry-test   Run the Python unittest suite inside Poetry' \
		'  make poetry-quality-python  Run Python quality checks inside Poetry' \
		'  make build         Build both Python and Rust artifacts' \
		'  make build-python  Build the Python wheel and sdist into dist/' \
		'  make build-rust    Build native Rust release binaries plus Linux amd64 artifacts' \
		'  make build-rust-native  Build native Rust release binaries in rust/target/release/' \
		'  make build-rust-macos-arm64  Build native macOS Apple Silicon (M1/M2/M3) Rust release binaries into dist/macos-arm64/' \
		'  make build-rust-linux-amd64  Build Linux amd64 Rust release binaries with Docker into dist/linux-amd64/ (containerized Linux build)' \
		'  make build-rust-linux-amd64-zig  Build Linux amd64 Rust release binaries with local zig into dist/linux-amd64/ (no Docker)' \
		'  make seed-grafana-sample-data  Seed a local Grafana with reusable developer sample orgs, datasources, folders, and dashboards' \
		'  make destroy-grafana-sample-data  Remove the developer sample orgs, datasources, folders, and dashboards seeded by the repo script' \
		'  make reset-grafana-all-data  Danger: delete repo-relevant test data from a disposable local Grafana instance' \
		'  make test          Run both Python and Rust test suites' \
		'  make test-python   Run the Python unittest suite' \
		'  make test-rust     Run the Rust cargo test suite' \
		'  make fmt-rust-check  Run cargo fmt --check' \
		'  make lint-rust     Run cargo clippy with warnings denied' \
		'  make quality       Run the repo quality gate scripts' \
		'  make quality-python  Run the Python quality gate script' \
		'  make quality-rust  Run the Rust quality gate script' \
		'  make quality-alert-rust  Run focused Rust alert contract checks' \
		'  make quality-sync-rust  Run focused Rust sync contract checks' \
		'  make test-rust-live Start Grafana in Docker and run the Rust live smoke test' \
		'  make test-sync-live Start Grafana in Docker and run the Rust sync live smoke path' \
		'  make test-alert-live Start Grafana in Docker and run the Rust alert live smoke path' \
		'  make test-alert-live-artifact Start Grafana in Docker and run the Rust alert artifact live smoke path' \
		'  make test-alert-live-replay Start Grafana in Docker and run the Rust alert replay live smoke path' \
		'  make test-access-live Start Grafana in Docker and run the Python access live smoke test' \
		'  make test-python-datasource-live Start Grafana in Docker and run the Python datasource live smoke test' \
		'  make test-datasource-live Start Grafana in Docker and run the Rust and Python datasource live smoke tests'

print-version:
	bash ./scripts/set-version.sh --print-current

sync-version:
	bash ./scripts/set-version.sh --sync-from-file

set-release-version:
	@test -n "$(VERSION)" || { echo "Usage: make set-release-version VERSION=X.Y.Z"; exit 1; }
	bash ./scripts/set-version.sh --version "$(VERSION)"

set-dev-version:
	@test -n "$(VERSION)" || { echo "Usage: make set-dev-version VERSION=X.Y.Z DEV_ITERATION=N"; exit 1; }
	bash ./scripts/set-version.sh --version "$(VERSION).dev$(DEV_ITERATION)"

poetry-install:
	$(POETRY) install --with dev

poetry-lock:
	$(POETRY) lock

poetry-test:
	$(POETRY) run $(PYTHON) -m unittest -v

poetry-quality-python:
	$(POETRY) run env PYTHON=python ./scripts/check-python-quality.sh

build: build-python build-rust
	@printf '%s\n' 'Build outputs:'
	@find $(PYTHON_DIST_DIR) -maxdepth 1 -type f \( -name '*.whl' -o -name '*.tar.gz' \) | sort
	@find $(RUST_DIR)/target/release -maxdepth 1 -type f -perm -111 | sort

build-python:
	$(POETRY) run python -m build --sdist --wheel --no-isolation --outdir $(PYTHON_DIST_DIR) .
	@printf '%s\n' 'Python build outputs:'
	@find $(PYTHON_DIST_DIR) -maxdepth 1 -type f \( -name '*.whl' -o -name '*.tar.gz' \) | sort

build-rust: build-rust-native build-rust-linux-amd64
	@printf '%s\n' 'Rust build outputs:'
	@printf '%s\n' "$(RUST_DIR)/target/release/grafana-util"
	@printf '%s\n' "dist/linux-amd64/grafana-util"

build-rust-native:
	cd $(RUST_DIR) && $(CARGO) build --release
	@printf '%s\n' 'Rust native build outputs:'
	@find $(RUST_DIR)/target/release -maxdepth 1 -type f -perm -111 | sort

build-rust-macos-arm64:
	bash ./scripts/build-rust-macos-arm64.sh

build-rust-linux-amd64:
	bash ./scripts/build-rust-linux-amd64.sh

build-rust-linux-amd64-zig:
	bash ./scripts/build-rust-linux-amd64-zig.sh

seed-grafana-sample-data:
	bash ./scripts/seed-grafana-sample-data.sh

destroy-grafana-sample-data:
	bash ./scripts/seed-grafana-sample-data.sh --destroy

reset-grafana-all-data:
	bash ./scripts/seed-grafana-sample-data.sh --reset-all-data --yes

test: test-python test-rust

test-python:
	$(PYTHON) -m unittest -v

test-rust:
	cd $(RUST_DIR) && $(CARGO) test

fmt-rust-check:
	cd $(RUST_DIR) && $(CARGO) fmt --check

lint-rust:
	cd $(RUST_DIR) && $(CARGO) clippy --all-targets -- -D warnings

quality: quality-python quality-rust

quality-python:
	./scripts/check-python-quality.sh

quality-rust:
	./scripts/check-rust-quality.sh

quality-alert-rust:
	cd $(RUST_DIR) && $(CARGO) test --quiet alert_
	cd $(RUST_DIR) && $(CARGO) test --quiet run_sync_cli_bundle_preserves_alert_export_artifact_metadata
	cd $(RUST_DIR) && $(CARGO) test --quiet build_sync_bundle_preflight_document_ignores_alert_replay_artifacts_but_keeps_zero_checks
	cd $(RUST_DIR) && $(CARGO) test --quiet render_sync_source_bundle_text_reports_alert_replay_artifact_counts
	cd $(RUST_DIR) && $(CARGO) fmt --check
	cd $(RUST_DIR) && $(CARGO) check --quiet

quality-sync-rust:
	cd $(RUST_DIR) && $(CARGO) test --quiet sync_
	cd $(RUST_DIR) && $(CARGO) test --quiet build_sync_source_bundle_document_matches_cross_domain_summary_contract
	cd $(RUST_DIR) && $(CARGO) test --quiet build_sync_source_bundle_document_preserves_alert_replay_artifact_summary_and_paths
	cd $(RUST_DIR) && $(CARGO) test --quiet build_sync_bundle_preflight_document_ignores_alert_replay_artifacts_but_keeps_zero_checks
	cd $(RUST_DIR) && $(CARGO) fmt --check
	cd $(RUST_DIR) && $(CARGO) check --quiet

test-rust-live:
	./scripts/test-rust-live-grafana.sh

test-sync-live:
	./scripts/test-rust-sync-live-grafana.sh

test-alert-live:
	./scripts/test-rust-alert-live-grafana.sh

test-alert-live-artifact:
	./scripts/test-rust-alert-artifact-live-grafana.sh

test-alert-live-replay:
	./scripts/test-rust-alert-replay-live-grafana.sh

test-access-live:
	./scripts/test-python-access-live-grafana.sh

test-python-datasource-live:
	bash ./scripts/test-python-datasource-live-grafana.sh

test-datasource-live:
	./scripts/test-combined-live-grafana.sh
