# `grafana-util status`

## Root

Purpose: render shared project-wide staged or live status.

When to use: when you need the final gate view for exported artifacts or live Grafana state.

Description: start here when you need a final readiness or health readout rather than a deep command-by-command walkthrough. The `status` namespace is the gate view that operators and CI jobs use to answer “is the staged bundle ready?” or “what does the live Grafana state look like right now?”.

Key flags: the root command is a namespace; staged and live inputs live on the subcommands. Common flags include `--output-format` and the shared live connection/auth options.

Examples:

```bash
# Purpose: Render staged status from dashboard and desired artifacts.
grafana-util status staged --dashboard-export-dir ./dashboards/raw --desired-file ./desired.json --output-format json
```

```bash
# Purpose: Render live status from a reusable profile.
grafana-util status live --profile prod --output-format yaml
```

Related commands: `grafana-util overview`, `grafana-util change check`, `grafana-util change apply`.

## `staged`

Purpose: render project status from staged artifacts.

When to use: when you need the machine-readable readiness gate for exported files before apply.

Key flags: `--dashboard-export-dir`, `--dashboard-provisioning-dir`, `--datasource-export-dir`, `--datasource-provisioning-file`, `--access-user-export-dir`, `--access-team-export-dir`, `--access-org-export-dir`, `--access-service-account-export-dir`, `--desired-file`, `--source-bundle`, `--target-inventory`, `--alert-export-dir`, `--availability-file`, `--mapping-file`, `--output-format`.

Examples:

```bash
# Purpose: staged.
grafana-util status staged --dashboard-export-dir ./dashboards/raw --desired-file ./desired.json --output-format table
```

```bash
# Purpose: staged.
grafana-util status staged --dashboard-provisioning-dir ./dashboards/provisioning --alert-export-dir ./alerts --output-format interactive
```

Related commands: `grafana-util overview`, `grafana-util change inspect`, `grafana-util change check`.

## `live`

Purpose: render project status from live Grafana read surfaces.

When to use: when you need current Grafana status, optionally deepened with staged context files.

Key flags: `--profile`, `--url`, `--token`, `--basic-user`, `--basic-password`, `--prompt-password`, `--prompt-token`, `--timeout`, `--verify-ssl`, `--insecure`, `--ca-cert`, `--all-orgs`, `--org-id`, `--sync-summary-file`, `--bundle-preflight-file`, `--promotion-summary-file`, `--mapping-file`, `--availability-file`, `--output-format`.

Notes:
- Prefer `--profile` for normal live status checks.
- `--all-orgs` is safest with admin-backed `--profile` or direct Basic auth because token scope can hide other orgs.

Examples:

```bash
# Purpose: live.
grafana-util status live --profile prod --output-format yaml
```

```bash
# Purpose: live.
grafana-util status live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format json
```

```bash
# Purpose: live.
grafana-util status live --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --sync-summary-file ./sync-summary.json --output-format interactive
```

Related commands: `grafana-util overview live`, `grafana-util change apply`, `grafana-util profile show`.
