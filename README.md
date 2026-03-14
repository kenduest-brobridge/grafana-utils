# Grafana Utilities

Language: English | [Traditional Chinese README.zh-TW.md](README.zh-TW.md)

Grafana Utilities is a practical operator toolkit for a common Grafana problem: Grafana is easy to click through, but hard to inventory, compare, back up, and replay cleanly across the same instance or between environments.

This project gives you one unified CLI to:

- inventory dashboards, datasources, alerting resources, users, teams, and service accounts
- export Grafana state into versionable JSON
- import and restore that state into the same environment or a different one
- compare local exports with live Grafana before you change anything
- run dry-runs first so migration and cleanup work is predictable

## Why This Exists

Traditional Grafana operations are painful when you need repeatable change control.

- Dashboard and alert changes are often spread across UI clicks instead of reviewable files.
- It is hard to answer simple operator questions such as "what exists now?", "what changed?", and "what will this import overwrite?"
- Moving content between dev, staging, and production is usually manual, fragile, and difficult to audit.
- Datasource, dashboard, and alert dependencies are easy to drift over time.
- Access management work is tedious when you need to review users, teams, service accounts, and tokens at scale.

Grafana Utilities turns those workflows into explicit CLI operations with stable output and export artifacts you can diff, review, and re-run.

## What It Solves Well

- Environment inventory: list what exists now instead of browsing the UI page by page.
- Backup and rollback: export Grafana resources into files you can keep in git.
- Same-environment restore: re-import exported state after accidental deletion or drift.
- Cross-environment migration: move dashboards, datasource inventory, and alerting resources from one Grafana to another in a controlled way.
- Change review: compare exported state to live state before import.
- Safer operations: use `--dry-run` to see predicted actions before writing anything.

## Core Capabilities

- Unified CLI domains:
  - `dashboard`
  - `datasource`
  - `alert`
  - `access`
- Two implementation paths with the same command model:
  - installed CLI / Python package
  - Rust source-tree CLI
- Export formats that support both:
  - API-friendly restore workflows
  - Grafana UI import workflows
- Operator-friendly outputs for:
  - table
  - csv
  - json

## How To Think About It

Use this tool when you want Grafana operations to behave more like infrastructure operations:

- observable
- reviewable
- repeatable
- scriptable

If your current process depends on "open Grafana, click around, hope nothing drifted", this tool is meant to replace that with explicit inventory, export, diff, and import flows.

## Entry Points

Installed CLI:

```text
grafana-util <domain> <command> [options]
```

Source-tree entrypoints:

```text
python3 -m grafana_utils <domain> <command> [options]
cargo run --bin grafana-util -- <domain> <command> [options]
```

Main domains:

- `dashboard`: export, import, list, diff, inspect
- `datasource`: list, export, import, diff
- `alert`: export, import, diff, list
- `access`: users, teams, service accounts, tokens

## Quick Start

Export dashboards:

```bash
grafana-util dashboard export \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --export-dir ./dashboards \
  --overwrite
```

List dashboards:

```bash
grafana-util dashboard list \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --table
```

Dry-run a dashboard restore:

```bash
grafana-util dashboard import \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --import-dir ./dashboards/raw \
  --replace-existing \
  --dry-run \
  --table
```

Compare exported dashboards with live Grafana:

```bash
grafana-util dashboard diff \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --import-dir ./dashboards/raw
```

## Install

Python package:

```bash
python3 -m pip install .
```

Rust binary from source:

```bash
cd rust
cargo build --release
```

## Documentation

- Operator guide: [docs/user-guide.md](docs/user-guide.md)
- Traditional Chinese operator guide: [docs/user-guide-TW.md](docs/user-guide-TW.md)
- Python implementation overview: [docs/overview-python.md](docs/overview-python.md)
- Rust implementation overview: [docs/overview-rust.md](docs/overview-rust.md)
- Maintainer notes: [DEVELOPER.md](DEVELOPER.md)

## Compatibility

- RHEL 8 and later
- Python runtime target: 3.9+

