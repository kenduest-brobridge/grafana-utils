# `grafana-util change`

## Root

Purpose: task-first staged change workflow with optional live Grafana preview and apply paths.

When to use: when you need to inspect a staged package, check whether it looks safe to continue, preview what would change, apply an approved preview, or drop into lower-level advanced review contracts.

Description: start here when you want the normal operator lane first. The default `change` surface is now `inspect -> check -> preview -> apply`. Lower-level steps such as `summary`, `plan`, `review`, `preflight`, `audit`, and bundle/promotion workflows still exist under `change advanced` when you need explicit staged contracts.

Before / After:

- **Before**: a change package is just a pile of files and a risky apply path.
- **After**: the same package moves through inspect, check, preview, and apply with explicit checkpoints, while advanced contracts stay available behind `change advanced`.

Key flags: the root command is a namespace; the main operational flags live on the subcommands. Common workflow inputs include `--workspace`, `--desired-file`, `--dashboard-export-dir`, `--dashboard-provisioning-dir`, `--alert-export-dir`, `--source-bundle`, `--target-inventory`, `--availability-file`, `--mapping-file`, `--fetch-live`, `--live-file`, `--preview-file`, `--approve`, `--execute-live`, and `--output-format`.

### First-run path

If you are starting fresh, use this order first:

1. `change inspect` to see what the staged package contains
2. `change check` to confirm it is structurally ready
3. `change preview` to see what would change
4. `change apply` only after the preview is reviewed and approved

### What `--workspace` tries to discover

When you pass `--workspace .`, `change` looks for the common staged inputs it can assemble into one review lane:

- dashboard export trees
- dashboard provisioning trees
- datasource provisioning files
- alert export trees
- staged desired change files
- source bundles, target inventory files, and promotion mapping files

If discovery finds nothing usable, stop and switch to explicit flags such as `--desired-file`, `--dashboard-export-dir`, `--alert-export-dir`, `--source-bundle`, or `--target-inventory`.

What success looks like:

- you can explain the size and risk of a change before apply
- staged inputs pass preflight before they enter plan or apply
- reviewed plans carry explicit evidence instead of relying on operator memory

Failure checks:

- if summary or preflight looks wrong, stop before plan or apply
- if live fetch changes the result unexpectedly, compare staged inputs against the live target first
- if automation consumes JSON, validate `kind` and `schemaVersion` before deeper parsing

Examples:

```bash
# Purpose: Inspect the staged package from common repo-local inputs.
grafana-util change inspect --workspace .
```

```bash
# Purpose: Check whether the staged package is safe to continue.
grafana-util change check --workspace . --fetch-live --output-format json
```

```bash
# Purpose: Preview what would change from discovered or explicit staged inputs.
grafana-util change preview --workspace . --fetch-live --profile prod
```

```bash
# Purpose: Apply a reviewed preview to live Grafana after explicit approval.
grafana-util change apply \
  --preview-file ./change-preview.json \
  --approve \
  --execute-live \
  --profile prod
```

Related commands: `grafana-util overview`, `grafana-util status`, `grafana-util snapshot`.

## Primary Subcommands

Use the root page to understand the overall lane, then open the subcommand page that matches the step you are actually performing:

- [change inspect](./change-inspect.md): identify what the staged package contains
- [change check](./change-check.md): confirm that the staged package is structurally ready
- [change preview](./change-preview.md): build the reviewable change preview
- [change apply](./change-apply.md): turn a reviewed preview into an apply intent or a live apply

This split is intentional. `change` is a namespace with one primary lane plus a set of advanced contracts; the step-by-step operator manual lives on the individual subcommand pages.

## `advanced`

Purpose: expose lower-level staged contracts and specialized sync workflows.

When to use: when you need explicit `summary`, `plan`, `review`, `preflight`, `audit`, `bundle`, or promotion handoff documents instead of the task-first lane.

Examples:

```bash
# Purpose: Enter lower-level staged contracts or specialized sync workflows only when the primary lane is not enough.
grafana-util change advanced bundle-preflight --source-bundle ./bundle.json --target-inventory ./target.json --output-format json
```

### For CI and scripts only

If you are reading this page as an operator, stop here and use the primary `inspect -> check -> preview -> apply` lane unless you know you need one of the lower-level contracts below.

If you are automating around `change` outputs, treat `kind` plus `schemaVersion` as the contract guard before you inspect the rest of the payload.

Quick lookups from the CLI:

- `grafana-util change --help-schema`
- `grafana-util change inspect --help`
- `grafana-util change preview --help-schema`
- `grafana-util change apply --help-schema`

| Command | Output kind | Top-level fields to expect |
| --- | --- | --- |
| `change inspect --output-format json` | overview/status-style staged summary | command-specific staged summary and discovered-input output |
| `change check --output-format json` | project-status staged status | staged readiness/status output plus blockers or warnings |
| `change preview --output-format json` | `grafana-utils-sync-plan` or bundle/promotion preflight kinds | preview uses the existing staged plan/bundle-preflight/promotion-preflight contracts under a task-first entrypoint |
| `change apply --output-format json` | `grafana-utils-sync-apply-intent` | `kind`, `schemaVersion`, `toolVersion`, `mode`, `reviewed`, `reviewRequired`, `allowPrune`, `approved`, `summary`, `alertAssessment`, `operations`, optional `preflightSummary`, optional `bundlePreflightSummary`, `appliedBy`, `appliedAt`, `approvalReason`, `applyNote`, `traceId`, `stage`, `stepIndex`, `parentTraceId` |
| `change apply --execute-live --output-format json` | live apply result | `mode`, `appliedCount`, `results` |
| `change advanced audit --output-format json` | `grafana-utils-sync-audit` | `kind`, `schemaVersion`, `toolVersion`, `summary`, `currentLock`, `baselineLock`, `drifts` |
| `change advanced preflight --output-format json` | `grafana-utils-sync-preflight` | `kind`, `schemaVersion`, `toolVersion`, `summary`, `checks` |
| `change advanced assess-alerts --output-format json` | `grafana-utils-alert-sync-plan` | `kind`, `schemaVersion`, `toolVersion`, `summary`, `alerts` |
| `change advanced bundle-preflight --output-format json` | `grafana-utils-sync-bundle-preflight` | `kind`, `schemaVersion`, `summary`, `syncPreflight`, `alertArtifactAssessment`, `secretPlaceholderAssessment`, `providerAssessment` |
| `change advanced promotion-preflight --output-format json` | `grafana-utils-sync-promotion-preflight` | `kind`, `schemaVersion`, `toolVersion`, `summary`, `bundlePreflight`, `mappingSummary`, `checkSummary`, `handoffSummary`, `continuationSummary`, `checks`, `resolvedChecks`, `blockingChecks` |

Notes:

- `change apply` has two JSON shapes. Without `--execute-live`, it emits a staged apply-intent document. With `--execute-live`, it emits the live execution result instead.
- `change preview` is task-first. It may emit the existing staged plan kind or the bundle/promotion preflight kinds depending on which staged inputs you provide.
- `change apply` accepts `--preview-file` and still accepts `--plan-file` as an alias for compatibility.
- `change advanced bundle` does not use `--output-format`; it writes the bundle contract with `--output-file`.

## `summary`

Purpose: summarize local desired sync resources.

When to use: when you want a quick size check before planning or applying.

Key flags: `--desired-file`, `--output-format`.

JSON shape:

- `kind`: `grafana-utils-sync-summary`
- `schemaVersion`: current contract version
- `toolVersion`: emitting CLI version
- `summary`: aggregate counts
  - `resourceCount`, `dashboardCount`, `datasourceCount`, `folderCount`, `alertCount`
- `resources`: normalized rows
  - `kind`, `identity`, `title`, `managedFields`, `bodyFieldCount`, `sourcePath`

Examples:

```bash
# Purpose: summary.
grafana-util change summary --desired-file ./desired.json
```

```bash
# Purpose: summary.
grafana-util change summary --desired-file ./desired.json --output-format json
```

**Expected Output:**
```json
{
  "kind": "grafana-utils-sync-summary",
  "summary": {
    "resourceCount": 8,
    "dashboardCount": 5,
    "datasourceCount": 1
  },
  "resources": []
}
```
Use this as the quickest “how big is this change package?” artifact before you spend time on preflight or plan work.

Related commands: `change plan`, `change preflight`.

## `plan`

Purpose: build a staged sync plan from desired and live state.

When to use: when you need a reviewable plan before marking work as reviewed or applying it.

Before / After:

- **Before**: "what will this change?" is answered by intuition or by reading raw desired files.
- **After**: one staged plan shows creates, updates, deletes, and blocked alert work before review or apply.

Key flags: `--desired-file`, `--live-file`, `--fetch-live`, `--org-id`, `--page-size`, `--allow-prune`, `--trace-id`, `--output-format`.

JSON shape:

- `kind`: `grafana-utils-sync-plan`
- `schemaVersion`, `toolVersion`
- staged metadata: `dryRun`, `reviewRequired`, `reviewed`, `allowPrune`
- lineage: `traceId`, `stage`, `stepIndex`, `parentTraceId`
- `summary`
  - `would_create`, `would_update`, `would_delete`, `noop`, `unmanaged`
  - `alert_candidate`, `alert_plan_only`, `alert_blocked`
- `alertAssessment`: nested alert sync summary and `alerts` rows
- `operations`: one row per desired/live comparison
  - `kind`, `identity`, `title`, `action`, `reason`, `changedFields`, `managedFields`, `desired`, `live`, `sourcePath`

Examples:

```bash
# Purpose: plan.
grafana-util change plan --desired-file ./desired.json --live-file ./live.json
```

```bash
# Purpose: plan.
grafana-util change plan \
  --desired-file ./desired.json \
  --fetch-live \
  --profile prod \
  --allow-prune \
  --output-format json
```

**Expected Output:**
```json
{
  "kind": "grafana-utils-sync-plan",
  "reviewed": false,
  "summary": {
    "would_create": 1,
    "would_update": 4,
    "would_delete": 0
  },
  "operations": []
}
```
This is the review artifact most teams actually pass around. The key signals are the `summary` counts and whether the plan is still unreviewed.

Related commands: `change review`, `change apply`, `change summary`.

## `review`

Purpose: mark a staged sync plan as reviewed.

When to use: when a plan has been inspected and should carry an explicit review token before apply.

Before / After:

- **Before**: a team may say a plan was reviewed, but the file itself still does not carry review evidence.
- **After**: the staged plan records who reviewed it, when it was reviewed, and any review note needed before apply.

Key flags: `--plan-file`, `--review-token`, `--reviewed-by`, `--reviewed-at`, `--review-note`, `--interactive`, `--output-format`.

JSON shape:

- same base shape as `change plan`
- review state changes:
  - `reviewed: true`
  - `stage: review`
  - `stepIndex: 2`
- review audit fields:
  - `reviewedBy`
  - `reviewedAt`
  - `reviewNote`

Examples:

```bash
# Purpose: review.
grafana-util change review --plan-file ./sync-plan.json
```

```bash
# Purpose: review.
grafana-util change review --plan-file ./sync-plan.json --review-note 'peer-reviewed' --output-format json
```

**Expected Output:**
```json
{
  "kind": "grafana-utils-sync-plan",
  "reviewed": true,
  "stage": "review",
  "stepIndex": 2,
  "reviewNote": "peer-reviewed"
}
```
Once review succeeds, downstream apply steps should no longer need to infer approval from filenames or operator memory.

What success looks like:

- the reviewed plan is a distinct artifact, not just a verbal approval
- downstream apply steps can tell that review already happened
- the review note or reviewer identity is preserved for later audit or handoff

Failure checks:

- if review output still shows `reviewed: false`, confirm you are reading the new reviewed file rather than the old plan
- if review metadata is missing, check whether you expected `--reviewed-by`, `--reviewed-at`, or `--review-note` to be recorded
- if a later step rejects the reviewed plan, inspect the `stage`, `stepIndex`, and review fields before assuming apply is broken

Related commands: `change plan`, `change apply`.

## `audit`

Purpose: audit managed Grafana resources against a checksum lock and current live state.

When to use: when you need a drift check or want to refresh a lock snapshot.

Key flags: `--managed-file`, `--lock-file`, `--live-file`, `--fetch-live`, `--org-id`, `--page-size`, `--write-lock`, `--fail-on-drift`, `--interactive`, `--output-format`.

JSON shape:

- `kind`: `grafana-utils-sync-audit`
- `schemaVersion`, `toolVersion`
- `summary`
  - `managedCount`, `baselineCount`, `currentPresentCount`, `currentMissingCount`
  - `inSyncCount`, `driftCount`, `missingLockCount`, `missingLiveCount`
- `currentLock`: newly built lock snapshot
- `baselineLock`: prior lock document or `null`
- `drifts`
  - `kind`, `identity`, `title`, `status`
  - `baselineStatus`, `currentStatus`
  - `baselineChecksum`, `currentChecksum`
  - `driftedFields`, `sourcePath`

Examples:

```bash
# Purpose: audit.
grafana-util change audit --managed-file ./desired.json --live-file ./live.json --write-lock ./sync-lock.json
```

```bash
# Purpose: audit.
grafana-util change audit --lock-file ./sync-lock.json --fetch-live --profile prod --fail-on-drift --output-format json
```

**Expected Output:**
```json
{
  "kind": "grafana-utils-sync-audit",
  "summary": {
    "inSyncCount": 12,
    "driftCount": 1
  },
  "drifts": []
}
```
Treat `driftCount > 0` as the top-level signal that your managed state and live Grafana have separated.

Related commands: `change preflight`, `change plan`, `status live`.

## `preflight`

Purpose: build a staged sync preflight document from desired resources and optional availability hints.

When to use: when you need a structural gate before planning or applying.

Before / After:

- **Before**: teams often discover missing folders, unavailable dependencies, or policy blockers only after they already built a plan.
- **After**: preflight turns those checks into an explicit document you can inspect before the workflow gets more expensive.

Key flags: `--desired-file`, `--availability-file`, `--fetch-live`, `--org-id`, `--output-format`.

JSON shape:

- `kind`: `grafana-utils-sync-preflight`
- `schemaVersion`, `toolVersion`
- `summary`
  - `checkCount`, `okCount`, `blockingCount`
- `checks`
  - `kind`, `identity`, `status`, `detail`, `blocking`

Examples:

```bash
# Purpose: preflight.
grafana-util change preflight --desired-file ./desired.json --availability-file ./availability.json
```

```bash
# Purpose: preflight.
grafana-util change preflight \
  --desired-file ./desired.json \
  --fetch-live \
  --profile prod \
  --output-format json
```

**Expected Output:**
```json
{
  "kind": "grafana-utils-sync-preflight",
  "summary": {
    "checkCount": 6,
    "okCount": 6,
    "blockingCount": 0
  },
  "checks": []
}
```
This is the structural gate. If `blockingCount` is not zero, stop here and fix the inputs before you build or approve a plan.

What success looks like:

- the preflight document tells you whether the change is structurally ready to enter plan or apply
- blocking checks are explicit enough that another operator or CI lane can stop safely
- availability hints and live fetch data line up with the target environment you intend to change

Failure checks:

- if preflight blocks unexpectedly, verify that the `desired` input and any `availability` input belong to the same environment
- if a live-backed preflight looks wrong, confirm the auth, org, and target Grafana before trusting the result
- if CI is parsing the JSON, use `kind` and `schemaVersion` first, then inspect `summary` and `checks`

Related commands: `change summary`, `change plan`, `status staged`.

## `assess-alerts`

Purpose: assess alert sync specs for candidate, plan-only, and blocked states.

When to use: when you want a focused readout of how alert resources will be classified before bundling or applying.

Key flags: `--alerts-file`, `--output-format`.

JSON shape:

- `kind`: `grafana-utils-alert-sync-plan`
- `schemaVersion`, `toolVersion`
- `summary`
  - `alertCount`, `candidateCount`, `planOnlyCount`, `blockedCount`
- `alerts`
  - `identity`, `title`, `managedFields`, `status`, `liveApplyAllowed`, `detail`

Examples:

```bash
# Purpose: assess-alerts.
grafana-util change assess-alerts --alerts-file ./alerts.json
```

```bash
# Purpose: assess-alerts.
grafana-util change assess-alerts --alerts-file ./alerts.json --output-format json
```

**Expected Output:**
```json
{
  "kind": "grafana-utils-alert-sync-plan",
  "summary": {
    "alertCount": 3,
    "candidateCount": 2,
    "blockedCount": 1
  },
  "alerts": []
}
```
This is useful when alert resources need a narrower explanation than the wider sync plan gives you.

Related commands: `change bundle`, `change bundle-preflight`, `overview`.

## `bundle`

Purpose: package exported dashboards, alerting resources, datasource inventory, and metadata into one local source bundle.

When to use: when you want a single bundle artifact for later sync, review, or preflight steps.

Key flags: `--dashboard-export-dir`, `--dashboard-provisioning-dir`, `--alert-export-dir`, `--datasource-export-file`, `--datasource-provisioning-file`, `--metadata-file`, `--output-file`, `--output-format`.

JSON shape:

- this command has two output modes
  - `--output-file`: writes the full source bundle contract to disk
  - `--output-format json`: prints the same source bundle contract to stdout
- source bundle top-level fields:
  - `kind`, `schemaVersion`, `toolVersion`
  - `summary`
  - `dashboards`, `datasources`, `folders`, `alerts`
  - optional `alerting`
  - optional `metadata`

Examples:

```bash
# Purpose: bundle.
grafana-util change bundle --dashboard-export-dir ./dashboards/raw --alert-export-dir ./alerts/raw --output-file ./sync-source-bundle.json
```

```bash
# Purpose: bundle.
grafana-util change bundle --dashboard-provisioning-dir ./dashboards/provisioning --alert-export-dir ./alerts/raw --output-file ./sync-source-bundle.json
```

**Expected Output:**
```json
{
  "kind": "grafana-utils-sync-source-bundle",
  "summary": {
    "dashboardCount": 5,
    "datasourceCount": 1,
    "alertCount": 3
  },
  "dashboards": [],
  "alerts": []
}
```
Whether it is written to disk or printed to stdout, this bundle is the packaging artifact used by later bundle and promotion checks.

Related commands: `change bundle-preflight`, `change promotion-preflight`, `snapshot export`.

## `bundle-preflight`

Purpose: build a staged bundle-level sync preflight document from a source bundle and target inventory.

When to use: when you need to compare a source bundle against a target inventory before apply.

Key flags: `--source-bundle`, `--target-inventory`, `--availability-file`, `--fetch-live`, `--org-id`, `--output-format`.

JSON shape:

- `kind`: `grafana-utils-sync-bundle-preflight`
- `schemaVersion`
- `summary`
  - `resourceCount`
  - `syncBlockingCount`
  - `providerBlockingCount`
  - `secretPlaceholderBlockingCount`
  - `alertArtifactCount`
  - `alertArtifactBlockedCount`
  - `alertArtifactPlanOnlyCount`
- `syncPreflight`: nested `grafana-utils-sync-preflight`
- `providerAssessment`: provider summary, plans, and checks
- `secretPlaceholderAssessment`: placeholder summary, plans, and checks
- `alertArtifactAssessment`: alert artifact summary and checks

Examples:

```bash
# Purpose: bundle-preflight.
grafana-util change bundle-preflight --source-bundle ./sync-source-bundle.json --target-inventory ./target-inventory.json --output-format json
```

```bash
# Purpose: bundle-preflight.
grafana-util change bundle-preflight --source-bundle ./sync-source-bundle.json --target-inventory ./target-inventory.json --availability-file ./availability.json --output-format json
```

**Expected Output:**
```json
{
  "kind": "grafana-utils-sync-bundle-preflight",
  "summary": {
    "resourceCount": 9,
    "syncBlockingCount": 0,
    "providerBlockingCount": 0
  },
  "syncPreflight": {}
}
```
This is the right artifact when you want one bundle-level go/no-go result before any promotion or apply handoff.

Related commands: `change bundle`, `change promotion-preflight`, `status staged`.

## `promotion-preflight`

Purpose: build a staged promotion review handoff from a source bundle and target inventory.

When to use: when you are preparing a promotion review and want an explicit mapping and availability view.

Key flags: `--source-bundle`, `--target-inventory`, `--mapping-file`, `--availability-file`, `--fetch-live`, `--org-id`, `--output-format`.

JSON shape:

- `kind`: `grafana-utils-sync-promotion-preflight`
- `schemaVersion`, `toolVersion`
- `summary`
  - `resourceCount`, `directMatchCount`, `mappedCount`
  - `missingMappingCount`, `bundleBlockingCount`, `blockingCount`
- `bundlePreflight`: nested bundle-preflight result
- `mappingSummary`
  - `mappingKind`, `mappingSchemaVersion`, `sourceEnvironment`, `targetEnvironment`
  - `folderMappingCount`, `datasourceUidMappingCount`, `datasourceNameMappingCount`
- `checkSummary`
  - `folderRemapCount`, `datasourceUidRemapCount`, `datasourceNameRemapCount`
  - `resolvedCount`, `directCount`, `mappedCount`, `missingTargetCount`
- `handoffSummary`
- `continuationSummary`
- `checks`, `resolvedChecks`, `blockingChecks`
  - each row includes `kind`, `identity`, `sourceValue`, `targetValue`, `resolution`, `mappingSource`, `status`, `detail`, `blocking`

Examples:

```bash
# Purpose: promotion-preflight.
grafana-util change promotion-preflight --source-bundle ./sync-source-bundle.json --target-inventory ./target-inventory.json --mapping-file ./promotion-map.json --output-format json
```

```bash
# Purpose: promotion-preflight.
grafana-util change promotion-preflight --source-bundle ./sync-source-bundle.json --target-inventory ./target-inventory.json --mapping-file ./promotion-map.json --availability-file ./availability.json --output-format json
```

**Expected Output:**
```json
{
  "kind": "grafana-utils-sync-promotion-preflight",
  "summary": {
    "resourceCount": 9,
    "directMatchCount": 6,
    "mappedCount": 3,
    "blockingCount": 0
  },
  "mappingSummary": {},
  "blockingChecks": []
}
```
Use this when the hard question is not “what will change?” but “is this source bundle promotable into that target environment with the current mappings?”.

Related commands: `change bundle-preflight`, `change apply`, `status live`.
