# grafana-util 🚀
### The Missing CLI for Professional Grafana Estate Management

[![CI](https://img.shields.io/github/actions/workflow/status/kendlee/grafana-utils/ci.yml?branch=main)](https://github.com/kendlee/grafana-utils/actions)
[![License](https://img.shields.io/github/license/kendlee/grafana-utils)](LICENSE)
[![Release](https://img.shields.io/github/v/release/kendlee/grafana-utils)](https://github.com/kendlee/grafana-utils/releases)

**Stop manually clicking. Start governing your Grafana at scale.**

`grafana-util` is a high-performance, Rust-powered CLI designed for SREs and Platform Engineers who manage complex Grafana environments across multiple organizations and instances. It bridges the gap between raw API calls and enterprise-grade governance.

---

## 🌟 Why `grafana-util`?

| Feature | Standard CLI / curl | **grafana-util** |
| :--- | :---: | :--- |
| **Multi-Org Discovery** | Manual per org | ✅ One command to scan all orgs |
| **Dependency Audit** | Impossible | ✅ Find broken datasources before importing |
| **Alerting Lifecycle** | "Blind" Apply | ✅ **Plan/Apply** cycle (Review before commit) |
| **Secret Safety** | Leaks secrets | ✅ **Masked Recovery** (Safe-for-Git exports) |
| **Visual Review** | Raw JSON | ✅ Interactive **TUI** and beautiful Tables |

---

## ⚡ 30-Second Quick Start

```bash
# 1. Install via One-Liner
curl -sSL https://raw.githubusercontent.com/kendlee/grafana-utils/main/scripts/install.sh | bash

# 2. Confirm the installed version
grafana-util --version

# 3. See your estate's health immediately
grafana-util overview live --url http://my-grafana:3000 --basic-user admin --prompt-password --output interactive
```

---

## 🚀 Key Workflows (The "Killer" Commands)

### 📊 Dashboard: Estate-Wide Management
```bash
# 1. Export ALL dashboards from ALL organizations with progress bars
grafana-util dashboard export --all-orgs --export-dir ./backup --progress

# 2. Convert ordinary/raw dashboard JSON into Grafana UI prompt JSON
grafana-util dashboard raw-to-prompt --input-dir ./backup/raw --output-dir ./backup/prompt --overwrite --progress

# 3. Dry-Run Import: Preview exactly what will happen before committing
grafana-util dashboard import --import-dir ./backup/raw --replace-existing --dry-run --table

# 4. Dependency Audit: Identify missing datasources in your export tree
grafana-util dashboard inspect-export --import-dir ./backup/raw --output-format report-table

# 5. Interactive Browser: Discover and search live dashboards in the terminal
grafana-util dashboard browse
```

### 🚨 Alerting: The Plan/Apply Lifecycle
```bash
# 1. Build a Change Plan: Compare local files vs live server
grafana-util alert plan --desired-dir ./alerts/desired --prune --output json

# 2. Safety First: Preview where an alert will land based on its labels
grafana-util alert preview-route --desired-dir ./alerts/desired --label team=sre --severity critical
```

### 🔐 Datasources: Masked Recovery
```bash
# Export datasources with secrets masked (Safe for Git!)
grafana-util datasource export --export-dir ./datasources --overwrite

# Import with automatic secret re-injection protocol
grafana-util datasource import --import-dir ./datasources --replace-existing --prompt-password
```

### 🛡️ Project Health: The Unified Surface
```bash
# Interactive TUI: A beautiful, live dashboard of your entire Grafana estate
grafana-util overview live --output interactive
```

---

## 🛠️ Core Capabilities

*   **Dashboards**: Full-fidelity export/import, variable inspection, and mass patching.
*   **Alerting**: Declarative management for Grafana Alerts. Preview routes and prune stale rules safely.
*   **Datasources**: Masked export/import. Safely recover datasources with secret re-injection.
*   **Access**: Audit and replay Organizations, Users, Teams, and Service Accounts.
*   **Status & Readiness**: Machine-readable contracts for CI/CD gates and human-friendly TUI reports.

---

## 📖 Operator Handbook

Don't just run commands—master the workflow. We have prepared a comprehensive **Operator Handbook** for you:

If plain Markdown is awkward to read, generate the local HTML docs site and open the entrypoint:

```bash
make html
open ./docs/html/index.html
```

On Linux, replace `open` with `xdg-open`. The checked-in HTML files are meant for local browsing from the repo checkout; GitHub itself does not present them as a fully navigable static docs site.

For a published browser-friendly copy, use the GitHub Pages site for this repository:

*   **Published HTML Docs**: <https://kendlee.github.io/grafana-utils/>
*   The site is generated from `docs/commands/*/*.md` and `docs/user-guide/*/*.md` and deployed from `main` by `.github/workflows/docs-pages.yml`.

*   **[Getting Started](./docs/user-guide/en/getting-started.md)**: Profiles and Setup.
*   **[Architecture & Principles](./docs/user-guide/en/architecture.md)**: The "Why" behind our lanes.
*   **[Real-World Recipes](./docs/user-guide/en/recipes.md)**: Solving common Grafana headaches.
*   **[Command Docs](./docs/commands/en/index.md)**: One page per command and subcommand, aligned to the current Rust CLI help.
*   **[HTML Docs Entry](./docs/html/index.html)**: Local handbook + command-reference entrypoint after `make html`.
*   **[Man Page](./docs/man/grafana-util.1)**: Top-level `man` format reference. View it locally with `man ./docs/man/grafana-util.1` on macOS or `man -l docs/man/grafana-util.1` on GNU/Linux.
*   **[Troubleshooting](./docs/user-guide/en/troubleshooting.md)**: Diagnostics and Glossary.

**[Full Handbook Table of Contents →](./docs/user-guide/en/index.md)**

---

## 🧭 Documentation Map

If you are not sure which document to open first, use this map:

*   **Operator handbook**: [docs/user-guide/en/](./docs/user-guide/en/index.md) for workflow, concepts, and guided reading order.
*   **Command reference**: [docs/commands/en/](./docs/commands/en/index.md) for one page per command and subcommand.
*   **Browsable HTML docs**: [docs/html/index.html](./docs/html/index.html) locally after `make html`, or <https://kendlee.github.io/grafana-utils/> remotely.
*   **Terminal manpage**: [docs/man/grafana-util.1](./docs/man/grafana-util.1) for `man`-style lookup.
*   **Maintainer entrypoint**: [docs/DEVELOPER.md](./docs/DEVELOPER.md) for code architecture, docs routing, build/validation flow, and maintainer pointers.
*   **Maintainer quickstart**: [docs/internal/maintainer-quickstart.md](./docs/internal/maintainer-quickstart.md) for the shortest first-day reading order, source-of-truth map, generated-file boundaries, and safe validation commands.
*   **Generated docs design**: [docs/internal/generated-docs-architecture.md](./docs/internal/generated-docs-architecture.md) for the Markdown-to-HTML/manpage system design.
*   **Generated docs playbook**: [docs/internal/generated-docs-playbook.md](./docs/internal/generated-docs-playbook.md) for step-by-step maintainer tasks.
*   **Secret storage architecture**: [docs/internal/profile-secret-storage-architecture.md](./docs/internal/profile-secret-storage-architecture.md) for profile secret modes, macOS/Linux support, limits, and maintainer rules.
*   **Internal docs index**: [docs/internal/README.md](./docs/internal/README.md) for the current internal spec, architecture, and trace inventory.

---

## 👥 Choose Your Path

Read by role instead of by file tree if that is easier:

*   **New user**: start with the dedicated [New User path](./docs/user-guide/en/role-new-user.md), then [Getting Started](./docs/user-guide/en/getting-started.md), then [Technical Reference](./docs/user-guide/en/reference.md).
*   **SRE / operator**: start with the dedicated [SRE / Ops path](./docs/user-guide/en/role-sre-ops.md), then [Change & Status](./docs/user-guide/en/change-overview-status.md), [Dashboard Management](./docs/user-guide/en/dashboard.md), [Datasource Management](./docs/user-guide/en/datasource.md), and [Troubleshooting](./docs/user-guide/en/troubleshooting.md).
*   **Automation / CI owner**: start with the dedicated [Automation / CI path](./docs/user-guide/en/role-automation-ci.md), then [Technical Reference](./docs/user-guide/en/reference.md), [Command Docs](./docs/commands/en/index.md), and the top-level [manpage](./docs/man/grafana-util.1).
*   **Platform architect / maintainer**: start with [Maintainer quickstart](./docs/internal/maintainer-quickstart.md), then [docs/DEVELOPER.md](./docs/DEVELOPER.md), [Maintainer Role Map](./docs/internal/maintainer-role-map.md), [generated docs architecture](./docs/internal/generated-docs-architecture.md), [generated docs playbook](./docs/internal/generated-docs-playbook.md), [secret storage architecture](./docs/internal/profile-secret-storage-architecture.md), and [docs/internal/README.md](./docs/internal/README.md).

---

## 🏗️ Technical Foundation
*   **Rust Engine**: Single static binary, no dependencies, blazing fast.
*   **Validated**: Tested against **Grafana 12.4.1** in Docker environments.
*   **CI/CD Ready**: Predictable exit codes and JSON-first output architecture.

---

## 🤝 Contributing
We welcome contributions! Please see our [Developer Guide](./docs/DEVELOPER.md) for setup instructions.

---
*Maintained by [kendlee](https://github.com/kendlee)*
