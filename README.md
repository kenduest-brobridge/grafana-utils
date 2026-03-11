# Grafana Utilities

Language / 語言: English | [繁體中文 README.zh-TW.md](README.zh-TW.md)

Export, back up, migrate, and re-import Grafana dashboards and alerting resources as JSON.

This repository provides two CLI tools in two implementations:

- `grafana-utils`: dashboard export and import
- `grafana-alert-utils`: alerting resource export and import
- packaged Python implementation under [`grafana_utils/`](grafana_utils/)
- Rust implementation under [`rust/`](rust/)

The repository is useful when you need to:

- back up dashboards or alerting resources from Grafana
- move Grafana content between environments
- keep Grafana JSON under version control
- prepare dashboard files either for API import or Grafana web UI import with datasource prompts

Compatibility:

- supported on RHEL 8 and later
- Python entrypoints stay parseable on Python 3.6 syntax for RHEL 8 environments

## Contents

- [Overview](#overview)
- [Choose Python or Rust](#choose-python-or-rust)
- [Quick Start](#quick-start)
- [Dashboard Utility](#dashboard-utility)
- [Alerting Utility](#alerting-utility)
- [Build and Install](#build-and-install)
- [Authentication and TLS](#authentication-and-tls)
- [Output Directory Layout](#output-directory-layout)
- [Validation](#validation)
- [Documentation](#documentation)

## Overview

The two command names are intentionally separate because dashboards and alerting use different Grafana APIs and different file shapes.

- `grafana-utils export ...`
- `grafana-utils import ...`
- `grafana-utils diff ...`
- `grafana-alert-utils ...`

The most important distinction in this repo is dashboard export format:

- `dashboards/raw/` is for Grafana API re-import
- `dashboards/prompt/` is for Grafana web UI import with datasource mapping prompts

## Choose Python or Rust

Use the path that matches how you want to operate the repo.

| Option | When to use it | Commands |
| --- | --- | --- |
| Installed Python package | Best default for normal usage | `grafana-utils ...`, `grafana-alert-utils ...` |
| Python from git checkout | Best when editing or testing the repo directly | `python3 cmd/grafana-utils.py ...`, `python3 cmd/grafana-alert-utils.py ...` |
| Rust from git checkout | Best when validating or developing the Rust implementation | `cargo run --bin grafana-utils -- ...`, `cargo run --bin grafana-alert-utils -- ...` |

Notes:

- the Python package is the normal install path from this repository
- the Rust binaries are built from [`rust/`](rust/) and are not installed by `python3 -m pip install .`
- both implementations use the same command names and the same operator concepts

## Quick Start

Dashboard export, writing both `raw/` and `prompt/` variants:

```bash
python3 cmd/grafana-utils.py export \
  --url http://127.0.0.1:3000 \
  --export-dir ./dashboards \
  --overwrite
```

Dashboard API import from the raw export:

```bash
python3 cmd/grafana-utils.py import \
  --url http://127.0.0.1:3000 \
  --import-dir ./dashboards/raw \
  --replace-existing
```

Dashboard diff against the current Grafana state:

```bash
python3 cmd/grafana-utils.py diff \
  --url http://127.0.0.1:3000 \
  --import-dir ./dashboards/raw
```

Alerting export:

```bash
python3 cmd/grafana-alert-utils.py \
  --url http://127.0.0.1:3000 \
  --output-dir ./alerts \
  --overwrite
```

Alerting import:

```bash
python3 cmd/grafana-alert-utils.py \
  --url http://127.0.0.1:3000 \
  --import-dir ./alerts/raw \
  --replace-existing
```

Alerting diff against the current Grafana state:

```bash
python3 cmd/grafana-alert-utils.py \
  --url http://127.0.0.1:3000 \
  --diff-dir ./alerts/raw
```

## Dashboard Utility

`grafana-utils` has explicit subcommands:

- `export`
- `import`
- `diff`

### Export Variants

One dashboard export run writes two variants by default:

- `dashboards/raw/`
- `dashboards/prompt/`

Use these flags when you want only one side:

- `--without-dashboard-raw`
- `--without-dashboard-prompt`

Use `raw/` when you want:

- the same dashboard `uid`
- minimal transformation
- API re-import through `grafana-utils import`

Use `prompt/` when you want:

- Grafana web UI import
- datasource mapping prompts
- Grafana-style `__inputs`

### Important Export Options

| Option | Purpose |
| --- | --- |
| `--url` | Grafana base URL. Default: `http://127.0.0.1:3000` |
| `--export-dir` | Root export directory. Default: `dashboards/` |
| `--page-size` | Dashboard search page size. Default: `500` |
| `--flat` | Do not create per-folder subdirectories |
| `--overwrite` | Replace existing exported files |
| `--without-dashboard-raw` | Skip the `raw/` export variant |
| `--without-dashboard-prompt` | Skip the `prompt/` export variant |
| `--dry-run` | Preview export output without writing files |
| `--verify-ssl` | Enable TLS certificate verification |

### Raw Export

Raw export preserves the Grafana dashboard identity as much as possible:

- preserves dashboard `uid`
- preserves dashboard `title`
- sets numeric dashboard `id` to `null`
- keeps datasource references unchanged

If you only want the prompt variant:

```bash
python3 cmd/grafana-utils.py export \
  --export-dir ./dashboards \
  --without-dashboard-raw
```

### Prompt Export

Prompt export rewrites the dashboard into a shape Grafana web import understands:

- creates non-empty `__inputs`
- keeps `__elements`
- rewrites datasource references into import placeholders
- may normalize panel datasource refs to `{"uid":"$datasource"}` when the dashboard uses one datasource type

Important notes:

- mixed-datasource dashboards keep explicit `DS_...` placeholders
- untyped datasource variables that cannot be converted safely are preserved as-is
- prompt JSON is for Grafana web UI import, not API import

If you only want the raw variant:

```bash
python3 cmd/grafana-utils.py export \
  --export-dir ./dashboards \
  --without-dashboard-prompt
```

### Dashboard Import

Dashboard import reads normal dashboard JSON through the Grafana API.

Example:

```bash
python3 cmd/grafana-utils.py import \
  --url http://127.0.0.1:3000 \
  --import-dir ./dashboards/raw \
  --replace-existing
```

Important rules:

- point `--import-dir` at `dashboards/raw/`, not the combined `dashboards/` root
- do not feed `prompt/` files into API import
- files containing `__inputs` should be imported through Grafana web UI
- `--import-folder-uid` overrides the target folder for all imported dashboards
- `--import-message` sets the dashboard version-history message
- `--dry-run` shows whether each dashboard would create or update without calling Grafana import APIs
- `diff` compares local raw files with the live Grafana dashboard payload and returns exit code `1` when differences are found

Dashboard export also writes small versioned manifest files named `export-metadata.json` at the root and per-variant directories. They describe the export schema version and help `import` and `diff` validate that a directory really contains the expected `raw/` format.

## Alerting Utility

`grafana-alert-utils` handles Grafana alerting resources separately from dashboards.

Supported resources:

- alert rules
- contact points
- mute timings
- notification policies
- notification message templates

### Alerting Export

Example:

```bash
python3 cmd/grafana-alert-utils.py \
  --url http://127.0.0.1:3000 \
  --output-dir ./alerts \
  --overwrite
```

Use `--flat` if you want a flatter directory layout:

```bash
python3 cmd/grafana-alert-utils.py --output-dir ./alerts --flat
```

### Alerting Import

Example:

```bash
python3 cmd/grafana-alert-utils.py \
  --url http://127.0.0.1:3000 \
  --import-dir ./alerts/raw \
  --replace-existing
```

Import with linked dashboard or panel remapping:

```bash
python3 cmd/grafana-alert-utils.py \
  --url https://grafana.example.com \
  --import-dir ./alerts/raw \
  --replace-existing \
  --dashboard-uid-map ./dashboard-map.json \
  --panel-id-map ./panel-map.json
```

Alerting diff:

```bash
python3 cmd/grafana-alert-utils.py \
  --url https://grafana.example.com \
  --diff-dir ./alerts/raw \
  --dashboard-uid-map ./dashboard-map.json \
  --panel-id-map ./panel-map.json
```

Example `dashboard-map.json`:

```json
{
  "old-dashboard-uid": "new-dashboard-uid"
}
```

Example `panel-map.json`:

```json
{
  "old-dashboard-uid": {
    "7": "19"
  }
}
```

### Alerting Import Rules

- `--replace-existing` updates existing rules by `uid`
- `--replace-existing` updates existing contact points by `uid`
- `--replace-existing` updates existing mute timings by `name`
- notification policies are always applied with `PUT`
- notification templates are applied with `PUT`, and with `--replace-existing` the tool reads the current template version first
- without `--replace-existing`, rule/contact-point/mute-timing import uses create and Grafana rejects conflicts
- without `--replace-existing`, template import fails if the template name already exists
- import expects files exported by this tool
- do not point `--import-dir` at the combined `alerts/` root
- `--dry-run` predicts whether each file would create, update, or fail without changing Grafana
- `--diff-dir` compares local exported files with live alerting resources and returns exit code `1` when differences are found

Important limitation:

- Grafana official alert provisioning `/export` output is not a supported import format for this tool
- this tool only guarantees round-trip import for files exported by `grafana-alert-utils`

Why this happens:

- Grafana's alert provisioning `export` payload is meant for provisioning-style representation, not direct HTTP API round-trip import
- Grafana's create/update APIs expect a different request shape than the `/export` response shape
- because of that mismatch, this tool uses its own export format for backup and restore workflows

For linked alert rules:

- use `--dashboard-uid-map` and `--panel-id-map` when dashboard or panel identities changed
- maintainer details about fallback matching and repair behavior are in [`DEVELOPER.md`](DEVELOPER.md)

## Build and Install

### Python Package

Install into the current Python environment:

```bash
python3 -m pip install .
```

Install into a user-local environment:

```bash
python3 -m pip install --user .
```

Optional HTTP/2 dependencies on Python 3.8+:

```bash
python3 -m pip install '.[http2]'
```

### Makefile Shortcuts

The repo root includes a [`Makefile`](Makefile):

- `make help`
- `make build-python`
- `make build-rust`
- `make build`
- `make test-python`
- `make test-rust`
- `make test`

Artifact locations:

- `make build-python` writes the wheel into `dist/`
- `make build-rust` writes release binaries into `rust/target/release/`

### Rust Build and Run

Build Rust release binaries:

```bash
make build-rust
```

Run the Rust dashboard CLI from the repo:

```bash
cd rust
cargo run --bin grafana-utils -- export -h
```

Run the Rust alerting CLI from the repo:

```bash
cd rust
cargo run --bin grafana-alert-utils -- -h
```

## Authentication and TLS

Authentication methods:

- API token
- username and password

API token example:

```bash
export GRAFANA_API_TOKEN='your-token'
python3 cmd/grafana-utils.py export --export-dir ./dashboards
```

Username/password example:

```bash
export GRAFANA_USERNAME='your-user'
export GRAFANA_PASSWORD='your-pass'
python3 cmd/grafana-utils.py export --export-dir ./dashboards
```

TLS note:

- SSL verification is disabled by default
- add `--verify-ssl` when you want strict certificate verification

Example:

```bash
python3 cmd/grafana-utils.py export --verify-ssl
```

## Output Directory Layout

Dashboard export layout:

```text
dashboards/
  index.json
  export-metadata.json
  raw/
    export-metadata.json
    index.json
    ...
  prompt/
    export-metadata.json
    index.json
    ...
```

Alerting export layout:

```text
alerts/
  index.json
  raw/
    rules/
    contact-points/
    mute-timings/
    policies/
    templates/
```

## Validation

Common validation commands:

```bash
make test
python3 -m unittest -v
cd rust && cargo test
```

## Documentation

- English README: [`README.md`](README.md)
- Traditional Chinese README: [`README.zh-TW.md`](README.zh-TW.md)
- maintainer and implementation notes: [`DEVELOPER.md`](DEVELOPER.md)
