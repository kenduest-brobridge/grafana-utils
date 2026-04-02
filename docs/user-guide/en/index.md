# Grafana Utilities User Guide

This handbook is organized as a map first and a reference second. It explains where each page fits, which chapter to open for a given task, and how the new directory layout separates general guidance from resource-specific workflows.

## Start Here

- [Getting started](./getting-started.md) introduces the CLI, the profile model, and the first safe commands to run.
- [Reference](./reference.md) describes the command families, global options, authentication rules, and shared surfaces.
- [Scenarios](./scenarios.md) turns the command families into operator workflows you can follow end to end.

## Domain Chapters

Use these when the work is focused on one resource family or one cross-domain surface:

- [Dashboard handbook](./dashboard.md) covers dashboard inventory, export/import, inspection, drift review, and provisioning boundaries.
- [Datasource handbook](./datasource.md) covers inventory, masked export, replay/import, and live datasource mutation.
- [Alert handbook](./alert.md) covers desired-state authoring, plan/apply, prune review, and migration-style replay.
- [Access handbook](./access.md) covers org, user, team, service-account, and token workflows.
- [Change, overview, and status handbook](./change-overview-status.md) covers staged change, project summaries, and readiness reporting.

## Reading Order

The pages follow the same path an operator usually takes:

1. Confirm the binary and choose a connection profile.
2. Learn the command surface and shared rules.
3. Pick the chapter that matches the task you need to perform.
4. Return to the reference page when you need exact flags or output behavior.

## Command Shape

Across the handbook, the unified command pattern is:

```text
grafana-util <domain> <command> [options]
```

The major domains in this guide are `dashboard`, `alert`, `datasource`, `access`, `change`, `overview`, `status`, and `profile`.

## Practical Entry Points

- If you are on a fresh machine or new checkout, begin with [Getting started](./getting-started.md).
- If you already know the task and need exact behavior, open [Reference](./reference.md).
- If you want a walk-through for a real operator job, open [Scenarios](./scenarios.md).
