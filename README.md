# 📊 Grafana Utilities (Operator Toolkit)

Language: **English** | [繁體中文版](README.zh-TW.md)

`grafana-utils` is an operator-focused toolkit designed for Grafana administrators and SREs.

## Contents

- [What This Is](#what-this-is)
- [Support Overview](#support-overview)
- [Get The Binary](#get-the-binary)
- [Quick Start](#quick-start)
- [Command Map](#command-map)
- [Documentation](#documentation)
- [Compatibility](#compatibility)
- [Project Status](#project-status)

## What This Is

`grafana-util` helps operators:
- inventory dashboards, datasources, alerts, orgs, users, teams, and service accounts
- export, import, diff, and dry-run Grafana state changes
- inspect dashboards for governance, query usage, and datasource dependencies
- capture dashboards and panels as screenshots or PDFs

## 🏗️ Technical Architecture

The current maintained CLI is the Rust-based `grafana-util` binary.
- User-facing docs and releases target the Rust binary.
- Older Python implementation details remain in maintainer docs as historical and maintainer-only reference material.

## Support Overview

Use this as a quick capability summary:

- `Dashboard`: list, inspect, capture, export/import/delete/diff. Import is workflow-driven with dry-run and folder-aware migration; delete supports UID, folder-path subtree selection, and interactive preview.
- `Alerting`: list plus export/import/diff for rules and related alerting resources.
- `Datasource`: list, export/import/diff, and live add/modify/delete. Includes dry-run and multi-org replay support.
- `Access User`: list, add/modify/delete, export/import/diff for global and org-scoped user lifecycle.
- `Access Org`: list, add/modify/delete, export/import for org lifecycle and membership replay.
- `Access Team`: list, add/modify/delete, export/import/diff with membership-aware sync.
- `Access Service Account`: list, add/delete, export/import/diff, plus token add/delete workflows.

## Get The Binary

Download pages:
- [Latest release](https://github.com/kenduest-brobridge/grafana-utils/releases/latest)
- [All releases](https://github.com/kenduest-brobridge/grafana-utils/releases)

Download flow:
- Open the release page.
- Expand `Assets`.
- Download the prebuilt `grafana-util` archive for your OS and CPU.

If you are not using a tagged release yet, build locally from source:
```bash
cd rust && cargo build --release
```

## 🛠️ Quick Start

Check the CLI surface first:
```bash
grafana-util -h
grafana-util dashboard -h
grafana-util datasource -h
grafana-util alert -h
grafana-util access -h
```

## Common Operator Scenarios

List dashboards:
```bash
grafana-util dashboard list \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --with-sources \
  --table
```

Inspect live dashboards:
```bash
grafana-util dashboard inspect-live \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --output-format governance-json
```

List datasources:
```bash
grafana-util datasource list \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --table
```

List users:
```bash
grafana-util access user list \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --scope global \
  --table
```

Export dashboards:
```bash
grafana-util dashboard export \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --export-dir ./dashboards \
  --overwrite
```

Preview a dashboard import:
```bash
grafana-util dashboard import \
  --url http://localhost:3000 \
  --import-dir ./dashboards/raw \
  --replace-existing \
  --dry-run --table
```

Preview a dashboard delete:
```bash
grafana-util dashboard delete \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --path "Platform / Infra" \
  --dry-run --table
```

## Command Map

Use this when you want the right entrypoint quickly.

- `grafana-util dashboard ...`
  - inventory, export/import/delete/diff, inspect, screenshot, PDF capture
- `grafana-util datasource ...`
  - inventory, export/import/diff, live add/modify/delete
- `grafana-util alert ...`
  - list, export/import/diff for alerting resources
- `grafana-util access ...`
  - org, user, team, and service-account inventory and change workflows
- `grafana-util sync ...`
  - staged bundle, preflight, review, and apply flows

## Documentation

- **[Traditional Chinese Guide](docs/user-guide-TW.md)**: Detailed commands and authentication rules.
- **[English User Guide](docs/user-guide.md)**: Standard operator instructions.
- **[Technical Overview (Rust)](docs/overview-rust.md)**
- **[Developer Guide](docs/DEVELOPER.md)**: Maintenance and contribution notes.

## Compatibility
- **OS**: Linux, macOS.
- **Runtime**: Rust release binary.
- **Grafana**: Supports v8.x, v9.x, v10.x+.

## Project Status

This project is under active development. Bug reports and operator feedback are welcome.
