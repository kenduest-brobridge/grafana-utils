# Maintainer Role Map

Use this page to route a change to the maintainer who should own it first.
It is a working map, not a policy rewrite. If a change spans more than one
role, start with the narrowest role that owns the primary contract.

## Runtime / CLI Maintainer

Typical triggers:

- a user-facing flag, example, or parser rule looks wrong
- help text and actual runtime behavior drift apart
- one command family needs a behavior change that may affect sibling surfaces

- Usual changes:
  - CLI behavior, parsing, flags, help text, runtime dispatch, output shapes,
    and command-specific error handling
  - shared runtime code in Rust or Python when it affects user-facing command
    execution
- First files to open:
  - [`docs/DEVELOPER.md`](../DEVELOPER.md)
  - [`rust/src/cli.rs`](../../rust/src/cli.rs)
  - relevant command module under [`rust/src/`](../../rust/src/)
  - [`python/grafana_utils/unified_cli.py`](../../python/grafana_utils/unified_cli.py)
    when the Python dispatcher is involved
- Do not change casually:
  - command names, flag meanings, output contracts, or baseline routing without
    checking the related parser and help text
  - shared runtime helpers that affect more than one surface
- Validation commands:
  - `cd rust && cargo test --quiet`
  - `cd python && PYTHONPATH=. poetry run python -m unittest -v tests`
  - targeted command tests for the touched surface when available

## Docs / Docs-Generator Maintainer

Typical triggers:

- README, handbook, command reference, HTML output, and manpages drift apart
- a new command or handbook page needs to appear in generated outputs
- locale routing, handbook order, or landing-page navigation changes

- Usual changes:
  - generated docs, command reference pages, handbook pages, locale trees,
    and generator behavior
  - maintainer-facing docs that explain doc structure, source-of-truth rules,
    or output inventory
- First files to open:
  - [`docs/DEVELOPER.md`](../DEVELOPER.md)
  - [`docs/internal/generated-docs-architecture.md`](generated-docs-architecture.md)
  - [`docs/internal/generated-docs-playbook.md`](generated-docs-playbook.md)
  - [`docs/internal/contract-doc-map.md`](contract-doc-map.md) when the change
    touches a current contract doc
- Do not change casually:
  - generator output inventory, locale routing, or command/handbook linking
    rules without checking the generator code and validation path
  - trace files as a place to restate full specs
- Validation commands:
  - `make man`
  - `make html`
  - `make man-check`
  - `make html-check`
  - `python3 -m unittest -v python.tests.test_python_generate_manpages python.tests.test_python_generate_command_html`

## Contract / Schema Maintainer

Typical triggers:

- a file format or exported field is gaining, dropping, or redefining meaning
- compatibility promises need to be tightened or relaxed
- a new workflow starts to behave like a stable contract instead of an internal detail

- Usual changes:
  - stable export/import contracts, schema fields, compatibility rules, and
    summary/spec/trace updates for active domain contracts
  - changes that affect what a file, manifest, or record means across versions
- First files to open:
  - [`docs/DEVELOPER.md`](../DEVELOPER.md)
  - [`docs/internal/contract-doc-map.md`](contract-doc-map.md)
  - the current contract doc for the affected area, such as:
    - [`dashboard-export-root-contract.md`](dashboard-export-root-contract.md)
    - [`datasource-masked-recovery-contract.md`](datasource-masked-recovery-contract.md)
    - [`alert-access-contract-policy.md`](alert-access-contract-policy.md)
- Do not change casually:
  - field names, compatibility promises, or promotion rules unless the contract
    doc is updated in the same patch
  - trace text that should stay short and decision-oriented
- Validation commands:
  - the narrowest unit or CLI tests for the affected contract area
  - `cd rust && cargo test --quiet`
  - `cd python && PYTHONPATH=. poetry run python -m unittest -v tests`

## Build / Release Maintainer

Typical triggers:

- release artifact names or feature sets change
- install or validation scripts stop matching what CI publishes
- Python and Rust package metadata drift apart

- Usual changes:
  - package metadata, build scripts, release artifacts, installer behavior, and
    CI wiring for build or publish paths
  - versioning and cross-language release alignment
- First files to open:
  - [`python/pyproject.toml`](../../python/pyproject.toml)
  - [`rust/Cargo.toml`](../../rust/Cargo.toml)
  - [`Makefile`](../../Makefile)
  - [`scripts/`](../../scripts/) build, install, and validation scripts
  - [`docs/DEVELOPER.md`](../DEVELOPER.md) for repo release policy
- Do not change casually:
  - version numbers, artifact names, feature flags, and install paths without
    checking every build or release consumer
  - CI release assumptions that depend on the published artifact layout
- Validation commands:
  - `make build-python`
  - `make build-rust`
  - `make build`
  - `make test`
  - targeted script or release checks for the touched path

## Routing Rule

- If the change is mostly user interaction or execution flow, start with
  runtime / CLI.
- If the change is mostly page generation, navigation, or doc output, start
  with docs / docs-generator.
- If the change changes meaning, compatibility, or schema, start with
  contract / schema.
- If the change changes packaging, artifacts, or release wiring, start with
  build / release.

## Cross-Role Warning Signs

- If you are changing CLI behavior and also editing stable exported fields, involve contract / schema.
- If you are changing docs-generator output inventory, involve docs / docs-generator even if the trigger came from README work.
- If you are changing artifact names, browser-feature defaults, or install paths, involve build / release even when the immediate code change lives in Rust.
