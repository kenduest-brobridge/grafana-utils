# `grafana-util status`

## Root

Purpose: render shared project-wide staged or live status.

When to use: when you need the final gate view for exported artifacts or live Grafana state.

Key flags: the root command is a namespace; staged and live inputs live on the subcommands. Common flags include `--output` and the shared live connection/auth options.

Examples:

```bash
grafana-util status staged --dashboard-export-dir ./dashboards/raw --desired-file ./desired.json --output json
grafana-util status live --profile prod --output yaml
```

Related commands: `grafana-util overview`, `grafana-util change preflight`, `grafana-util change apply`.

## `staged`

Purpose: render project status from staged artifacts.

When to use: when you need the machine-readable readiness gate for exported files before apply.

Key flags: `--dashboard-export-dir`, `--dashboard-provisioning-dir`, `--datasource-export-dir`, `--datasource-provisioning-file`, `--access-user-export-dir`, `--access-team-export-dir`, `--access-org-export-dir`, `--access-service-account-export-dir`, `--desired-file`, `--source-bundle`, `--target-inventory`, `--alert-export-dir`, `--availability-file`, `--mapping-file`, `--output`.

Examples:

```bash
grafana-util status staged --dashboard-export-dir ./dashboards/raw --desired-file ./desired.json --output table
grafana-util status staged --dashboard-provisioning-dir ./dashboards/provisioning --alert-export-dir ./alerts --output interactive
```

Related commands: `grafana-util overview`, `grafana-util change summary`, `grafana-util change preflight`.

## `live`

Purpose: render project status from live Grafana read surfaces.

When to use: when you need current Grafana status, optionally deepened with staged context files.

Key flags: `--profile`, `--url`, `--token`, `--basic-user`, `--basic-password`, `--prompt-password`, `--prompt-token`, `--timeout`, `--verify-ssl`, `--insecure`, `--ca-cert`, `--all-orgs`, `--org-id`, `--sync-summary-file`, `--bundle-preflight-file`, `--promotion-summary-file`, `--mapping-file`, `--availability-file`, `--output`.

Notes:
- Prefer `--profile` for normal live status checks.
- `--all-orgs` is safest with admin-backed `--profile` or direct Basic auth because token scope can hide other orgs.

Examples:

```bash
grafana-util status live --profile prod --output yaml
grafana-util status live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output json
grafana-util status live --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --sync-summary-file ./sync-summary.json --output interactive
```

Related commands: `grafana-util overview live`, `grafana-util change apply`, `grafana-util profile show`.
