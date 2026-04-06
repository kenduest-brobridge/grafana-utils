# grafana-util
### A Rust CLI for Grafana Operations and Administration

[![CI](https://img.shields.io/github/actions/workflow/status/kenduest-brobridge/grafana-util/ci.yml?branch=main)](https://github.com/kenduest-brobridge/grafana-util/actions)
[![License](https://img.shields.io/github/license/kenduest-brobridge/grafana-util)](LICENSE)
[![Version](https://img.shields.io/github/v/tag/kenduest-brobridge/grafana-util)](https://github.com/kenduest-brobridge/grafana-util/tags)

English | [繁體中文](./README.zh-TW.md)

**Standardized Grafana workflows for dashboards, alerts, datasources, access control, and operational review.**

`grafana-util` is a Rust-based CLI designed for day-to-day Grafana operations. It focuses on reviewable inventory, export/import, diff, replay, profile management, and secret handling so SREs and platform engineers can inspect changes before they apply them.

Its main strengths are a review-first workflow, separate paths for dashboard import/export formats, and reusable connection profiles that keep repeatable operations short and predictable.

---

## Supported Workflows

- **Dashboards**: Browse, list, export/import, diff, review, publish, and analyze. Supports `raw` (API-driven import), `prompt` (UI import), and `provisioning` (file-based) paths.
- **Datasources**: Masked recovery, secret-aware imports, and provisioning projection.
- **Alerts**: Export/import, diffing, planning (`plan`/`apply`), and routing preview.
- **Access**: Management of users, teams, organizations, service accounts, and tokens.
- **Change Management**: Review-first workflows (`inspect`, `check`, `preview`) before live mutation.
- **Status / Overview**: Readiness checks for live and staged environments.
- **Profiles**: Centralized connection management with support for `file`, `os`, and `encrypted-file` secret storage.
- **Snapshot**: Export and review of resource bundles.
- **Resource**: Read-only `inspect`/`get`/`list`/`describe` for live Grafana resources.

---

## Operational Shift

| Feature | Legacy Approach | with `grafana-util` |
| :--- | :--- | :--- |
| **Discovery** | Manual UI navigation or ad-hoc API calls to understand current state. | Start with `overview live` or `status live` for a unified environment snapshot. |
| **Dashboard Paths** | Ambiguity between API-driven import and UI import formats. | Dedicated paths for `raw`, `prompt`, and `provisioning` with conversion tools like `raw-to-prompt`. |
| **Reviews** | Changes applied directly without an intermediate review surface. | Use `change inspect`, `check`, and `preview` to audit changes before they touch the live server. |
| **Security** | Secrets often stored in shell history or plaintext files. | Centralized `profile` management with OS keyring or encrypted storage. |

---

## Quick Start

### Install

```bash
# Install via one-liner
curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-util/main/scripts/install.sh | sh
```

```bash
# Verify the installation
grafana-util --version
```

```bash
# Inspect current Grafana status
grafana-util overview live --url http://my-grafana:3000 --basic-user admin --prompt-password --output-format interactive
```

### Install Options

Pinned release:

```bash
# Install a specific pinned release.
VERSION=0.8.0 \
  curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-util/main/scripts/install.sh | sh
```

Custom install directory:

```bash
# Install into a specific binary directory.
BIN_DIR="$HOME/.local/bin" \
  curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-util/main/scripts/install.sh | sh
```

Show installer help:

```bash
sh ./scripts/install.sh --help
```

- **Releases**: <https://github.com/kenduest-brobridge/grafana-util/releases>
- **Binaries**: Standard builds for `linux-amd64` and `macos-arm64`. Browser-enabled versions (for screenshots) are labeled `*-browser-*`.
- **Default path**: Uses `/usr/local/bin` if writable, otherwise falls back to `$HOME/.local/bin`.
- **PATH setup**: If the chosen directory is not on `PATH`, the script provides the exact snippet for `zsh` or `bash`.

---

## Practical Examples

The following examples demonstrate core operational workflows. You can connect with direct flags such as `--basic-password`, prompt-based input such as `--prompt-password`, token auth, `export`-based environment variables in `bash` or `zsh`, or centralized `profile` configurations. For a full connection setup guide, refer to [Getting Started](./docs/user-guide/en/getting-started.md).

```bash
# bash / zsh
export GRAFANA_USERNAME=admin
export GRAFANA_PASSWORD=admin
```

If you want to keep those settings in a profile instead, `profile add` can store them separately:

```bash
grafana-util profile add prod \
  --url http://my-grafana:3000 \
  --basic-user admin \
  --prompt-password \
  --store-secret os

grafana-util profile add ci \
  --url http://my-grafana:3000 \
  --token-env GRAFANA_CI_TOKEN \
  --store-secret encrypted-file
```

From example 2 onward, the connection details are omitted for brevity. You can still pass them directly with `--url`, `--basic-user`, `--basic-password`, or `--token`, or keep them in `export`ed environment variables or a shared `profile`.

### 1. Get a live operational overview
```bash
grafana-util overview live \
  --url http://my-grafana:3000 \
  --basic-user admin \
  --basic-password admin \
  --output-format interactive
```

### 2. List dashboards first
```bash
# See what is already there before exporting or changing anything.
grafana-util dashboard list --all-orgs --table
```

### 3. Export dashboards for review
```bash
# Export all dashboards into a reviewable local tree.
grafana-util dashboard export --all-orgs --output-dir ./backup --progress
```

### 4. Analyze dashboard dependencies
```bash
# Audit datasource references and structure before import.
grafana-util dashboard analyze \
  --input-dir ./backup/raw \
  --input-format raw \
  --output-format tree-table
```

### 5. Open the dashboard TUI
```bash
# Open the interactive dashboard analysis workbench.
grafana-util dashboard analyze \
  --input-dir ./backup/raw \
  --input-format raw \
  --interactive
```

### 6. Dry-run dashboard import
```bash
grafana-util dashboard import \
  --input-dir ./backup/raw \
  --replace-existing \
  --dry-run \
  --table
```

### 7. Rapid dashboard iteration
```bash
# Review a locally generated dashboard JSON without touching Grafana.
cat cpu.json | grafana-util dashboard review --input - --output-format json
```

### 8. Review alerts before you change them
```bash
# See what the alert changes would do before applying them.
grafana-util alert plan --desired-dir ./alerts/desired --prune

# Preview where an alert would go.
grafana-util alert preview-route \
  --desired-dir ./alerts/desired \
  --label team=sre --severity critical
```

### 9. Datasource export and restore
```bash
# Export with secrets masked, then restore the connection details when you import.
grafana-util datasource export --output-dir ./datasources
grafana-util datasource import --input-dir ./datasources --prompt-password
```

---

## Docs & Guides

Use the handbook for workflow context and the command reference for exact CLI syntax.

If you prefer a browser view, open the local HTML docs at [docs/html/index.html](./docs/html/index.html) or visit the published site: <https://kenduest-brobridge.github.io/grafana-utils/>.

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

## Active Development
This repository is actively maintained. The CLI surface and documentation continue to evolve; please refer to the command reference for the latest syntax.

## Contributing
We welcome contributions! Please see our [Developer Guide](./docs/DEVELOPER.md) for setup instructions.
