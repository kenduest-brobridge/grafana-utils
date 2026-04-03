# `grafana-util overview`

## Root

Purpose: summarize staged artifacts into one project-wide overview.

When to use: when you want a single readout across dashboard, datasource, access, alert, and change artifacts before checking status or promoting changes.

Key flags: staged inputs such as `--dashboard-export-dir`, `--dashboard-provisioning-dir`, `--datasource-export-dir`, `--datasource-provisioning-file`, `--access-user-export-dir`, `--access-team-export-dir`, `--access-org-export-dir`, `--access-service-account-export-dir`, `--desired-file`, `--source-bundle`, `--target-inventory`, `--alert-export-dir`, `--availability-file`, `--mapping-file`, and `--output`.

Examples:

```bash
grafana-util overview --dashboard-export-dir ./dashboards/raw --alert-export-dir ./alerts --desired-file ./desired.json --output table
grafana-util overview --source-bundle ./sync-source-bundle.json --target-inventory ./target-inventory.json --availability-file ./availability.json --mapping-file ./mapping.json --output text
```

Related commands: `grafana-util status staged`, `grafana-util change summary`, `grafana-util snapshot review`.

## `live`

Purpose: render the live overview by delegating to the shared status live path.

When to use: when you need the same live readout that `status live` uses, but want it under the overview namespace.

Key flags: live connection and auth flags from the shared status live path, plus `--sync-summary-file`, `--bundle-preflight-file`, `--promotion-summary-file`, `--mapping-file`, `--availability-file`, and `--output`.

Notes:
- Prefer `--profile` for repeatable live overview work.
- Direct Basic auth is the safer fallback for broader org visibility.
- Token auth is fine for scoped reads, but the visible results still follow the token's permission envelope.

Examples:

```bash
grafana-util overview live --profile prod --output yaml
grafana-util overview live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output json
grafana-util overview live --url http://localhost:3000 --basic-user admin --basic-password admin --output interactive
```

Related commands: `grafana-util status live`, `grafana-util change apply`, `grafana-util profile show`.
