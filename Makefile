.PHONY: help poetry-install poetry-lock poetry-test poetry-quality-python version-show sync-version set-version version-dev version-release git-tag release-tag build build-python build-rust build-rust-macos-arm64 build-rust-linux-amd64 build-rust-linux-amd64-zig seed-grafana-sample-data verify-grafana-sample-data destroy-grafana-sample-data reset-grafana-all-data test test-python test-rust fmt-rust-check lint-rust quality quality-python quality-rust test-rust-live test-access-live test-python-datasource-live

PYTHON ?= python3
PIP ?= $(PYTHON) -m pip
POETRY ?= poetry
CARGO ?= cargo
RUST_DIR ?= rust
PYTHON_DIST_DIR ?= dist
RUST_TARGET_DIR ?= $(CURDIR)/build/rust

help:
	@printf '%s\n' \
		'Available targets:' \
		'  make poetry-install  Install the Poetry-managed development environment' \
		'  make poetry-lock   Refresh poetry.lock from pyproject.toml' \
		'  make poetry-test   Run the Python unittest suite inside Poetry' \
		'  make poetry-quality-python  Run Python quality checks inside Poetry' \
		'  make version-show  Print the canonical VERSION file plus Python/Rust source versions' \
		'  make sync-version  Sync pyproject.toml and rust/Cargo.toml from the VERSION file' \
		'  make set-version VERSION=0.2.9.dev1  Update VERSION and sync Python/Rust source versions (also accepts TAG=v0.2.9)' \
		'  make version-dev VERSION=0.2.9.dev1  Validate and set preview versions for dev branch work' \
		'  make version-release VERSION=0.2.9  Validate and set release versions for main/release prep' \
		'  make git-tag TAG=v0.2.9  Create a local annotated git tag after version files are ready' \
		'  make release-tag VERSION=0.2.9  Verify source versions match and create v0.2.9' \
		'  make build         Build both Python and Rust artifacts' \
		'  make build-python  Build the Python wheel and sdist into dist/' \
		'  make build-rust    Build Rust release binaries in build/rust/release/' \
		'  make build-rust-macos-arm64  Build native macOS Apple Silicon (M1/M2/M3) Rust release binaries into dist/macos-arm64/' \
		'  make build-rust-linux-amd64  Build Linux amd64 Rust release binaries with Docker into dist/linux-amd64/ (containerized Linux build)' \
		'  make build-rust-linux-amd64-zig  Build Linux amd64 Rust release binaries with local zig into dist/linux-amd64/ (no Docker)' \
		'  make seed-grafana-sample-data  Seed a local Grafana with reusable developer sample orgs, users, teams, service accounts, datasources, folders, and dashboards' \
		'  make verify-grafana-sample-data  Verify that the expected developer sample data already exists in a local Grafana' \
		'  make destroy-grafana-sample-data  Remove the developer sample orgs, users, teams, service accounts, datasources, folders, and dashboards seeded by the repo script' \
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

version-show:
	bash ./scripts/set-version.sh --print-current

sync-version:
	bash ./scripts/set-version.sh --sync-from-file

set-version:
	@if [ -n "$(VERSION)" ]; then \
		bash ./scripts/set-version.sh --version "$(VERSION)"; \
	elif [ -n "$(TAG)" ]; then \
		bash ./scripts/set-version.sh --tag "$(TAG)"; \
	else \
		echo "Error: set-version requires VERSION=... or TAG=..."; \
		exit 1; \
	fi

version-dev:
	@if [ -z "$(VERSION)" ]; then \
		echo "Error: version-dev requires VERSION=X.Y.Z.devN"; \
		exit 1; \
	fi
	@if ! printf '%s\n' "$(VERSION)" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+\.dev[0-9]+$$'; then \
		echo "Error: version-dev requires Python dev format X.Y.Z.devN"; \
		exit 1; \
	fi
	bash ./scripts/set-version.sh --version "$(VERSION)"

version-release:
	@if [ -z "$(VERSION)" ]; then \
		echo "Error: version-release requires VERSION=X.Y.Z"; \
		exit 1; \
	fi
	@if ! printf '%s\n' "$(VERSION)" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$$'; then \
		echo "Error: version-release requires release format X.Y.Z"; \
		exit 1; \
	fi
	bash ./scripts/set-version.sh --version "$(VERSION)"

git-tag:
	@if [ -z "$(TAG)" ]; then \
		echo "Error: git-tag requires TAG=vX.Y.Z"; \
		exit 1; \
	fi
	git tag -a "$(TAG)" -m "Release $(TAG)"

release-tag:
	@if [ -z "$(VERSION)" ]; then \
		echo "Error: release-tag requires VERSION=X.Y.Z"; \
		exit 1; \
	fi
	@if ! printf '%s\n' "$(VERSION)" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$$'; then \
		echo "Error: release-tag requires release format X.Y.Z"; \
		exit 1; \
	fi
	@CANONICAL="$$(tr -d '[:space:]' < VERSION)"; \
	PYTHON_VERSION="$$(sed -n 's/^version = \"\\(.*\\)\"$$/\\1/p' pyproject.toml | head -n 1)"; \
	RUST_VERSION="$$(sed -n 's/^version = \"\\(.*\\)\"$$/\\1/p' rust/Cargo.toml | head -n 1)"; \
	if [ "$$CANONICAL" != "$(VERSION)" ]; then \
		echo "Error: VERSION file is $$CANONICAL but expected $(VERSION)"; \
		exit 1; \
	fi; \
	if [ "$$PYTHON_VERSION" != "$(VERSION)" ]; then \
		echo "Error: pyproject.toml version is $$PYTHON_VERSION but expected $(VERSION)"; \
		exit 1; \
	fi; \
	if [ "$$RUST_VERSION" != "$(VERSION)" ]; then \
		echo "Error: rust/Cargo.toml version is $$RUST_VERSION but expected $(VERSION)"; \
		exit 1; \
	fi
	git tag -a "v$(VERSION)" -m "Release v$(VERSION)"

build: sync-version build-python build-rust

build-python: sync-version
	$(POETRY) run python -m build --sdist --wheel --no-isolation --outdir $(PYTHON_DIST_DIR) .

build-rust: sync-version
	cd $(RUST_DIR) && CARGO_TARGET_DIR="$(RUST_TARGET_DIR)" $(CARGO) build --release

build-rust-macos-arm64: sync-version
	bash ./scripts/build-rust-macos-arm64.sh

build-rust-linux-amd64: sync-version
	bash ./scripts/build-rust-linux-amd64.sh

build-rust-linux-amd64-zig: sync-version
	bash ./scripts/build-rust-linux-amd64-zig.sh

seed-grafana-sample-data:
	bash ./scripts/seed-grafana-sample-data.sh

verify-grafana-sample-data:
	bash ./scripts/seed-grafana-sample-data.sh --verify

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
