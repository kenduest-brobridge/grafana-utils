# Operator Handbook

## Language

- English handbook: [current page](./index.md)
- Traditional Chinese handbook: [繁體中文手冊](../zh-TW/index.md)
- English command reference: [Command Docs](../../commands/en/index.md)
- Traditional Chinese command reference: [繁體中文逐指令說明](../../commands/zh-TW/index.md)

---

Welcome to the `grafana-util` handbook. Start here if you need a practical operator path from first connection, to repeatable profiles, to day-to-day Grafana maintenance and automation.

`grafana-util` is a personal long-term tool built around real Grafana maintenance pain. This handbook is written from that same angle: not "how to expose every API surface", but how to move through inventory, inspection, review, migration, and safer live changes without rebuilding the workflow every time.

This handbook is organized around the way operators actually work: first understand what the tool is for, then get a safe connection working, then move into dashboards, alerts, access, and review workflows.

If you want the high-level framing first, including the pain points this tool is meant to solve and when it is the right fit, start here:

- [What grafana-util is for](what-is-grafana-util.md)

## Before / After

- Before: you had to guess which chapter to open when you wanted to understand the workflow.
- After: start with the purpose page, then choose the handbook path that matches your role.

## What success looks like

- You know which chapter to open for your current task.
- You can separate the handbook from the exact command reference.
- You can move from first read-only checks into the right workflow lane without guessing.

## Failure checks

- If you still do not know whether you need dashboard, alert, access, or change guidance, stop at the purpose page and read the role map first.
- If the first live read fails, fix connection or auth before opening mutation chapters.
- If you only need exact flags, switch to the command docs instead of trying to infer them from the handbook.

---

## 30-Second Quick Start

Get from zero to a first install check, connectivity check, and estate-level overview in three commands.

### 1. Install (Global Binary)
```bash
# Downloads and installs the latest version to your local bin directory
curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-util/main/scripts/install.sh | sh
```

### 2. Confirm the Installed Version
```bash
# Purpose: 2. Confirm the Installed Version.
grafana-util --version
```

### 3. Run Your First Global Audit
```bash
# Generates a high-level health and inventory report of your Grafana estate
grafana-util overview live --url http://localhost:3000 --basic-user admin --prompt-password --output-format interactive
```

**Why this matters:** In 30 seconds, you have confirmed connectivity, checked dashboards and alerts, and found the first obvious data source problems before you make changes.

---

## Navigation Map

### Phase 1: Foundation
*   **[What grafana-util is for](what-is-grafana-util.md)**: Start with the problems it solves and the operator workflows it is meant to support.
*   **[Getting Started](getting-started.md)**: Installation, connection setup, profiles, and auth options.
*   **[New User Path](role-new-user.md)**: The shortest safe path from install to first successful live read.
*   **[SRE / Ops Path](role-sre-ops.md)**: The operator path for day-to-day governance, review-first change flows, and troubleshooting.
*   **[Automation / CI Path](role-automation-ci.md)**: The profile, output, and command-reference path for scripting and automation.
*   **[Architecture & Design Principles](architecture.md)**: The reasoning behind the workflow and command design.

### Phase 2: Core Asset Management
*   **[Dashboard Management](dashboard.md)**: Export, import, inspection, screenshots, and governance checks for dashboard assets.
*   **[Data source Management](datasource.md)**: Export, import, inspection, and live mutation guidance for Grafana data sources.
*   **[Alerting Governance](alert.md)**: Review, planning, and apply flow for Grafana alerts.

### Phase 3: Identity & Access
*   **[Access Management](access.md)**: org, user, team, and service account operations.

### Phase 4: Governance & Readiness
*   **[Change & Status](change-overview-status.md)**: staged workflows, project snapshots, and preflight checks.

### Phase 5: Deep Dive
*   **[Practical Scenarios](scenarios.md)**: end-to-end task recipes such as backups, DR, and audits.
*   **[Best Practices & Recipes](recipes.md)**: recommended ways to handle common Grafana operator problems.
*   **[Technical Reference](reference.md)**: command map, profile behavior, auth handling, common flags, and output guidance.
*   **[Command Docs](../../commands/en/index.md)**: One page per command and subcommand, aligned to the current Rust CLI help.
*   **[Troubleshooting & Glossary](troubleshooting.md)**: Diagnostic guides and terminology index.

---

## Choose Your Role

Different readers usually need different paths through the handbook:

*   **New user**
  Start with [What grafana-util is for](what-is-grafana-util.md), then [New User Path](role-new-user.md), then [Getting Started](getting-started.md), then open [Command Docs](../../commands/en/index.md) when you need exact flags.
*   **SRE / operator**
  Start with [SRE / Ops Path](role-sre-ops.md), then [Change & Status](change-overview-status.md), [Dashboard Management](dashboard.md), [Data source Management](datasource.md), and [Troubleshooting](troubleshooting.md).
*   **Identity / access administrator**
  Start with [Access Management](access.md), then [Technical Reference](reference.md), then the [Command Docs](../../commands/en/index.md).
*   **Automation / CI owner**
  Start with [Automation / CI Path](role-automation-ci.md), then [Technical Reference](reference.md), then the [Command Docs](../../commands/en/index.md), then validate exact terminal lookup with the top-level manpage at `docs/man/grafana-util.1`.
*   **Maintainer / architect**
  Start with [docs/DEVELOPER.md](/Users/kendlee/work/grafana-utils/docs/DEVELOPER.md), then [maintainer-role-map.md](/Users/kendlee/work/grafana-utils/docs/internal/maintainer-role-map.md), then the internal design and playbook docs under [docs/internal/README.md](/Users/kendlee/work/grafana-utils/docs/internal/README.md).

---

## How to use this guide
If you are new, start with **What grafana-util is for**, then **Getting Started**, then follow the **Next Page** links at the bottom of each chapter.

---
**Next Step**: [What grafana-util is for](what-is-grafana-util.md)
