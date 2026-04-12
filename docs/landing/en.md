# Start Here

Start with the handbook for the reading path, or open the command reference for exact syntax.

- [Start with the handbook](../user-guide/en/index.md)
- [Open command reference](../commands/en/index.md)

## First Run

If this is a new machine or a new Grafana environment, follow this order:

### Confirm the binary

Start by checking `grafana-util --version` so you know the CLI is installed and on your path.

- [Getting Started](../user-guide/en/getting-started.md)
- [Command Reference](../commands/en/version.md)

### Run the first read-only check

Use `grafana-util status live` as the first live read against Grafana before attempting any broader workflow.

- [Getting Started](../user-guide/en/getting-started.md)
- [Status Command Reference](../commands/en/status.md)

### Save a reusable connection profile

After the first successful read, store host and credentials with `grafana-util config profile add ...`.

- [New User Path](../user-guide/en/role-new-user.md)
- [Profile Command Reference](../commands/en/profile.md)

## Read By Role

Choose the reading path that best matches who is operating Grafana today.

### New user

Use the shortest safe path when this is your first time with the CLI or your first Grafana connection.

- [New User Path](../user-guide/en/role-new-user.md)
- [Getting Started](../user-guide/en/getting-started.md)

### SRE / operator

Start from day-two operations, workspace review, and pre-change inspection rather than the syntax index.

- [SRE / Ops Path](../user-guide/en/role-sre-ops.md)
- [Workspace Review & Status](../user-guide/en/status-workspace.md)

### Automation / CI

Go here when the CLI will run inside pipelines, release automation, or repeatable validation flows.

- [Automation / CI Path](../user-guide/en/role-automation-ci.md)
- [Technical Reference](../user-guide/en/reference.md)

### Maintainer / architect

This route is for repository structure, design rules, and implementation-facing contracts.

- [Architecture & Design Principles](../user-guide/en/architecture.md)
- [Developer guide](../DEVELOPER.md)

## Read By Task

Start from the thing you are trying to get done, then move into the matching chapter set.

### Understand what the tool is for

Read this before choosing a workflow if you still need the mental model for what the CLI is trying to protect and automate.

- [What grafana-util is for](../user-guide/en/what-is-grafana-util.md)

### Check live or staged state

Use this path for read-only inspection, staged review, or pre-change status checks.

- [Workspace Review & Status](../user-guide/en/status-workspace.md)
- [Status Command Reference](../commands/en/status.md)

### Work on dashboards

This path covers browse, export, analysis, review, patch, publish, and capture flows around dashboards.

- [Dashboard Management](../user-guide/en/dashboard.md)
- [Dashboard Command Reference](../commands/en/dashboard.md)

### Work on data sources or alerts

Use this when you are changing Grafana integrations, alert rules, contact points, or governance checks.

- [Data source Management](../user-guide/en/datasource.md)
- [Alerting Governance](../user-guide/en/alert.md)

### Manage access and credentials

Start here for orgs, teams, service accounts, tokens, and access-oriented operational changes.

- [Access Management](../user-guide/en/access.md)
- [Access Command Reference](../commands/en/access.md)

## Browse By Command Family

Use this when you already know the root command family and want the shortest route into syntax and workflow context.

### `status` and `workspace`

These roots cover inspection, staging review, and workspace-oriented validation paths.

- [Status / Workspace chapters](../user-guide/en/status-workspace.md)
- [Workspace Command Reference](../commands/en/workspace.md)

### `config profile`

Use this to manage reusable connection defaults and secret storage for later commands.

- [Getting Started](../user-guide/en/getting-started.md)
- [Profile Command Reference](../commands/en/profile.md)

### `dashboard`

This root owns browse, summary, variables, export, diff, patch, publish, and screenshot workflows.

- [Dashboard chapters](../user-guide/en/dashboard.md)
- [Dashboard Command Reference](../commands/en/dashboard.md)

### `datasource`, `alert`, and `access`

These roots cover change workflows for integrations, alerting, and Grafana identity surfaces.

- [Datasource Command Reference](../commands/en/datasource.md)
- [Alert Command Reference](../commands/en/alert.md)
- [Access Command Reference](../commands/en/access.md)

## Complete Reference

When you need full coverage rather than a curated starting path, use the complete source surfaces here.

### Read the full handbook

Use the handbook when you want chapters, operating context, and recommended reading order.

- [Operator Handbook](../user-guide/en/index.md)

### Read the full command reference

Use the command reference when you need per-command pages, subcommand routing, and stable syntax lookup.

- [Command Reference](../commands/en/index.md)
- [grafana-util(1)](../html/man/grafana-util.html)

### Check source and releases

Use the repository when you need change history, release notes, or issue tracking rather than operator docs.

- [GitHub repository](https://github.com/kenduest-brobridge/grafana-util)
- [GitHub releases](https://github.com/kenduest-brobridge/grafana-util/releases)
- [Issue tracker](https://github.com/kenduest-brobridge/grafana-util/issues)

## Maintainer

Maintainer guidance stays in the repository docs rather than the public handbook.

- [Developer guide](../DEVELOPER.md)
