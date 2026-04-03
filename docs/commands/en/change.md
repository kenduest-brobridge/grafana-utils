# `grafana-util change`

## Root

Purpose: review-first sync workflows with optional live Grafana fetch and apply paths.

When to use: when you need to summarize desired resources, plan against live state, review a plan, apply a reviewed plan, audit drift, or build bundle and promotion preflight documents.

Key flags: the root command is a namespace; the main operational flags live on the subcommands. Common workflow inputs include `--desired-file`, `--plan-file`, `--live-file`, `--fetch-live`, `--approve`, `--execute-live`, `--source-bundle`, `--target-inventory`, `--availability-file`, `--mapping-file`, and `--output`.

Examples:

```bash
grafana-util change summary --desired-file ./desired.json
grafana-util change plan --desired-file ./desired.json --fetch-live --profile prod
grafana-util change apply --plan-file ./sync-plan-reviewed.json --approve --execute-live --profile prod
```

Related commands: `grafana-util overview`, `grafana-util status`, `grafana-util snapshot`.

## `summary`

Purpose: summarize local desired sync resources.

When to use: when you want a quick size check before planning or applying.

Key flags: `--desired-file`, `--output`.

Examples:

```bash
grafana-util change summary --desired-file ./desired.json
grafana-util change summary --desired-file ./desired.json --output json
```

Related commands: `change plan`, `change preflight`.

## `plan`

Purpose: build a staged sync plan from desired and live state.

When to use: when you need a reviewable plan before marking work as reviewed or applying it.

Key flags: `--desired-file`, `--live-file`, `--fetch-live`, `--org-id`, `--page-size`, `--allow-prune`, `--trace-id`, `--output`.

Examples:

```bash
grafana-util change plan --desired-file ./desired.json --live-file ./live.json
grafana-util change plan --desired-file ./desired.json --fetch-live --profile prod --allow-prune --output json
```

Related commands: `change review`, `change apply`, `change summary`.

## `review`

Purpose: mark a staged sync plan as reviewed.

When to use: when a plan has been inspected and should carry an explicit review token before apply.

Key flags: `--plan-file`, `--review-token`, `--reviewed-by`, `--reviewed-at`, `--review-note`, `--interactive`, `--output`.

Examples:

```bash
grafana-util change review --plan-file ./sync-plan.json
grafana-util change review --plan-file ./sync-plan.json --review-note 'peer-reviewed' --output json
```

Related commands: `change plan`, `change apply`.

## `apply`

Purpose: build a gated apply intent from a reviewed sync plan, and optionally execute it live.

When to use: when a plan is already reviewed and you are ready to emit or execute the apply step.

Key flags: `--plan-file`, `--preflight-file`, `--bundle-preflight-file`, `--approve`, `--execute-live`, `--allow-folder-delete`, `--allow-policy-reset`, `--org-id`, `--output`, `--applied-by`, `--applied-at`, `--approval-reason`, `--apply-note`.

Examples:

```bash
grafana-util change apply --plan-file ./sync-plan-reviewed.json --approve
grafana-util change apply --plan-file ./sync-plan-reviewed.json --approve --execute-live --allow-folder-delete --profile prod
```

Related commands: `change review`, `change preflight`, `change bundle-preflight`.

## `audit`

Purpose: audit managed Grafana resources against a checksum lock and current live state.

When to use: when you need a drift check or want to refresh a lock snapshot.

Key flags: `--managed-file`, `--lock-file`, `--live-file`, `--fetch-live`, `--org-id`, `--page-size`, `--write-lock`, `--fail-on-drift`, `--interactive`, `--output`.

Examples:

```bash
grafana-util change audit --managed-file ./desired.json --live-file ./live.json --write-lock ./sync-lock.json
grafana-util change audit --lock-file ./sync-lock.json --fetch-live --profile prod --fail-on-drift --output json
```

Related commands: `change preflight`, `change plan`, `status live`.

## `preflight`

Purpose: build a staged sync preflight document from desired resources and optional availability hints.

When to use: when you need a structural gate before planning or applying.

Key flags: `--desired-file`, `--availability-file`, `--fetch-live`, `--org-id`, `--output`.

Examples:

```bash
grafana-util change preflight --desired-file ./desired.json --availability-file ./availability.json
grafana-util change preflight --desired-file ./desired.json --fetch-live --profile prod --output json
```

Related commands: `change summary`, `change plan`, `status staged`.

## `assess-alerts`

Purpose: assess alert sync specs for candidate, plan-only, and blocked states.

When to use: when you want a focused readout of how alert resources will be classified before bundling or applying.

Key flags: `--alerts-file`, `--output`.

Examples:

```bash
grafana-util change assess-alerts --alerts-file ./alerts.json
grafana-util change assess-alerts --alerts-file ./alerts.json --output json
```

Related commands: `change bundle`, `change bundle-preflight`, `overview`.

## `bundle`

Purpose: package exported dashboards, alerting resources, datasource inventory, and metadata into one local source bundle.

When to use: when you want a single bundle artifact for later sync, review, or preflight steps.

Key flags: `--dashboard-export-dir`, `--dashboard-provisioning-dir`, `--alert-export-dir`, `--datasource-export-file`, `--datasource-provisioning-file`, `--metadata-file`, `--output-file`, `--output`.

Examples:

```bash
grafana-util change bundle --dashboard-export-dir ./dashboards/raw --alert-export-dir ./alerts/raw --output-file ./sync-source-bundle.json
grafana-util change bundle --dashboard-provisioning-dir ./dashboards/provisioning --alert-export-dir ./alerts/raw --output-file ./sync-source-bundle.json
```

Related commands: `change bundle-preflight`, `change promotion-preflight`, `snapshot export`.

## `bundle-preflight`

Purpose: build a staged bundle-level sync preflight document from a source bundle and target inventory.

When to use: when you need to compare a source bundle against a target inventory before apply.

Key flags: `--source-bundle`, `--target-inventory`, `--availability-file`, `--fetch-live`, `--org-id`, `--output`.

Examples:

```bash
grafana-util change bundle-preflight --source-bundle ./sync-source-bundle.json --target-inventory ./target-inventory.json --output json
grafana-util change bundle-preflight --source-bundle ./sync-source-bundle.json --target-inventory ./target-inventory.json --availability-file ./availability.json --output json
```

Related commands: `change bundle`, `change promotion-preflight`, `status staged`.

## `promotion-preflight`

Purpose: build a staged promotion review handoff from a source bundle and target inventory.

When to use: when you are preparing a promotion review and want an explicit mapping and availability view.

Key flags: `--source-bundle`, `--target-inventory`, `--mapping-file`, `--availability-file`, `--fetch-live`, `--org-id`, `--output`.

Examples:

```bash
grafana-util change promotion-preflight --source-bundle ./sync-source-bundle.json --target-inventory ./target-inventory.json --mapping-file ./promotion-map.json --output json
grafana-util change promotion-preflight --source-bundle ./sync-source-bundle.json --target-inventory ./target-inventory.json --mapping-file ./promotion-map.json --availability-file ./availability.json --output json
```

Related commands: `change bundle-preflight`, `change apply`, `status live`.
