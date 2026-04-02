# Grafana Utilities Operator Handbook

Welcome to the official operator-facing guide for `grafana-util`. This handbook is designed for engineers managing Grafana at scale, providing structured workflows for inventory, migration, and governance.

---

## 📖 How to Use This Handbook

This guide is organized into thematic chapters to minimize context switching. Choose your starting point based on your current task:

| If you are... | Recommended Start | Why |
| :--- | :--- | :--- |
| **New to the tool** | [Getting Started](./getting-started.md) | Verify installation and establish your first safe connection. |
| **Performing a task** | [Scenarios](./scenarios.md) | Follow end-to-end workflows for common operator jobs. |
| **Looking for flags** | [Reference](./reference.md) | Detailed command syntax, authentication, and output contracts. |
| **Managing resources** | Domain Chapters | Deep dives into Dashboards, Datasources, or Alerts. |

---

## 🗺️ Handbook Map

### 1. Orientation & Core Concepts
- [**Getting Started**](./getting-started.md): Installation, profiles, and initial connectivity.
- [**Reference**](./reference.md): Global flags, auth rules, and output formats (JSON, Table, TUI).
- [**Scenarios**](./scenarios.md): Task-driven guides connecting multiple command families.

### 2. Domain Handbooks
- [**Dashboard Handbook**](./dashboard.md): Inventory, drift review, and multi-lane export/import.
- [**Datasource Handbook**](./datasource.md): Masked recovery, live mutation, and provisioning projections.
- [**Alert Handbook**](./alert.md): Plan/Apply workflows, state authoring, and migration bundles.
- [**Access Handbook**](./access.md): Org, user, team, and service-account management.

### 3. Advanced Operations
- [**Change, Overview & Status**](./change-overview-status.md): Cross-domain staged changes and project-wide readiness.

---

## ⚙️ Command Architecture

All commands follow a consistent, predictable pattern:

```bash
grafana-util <domain> <command> [options]
```

### Supported Output Modes
Different tasks require different data surfaces. `grafana-util` supports:
- 📝 **Plain Text**: Default for human-readable summaries and dry-run previews.
- 🔢 **JSON**: Optimized for CI/CD pipelines and stable machine-readable handoffs.
- 📊 **Table/CSV**: Ideal for audits, inventory listings, and side-by-side review.
- 🖥️ **Interactive TUI**: Available for guided browsing (e.g., `dashboard browse`).

---

## 🎯 Target Environments
Documentation examples are validated against **Grafana 12.4.1**. While the tool supports a wide range of versions, always verify command behavior in a staging environment before performing large-scale mutations.
