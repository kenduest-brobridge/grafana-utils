.PHONY: help poetry-install poetry-lock poetry-test poetry-quality-python build build-python build-rust build-rust-macos-arm64 build-rust-linux-amd64 build-rust-linux-amd64-zig seed-grafana-sample-data destroy-grafana-sample-data reset-grafana-all-data test test-python test-rust fmt-rust-check lint-rust quality quality-python quality-rust test-rust-live test-access-live test-python-datasource-live

PYTHON ?= python3
PIP ?= $(PYTHON) -m pip
POETRY ?= poetry
CARGO ?= cargo
RUST_DIR ?= rust
PYTHON_DIST_DIR ?= dist

help:
	@printf '%s\n' \
		'Available targets:' \
		'  make poetry-install  Install the Poetry-managed development environment' \
		'  make poetry-lock   Refresh poetry.lock from pyproject.toml' \
		'  make poetry-test   Run the Python unittest suite inside Poetry' \
		'  make poetry-quality-python  Run Python quality checks inside Poetry' \
		'  make build         Build both Python and Rust artifacts' \
		'  make build-python  Build the Python wheel and sdist into dist/' \
		'  make build-rust    Build Rust release binaries in rust/target/release/' \
		'  make build-rust-macos-arm64  Build native macOS Apple Silicon Rust binaries plus tar.gz package into dist/macos-arm64/' \
		'  make build-rust-linux-amd64  Build Linux amd64 Rust binaries plus tar.gz package with Docker into dist/linux-amd64/' \
		'  make build-rust-linux-amd64-zig  Build Linux amd64 Rust binaries plus tar.gz package with local zig into dist/linux-amd64/' \
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
		'  make test-rust-live Start Grafana in Docker and run the Rust live smoke test' \
		'  make test-access-live Start Grafana in Docker and run the Python access live smoke test' \
		'  make test-python-datasource-live Start Grafana in Docker and run the Python datasource live smoke test'

poetry-install:
	$(POETRY) install --with dev

poetry-lock:
	$(POETRY) lock

poetry-test:
	$(POETRY) run $(PYTHON) -m unittest -v

poetry-quality-python:
	$(POETRY) run env PYTHON=python ./scripts/check-python-quality.sh

build: build-python build-rust

build-python:
	$(POETRY) run python -m build --sdist --wheel --no-isolation --outdir $(PYTHON_DIST_DIR) .

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

quality: quality-python quality-rust

quality-python:
	./scripts/check-python-quality.sh

quality-rust:
	./scripts/check-rust-quality.sh

test-rust-live:
	./scripts/test-rust-live-grafana.sh

test-access-live:
	./scripts/test-python-access-live-grafana.sh

test-python-datasource-live:
	bash ./scripts/test-python-datasource-live-grafana.sh
