.PHONY: help build build-python build-rust build-rust-macos-arm64 build-rust-linux-amd64 build-rust-linux-amd64-zig seed-grafana-sample-data destroy-grafana-sample-data reset-grafana-all-data test test-python test-rust fmt-rust-check lint-rust quality test-rust-live test-access-live

PYTHON ?= python3
PIP ?= $(PYTHON) -m pip
CARGO ?= cargo
RUST_DIR ?= rust
PYTHON_DIST_DIR ?= dist

help:
	@printf '%s\n' \
		'Available targets:' \
		'  make build         Build both Python and Rust artifacts' \
		'  make build-python  Build the Python wheel into dist/' \
		'  make build-rust    Build Rust release binaries in rust/target/release/' \
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
		'  make quality       Run the basic repo quality gates' \
		'  make test-rust-live Start Grafana in Docker and run the Rust live smoke test' \
		'  make test-access-live Start Grafana in Docker and run the Python access live smoke test'

build: build-python build-rust

build-python:
	$(PIP) wheel --no-deps --no-build-isolation --wheel-dir $(PYTHON_DIST_DIR) .

build-rust:
	cd $(RUST_DIR) && $(CARGO) build --release

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

quality: test-python test-rust fmt-rust-check lint-rust

test-rust-live:
	./scripts/test-rust-live-grafana.sh

test-access-live:
	./scripts/test-python-access-live-grafana.sh
