# 📊 Grafana Utilities (Operator Toolkit)

Language: **English** | [繁體中文版](README.zh-TW.md)

`grafana-utils` is an operator-focused toolkit designed for Grafana administrators and SREs.

## Contents

- [What This Is](#what-this-is)
- [Get The Binary](#get-the-binary)
- [Quick Start](#quick-start)
- [Command Map](#command-map)
- [Support Matrix](#support-matrix)
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
- Python implementation details remain in maintainer docs for parity and validation work.

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

## Command Map

Use this when you want the right entrypoint quickly.

- `grafana-util dashboard ...`
  - inventory, export/import/diff, inspect, screenshot, PDF capture
- `grafana-util datasource ...`
  - inventory, export/import/diff, live add/modify/delete
- `grafana-util alert ...`
  - list, export/import/diff for alerting resources
- `grafana-util access ...`
  - org, user, team, and service-account inventory and change workflows
- `grafana-util sync ...`
  - staged bundle, preflight, review, and apply flows

## Support Matrix

| Domain | List / Inspect / Capture | Add / Modify / Delete | Export / Import / Diff | Notes |
| --- | --- | --- | --- | --- |
| Dashboard | Yes | No | Yes | Import-driven changes, folder-aware migration, dry-run support, and screenshot/PDF capture |
| Alerting | Yes | No | Yes | Import-driven rule and contact-point workflows |
| Datasource | Yes | Yes | Yes | Dry-run and diff supported, plus all-org export and routed multi-org import with missing-org creation |
| Access User | Yes | Yes | Yes | Supports `--password-file` / `--prompt-user-password` and `--set-password-file` / `--prompt-set-password` |
| Access Org | Yes | Yes | Yes | Includes org membership replay during import |
| Access Team | Yes | Yes | Yes | Membership-aware export/import/diff |
| Access Service Account | Yes | Yes | Yes | Snapshot export/import/diff plus token add/delete workflows |

## Documentation

- **[Traditional Chinese Guide](docs/user-guide-TW.md)**: Detailed commands and authentication rules.
- **[English User Guide](docs/user-guide.md)**: Standard operator instructions.
- **[Technical Overview (Rust)](docs/overview-rust.md)**
- **[Developer Guide](docs/DEVELOPER.md)**: Maintenance and contribution notes.

## Compatibility
- **OS**: RHEL 8+, macOS (ARM/Intel), Linux.
- **Runtime**: Rust release binary.
- **Grafana**: Supports v8.x, v9.x, v10.x+.

## Project Status

This project is under active development. Bug reports and operator feedback are welcome.
