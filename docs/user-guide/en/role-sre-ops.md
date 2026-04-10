# SRE / Operator Handbook

This page is for on-call operators and SREs who need a repeatable way to check readiness, inventory the estate, and move through dashboard, alert, and access workflows safely.

## Who It Is For

- On-call SREs.
- Platform and Grafana operators.
- Anyone who needs cross-org visibility, export/import checks, or break-glass access.

## Primary Goals

- Confirm live readiness before you touch anything.
 - Keep a reliable `config profile` for routine checks and repeatable maintenance.
- Choose an auth path that can actually see the scope you need.

## Before / After

- Before: SREs had to infer readiness, scope, and replay risk from a chain of ad hoc commands.
- After: use a repeatable profile, then move through live checks, staged review, and apply only after preflight.

## What success looks like

- You can tell whether the credential really sees the scope you need.
- You can separate live reads from staged review and apply paths.
- You have a reliable operator path for dashboard, alert, and access workflows.

## Failure checks

- If the token scope is narrower than the task, stop and fetch a credential that can see the real estate you need.
- If the live check passes but the apply path fails, verify write permissions and staged inputs before blaming the renderer.
- If you cannot explain which lane the task belongs to, pause and open the workflow chapter first.

## Typical Operator Tasks

- Run a live readiness check before a maintenance window.
- Inspect dashboards or datasources across visible orgs.
- Inspect, check, and preview staged changes before an apply path.
- Export or review assets during backup, drift review, or break-glass recovery.

## Recommended connection and secret handling

Use a profile backed by admin-capable credentials for day-to-day work.

1. `config profile` with `password_env`, `token_env`, or an OS-backed secret store for repeatable operator use.
2. Direct Basic auth with `--prompt-password` for bootstrap or break-glass work.
3. Token auth only for narrow reads where you already know the token can see every target org and resource.

## First commands to run

```bash
# Purpose: First commands to run.
grafana-util observe live --profile prod --output-format table
```

```bash
# Purpose: First commands to run.
grafana-util observe overview live --profile prod --output-format interactive
```

```bash
# Purpose: First commands to run.
grafana-util change inspect --workspace .
```

```bash
# Purpose: First commands to run.
grafana-util change check --workspace . --fetch-live --output-format json
```

```bash
# Purpose: First commands to run.
grafana-util change preview --workspace . --fetch-live --output-format json
```

```bash
# Purpose: First commands to run.
grafana-util export dashboard --output-dir ./backups --overwrite --progress
```

If you need to start from the access layer instead, swap the last line for:

```bash
# Purpose: If you need to start from the access layer instead, swap the last line for.
grafana-util advanced access org list --table
```

If you are checking a host directly, Basic auth is the safest fallback for broad visibility:

```bash
# Purpose: If you are checking a host directly, Basic auth is the safest fallback for broad visibility.
grafana-util observe live --url http://localhost:3000 --basic-user admin --prompt-password --all-orgs --output-format table
```

Use token auth only when the scope matches the work:

```bash
# Purpose: Use token auth only when the scope matches the work.
grafana-util observe overview live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format json
```

## What good operator posture looks like

You are in a good operator posture when:

- you can tell whether the current credential can really see the org or admin scope you need
- you can separate live reads from staged review from actual apply paths
- you run preflight or dry-run checks before destructive actions
- you know which command page to open when the surface shifts from status into dashboard, alert, or access work

## Read next

- [Change & Status](change-overview-status.md)
- [Dashboard Management](dashboard.md)
- [Data source Management](datasource.md)
- [Alerting Governance](alert.md)
- [Access Management](access.md)
- [Troubleshooting](troubleshooting.md)

## Keep open

- [config profile](../../commands/en/profile.md)
- [observe live](../../commands/en/observe.md)
- [observe overview](../../commands/en/observe.md)
- [export dashboard](../../commands/en/export.md)
- [advanced](../../commands/en/advanced.md)
- [change](../../commands/en/change.md)
- [full command index](../../commands/en/index.md)

## Common mistakes and limits

- Do not assume a token can see `--all-orgs`; that is one of the easiest ways to get partial inventory and miss a problem.
- Do not paste `--basic-password` into shared shell history unless you are deliberately in a throwaway session.
- Do not use `--show-secrets` outside a local, controlled inspection step.
- Do not treat a successful read-only check as proof that write or admin workflows will also work.
- Do not skip `change check`, `change preview`, or command-specific `--dry-run` paths before high-impact changes.

## When to switch to deeper docs

- Switch to [Dashboard Management](dashboard.md) when the issue is inventory, export/import, inspection, or screenshot workflow.
- Switch to [Alerting Governance](alert.md) when the problem is rule ownership, contact points, routes, or plan/apply flow.
- Switch to [Access Management](access.md) when org, user, team, or service-account scope becomes part of the incident or maintenance task.
- Switch to the [Command Docs](../../commands/en/index.md) when you already know the workflow and just need the exact flags.

## Next steps

- [Home](index.md)
- [Change & Status](change-overview-status.md)
- [Dashboard Management](dashboard.md)
- [Command Docs](../../commands/en/index.md)
