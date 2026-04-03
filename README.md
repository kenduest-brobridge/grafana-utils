# grafana-util
### A Rust CLI for Grafana Operations and Administration

[![CI](https://img.shields.io/github/actions/workflow/status/kenduest-brobridge/grafana-utils/ci.yml?branch=main)](https://github.com/kenduest-brobridge/grafana-utils/actions)
[![License](https://img.shields.io/github/license/kenduest-brobridge/grafana-utils)](LICENSE)
[![Version](https://img.shields.io/github/v/tag/kenduest-brobridge/grafana-utils)](https://github.com/kenduest-brobridge/grafana-utils/tags)

English | [繁體中文](./README.zh-TW.md)

**Repeatable Grafana workflows for dashboards, alerts, datasources, access control, and operational review.**

`grafana-util` is a Rust CLI for teams that operate Grafana in a disciplined way across dashboards, alerts, datasources, access control, and environment-wide status surfaces. It is intended for SREs, platform engineers, sysadmins, and maintainers who need reviewable workflows, safer change paths, and automation-friendly output instead of ad hoc API calls or one-off scripts.

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

## Quick Start

```bash
# 1. Install via One-Liner
curl -sSL https://raw.githubusercontent.com/kendlee/grafana-utils/main/scripts/install.sh | bash

# 2. Confirm the installed version
grafana-util --version

# 3. Inspect current Grafana status
grafana-util overview live --url http://my-grafana:3000 --basic-user admin --prompt-password --output interactive
```

---

## Key Workflows

### Dashboard: Export, Review, and Migration
```bash
# 1. Export dashboards across all organizations
grafana-util dashboard export --all-orgs --export-dir ./backup --progress

# 2. Convert ordinary/raw dashboard JSON into Grafana UI prompt JSON
grafana-util dashboard raw-to-prompt --input-dir ./backup/raw --output-dir ./backup/prompt --overwrite --progress

# 3. Preview dashboard import behavior before committing
grafana-util dashboard import --import-dir ./backup/raw --replace-existing --dry-run --table

# 4. Audit datasource dependencies in an export tree
grafana-util dashboard inspect-export --import-dir ./backup/raw --output-format report-table

# 5. Browse and search live dashboards in the terminal
grafana-util dashboard browse
```

### Alerting: Review Before Apply
```bash
# 1. Build a change plan from local desired state vs live server
grafana-util alert plan --desired-dir ./alerts/desired --prune --output json

# 2. Preview alert routing before apply
grafana-util alert preview-route --desired-dir ./alerts/desired --label team=sre --severity critical
```

### Datasources: Export and Secret Recovery
```bash
# Export datasources with secrets masked for review or version control
grafana-util datasource export --export-dir ./datasources --overwrite

# Import with secret re-injection when credentials are required again
grafana-util datasource import --import-dir ./datasources --replace-existing --prompt-password
```

### Project Health: Unified Runtime Review
```bash
# Interactive TUI for environment-wide Grafana review
grafana-util overview live --output interactive
```

---

## Core Capabilities

*   **Dashboards**: Export, import, inspect, patch, review, and raw-to-prompt conversion workflows.
*   **Alerting**: Desired-state management, route preview, plan/apply review, and controlled pruning.
*   **Datasources**: Export/import, masked recovery, provisioning projections, and inspection support.
*   **Access**: Audit and replay organizations, users, teams, and service accounts.
*   **Status & Readiness**: Structured output for CI/CD gates plus interactive and table-based operator views.

---

## Operator Handbook

Use the handbook and command reference together: the handbook explains workflow and operational intent, while the command pages stay close to the current CLI surface.

If plain Markdown is awkward to read, generate the local HTML docs site and open the entrypoint:

```bash
make html
open ./docs/html/index.html
```

On Linux, replace `open` with `xdg-open`. The checked-in HTML files are meant for local browsing from the repo checkout; GitHub itself does not present them as a fully navigable static docs site.

For a published browser-friendly copy, use the GitHub Pages site for this repository:

*   **Published HTML Docs**: <https://kenduest-brobridge.github.io/grafana-utils/>
*   The site is generated from `docs/commands/*/*.md` and `docs/user-guide/*/*.md` and deployed from `main` by `.github/workflows/docs-pages.yml`.

*   **[Getting Started](./docs/user-guide/en/getting-started.md)**: Installation, profiles, and first commands.
*   **[Architecture & Principles](./docs/user-guide/en/architecture.md)**: Operational model, lanes, and design boundaries.
*   **[Real-World Recipes](./docs/user-guide/en/recipes.md)**: Common operational tasks and example flows.
*   **[Command Docs](./docs/commands/en/index.md)**: One page per command and subcommand, aligned to the current Rust CLI help, with command-map entrypoints for deeper surfaces such as `dashboard screenshot`, `access service-account token`, and `change bundle-preflight`.
*   **[HTML Docs Entry](./docs/html/index.html)**: Local handbook and command-reference entrypoint after `make html`.
*   **[Man Page](./docs/man/grafana-util.1)**: Top-level `man` format reference. View it locally with `man ./docs/man/grafana-util.1` on macOS or `man -l docs/man/grafana-util.1` on GNU/Linux.
*   **[Troubleshooting](./docs/user-guide/en/troubleshooting.md)**: Diagnostics, limits, and recovery guidance.

**[Full Handbook Table of Contents →](./docs/user-guide/en/index.md)**

---

## Documentation Map

If you are not sure which document to open first, use this map:

*   **Operator handbook**: [docs/user-guide/en/](./docs/user-guide/en/index.md) for workflow, concepts, and guided reading order.
*   **Command reference**: [docs/commands/en/](./docs/commands/en/index.md) for one page per command and subcommand.
*   **Browsable HTML docs**: [docs/html/index.html](./docs/html/index.html) locally after `make html`, or <https://kenduest-brobridge.github.io/grafana-utils/> remotely.
*   **Terminal manpage**: [docs/man/grafana-util.1](./docs/man/grafana-util.1) for `man`-style lookup.
*   **Maintainer entrypoint**: [docs/DEVELOPER.md](./docs/DEVELOPER.md) for code architecture, docs routing, build/validation flow, and maintainer pointers.
*   **Maintainer quickstart**: [docs/internal/maintainer-quickstart.md](./docs/internal/maintainer-quickstart.md) for the shortest first-day reading order, source-of-truth map, generated-file boundaries, and safe validation commands.
*   **Generated docs design**: [docs/internal/generated-docs-architecture.md](./docs/internal/generated-docs-architecture.md) for the Markdown-to-HTML/manpage system design.
*   **Generated docs playbook**: [docs/internal/generated-docs-playbook.md](./docs/internal/generated-docs-playbook.md) for step-by-step maintainer tasks.
*   **Secret storage architecture**: [docs/internal/profile-secret-storage-architecture.md](./docs/internal/profile-secret-storage-architecture.md) for profile secret modes, macOS/Linux support, limits, and maintainer rules.
*   **Internal docs index**: [docs/internal/README.md](./docs/internal/README.md) for the current internal spec, architecture, and trace inventory.

---

## Choose Your Path

Read by role instead of by file tree if that is easier:

*   **New user**: start with the dedicated [New User path](./docs/user-guide/en/role-new-user.md), then [Getting Started](./docs/user-guide/en/getting-started.md), then [Technical Reference](./docs/user-guide/en/reference.md).
*   **SRE / operator**: start with the dedicated [SRE / Ops path](./docs/user-guide/en/role-sre-ops.md), then [Change & Status](./docs/user-guide/en/change-overview-status.md), [Dashboard Management](./docs/user-guide/en/dashboard.md), [Datasource Management](./docs/user-guide/en/datasource.md), and [Troubleshooting](./docs/user-guide/en/troubleshooting.md).
*   **Automation / CI owner**: start with the dedicated [Automation / CI path](./docs/user-guide/en/role-automation-ci.md), then [Technical Reference](./docs/user-guide/en/reference.md), [Command Docs](./docs/commands/en/index.md), and the top-level [manpage](./docs/man/grafana-util.1).
*   **Platform architect / maintainer**: start with [Maintainer quickstart](./docs/internal/maintainer-quickstart.md), then [docs/DEVELOPER.md](./docs/DEVELOPER.md), [Maintainer Role Map](./docs/internal/maintainer-role-map.md), [generated docs architecture](./docs/internal/generated-docs-architecture.md), [generated docs playbook](./docs/internal/generated-docs-playbook.md), [secret storage architecture](./docs/internal/profile-secret-storage-architecture.md), and [docs/internal/README.md](./docs/internal/README.md).

---

## Technical Foundation
*   **Rust Engine**: Single-binary CLI with a Rust-first implementation.
*   **Validated**: Exercised against **Grafana 12.4.1** in Docker-based environments.
*   **Automation-Friendly**: Predictable exit codes and structured output for CI/CD and batch workflows.

---

## Contributing
We welcome contributions! Please see our [Developer Guide](./docs/DEVELOPER.md) for setup instructions.

---
*Maintained by [kendlee](https://github.com/kendlee)*
