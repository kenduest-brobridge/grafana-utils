# SRE / Operator Handbook

This page is for on-call operators and SREs who need a repeatable way to check readiness, inventory the estate, and move through dashboard, alert, and access workflows safely.

## Who It Is For

- On-call SREs.
- Platform and Grafana operators.
- Anyone who needs cross-org visibility, export/import checks, or break-glass access.

## Primary Goals

- Confirm live readiness before you touch anything.
- Keep a reliable profile for routine checks and repeatable maintenance.
- Choose an auth path that can actually see the scope you need.

## Typical Operator Tasks

- Run a live readiness check before a maintenance window.
- Inspect dashboards or datasources across visible orgs.
- Build staged summaries and preflight documents before an apply path.
- Export or review assets during backup, drift review, or break-glass recovery.

## Recommended Auth And Secret Approach

Use a profile backed by admin-capable credentials for day-to-day work.

1. `--profile` with `password_env`, `token_env`, or an OS-backed secret store for repeatable operator use.
2. Direct Basic auth with `--prompt-password` for bootstrap or break-glass work.
3. Token auth only for narrow reads where you already know the token can see every target org and resource.

## First Commands To Run

```bash
grafana-util profile list
grafana-util profile show --profile prod --output-format yaml
grafana-util status live --profile prod --output yaml
grafana-util overview live --profile prod --output interactive
grafana-util change preflight --desired-file ./desired.json --fetch-live --profile prod --output json
grafana-util dashboard list --profile prod --all-orgs --with-sources --table
```

If you are checking a host directly, Basic auth is the safest fallback for broad visibility:

```bash
grafana-util status live --url http://localhost:3000 --basic-user admin --prompt-password --all-orgs --output table
```

Use token auth only when the scope matches the work:

```bash
grafana-util overview live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output json
```

## What Good Looks Like

You are in a good operator posture when:

- you can tell whether the current credential can really see the org or admin scope you need
- you can separate live reads from staged review from actual apply paths
- you run preflight or dry-run checks before destructive actions
- you know which command page to open when the surface shifts from status into dashboard, alert, or access work

## Read Next

- [Dashboard Management](dashboard.md)
- [Alerting Governance](alert.md)
- [Access Management](access.md)
- [Change & Status](change-overview-status.md)

## Keep Open

- [profile](../../commands/en/profile.md)
- [status](../../commands/en/status.md)
- [overview](../../commands/en/overview.md)
- [dashboard](../../commands/en/dashboard.md)
- [alert](../../commands/en/alert.md)
- [access](../../commands/en/access.md)
- [change](../../commands/en/change.md)

## Common Mistakes And Limits

- Do not assume a token can see `--all-orgs`; that is one of the easiest ways to get partial inventory and miss a problem.
- Do not paste `--basic-password` into shared shell history unless you are deliberately in a throwaway session.
- Do not use `--show-secrets` outside a local, controlled inspection step.
- Do not treat a successful read-only check as proof that write or admin workflows will also work.
- Do not skip `change preflight`, `change plan`, or command-specific `--dry-run` paths before high-impact changes.

## When To Switch To Deeper Docs

- Switch to [Dashboard Management](dashboard.md) when the issue is inventory, export/import, inspection, or screenshot workflow.
- Switch to [Alerting Governance](alert.md) when the problem is rule ownership, contact points, routes, or plan/apply flow.
- Switch to [Access Management](access.md) when org, user, team, or service-account scope becomes part of the incident or maintenance task.
- Switch to the [Command Docs](../../commands/en/index.md) when you already know the workflow and just need the exact flags.

## Next Steps

- [Operator Scenarios](scenarios.md)
- [Troubleshooting](troubleshooting.md)
- [Command Docs](../../commands/en/index.md)
