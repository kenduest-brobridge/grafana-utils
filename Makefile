.PHONY: help print-version sync-version set-release-version set-dev-version poetry-install poetry-lock poetry-test poetry-quality-python man man-check html html-check build build-python build-rust build-rust-browser build-rust-native build-rust-native-browser build-rust-host build-rust-host-browser build-rust-macos-arm64 build-rust-macos-arm64-browser build-rust-linux-amd64 build-rust-linux-amd64-browser build-rust-linux-amd64-docker build-rust-linux-amd64-browser-docker build-rust-linux-amd64-zig validate-rust-linux-amd64-artifact validate-rust-linux-amd64-browser-artifact seed-grafana-sample-data destroy-grafana-sample-data reset-grafana-all-data test test-python test-rust fmt-rust-check lint-rust quality quality-python quality-rust quality-alert-rust quality-sync-rust test-rust-live test-sync-live test-alert-live test-alert-live-artifact test-alert-live-replay test-access-live test-python-datasource-live test-datasource-live

PYTHON ?= python3
PIP ?= $(PYTHON) -m pip
POETRY ?= poetry
CARGO ?= cargo
RUST_DIR ?= rust
PYTHON_DIST_DIR ?= dist
DEV_ITERATION ?= 1
RUST_RELEASE_RUSTFLAGS ?= -C debuginfo=0
HOST_OS := $(shell uname -s)

help:
	@BOLD="$$(printf '\033[1m')"; \
	RESET="$$(printf '\033[0m')"; \
	BLUE="$$(printf '\033[34m')"; \
	GREEN="$$(printf '\033[32m')"; \
	CYAN="$$(printf '\033[36m')"; \
	YELLOW="$$(printf '\033[33m')"; \
	printf '%b\n' "$${BOLD}Available targets$${RESET}"; \
	printf '\n'; \
	printf '%b\n' "$${BLUE}$${BOLD}Versioning$${RESET}"; \
	printf '  %b\n' "$${GREEN}make print-version$${RESET}  Show VERSION plus Python/Rust package versions"; \
	printf '  %b\n' "$${GREEN}make sync-version$${RESET}  Sync python/pyproject.toml, rust/Cargo.toml, and rust/Cargo.lock from VERSION"; \
	printf '  %b\n' "$${GREEN}make set-release-version VERSION=X.Y.Z$${RESET}  Set VERSION and package metadata to a release version"; \
	printf '  %b\n' "$${GREEN}make set-dev-version VERSION=X.Y.Z DEV_ITERATION=N$${RESET}  Optionally set VERSION and package metadata to a preview version"; \
	printf '\n'; \
	printf '%b\n' "$${BLUE}$${BOLD}Python$${RESET}"; \
	printf '  %b\n' "$${GREEN}make poetry-install$${RESET}  Install the Poetry-managed development environment"; \
	printf '  %b\n' "$${GREEN}make poetry-lock$${RESET}  Refresh python/poetry.lock from python/pyproject.toml"; \
	printf '  %b\n' "$${GREEN}make poetry-test$${RESET}  Run the Python unittest suite inside Poetry"; \
	printf '  %b\n' "$${GREEN}make poetry-quality-python$${RESET}  Run Python quality checks inside Poetry"; \
	printf '  %b\n' "$${GREEN}make build-python$${RESET}  Build the Python wheel and sdist into dist/"; \
	printf '\n'; \
	printf '%b\n' "$${BLUE}$${BOLD}Docs$${RESET}"; \
	printf '  %b\n' "$${GREEN}make man$${RESET}  Regenerate grafana-util, dashboard, alert, datasource, access, profile, status, overview, change, and snapshot manpages"; \
	printf '  %b\n' "$${GREEN}make man-check$${RESET}  Fail if those checked-in docs/man/*.1 pages are out of date"; \
	printf '  %b\n' "$${GREEN}make html$${RESET}  Regenerate the HTML docs site: handbook + command reference, with docs/html/index.html as the entrypoint"; \
	printf '  %b\n' "$${GREEN}make html-check$${RESET}  Fail if checked-in docs/html/**/*.html is out of date"; \
	printf '\n'; \
	printf '%b\n' "$${BLUE}$${BOLD}Rust build$${RESET}"; \
	printf '  %b%s%b\n' "$${GREEN}make build-rust$${RESET}  Build the default native, host release artifact, and Linux amd64 Rust artifacts " "$${CYAN}(no browser feature)" "$${RESET}"; \
	printf '  %b\n' "$${GREEN}make build-rust-browser$${RESET}  Build the browser-enabled native, host release artifact, and Linux amd64 Rust artifacts"; \
	printf '  %b\n' "$${GREEN}make build-rust-native$${RESET}  Build the default native Rust release binary in rust/target/release/"; \
	printf '  %b\n' "$${GREEN}make build-rust-native-browser$${RESET}  Build the browser-enabled native Rust release binary in rust/target/release/"; \
	printf '  %b\n' "$${GREEN}make build-rust-macos-arm64$${RESET}  Build the default macOS Apple Silicon artifact into dist/macos-arm64/"; \
	printf '  %b\n' "$${GREEN}make build-rust-macos-arm64-browser$${RESET}  Build the browser-enabled macOS Apple Silicon artifact into dist/macos-arm64-browser/"; \
	printf '  %b%s%b\n' "$${GREEN}make build-rust-linux-amd64$${RESET}  Build the default Linux amd64 artifact with local zig into dist/linux-amd64/ " "$${CYAN}(preferred; defaults to LTO off and codegen-units=1 for cross-link stability)" "$${RESET}"; \
	printf '  %b\n' "$${GREEN}make build-rust-linux-amd64-browser$${RESET}  Build the browser-enabled Linux amd64 artifact with local zig into dist/linux-amd64-browser/"; \
	printf '  %b%s%b\n' "$${GREEN}make build-rust-linux-amd64-docker$${RESET}  Build the default Linux amd64 artifact with Docker into dist/linux-amd64/ " "$${CYAN}(fallback)" "$${RESET}"; \
	printf '  %b%s%b\n' "$${GREEN}make build-rust-linux-amd64-browser-docker$${RESET}  Build the browser-enabled Linux amd64 artifact with Docker into dist/linux-amd64-browser/ " "$${CYAN}(fallback)" "$${RESET}"; \
	printf '  %b%s%b\n' "$${GREEN}make build-rust-linux-amd64-zig$${RESET}  Alias for the preferred local zig Linux amd64 build path " "$${CYAN}(same as build-rust-linux-amd64)" "$${RESET}"; \
	printf '\n'; \
	printf '%b\n' "$${BLUE}$${BOLD}Artifact validation$${RESET}"; \
	printf '  %b\n' "$${GREEN}make validate-rust-linux-amd64-artifact$${RESET}  Run the default Linux amd64 artifact in a fixed-name Linux Docker container"; \
	printf '  %b\n' "$${GREEN}make validate-rust-linux-amd64-browser-artifact$${RESET}  Run the browser-enabled Linux amd64 artifact in a fixed-name Linux Docker container"; \
	printf '\n'; \
	printf '%b\n' "$${BLUE}$${BOLD}Quality and tests$${RESET}"; \
	printf '  %b\n' "$${GREEN}make test$${RESET}  Run both Python and Rust test suites"; \
	printf '  %b\n' "$${GREEN}make test-python$${RESET}  Run the Python unittest suite"; \
	printf '  %b\n' "$${GREEN}make test-rust$${RESET}  Run the Rust cargo test suite"; \
	printf '  %b\n' "$${GREEN}make fmt-rust-check$${RESET}  Run cargo fmt --check"; \
	printf '  %b\n' "$${GREEN}make lint-rust$${RESET}  Run cargo clippy with warnings denied"; \
	printf '  %b\n' "$${GREEN}make quality$${RESET}  Run the repo quality gate scripts"; \
	printf '  %b\n' "$${GREEN}make quality-python$${RESET}  Run the Python quality gate script"; \
	printf '  %b\n' "$${GREEN}make quality-rust$${RESET}  Run the Rust quality gate script"; \
	printf '  %b\n' "$${GREEN}make quality-alert-rust$${RESET}  Run focused Rust alert contract checks"; \
	printf '  %b\n' "$${GREEN}make quality-sync-rust$${RESET}  Run focused Rust sync contract checks"; \
	printf '\n'; \
	printf '%b\n' "$${BLUE}$${BOLD}Live smoke and sample data$${RESET}"; \
	printf '  %b\n' "$${GREEN}make seed-grafana-sample-data$${RESET}  Seed a local Grafana with reusable developer sample orgs, datasources, folders, and dashboards"; \
	printf '  %b\n' "$${GREEN}make destroy-grafana-sample-data$${RESET}  Remove the developer sample orgs, datasources, folders, and dashboards seeded by the repo script"; \
	printf '  %b%s%b%s\n' "$${GREEN}make reset-grafana-all-data$${RESET}  " "$${YELLOW}Danger:$${RESET}" " " "delete repo-relevant test data from a disposable local Grafana instance"; \
	printf '  %b\n' "$${GREEN}make test-rust-live$${RESET}  Start Grafana in Docker and run the Rust live smoke test"; \
	printf '  %b\n' "$${GREEN}make test-sync-live$${RESET}  Start Grafana in Docker and run the Rust sync live smoke path"; \
	printf '  %b\n' "$${GREEN}make test-alert-live$${RESET}  Start Grafana in Docker and run the Rust alert live smoke path"; \
	printf '  %b\n' "$${GREEN}make test-alert-live-artifact$${RESET}  Start Grafana in Docker and run the Rust alert artifact live smoke path"; \
	printf '  %b\n' "$${GREEN}make test-alert-live-replay$${RESET}  Start Grafana in Docker and run the Rust alert replay live smoke path"; \
	printf '  %b\n' "$${GREEN}make test-access-live$${RESET}  Start Grafana in Docker and run the Python access live smoke test"; \
	printf '  %b\n' "$${GREEN}make test-python-datasource-live$${RESET}  Start Grafana in Docker and run the Python datasource live smoke test"; \
	printf '  %b\n' "$${GREEN}make test-datasource-live$${RESET}  Start Grafana in Docker and run the Rust and Python datasource live smoke tests"; \
	printf '\n'; \
	printf '%b\n' "$${BLUE}$${BOLD}Meta$${RESET}"; \
	printf '  %b\n' "$${GREEN}make build$${RESET}  Build both Python and Rust artifacts"

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
	$(POETRY) --directory python install --with dev

poetry-lock:
	$(POETRY) --directory python lock

poetry-test:
	cd python && $(POETRY) run env PYTHONPATH=. $(PYTHON) -m unittest -v tests

poetry-quality-python:
	cd python && $(POETRY) run env PYTHON=python PYTHONPATH=. ../scripts/check-python-quality.sh

man:
	$(PYTHON) ./scripts/generate_manpages.py --write

man-check:
	$(PYTHON) ./scripts/generate_manpages.py --check

html:
	$(PYTHON) ./scripts/generate_command_html.py --write

html-check:
	$(PYTHON) ./scripts/generate_command_html.py --check

build: build-python build-rust
	@printf '%s\n' 'Build outputs:'
	@find $(PYTHON_DIST_DIR) -maxdepth 1 -type f \( -name '*.whl' -o -name '*.tar.gz' \) | sort
	@find $(RUST_DIR)/target/release -maxdepth 1 -type f -perm -111 | sort

build-python:
	cd python && $(POETRY) run python -m build --sdist --wheel --no-isolation --outdir ../$(PYTHON_DIST_DIR) .
	@printf '%s\n' 'Python build outputs:'
	@find $(PYTHON_DIST_DIR) -maxdepth 1 -type f \( -name '*.whl' -o -name '*.tar.gz' \) | sort

build-rust: build-rust-native build-rust-host build-rust-linux-amd64
	@printf '%s\n' 'Rust build outputs:'
	@printf '%s\n' "$(RUST_DIR)/target/release/grafana-util"
ifeq ($(HOST_OS),Darwin)
	@printf '%s\n' "dist/macos-arm64/grafana-util"
endif
	@printf '%s\n' "dist/linux-amd64/grafana-util"

build-rust-browser: build-rust-native-browser build-rust-host-browser build-rust-linux-amd64-browser
	@printf '%s\n' 'Rust browser-enabled build outputs:'
	@printf '%s\n' "$(RUST_DIR)/target/release/grafana-util"
ifeq ($(HOST_OS),Darwin)
	@printf '%s\n' "dist/macos-arm64-browser/grafana-util"
endif
	@printf '%s\n' "dist/linux-amd64-browser/grafana-util"

build-rust-native:
	cd $(RUST_DIR) && RUSTFLAGS="$(RUST_RELEASE_RUSTFLAGS)" $(CARGO) build --release
	@printf '%s\n' 'Rust native build outputs:'
	@find $(RUST_DIR)/target/release -maxdepth 1 -type f -perm -111 | sort

build-rust-native-browser:
	cd $(RUST_DIR) && RUSTFLAGS="$(RUST_RELEASE_RUSTFLAGS)" $(CARGO) build --release --features browser
	@printf '%s\n' 'Rust native browser-enabled build outputs:'
	@find $(RUST_DIR)/target/release -maxdepth 1 -type f -perm -111 | sort

build-rust-host:
ifeq ($(HOST_OS),Darwin)
	@$(MAKE) --no-print-directory build-rust-macos-arm64
else
	@printf '%s\n' "Skipping host packaged Rust artifact: unsupported host OS $(HOST_OS)"
endif

build-rust-host-browser:
ifeq ($(HOST_OS),Darwin)
	@$(MAKE) --no-print-directory build-rust-macos-arm64-browser
else
	@printf '%s\n' "Skipping host packaged browser Rust artifact: unsupported host OS $(HOST_OS)"
endif

build-rust-macos-arm64:
	RUSTFLAGS="$(RUST_RELEASE_RUSTFLAGS)" bash ./scripts/build-rust-macos-arm64.sh

build-rust-macos-arm64-browser:
	BUILD_BROWSER=1 RUSTFLAGS="$(RUST_RELEASE_RUSTFLAGS)" bash ./scripts/build-rust-macos-arm64.sh

build-rust-linux-amd64: build-rust-linux-amd64-zig

build-rust-linux-amd64-browser:
	BUILD_BROWSER=1 RUSTFLAGS="$(RUST_RELEASE_RUSTFLAGS)" bash ./scripts/build-rust-linux-amd64-zig.sh

build-rust-linux-amd64-docker:
	RUSTFLAGS="$(RUST_RELEASE_RUSTFLAGS)" bash ./scripts/build-rust-linux-amd64.sh

build-rust-linux-amd64-browser-docker:
	BUILD_BROWSER=1 RUSTFLAGS="$(RUST_RELEASE_RUSTFLAGS)" bash ./scripts/build-rust-linux-amd64.sh

build-rust-linux-amd64-zig:
	RUSTFLAGS="$(RUST_RELEASE_RUSTFLAGS)" bash ./scripts/build-rust-linux-amd64-zig.sh

validate-rust-linux-amd64-artifact:
	bash ./scripts/validate-rust-linux-amd64-artifact.sh --version

validate-rust-linux-amd64-browser-artifact:
	BUILD_BROWSER=1 bash ./scripts/validate-rust-linux-amd64-artifact.sh --version

seed-grafana-sample-data:
	bash ./scripts/seed-grafana-sample-data.sh

destroy-grafana-sample-data:
	bash ./scripts/seed-grafana-sample-data.sh --destroy

reset-grafana-all-data:
	bash ./scripts/seed-grafana-sample-data.sh --reset-all-data --yes

test: test-python test-rust

test-python:
	PYTHONPATH=python $(PYTHON) -m unittest discover -s python/tests -v

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
