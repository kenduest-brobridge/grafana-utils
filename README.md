# Grafana Utilities

Language / 語言: English | [繁體中文 README.zh-TW.md](README.zh-TW.md)

Export, back up, migrate, and re-import Grafana dashboards and alerting resources as JSON.

This repository provides two primary CLI tools in two implementations, plus an access-management CLI that is starting as a Python-only workflow:

- `grafana-utils`: dashboard export and import
- `grafana-alert-utils`: alerting resource export and import
- `grafana-access-utils`: access-management workflow, currently covering `user list`, `user add`, `user modify`, `user delete`, `team list`, `team add`, `team modify`, and initial service-account commands
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
- [Access Utility](#access-utility)
- [Build and Install](#build-and-install)
- [Authentication and TLS](#authentication-and-tls)
- [Output Directory Layout](#output-directory-layout)
- [Validation](#validation)
- [Documentation](#documentation)

## Overview

The two command names are intentionally separate because dashboards and alerting use different Grafana APIs and different file shapes.

- `grafana-utils export-dashboard ...`
- `grafana-utils list-dashboard ...`
- `grafana-utils list-data-sources ...`
- `grafana-utils import-dashboard ...`
- `grafana-utils diff ...`
- `grafana-alert-utils ...`
- `grafana-access-utils user list ...`
- `grafana-access-utils user add ...`
- `grafana-access-utils user modify ...`
- `grafana-access-utils user delete ...`
- `grafana-access-utils team list ...`
- `grafana-access-utils team add ...`
- `grafana-access-utils team modify ...`
- `grafana-access-utils service-account ...`

The most important distinction in this repo is dashboard export format:

- `dashboards/raw/` is for Grafana API re-import
- `dashboards/prompt/` is for Grafana web UI import with datasource mapping prompts

## Choose Python or Rust

Use the path that matches how you want to operate the repo.

| Option | When to use it | Commands |
| --- | --- | --- |
| Installed Python package | Best default for normal usage | `grafana-utils ...`, `grafana-alert-utils ...`, `grafana-access-utils ...` |
| Python from git checkout | Best when editing or testing the repo directly | `python3 cmd/grafana-utils.py ...`, `python3 cmd/grafana-alert-utils.py ...`, `python3 cmd/grafana-access-utils.py ...` |
| Rust from git checkout | Best when validating or developing the Rust implementation | `cargo run --bin grafana-utils -- ...`, `cargo run --bin grafana-alert-utils -- ...` |

Notes:

- the Python package is the normal install path from this repository
- `grafana-access-utils` is currently Python-only; there is no Rust access-management CLI yet
- the Rust binaries are built from [`rust/`](rust/) and are not installed by `python3 -m pip install .`
- both implementations use the same command names and the same operator concepts

## Quick Start

Dashboard export, writing both `raw/` and `prompt/` variants:

```bash
python3 cmd/grafana-utils.py export-dashboard \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --export-dir ./dashboards \
  --overwrite
```

Dashboard export from one explicit Grafana org:

```bash
python3 cmd/grafana-utils.py export-dashboard \
  --url http://localhost:3000 \
  --basic-user "$GRAFANA_USERNAME" \
  --basic-password "$GRAFANA_PASSWORD" \
  --org-id 2 \
  --export-dir ./dashboards \
  --overwrite
```

Dashboard export across every visible Grafana org:

```bash
python3 cmd/grafana-utils.py export-dashboard \
  --url http://localhost:3000 \
  --basic-user "$GRAFANA_USERNAME" \
  --basic-password "$GRAFANA_PASSWORD" \
  --all-orgs \
  --export-dir ./dashboards \
  --overwrite
```

Dashboard list, including resolved datasource names per dashboard:

```bash
python3 cmd/grafana-utils.py list-dashboard \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --with-sources \
  --table
```

List dashboards from one explicit Grafana org:

```bash
python3 cmd/grafana-utils.py list-dashboard \
  --url http://localhost:3000 \
  --basic-user "$GRAFANA_USERNAME" \
  --basic-password "$GRAFANA_PASSWORD" \
  --org-id 2 \
  --json
```

List dashboards across every visible Grafana org:

```bash
python3 cmd/grafana-utils.py list-dashboard \
  --url http://localhost:3000 \
  --basic-user "$GRAFANA_USERNAME" \
  --basic-password "$GRAFANA_PASSWORD" \
  --all-orgs \
  --json
```

List live dashboards without writing export files:

```bash
python3 cmd/grafana-utils.py list-dashboard \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin
```

List live dashboards as a table with folder tree path:

```bash
python3 cmd/grafana-utils.py list-dashboard \
  --table \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin
```

List live dashboards as CSV:

```bash
python3 cmd/grafana-utils.py list-dashboard \
  --csv \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin
```

List live dashboards as JSON:

```bash
python3 cmd/grafana-utils.py list-dashboard \
  --json \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin
```

List live Grafana data sources as a table:

```bash
python3 cmd/grafana-utils.py list-data-sources \
  --table \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin
```

Dashboard API import from the raw export:

```bash
python3 cmd/grafana-utils.py import-dashboard \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --import-dir ./dashboards/raw \
  --replace-existing
```

Dashboard diff against the current Grafana state:

```bash
python3 cmd/grafana-utils.py diff \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
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

Access-management user listing, org scope with token auth:

```bash
python3 cmd/grafana-access-utils.py user list \
  --url http://127.0.0.1:3000 \
  --scope org \
  --token "$GRAFANA_API_TOKEN" \
  --table
```

Access-management user listing, global scope with Basic auth:

```bash
python3 cmd/grafana-access-utils.py user list \
  --url http://127.0.0.1:3000 \
  --scope global \
  --basic-user "$GRAFANA_USERNAME" \
  --basic-password "$GRAFANA_PASSWORD" \
  --json
```

Access-management user creation, global scope with Basic auth:

```bash
python3 cmd/grafana-access-utils.py user add \
  --url http://127.0.0.1:3000 \
  --basic-user "$GRAFANA_USERNAME" \
  --basic-password "$GRAFANA_PASSWORD" \
  --login automation-user \
  --email automation-user@example.com \
  --name "Automation User" \
  --password temporary-password \
  --json
```

Access-management user update, global scope with Basic auth:

```bash
python3 cmd/grafana-access-utils.py user modify \
  --url http://127.0.0.1:3000 \
  --basic-user "$GRAFANA_USERNAME" \
  --basic-password "$GRAFANA_PASSWORD" \
  --login automation-user \
  --set-email automation-user+ops@example.com \
  --set-name "Automation User Ops" \
  --set-org-role Editor \
  --set-grafana-admin true \
  --json
```

Access-management user delete, global scope with Basic auth:

```bash
python3 cmd/grafana-access-utils.py user delete \
  --url http://127.0.0.1:3000 \
  --basic-user "$GRAFANA_USERNAME" \
  --basic-password "$GRAFANA_PASSWORD" \
  --login automation-user \
  --scope global \
  --yes \
  --json
```

Access-management team listing, org scope with token auth:

```bash
python3 cmd/grafana-access-utils.py team list \
  --url http://127.0.0.1:3000 \
  --token "$GRAFANA_API_TOKEN" \
  --table
```

Access-management team membership change, org scope with token auth:

```bash
python3 cmd/grafana-access-utils.py team modify \
  --url http://127.0.0.1:3000 \
  --token "$GRAFANA_API_TOKEN" \
  --name Ops \
  --add-member alice@example.com \
  --add-admin bob@example.com \
  --json
```

Access-management team creation, org scope with token auth:

```bash
python3 cmd/grafana-access-utils.py team add \
  --url http://127.0.0.1:3000 \
  --token "$GRAFANA_API_TOKEN" \
  --name platform-operators \
  --email platform-operators@example.com \
  --member alice@example.com \
  --admin bob@example.com
```

Access-management service-account listing, org scope with token auth:

```bash
python3 cmd/grafana-access-utils.py service-account list \
  --url http://127.0.0.1:3000 \
  --token "$GRAFANA_API_TOKEN" \
  --table
```

Access-management service-account creation:

```bash
python3 cmd/grafana-access-utils.py service-account add \
  --url http://127.0.0.1:3000 \
  --token "$GRAFANA_API_TOKEN" \
  --name automation-bot \
  --role Editor \
  --json
```

Access-management service-account token creation:

```bash
python3 cmd/grafana-access-utils.py service-account token add \
  --url http://127.0.0.1:3000 \
  --token "$GRAFANA_API_TOKEN" \
  --name automation-bot \
  --token-name automation-bot-short-lived \
  --seconds-to-live 3600 \
  --json
```

## Dashboard Utility

`grafana-utils` has explicit subcommands:

- `list`
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
- API re-import through `grafana-utils import-dashboard`

Use `prompt/` when you want:

- Grafana web UI import
- datasource mapping prompts
- Grafana-style `__inputs`

### Important Export Options

| Option | Purpose |
| --- | --- |
| `--url` | Grafana base URL. Default: `http://localhost:3000` |
| `--export-dir` | Root export directory. Default: `dashboards/` |
| `--page-size` | Dashboard search page size. Default: `500` |
| `--org-id ORG_ID` | For `list-dashboard` or `export-dashboard`, switch to one explicit Grafana org ID; requires Basic auth |
| `--all-orgs` | For `list-dashboard` or `export-dashboard`, enumerate visible Grafana orgs and aggregate list output or export each org; requires Basic auth |
| `--with-sources` | For `list-dashboard`, fetch each dashboard payload and include datasource names used by that dashboard; CSV and JSON also add datasource UIDs |
| `list-data-sources --table|--csv|--json` | List live Grafana data sources in human-readable or machine-readable output |
| `--flat` | Do not create per-folder subdirectories |
| `--overwrite` | Replace existing exported files |
| `--without-dashboard-raw` | Skip the `raw/` export variant |
| `--without-dashboard-prompt` | Skip the `prompt/` export variant |
| `--dry-run` | Preview export output without writing files |
| `--verify-ssl` | Enable TLS certificate verification |

For dashboard listing:

- default `list-dashboard` output shows `uid`, `name`, `folder`, `folderUid`, resolved folder tree path, `org`, and `orgId`
- `list-dashboard --org-id <ID>` reads dashboards from that explicit org instead of the current auth context and requires Basic auth
- `list-dashboard --all-orgs` aggregates dashboards across every visible org and requires Basic auth
- `list-dashboard --with-sources` adds datasource names per dashboard to text, table, CSV, and JSON output
- `list-dashboard --with-sources --csv` also adds a `sourceUids` column with best-effort datasource UIDs
- `list-dashboard --with-sources --json` also adds a `sourceUids` array with best-effort datasource UIDs
- `list-dashboard --with-sources` is slower than plain `list-dashboard` because it fetches each dashboard payload and the datasource catalog

For dashboard export:

- `export-dashboard --org-id <ID>` exports dashboards from that explicit org instead of the current auth context and requires Basic auth
- `export-dashboard --all-orgs` exports dashboards from every visible org and requires Basic auth
- `export-dashboard --all-orgs` writes per-org trees such as `org_2_Org_Two/raw/...` and `org_2_Org_Two/prompt/...` to avoid cross-org file collisions

For datasource listing:

- `list-data-sources` shows `uid`, `name`, `type`, `url`, and `isDefault`
- `list-data-sources --table` renders fixed-width columns
- `list-data-sources --csv` emits `uid,name,type,url,isDefault`
- `list-data-sources --json` emits an array of datasource objects

### Raw Export

Raw export preserves the Grafana dashboard identity as much as possible:

- preserves dashboard `uid`
- preserves dashboard `title`
- sets numeric dashboard `id` to `null`
- keeps datasource references unchanged

If you only want the prompt variant:

```bash
python3 cmd/grafana-utils.py export-dashboard \
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
python3 cmd/grafana-utils.py export-dashboard \
  --export-dir ./dashboards \
  --without-dashboard-prompt
```

### Dashboard Import

Dashboard import reads normal dashboard JSON through the Grafana API.

Example:

```bash
python3 cmd/grafana-utils.py import-dashboard \
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

## Access Utility

`grafana-access-utils` is the access-management CLI track.

Current implementation scope:

- Python implementation only
- `user list`
- `user add`
- `user modify`
- `user delete`
- `team list`
- `team modify`
- `team add`
- `service-account list`
- `service-account add`
- `service-account token add`
- no `team delete` or `group` alias commands yet

Initial auth model:

- `user list --scope org` may use token auth or Basic auth
- `user list --scope global` requires Basic auth because Grafana global user APIs are Basic-auth-first admin workflows
- `user add` requires Basic auth because Grafana user-creation is a server-admin workflow
- `user modify` requires Basic auth because it uses global and admin user-management APIs
- `user delete --scope global` requires Basic auth because it uses the global admin delete API
- `user delete --scope org` may use token auth or Basic auth
- `team list` is org-scoped and may use token auth or Basic auth
- `team modify` is org-scoped and may use token auth or Basic auth
- `team add` is org-scoped and may use token auth or Basic auth
- service-account commands are org-scoped and may use token auth or Basic auth

Output modes:

- compact text by default
- `--table`
- `--csv`
- `--json`

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
- `make build-rust-macos-arm64`
- `make build-rust-linux-amd64`
- `make build-rust-linux-amd64-zig`
- `make build`
- `make test-python`
- `make test-rust`
- `make test-rust-live`
- `make test-access-live`
- `make test`

Artifact locations:

- `make build-python` writes the wheel into `dist/`
- `make build-rust` writes release binaries into `rust/target/release/`
- `make build-rust-macos-arm64` writes native Apple Silicon Rust binaries into `dist/macos-arm64/`
- `make build-rust-linux-amd64` writes Linux `amd64` Rust binaries into `dist/linux-amd64/`
- `make build-rust-linux-amd64-zig` writes Linux `amd64` Rust binaries into `dist/linux-amd64/` without Docker

### Rust Build and Run

Build Rust release binaries:

```bash
make build-rust
```

Build native macOS Apple Silicon Rust release binaries into a platform output directory:

```bash
make build-rust-macos-arm64
```

Build Linux `amd64` Rust release binaries from macOS or another non-Linux host with Docker:

```bash
make build-rust-linux-amd64
```

Build Linux `amd64` Rust release binaries from macOS without Docker, using `zig`:

```bash
make build-rust-linux-amd64-zig
```

Run the Rust dashboard CLI from the repo and export dashboards from local Grafana:

```bash
cd rust
cargo run --bin grafana-utils -- export-dashboard \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --export-dir ./dashboards
```

List dashboards from local Grafana with the Rust CLI:

```bash
cd rust
cargo run --bin grafana-utils -- list-dashboard \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --table
```

Compare raw dashboard exports with local Grafana using the Rust CLI:

```bash
cd rust
cargo run --bin grafana-utils -- diff \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --import-dir ./dashboards/raw
```

Show Rust dashboard CLI help:

```bash
cd rust
cargo run --bin grafana-utils -- -h
```

Linux `amd64` build notes:

- `make build-rust-macos-arm64` is the explicit native Apple Silicon release path and copies binaries into `dist/macos-arm64/`
- `make build-rust-linux-amd64` uses Docker with the official Rust image
- `make build-rust-linux-amd64-zig` uses local `zig`, `cargo-zigbuild`, and a `rustup` target instead of Docker
- output binaries are written to `dist/linux-amd64/`
- current output names are `dist/linux-amd64/grafana-utils` and `dist/linux-amd64/grafana-alert-utils`
- this path is intended for macOS hosts that need Linux release artifacts without installing a local cross-linker

Run the Rust alerting CLI from the repo:

```bash
cd rust
cargo run --bin grafana-alert-utils -- -h
```

Run the Docker-backed Rust live smoke test:

```bash
make test-rust-live
```

Run the Docker-backed Python access live smoke test:

```bash
make test-access-live
```

Notes:

- requires Docker plus local access to the Docker daemon
- defaults to `grafana/grafana:12.4.1` and can be overridden with `GRAFANA_IMAGE=...`
- uses a random localhost port by default; set `GRAFANA_PORT=43000` if you want a fixed port
- starts a temporary Grafana container, seeds one dashboard, one datasource, and one contact point
- validates Rust dashboard export/import/diff/dry-run and Rust alerting export/import/diff/dry-run

Python access live smoke test notes:

- `make test-access-live` runs `scripts/test-python-access-live-grafana.sh`
- the script defaults to `grafana/grafana:12.4.1` and binds Grafana to a random localhost port unless `GRAFANA_PORT` is set explicitly
- it bootstraps an API token, then validates `user add`, `user modify`, `user delete`, `team add`, `team modify`, `team list`, `service-account add`, `service-account token add`, and `service-account list`
- useful overrides: `GRAFANA_IMAGE`, `GRAFANA_PORT`, `GRAFANA_USER`, `GRAFANA_PASSWORD`, `PYTHON_BIN`

## Authentication and TLS

Authentication methods:

- API token with `--token` or legacy `--api-token`
- Basic auth with `--basic-user` and `--basic-password` or legacy `--username` and `--password`

Auth note:

- prefer either token auth or Basic auth for one command, not both
- the CLIs reject partial Basic auth input such as only `--basic-user` without `--basic-password`
- `GRAFANA_API_TOKEN`, `GRAFANA_USERNAME`, and `GRAFANA_PASSWORD` still work as environment fallbacks
- for `grafana-access-utils`, org-scoped `user list` can use token auth or Basic auth
- for `grafana-access-utils`, global `user list` requires Basic auth
- for `grafana-access-utils`, `user add` requires Basic auth
- for `grafana-access-utils`, `user modify` requires Basic auth
- for `grafana-access-utils`, `user delete --scope global` requires Basic auth
- for `grafana-access-utils`, `user delete --scope org` can use token auth or Basic auth
- for `grafana-access-utils`, `team list` is org-scoped and can use token auth or Basic auth
- for `grafana-access-utils`, `team add` is org-scoped and can use token auth or Basic auth
- for `grafana-access-utils`, `team modify` is org-scoped and can use token auth or Basic auth
- for `grafana-access-utils`, service-account commands are org-scoped and can use token auth or Basic auth

Username/password example:

```bash
export GRAFANA_USERNAME='admin'
export GRAFANA_PASSWORD='admin'
python3 cmd/grafana-utils.py export-dashboard \
  --url http://localhost:3000 \
  --basic-user "$GRAFANA_USERNAME" \
  --basic-password "$GRAFANA_PASSWORD" \
  --export-dir ./dashboards
```

TLS note:

- SSL verification is disabled by default
- add `--verify-ssl` when you want strict certificate verification

Example:

```bash
python3 cmd/grafana-utils.py export-dashboard --verify-ssl
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
python3 cmd/grafana-access-utils.py -h
python3 cmd/grafana-access-utils.py user list -h
python3 cmd/grafana-access-utils.py user add -h
python3 cmd/grafana-access-utils.py user modify -h
python3 cmd/grafana-access-utils.py user delete -h
python3 cmd/grafana-access-utils.py team list -h
python3 cmd/grafana-access-utils.py team add -h
python3 cmd/grafana-access-utils.py team modify -h
python3 cmd/grafana-access-utils.py service-account list -h
python3 cmd/grafana-access-utils.py service-account add -h
python3 cmd/grafana-access-utils.py service-account token add -h
cd rust && cargo test
make test-rust-live
make test-access-live
```

## Documentation

- English README: [`README.md`](README.md)
- Traditional Chinese README: [`README.zh-TW.md`](README.zh-TW.md)
- recent change history: [`CHANGELOG.md`](CHANGELOG.md)
- maintainer and implementation notes: [`DEVELOPER.md`](DEVELOPER.md)
