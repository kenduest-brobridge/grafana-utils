# 📖 Operator Handbook: grafana-util

Welcome to the official operator handbook for `grafana-util`. This guide is designed to take you from a fresh installation to mastering estate-level Grafana governance.

---

## ⚡ 30-Second Quick Start

Get from zero to a full project health report in three commands.

### 1. Install (Global Binary)
```bash
# Downloads and installs the latest version to your local bin directory
curl -sSL https://raw.githubusercontent.com/kendlee/grafana-utils/main/scripts/install.sh | bash
```

### 2. Confirm the Installed Version
```bash
grafana-util --version
```

### 3. Run Your First Global Audit
```bash
# Generates a high-level health & inventory report of your entire Grafana estate
grafana-util overview live --url http://localhost:3000 --basic-user admin --prompt-password --output interactive
```

**Why this matters:** In 30 seconds, you have verified connectivity, inventoried your dashboards/alerts, and identified any broken datasource configurations across all organizations.

---

## 🧭 Navigation Map

### 🚀 Phase 1: Foundation
*   **[Getting Started](getting-started.md)**: Advanced installation, Profiles, and Auth rules.
*   **[New User Path](role-new-user.md)**: The shortest safe path from install to first successful live read.
*   **[SRE / Ops Path](role-sre-ops.md)**: The operator path for day-to-day governance, review-first change flows, and troubleshooting.
*   **[Automation / CI Path](role-automation-ci.md)**: The profile, output, and command-reference path for scripting and automation.
*   **[Architecture & Design Principles](architecture.md)**: The "Why" behind our design decisions.

### 🛠️ Phase 2: Core Asset Management
*   **[Dashboard Management](dashboard.md)**: Export, Import, and Live Inspection.
*   **[Datasource Management](datasource.md)**: Masked Recovery and Live Mutations.
*   **[Alerting Governance](alert.md)**: The Plan/Apply lifecycle for Grafana Alerts.

### 🔐 Phase 3: Identity & Access
*   **[Access Management](access.md)**: Organizations, Users, Teams, and Service Accounts.

### 🛡️ Phase 4: Governance & Readiness
*   **[Change & Status](change-overview-status.md)**: Staged workflows, project snapshots, and health gates.

### 📖 Phase 5: Deep Dive
*   **[Practical Scenarios](scenarios.md)**: End-to-end task recipes (Backups, DR, Audits).
*   **[Best Practices & Recipes](recipes.md)**: Surgical solutions for common Grafana headaches.
*   **[Technical Reference](reference.md)**: Full command map and global flag dictionary.
*   **[Command Docs](../../commands/en/index.md)**: One page per command and subcommand, aligned to the current Rust CLI help.
*   **[Troubleshooting & Glossary](troubleshooting.md)**: Diagnostic guides and terminology index.

---

## 👥 Choose Your Role

Different readers usually need different paths through the handbook:

*   **New user**
  Start with [New User Path](role-new-user.md), then [Getting Started](getting-started.md), then open [Command Docs](../../commands/en/index.md) when you need exact flags.
*   **SRE / operator**
  Start with [SRE / Ops Path](role-sre-ops.md), then [Change & Status](change-overview-status.md), [Dashboard Management](dashboard.md), [Datasource Management](datasource.md), and [Troubleshooting](troubleshooting.md).
*   **Identity / access administrator**
  Start with [Access Management](access.md), then [Technical Reference](reference.md), then the [Command Docs](../../commands/en/index.md).
*   **Automation / CI owner**
  Start with [Automation / CI Path](role-automation-ci.md), then [Technical Reference](reference.md), then the [Command Docs](../../commands/en/index.md), then validate exact terminal lookup with the top-level manpage at `docs/man/grafana-util.1`.
*   **Maintainer / architect**
  Start with [docs/DEVELOPER.md](/Users/kendlee/work/grafana-utils/docs/DEVELOPER.md), then [maintainer-role-map.md](/Users/kendlee/work/grafana-utils/docs/internal/maintainer-role-map.md), then the internal design and playbook docs under [docs/internal/README.md](/Users/kendlee/work/grafana-utils/docs/internal/README.md).

---

## 🎯 How to use this guide
If you are new, start with **Getting Started** and follow the **"Next Page"** links at the bottom of each chapter for a guided learning path.

---
**Next Step**: [🚀 Getting Started](getting-started.md)
