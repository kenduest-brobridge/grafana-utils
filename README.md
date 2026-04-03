# Grafana Utilities (`grafana-util`)

[![Version](https://img.shields.io/badge/version-0.6.3-blue.svg)](https://github.com/kenduest-brobridge/grafana-utils/releases)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-linux%20%7C%20macos-lightgrey.svg)](#install)

Language: **English** | [繁體中文版](README.zh-TW.md)

`grafana-util` is a high-performance, Rust-powered CLI designed for **Grafana Operators**. It specializes in inventory management, export/import workflows, drift detection, and staged change review at scale.

## 🚀 Why `grafana-util`?

Unlike basic API scripts, `grafana-util` is built for estate-level operations where safety and reproducibility matter:

- 🛡️ **Safe Mutations**: Use `dry-run`, `plan`, and `review` workflows to avoid "blind" changes.
- 📂 **Structured Exports**: Clean, version-control-friendly directory structures for Dashboards, Datasources, and Alerts.
- 🔍 **Deep Inspection**: Analyze dashboard queries and datasource dependencies in one command.
- ⚡ **Performance**: Native Rust binary for fast processing of large Grafana estates.

## 🛠️ Main Command Areas

| Area | Primary use | Typical commands |
| --- | --- | --- |
| `dashboard` | dashboard inventory, export/import, diff, analysis, screenshot | `list`, `export`, `import`, `diff`, `inspect-export`, `inspect-live`, `browse`, `screenshot` |
| `datasource` | datasource inventory, masked recovery, live mutation | `list`, `export`, `import`, `diff`, `browse`, `add`, `modify`, `delete` |
| `alert` | alert management, review/apply workflows | `plan`, `apply`, `delete`, `export`, `import`, `diff`, `list-*`, `add-rule`, `clone-rule` |
| `access` | org, user, team, and service-account inventory | `org ...`, `user ...`, `team ...`, `service-account ...` |
| `change` | staged review-first change workflow | `summary`, `plan`, `review`, `apply`, `preflight`, `bundle-preflight` |
| `overview` | operator-facing whole-project summary | `overview`, `overview live` |
| `status` | canonical staged/live status contract | `status staged`, `status live` |
| `profile` | repo-local connection defaults | `init`, `list`, `show` |

## 📥 Install

Install the latest release:
```bash
curl -fsSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-utils/main/scripts/install.sh | sh
```

Pin a version or install location:
```bash
curl -fsSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-utils/main/scripts/install.sh | BIN_DIR=/usr/local/bin VERSION=v0.6.3 sh
```

If you already have a checkout:
```bash
sh ./scripts/install.sh
```

Build from source:
```bash
cd rust && cargo build --release
```

## 🏎️ Quick Start

Check the installed version and available command surface:

```bash
grafana-util --version
grafana-util version
grafana-util -h
grafana-util dashboard -h
grafana-util datasource -h
grafana-util alert -h
grafana-util access -h
grafana-util change -h
grafana-util overview -h
grafana-util status -h
```

### Examples (Validated against Grafana `12.4.1`)

**List dashboards across orgs with datasource context:**
```bash
grafana-util dashboard list \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --all-orgs \
  --with-sources \
  --table
```

**Inspect an exported dashboard tree:**
```bash
grafana-util dashboard inspect-export \
  --import-dir ./dashboards/raw \
  --output-format report-table
```

**Preview a dashboard import before applying it:**
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

**Export alerting resources for review or migration:**
```bash
grafana-util alert export \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --output-dir ./alerts \
  --overwrite
```

**Build a reviewable alert plan from desired files:**
```bash
grafana-util alert plan \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --desired-dir ./alerts/desired \
  --prune \
  --output json
```

## ⚠️ Important Workflow Rules

### Dashboard exports are split into distinct lanes
`dashboard export` writes three different outputs on purpose:
- `raw/`: the canonical grafana-util replay/import lane.
- `prompt/`: Grafana UI import lane.
- `provisioning/`: Grafana file provisioning lane.

These lanes are not interchangeable. Use the lane that matches the workflow you are performing.

### Datasource export uses a masked recovery contract
`datasource export` writes:
- `datasources.json`: the canonical masked recovery and replay contract.
- `provisioning/datasources.yaml`: a derived provisioning projection.

Treat `datasources.json` for restore and replay. Treat `provisioning/` as a projection for Grafana file provisioning, not as the primary restore source.

### Alert management has separate authoring and apply phases
The alert surface is intentionally split:
- Author desired-state files with commands such as `add-rule`, `clone-rule`, and `add-contact-point`.
- Review the delta with `alert plan`.
- Execute only reviewed operations with `alert apply`.

## 📊 Coverage By Area (Maturity Map)

| Area | Current depth | Notes |
| --- | --- | --- |
| `dashboard` | deepest surface | most complete analysis, export/import, and review tooling |
| `datasource` | deep and mature | supports live mutation plus file-based recovery/replay |
| `alert` | mature | supports both review/apply management and migration-style export/import |
| `access` | mature | strongest for inventory, replay, and controlled rebuilds |
| `change` | advanced | review-first staged workflow rather than direct mutation |
| `overview` | stable human entrypoint | best first stop for operator handoff and triage |
| `status` | stable contract surface | use when you need one cross-domain staged/live status view |

## 📖 Documentation

- 📘 [English User Guide](docs/user-guide/en/index.md)
- 📙 [繁體中文使用者指南](docs/user-guide/zh-TW/index.md)
- 🛠️ [Developer Guide](docs/DEVELOPER.md)
- 📜 [Changelog](CHANGELOG.md)

## ⚖️ License

Distributed under the MIT License. See `LICENSE` for more information.
