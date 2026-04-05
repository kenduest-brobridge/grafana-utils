# grafana-util
### A Rust CLI for Grafana Operations and Administration

[![CI](https://img.shields.io/github/actions/workflow/status/kenduest-brobridge/grafana-util/ci.yml?branch=main)](https://github.com/kenduest-brobridge/grafana-util/actions)
[![License](https://img.shields.io/github/license/kenduest-brobridge/grafana-util)](LICENSE)
[![Version](https://img.shields.io/github/v/tag/kenduest-brobridge/grafana-util)](https://github.com/kenduest-brobridge/grafana-util/tags)

English | [繁體中文](./README.zh-TW.md)

**Repeatable Grafana workflows for dashboards, alerts, datasources, access control, and operational review.**

`grafana-util` is a personal long-term Rust tool built around real Grafana maintenance pain. I use it to make day-to-day workflows more reviewable, governance-aware, and repeatable across dashboards, alerts, datasources, access control, and estate-wide status checks. It is aimed at SREs, platform engineers, sysadmins, and maintainers who want safer operating paths and automation-friendly output instead of ad hoc API calls, UI-only changes, or one-off scripts.

It is not trying to be a complete Grafana platform or a replacement for every other CLI. The design center here is operator workflow: inspect first, review changes before apply, keep secrets handled deliberately, and prefer repeatable paths over hand-built command sequences.

If you already know tools such as `grafanactl` or `grizzly`, the difference here is mostly design orientation:

- `grafanactl` is closer to a general resource and API-oriented Grafana CLI.
- `grizzly` is closer to declarative Grafana-as-code management.
- `grafana-util` is more focused on reviewable operations, inspection/governance flows, and safer migration or replay paths.

---

## Why `grafana-util`?

| Capability | Standard CLI / curl | **grafana-util** |
| :--- | :---: | :--- |
| **Multi-Org Discovery** | Manual per org | ✅ One command to scan all orgs |
| **Dependency Audit** | Limited | ✅ Detect broken datasource dependencies before import |
| **Alerting Lifecycle** | Direct mutation only | ✅ Reviewable **Plan/Apply** workflow |
| **Secret Handling** | Easy to mishandle | ✅ **Masked Recovery** and profile secret modes |
| **Review Surface** | Raw JSON | ✅ Interactive TUI and structured table/report output |

---

## Before / After

| Before | After with `grafana-util` |
| :--- | :--- |
| You click through Grafana UI screens or piece together raw API calls just to understand what exists. | Start with `overview live` or `status live`, then decide where to drill in. |
| Export/import is a one-off action with weak review points. | Export to a reviewable tree, inspect dependencies, dry-run replay, then import. |
| Alerting changes are hard to explain before apply. | Use `change summary`, `change preflight`, and `alert plan` before live mutation. |
| Secrets get copied into shell history or flat files. | Keep auth in prompt flows, env variables, or repo-local profiles with explicit secret modes. |

This is the main shift: the tool does not just give you commands. It gives you a safer operating sequence.

---

## Quick Start

### Install

```bash
# 1. Install via one-liner
curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-util/main/scripts/install.sh | sh
```

```bash
# 2. Confirm the installed version
grafana-util --version
```

```bash
# 3. Inspect current Grafana status
grafana-util overview live --url http://my-grafana:3000 --basic-user admin --prompt-password --output-format interactive
```

### Install options

Pinned release:

```bash
# Install one pinned release.
VERSION=0.8.0 \
  curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-util/main/scripts/install.sh | sh
```

Custom install directory:

```bash
# Install into one explicit binary directory.
BIN_DIR="$HOME/.local/bin" \
  curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-util/main/scripts/install.sh | sh
```

Show installer help first:

```bash
sh ./scripts/install.sh --help
```

- Release downloads: <https://github.com/kenduest-brobridge/grafana-util/releases>
- Published binaries: standard release binaries are published for `linux-amd64` and `macos-arm64`. If you need the browser-enabled screenshot build, download the `*-browser-*` archive from the same release.
- Default install location: the script uses `BIN_DIR` when you set it, otherwise `/usr/local/bin` if writable, and otherwise falls back to `$HOME/.local/bin`.
- PATH setup: if the chosen install directory is not on `PATH`, the script prints the exact `zsh` / `bash` snippet to add it.

---

## Useful Examples

These are the examples most people actually look for first: inspect the current environment, export a safe reviewable tree, preview changes before apply, and recover datasource secrets without hand-editing JSON.

Most examples below focus on the workflow itself, so repeated connection flags are omitted after the first few samples. In practice you can supply Grafana connection details with `--url`, `--basic-user`, `--basic-password`, `--prompt-password`, `--token`, or `--profile`. Environment variables such as `GRAFANA_USERNAME`, `GRAFANA_PASSWORD`, and `GRAFANA_API_TOKEN` also work where supported. If you need the full connection setup first, start with [Getting Started](./docs/user-guide/en/getting-started.md).

### 1. Get a live operational overview before changing anything

```bash
# Open the interactive overview for the current Grafana environment.
grafana-util overview live \
  --url http://my-grafana:3000 \
  --basic-user admin \
  --prompt-password \
  --output-format interactive
```

Use this first when you need to answer: "What does this Grafana look like right now?" without clicking through the UI.

Expected result:

```text
Live status: ready
Dashboards: ...
Alerts: ...
Datasources: ...
```

### 2. Export dashboards into a reviewable tree

```bash
# Export dashboards across all organizations into a local backup tree.
grafana-util dashboard export --all-orgs --export-dir ./backup --progress
```

This is the starting point for backup, migration, review, and CI-style inspection.

### 3. Check whether an export tree is safe to import

```bash
# Audit datasource dependencies and structural issues in an exported dashboard tree.
grafana-util dashboard inspect-export \
  --import-dir ./backup/raw \
  --output-format report-table
```

Use this before import when you want to catch broken datasource references or suspicious structure early.

Expected result:

```text
Sources
  prometheus-main
  loki-prod
```

### 4. Preview dashboard import behavior before applying it

```bash
# Dry-run a dashboard import and render the result as a table.
grafana-util dashboard import \
  --import-dir ./backup/raw \
  --replace-existing \
  --dry-run \
  --table
```

This is useful when you want to see what would change before touching the live server.

### 5. Review alerting changes before apply

```bash
# Build a reviewable alert plan from desired state versus the live server.
grafana-util alert plan \
  --desired-dir ./alerts/desired \
  --prune \
  --output-format json
```

```bash
# Preview where a critical alert would route before applying the change.
grafana-util alert preview-route \
  --desired-dir ./alerts/desired \
  --label team=sre \
  --severity critical
```

Use these together when you need a review surface instead of mutating Grafana alerting blindly.

### 6. Export and re-import datasources with secret recovery

```bash
# Export datasources with secrets masked for review or version control.
grafana-util datasource export --export-dir ./datasources --overwrite
```

```bash
# Re-import datasources and recover required secrets interactively.
grafana-util datasource import \
  --import-dir ./datasources \
  --replace-existing \
  --prompt-password
```

This is the practical path for moving datasource configuration between environments without committing raw credentials.

---

## A First Operator Flow

If you want one end-to-end path instead of isolated commands, use this sequence:

1. `overview live` to confirm the target Grafana is reachable and worth inspecting
2. `dashboard export` to create a reviewable tree
3. `dashboard inspect-export` to find missing datasource dependencies before import
4. `dashboard import --dry-run` to preview replay behavior before live mutation

That sequence is the shortest public proof that the tool is doing more than wrapping a single endpoint.

---

## At a Glance

*   **Inspect before you change anything**: `overview`, `status`, export inspection, governance checks, and review surfaces for dashboards and alerts.
*   **Move Grafana assets safely between environments**: reviewable export/import workflows for dashboards, alerts, datasources, and access resources.
*   **Automate repeatable operations**: table/JSON-oriented output, non-interactive paths, and safer secret handling for CI/CD and batch jobs.

---

## Docs & Guides

Use the handbook for workflow and operational context, and the command pages when you need exact CLI syntax. The goal here is quick routing, not a second full manual.

If plain Markdown is awkward to read, generate the local HTML docs site and open the entrypoint:

```bash
# Purpose: Build the local HTML docs site and open the main entrypoint.
make html
open ./docs/html/index.html
```

On Linux, replace `open` with `xdg-open`. For a published browser-friendly copy, use <https://kenduest-brobridge.github.io/grafana-utils/>.

Open by need:

*   **Getting started**: [docs/user-guide/en/getting-started.md](./docs/user-guide/en/getting-started.md)
*   **Full handbook**: [docs/user-guide/en/index.md](./docs/user-guide/en/index.md)
*   **Command reference**: [docs/commands/en/index.md](./docs/commands/en/index.md)
*   **Troubleshooting**: [docs/user-guide/en/troubleshooting.md](./docs/user-guide/en/troubleshooting.md)
*   **Manpage**: [docs/man/grafana-util.1](./docs/man/grafana-util.1)

Open by role:

*   **New user**: [docs/user-guide/en/role-new-user.md](./docs/user-guide/en/role-new-user.md)
*   **SRE / operator**: [docs/user-guide/en/role-sre-ops.md](./docs/user-guide/en/role-sre-ops.md)
*   **Automation / CI owner**: [docs/user-guide/en/role-automation-ci.md](./docs/user-guide/en/role-automation-ci.md)
*   **Maintainer / developer**: [docs/DEVELOPER.md](./docs/DEVELOPER.md) and [docs/internal/maintainer-quickstart.md](./docs/internal/maintainer-quickstart.md)

---

## Project Notes
*   **Rust-first CLI**: the primary implementation lives under `rust/src/`.
*   **Validated against Grafana 12.4.1** in Docker-based environments.
*   **Automation-friendly**: predictable exit codes and structured output for CI/CD and batch workflows.

---

## Contributing
We welcome contributions! Please see our [Developer Guide](./docs/DEVELOPER.md) for setup instructions.

---
