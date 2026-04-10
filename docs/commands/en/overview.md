# Legacy: `grafana-util overview`

## Root

Purpose: compatibility reference for the old project-wide overview root.

When to use: when you are translating older docs or scripts to the current `observe` surface.

Description: the public overview surface now lives under `grafana-util observe`. Use `observe overview` or `observe live` instead of the legacy top-level `overview` root.

Key flags: staged inputs such as `--dashboard-export-dir`, `--dashboard-provisioning-dir`, `--datasource-export-dir`, `--datasource-provisioning-file`, `--access-user-export-dir`, `--access-team-export-dir`, `--access-org-export-dir`, `--access-service-account-export-dir`, `--desired-file`, `--source-bundle`, `--target-inventory`, `--alert-export-dir`, `--availability-file`, `--mapping-file`, and `--output-format`.

Examples:

```bash
# Purpose: Summarize staged dashboard, alert, and access artifacts.
grafana-util observe overview --dashboard-export-dir ./dashboards/raw --alert-export-dir ./alerts --desired-file ./desired.json --output-format table
```

```bash
# Purpose: Review sync bundle inputs before promotion.
grafana-util observe overview --source-bundle ./sync-source-bundle.json --target-inventory ./target-inventory.json --availability-file ./availability.json --mapping-file ./mapping.json --output-format text
```

Related commands: `grafana-util observe staged`, `grafana-util change inspect`, `grafana-util snapshot review`.

## `live`

Purpose: render the live overview by delegating to the shared observe live path.

When to use: when you need the same live readout that `observe live` uses, but want it under the overview namespace.

Key flags: live connection and auth flags from the shared observe live path, plus `--sync-summary-file`, `--bundle-preflight-file`, `--promotion-summary-file`, `--mapping-file`, `--availability-file`, and `--output-format`.

Notes:
- Prefer `--profile` for repeatable live overview work.
- Direct Basic auth is the safer fallback for broader org visibility.
- Token auth is fine for scoped reads, but the visible results still follow the token's permission envelope.

Examples:

```bash
# Purpose: live.
grafana-util observe overview --profile prod --output-format yaml
```

```bash
# Purpose: live.
grafana-util observe overview --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format json
```

```bash
# Purpose: live.
grafana-util observe overview --url http://localhost:3000 --basic-user admin --basic-password admin --output-format interactive
```

Related commands: `grafana-util observe live`, `grafana-util change apply`, `grafana-util config profile show`.
