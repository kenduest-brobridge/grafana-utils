VERSIONING_TARGETS := help print-version sync-version set-release-version set-dev-version
PYTHON_TARGETS := poetry-install poetry-lock poetry-test poetry-quality-python build-python
DOC_TARGETS := man man-check html html-check pages-site schema schema-check quality-docs-surface
RUST_BUILD_TARGETS := build-rust build-rust-browser build-rust-native build-rust-native-browser build-rust-host build-rust-host-browser build-rust-macos-arm64 build-rust-macos-arm64-browser build-rust-linux-amd64 build-rust-linux-amd64-browser build-rust-linux-amd64-docker build-rust-linux-amd64-browser-docker build-rust-linux-amd64-zig validate-rust-linux-amd64-artifact validate-rust-linux-amd64-browser-artifact
INSTALLER_TARGETS := install-local install-local-interactive test-installer-local
QUALITY_TARGETS := test test-python test-rust fmt-rust-check lint-rust quality quality-python quality-rust quality-ai-workflow quality-architecture quality-docs-surface quality-alert-rust quality-sync-rust quality-workspace-noise
LIVE_TARGETS := seed-grafana-sample-data destroy-grafana-sample-data reset-grafana-all-data test-rust-live test-sync-live test-alert-live test-alert-live-artifact test-alert-live-replay test-access-live test-python-datasource-live test-datasource-live
META_TARGETS := build

.PHONY: $(VERSIONING_TARGETS) $(PYTHON_TARGETS) $(DOC_TARGETS) $(RUST_BUILD_TARGETS) $(INSTALLER_TARGETS) $(QUALITY_TARGETS) $(LIVE_TARGETS) $(META_TARGETS)

PYTHON ?= python3
PIP ?= $(PYTHON) -m pip
POETRY ?= poetry
CARGO ?= cargo
RUST_DIR ?= rust
PYTHON_DIST_DIR ?= dist
DEV_ITERATION ?= 1
RUST_RELEASE_RUSTFLAGS ?= -C debuginfo=0
HOST_OS := $(shell uname -s)
PYTHON_DIR := python
ESC := \033
BOLD := $(ESC)[1m
RESET := $(ESC)[0m
BLUE := $(ESC)[34m
GREEN := $(ESC)[32m
CYAN := $(ESC)[36m
YELLOW := $(ESC)[33m
define NL


endef

define HELP_TITLE
$(BOLD)Available targets$(RESET)

endef

define HELP_VERSIONING
$(BLUE)$(BOLD)Versioning$(RESET)
  $(GREEN)make print-version$(RESET)  Show VERSION plus Python/Rust package versions
  $(GREEN)make sync-version$(RESET)  Sync python/pyproject.toml, rust/Cargo.toml, and rust/Cargo.lock from VERSION
  $(GREEN)make set-release-version VERSION=X.Y.Z$(RESET)  Set VERSION and package metadata to a release version
  $(GREEN)make set-dev-version VERSION=X.Y.Z DEV_ITERATION=N$(RESET)  Optionally set VERSION and package metadata to a preview version

endef

define HELP_PYTHON
$(BLUE)$(BOLD)Python$(RESET)
  $(GREEN)make poetry-install$(RESET)  Install the Poetry-managed development environment
  $(GREEN)make poetry-lock$(RESET)  Refresh python/poetry.lock from python/pyproject.toml
  $(GREEN)make poetry-test$(RESET)  Run the Python unittest suite inside Poetry
  $(GREEN)make poetry-quality-python$(RESET)  Run Python quality checks inside Poetry
  $(GREEN)make build-python$(RESET)  Build the Python wheel and sdist into dist/

endef

define HELP_DOCS
$(BLUE)$(BOLD)Docs$(RESET)
  $(GREEN)make schema$(RESET)  Regenerate checked-in JSON Schema and schema-help artifacts from schemas/manifests/
  $(GREEN)make schema-check$(RESET)  Fail if checked-in schemas/jsonschema/ or schemas/help/ artifacts are out of date
  $(GREEN)make man$(RESET)  Regenerate grafana-util, dashboard, alert, datasource, access, profile, status, overview, change, and snapshot manpages
  $(GREEN)make man-check$(RESET)  Fail if those checked-in docs/man/*.1 pages are out of date
  $(GREEN)make html$(RESET)  Regenerate the HTML docs site: handbook + command reference, with docs/html/index.html as the entrypoint
  $(GREEN)make html-check$(RESET)  Fail if checked-in docs/html/**/*.html is out of date
  $(GREEN)make pages-site$(RESET)  Assemble the multi-version GitHub Pages docs artifact into build/docs-pages/
  $(GREEN)make quality-docs-surface$(RESET)  Fail when Markdown command examples, locale parity, links, or help-full paths drift from the Rust CLI

endef

define HELP_RUST_BUILD
$(BLUE)$(BOLD)Rust build$(RESET)
  $(GREEN)make build-rust$(RESET)  Build the default native, host release artifact, and Linux amd64 Rust artifacts $(CYAN)(no browser feature)$(RESET)
  $(GREEN)make build-rust-browser$(RESET)  Build the browser-enabled native, host release artifact, and Linux amd64 Rust artifacts
  $(GREEN)make build-rust-native$(RESET)  Build the default native Rust release binary in rust/target/release/
  $(GREEN)make build-rust-native-browser$(RESET)  Build the browser-enabled native Rust release binary in rust/target/release/
  $(GREEN)make build-rust-macos-arm64$(RESET)  Build the default macOS Apple Silicon artifact into dist/macos-arm64/
  $(GREEN)make build-rust-macos-arm64-browser$(RESET)  Build the browser-enabled macOS Apple Silicon artifact into dist/macos-arm64-browser/
  $(GREEN)make build-rust-linux-amd64$(RESET)  Build the default Linux amd64 artifact with local zig into dist/linux-amd64/ $(CYAN)(preferred; defaults to LTO off and codegen-units=1 for cross-link stability)$(RESET)
  $(GREEN)make build-rust-linux-amd64-browser$(RESET)  Build the browser-enabled Linux amd64 artifact with local zig into dist/linux-amd64-browser/
  $(GREEN)make build-rust-linux-amd64-docker$(RESET)  Build the default Linux amd64 artifact with Docker into dist/linux-amd64/ $(CYAN)(fallback)$(RESET)
  $(GREEN)make build-rust-linux-amd64-browser-docker$(RESET)  Build the browser-enabled Linux amd64 artifact with Docker into dist/linux-amd64-browser/ $(CYAN)(fallback)$(RESET)
  $(GREEN)make build-rust-linux-amd64-zig$(RESET)  Alias for the preferred local zig Linux amd64 build path $(CYAN)(same as build-rust-linux-amd64)$(RESET)

endef

define HELP_ARTIFACT_VALIDATION
$(BLUE)$(BOLD)Artifact validation$(RESET)
  $(GREEN)make validate-rust-linux-amd64-artifact$(RESET)  Run the default Linux amd64 artifact in a fixed-name Linux Docker container
  $(GREEN)make validate-rust-linux-amd64-browser-artifact$(RESET)  Run the browser-enabled Linux amd64 artifact in a fixed-name Linux Docker container
  $(GREEN)make install-local$(RESET)  Build a local debug binary and install it through scripts/install.sh
  $(GREEN)make install-local-interactive$(RESET)  Build a local debug binary and run the interactive installer flow
  $(GREEN)make test-installer-local$(RESET)  Build a local binary, pack a local archive, and test install.sh with INSTALL_COMPLETION=auto

endef

define HELP_QUALITY
$(BLUE)$(BOLD)Quality and tests$(RESET)
  $(GREEN)make test$(RESET)  Run both Python and Rust test suites
  $(GREEN)make test-python$(RESET)  Run the Python unittest suite
  $(GREEN)make test-rust$(RESET)  Run the Rust cargo test suite
  $(GREEN)make fmt-rust-check$(RESET)  Run cargo fmt --check
  $(GREEN)make lint-rust$(RESET)  Run cargo clippy with warnings denied
  $(GREEN)make quality$(RESET)  Run the repo quality gate scripts
  $(GREEN)make quality-python$(RESET)  Run the Python quality gate script
  $(GREEN)make quality-rust$(RESET)  Run the Rust quality gate script
  $(GREEN)make quality-ai-workflow$(RESET)  Run lightweight AI workflow drift checks for the current change set
  $(GREEN)make quality-architecture$(RESET)  Run Rust architecture guardrail checks for root noise, file size, render risk, and help-test brittleness
  $(GREEN)make quality-docs-surface$(RESET)  Run command-surface, locale parity, and local-link drift checks for Markdown docs
  $(GREEN)make quality-alert-rust$(RESET)  Run focused Rust alert contract checks
  $(GREEN)make quality-sync-rust$(RESET)  Run focused Rust sync contract checks
  $(GREEN)make quality-workspace-noise$(RESET)  Fail when scratch/noise files currently show up in git status

endef

define HELP_LIVE
$(BLUE)$(BOLD)Live smoke and sample data$(RESET)
  $(GREEN)make seed-grafana-sample-data$(RESET)  Seed a local Grafana with reusable developer sample orgs, datasources, folders, and dashboards
  $(GREEN)make destroy-grafana-sample-data$(RESET)  Remove the developer sample orgs, datasources, folders, and dashboards seeded by the repo script
  $(GREEN)make reset-grafana-all-data$(RESET)  $(YELLOW)Danger:$(RESET) delete repo-relevant test data from a disposable local Grafana instance
  $(GREEN)make test-rust-live$(RESET)  Start Grafana in Docker and run the Rust live smoke test, including dashboard stdin/watch authoring
  $(GREEN)make test-sync-live$(RESET)  Start Grafana in Docker and run the Rust sync live smoke path
  $(GREEN)make test-alert-live$(RESET)  Start Grafana in Docker and run the Rust alert live smoke path
  $(GREEN)make test-alert-live-artifact$(RESET)  Start Grafana in Docker and run the Rust alert artifact live smoke path
  $(GREEN)make test-alert-live-replay$(RESET)  Start Grafana in Docker and run the Rust alert replay live smoke path
  $(GREEN)make test-access-live$(RESET)  Start Grafana in Docker and run the Python access live smoke test
  $(GREEN)make test-python-datasource-live$(RESET)  Start Grafana in Docker and run the Python datasource live smoke test
  $(GREEN)make test-datasource-live$(RESET)  Start Grafana in Docker and run the Rust and Python datasource live smoke tests

endef

define HELP_META
$(BLUE)$(BOLD)Meta$(RESET)
  $(GREEN)make build$(RESET)  Build both Python and Rust artifacts
endef

HELP_ALL := $(HELP_TITLE)$(HELP_VERSIONING)$(HELP_PYTHON)$(HELP_DOCS)$(HELP_RUST_BUILD)$(HELP_ARTIFACT_VALIDATION)$(HELP_QUALITY)$(HELP_LIVE)$(HELP_META)
RUST_RUN := cd $(RUST_DIR) &&
PYTHON_RUN := cd $(PYTHON_DIR) &&
PYTHON_POETRY_RUN := cd $(PYTHON_DIR) && $(POETRY) run

help:
	@printf '%b' '$(subst $(NL),\n,$(HELP_ALL))'

print-version:
	bash ./scripts/set-version.sh --print-current

sync-version:
	bash ./scripts/set-version.sh --sync-from-file

set-release-version:
	@test -n "$(VERSION)" || { printf '%s\n' "Usage: make set-release-version VERSION=X.Y.Z"; exit 1; }
	bash ./scripts/set-version.sh --version "$(VERSION)"

set-dev-version:
	@test -n "$(VERSION)" || { printf '%s\n' "Usage: make set-dev-version VERSION=X.Y.Z DEV_ITERATION=N"; exit 1; }
	bash ./scripts/set-version.sh --version "$(VERSION).dev$(DEV_ITERATION)"

poetry-install:
	$(POETRY) --directory $(PYTHON_DIR) install --with dev

poetry-lock:
	$(POETRY) --directory $(PYTHON_DIR) lock

poetry-test:
	$(PYTHON_POETRY_RUN) env PYTHONPATH=. $(PYTHON) -m unittest -v tests

poetry-quality-python:
	$(PYTHON_POETRY_RUN) env PYTHON=python PYTHONPATH=. ../scripts/check-python-quality.sh

schema:
	$(PYTHON) ./scripts/generate_schema_artifacts.py --write

schema-check:
	$(PYTHON) ./scripts/generate_schema_artifacts.py --check

man:
	$(PYTHON) ./scripts/generate_manpages.py --write

man-check:
	$(PYTHON) ./scripts/generate_manpages.py --check

html:
	$(PYTHON) ./scripts/generate_command_html.py --write

html-check:
	$(PYTHON) ./scripts/generate_command_html.py --check

pages-site:
	$(PYTHON) ./scripts/build_pages_site.py --output-dir ./build/docs-pages

quality-docs-surface:
	$(PYTHON) ./scripts/check_docs_surface.py

build: build-python build-rust
	@printf '%s\n' 'Build outputs:'
	@find $(PYTHON_DIST_DIR) -maxdepth 1 -type f \( -name '*.whl' -o -name '*.tar.gz' \) | sort
	@find $(RUST_DIR)/target/release -maxdepth 1 -type f -perm -111 | sort

build-python:
	$(PYTHON_POETRY_RUN) python -m build --sdist --wheel --no-isolation --outdir ../$(PYTHON_DIST_DIR) .
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
	$(RUST_RUN) RUSTFLAGS="$(RUST_RELEASE_RUSTFLAGS)" $(CARGO) build --release
	@printf '%s\n' 'Rust native build outputs:'
	@find $(RUST_DIR)/target/release -maxdepth 1 -type f -perm -111 | sort

build-rust-native-browser:
	$(RUST_RUN) RUSTFLAGS="$(RUST_RELEASE_RUSTFLAGS)" $(CARGO) build --release --features browser
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

install-local:
	bash ./scripts/install-local.sh

install-local-interactive:
	bash ./scripts/install-local.sh --interactive

test-installer-local:
	./scripts/test-local-installer.sh

seed-grafana-sample-data:
	bash ./scripts/seed-grafana-sample-data.sh

destroy-grafana-sample-data:
	bash ./scripts/seed-grafana-sample-data.sh --destroy

reset-grafana-all-data:
	bash ./scripts/seed-grafana-sample-data.sh --reset-all-data --yes

test: test-python test-rust

test-python:
	PYTHONPATH=$(PYTHON_DIR) $(PYTHON) -m unittest discover -s $(PYTHON_DIR)/tests -v

test-rust:
	$(RUST_RUN) $(CARGO) test

fmt-rust-check:
	$(RUST_RUN) $(CARGO) fmt --check

lint-rust:
	$(RUST_RUN) $(CARGO) clippy --all-targets -- -D warnings

quality: quality-python quality-rust

quality-python:
	./scripts/check-python-quality.sh

quality-rust:
	./scripts/check-rust-quality.sh

quality-ai-workflow:
	$(PYTHON) ./scripts/check_ai_workflow.py

quality-architecture:
	$(PYTHON) ./scripts/check_rust_architecture.py

quality-alert-rust:
	$(RUST_RUN) $(CARGO) test --quiet alert_
	$(RUST_RUN) $(CARGO) test --quiet run_sync_cli_bundle_preserves_alert_export_artifact_metadata
	$(RUST_RUN) $(CARGO) test --quiet build_sync_bundle_preflight_document_ignores_alert_replay_artifacts_but_keeps_zero_checks
	$(RUST_RUN) $(CARGO) test --quiet render_sync_source_bundle_text_reports_alert_replay_artifact_counts
	$(RUST_RUN) $(CARGO) fmt --check
	$(RUST_RUN) $(CARGO) check --quiet

quality-sync-rust:
	$(RUST_RUN) $(CARGO) test --quiet sync_
	$(RUST_RUN) $(CARGO) test --quiet build_sync_source_bundle_document_matches_cross_domain_summary_contract
	$(RUST_RUN) $(CARGO) test --quiet build_sync_source_bundle_document_preserves_alert_replay_artifact_summary_and_paths
	$(RUST_RUN) $(CARGO) test --quiet build_sync_bundle_preflight_document_ignores_alert_replay_artifacts_but_keeps_zero_checks
	$(RUST_RUN) $(CARGO) fmt --check
	$(RUST_RUN) $(CARGO) check --quiet

quality-workspace-noise:
	$(PYTHON) ./scripts/workspace_noise_auditor.py --check-git-status

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
