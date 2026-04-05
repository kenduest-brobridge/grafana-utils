# Project Status & Change Overview

Use this chapter when you need to answer two questions before or after a change: is the estate ready, and what exactly will move?

This domain focuses on the governance gate: the final layer of validation before and after making changes.

## Who It Is For

- Operators reviewing readiness before a maintenance window or apply path.
- Engineers who need to summarize staged inputs before promotion.
- Reviewers who want to separate “is the estate healthy?” from “what will this bundle do?”

## Primary Goals

- Separate live health checks from staged change inspection.
- Catch broken inputs before an apply path starts.
- Give reviewers a stable summary of what changed and what still looks risky.

## Before / After

- Before: status, snapshots, and change reviews could feel like separate tools with overlapping names.
- After: live checks, staged reviews, and snapshot-style summaries are grouped into one guided path.

## What success looks like

- You can tell whether the task is about readiness, snapshots, or review before change.
- You know which command to open when a workflow moves from status into mutation.
- You can explain what should happen before a change is applied.

## Failure checks

- If the staged and live surfaces disagree, stop and identify which lane is stale before applying anything.
- If a snapshot or summary does not match your expectation, treat that as a workflow warning, not a cosmetic issue.
- If you cannot say why you need this chapter, you may be in the wrong workflow lane.

## 🔗 Command Pages

Need the command-by-command surface instead of the workflow guide?

Primary lane:

- [change](../../commands/en/change.md)
- [change inspect](../../commands/en/change.md#inspect)
- [change check](../../commands/en/change.md#check)
- [change preview](../../commands/en/change.md#preview)
- [change apply](../../commands/en/change.md#apply)
- [status](../../commands/en/status.md)
- [status staged](../../commands/en/status.md#staged)
- [status live](../../commands/en/status.md#live)
- [overview](../../commands/en/overview.md)
- [overview live](../../commands/en/overview.md#live)

Advanced workflows:

- Need lower-level staged contracts or bundle/promotion handoff docs? Start at [change advanced](../../commands/en/change.md#advanced) or the [full command index](../../commands/en/index.md).
- [snapshot](../../commands/en/snapshot.md)
- [snapshot export](../../commands/en/snapshot.md#export)
- [snapshot review](../../commands/en/snapshot.md#review)
- [profile](../../commands/en/profile.md)
- [profile list](../../commands/en/profile.md#list)
- [profile show](../../commands/en/profile.md#show)
- [profile add](../../commands/en/profile.md#add)
- [profile example](../../commands/en/profile.md#example)
- [profile init](../../commands/en/profile.md#init)

---

## 🚦 Status Surfaces

We distinguish between **Live** (what is actually running) and **Staged** (what you intend to deploy).

### 1. Live Readiness Check
```bash
# Purpose: 1. Live Readiness Check.
grafana-util status live --output-format table
```

```bash
# Purpose: 1. Live Readiness Check.
grafana-util status live --profile prod --sync-summary-file ./sync-summary.json --bundle-preflight-file ./bundle-preflight.json --output-format json
```
**Expected Output:**
```text
OVERALL: status=ready

COMPONENT    HEALTH   REASON
Dashboards   ok       32/32 Accessible
Datasources  ok       Secret recovery verified
Alerts       ok       No dangling rules
```
Use `status live` when you want the shared live status path to tell you whether Grafana is safe to read from or promote into. The extra staged sync files deepen the live view without changing the command shape.

### 2. Staged Readiness Check
Use this as a mandatory CI/CD gate before running `apply`.
```bash
# Purpose: Use this as a mandatory CI/CD gate before running apply.
grafana-util status staged --desired-file ./desired.json --output-format json
```

```bash
# Purpose: Use this as a mandatory CI/CD gate before running apply.
grafana-util status staged --dashboard-export-dir ./dashboards/raw --alert-export-dir ./alerts --desired-file ./desired.json --output-format table
```
**Expected Output:**
```json
{
  "status": "ready",
  "blockers": [],
  "warnings": ["1 dashboard missing a unique folder assignment"]
}
```
`status staged` is the machine-readable gate. Treat `blockers` as hard stops and `warnings` as review items.

---

## 📋 Change Lifecycle

Manage the transition from Git to production Grafana.

### First-run shortcut

If you are not sure where to start, use this sequence:

1. `change inspect --workspace .`
2. `change check --workspace .`
3. `change preview --workspace . --fetch-live --profile <profile>`
4. `change apply --preview-file ./change-preview.json --approve --execute-live --profile <profile>`

`--workspace` is the shortest path because `change` will try to discover common staged inputs in the current repo or working tree. If that does not match your layout, switch to explicit flags such as `--desired-file`, `--dashboard-export-dir`, `--alert-export-dir`, `--source-bundle`, or `--target-inventory`.

### 1. Change Inspect
Get a fast, task-first summary of what the staged package contains.
```bash
# Purpose: Inspect the staged package from the current workspace.
grafana-util change inspect --workspace .
```

```bash
# Purpose: Inspect explicit staged exports as JSON.
grafana-util change inspect --dashboard-export-dir ./dashboards/raw --alert-export-dir ./alerts/raw --output-format json
```
**Expected Output:**
```text
CHANGE PACKAGE SUMMARY:
- dashboards: 5 modified, 2 added
- alerts: 3 modified
- access: 1 added
- total impact: 11 operations
```
Use inspect to size the change before you fetch live state. If the total is unexpectedly large, stop and review the staged inputs first.

### 2. Change Check
Verify staged readiness before you preview or apply anything.
```bash
# Purpose: Check the discovered staged package with staged availability hints.
grafana-util change check --workspace . --availability-file ./availability.json
```

```bash
# Purpose: Check the staged package and merge live availability hints.
grafana-util change check --workspace . --fetch-live --output-format json
```
**Expected Output:**
```text
PREFLIGHT CHECK:
- dashboards: valid (7 files)
- datasources: valid (1 inventory found)
- result: 0 errors, 0 blockers
```
Use check when you need a readiness gate before preview or apply. A clean check means the inputs are shaped correctly and any requested availability checks passed; it does not mean live Grafana already matches them.

### 3. Change Preview
Build the actionable preview that shows what would change.
```bash
# Purpose: Preview the current staged package against live Grafana.
grafana-util change preview --workspace . --fetch-live --profile prod
```

```bash
# Purpose: Preview one explicit desired/live pair as JSON.
grafana-util change preview --desired-file ./desired.json --live-file ./live.json --output-format json
```

Preview is the task-first replacement for the common `plan` step. It still emits the same reviewable staged contract underneath, but the operator entrypoint is now “preview what would change” instead of “build a plan document.”

---

## 🖥️ Interactive Mode (TUI) Semantics

`overview live --output-format interactive` opens the live project overview through the shared status live path.

```bash
# Purpose: overview live --output-format interactive opens the live project overview through the shared status live path.
grafana-util overview live --url http://localhost:3000 --basic-user admin --basic-password admin --output-format interactive
```

The TUI uses the following visual language:
- **🟢 Green**: The component is healthy and fully reachable.
- **🟡 Yellow**: The component is functional but has warnings, such as missing metadata.
- **🔴 Red**: The component is blocked and needs action before deployment.

Use `overview` without `live` for staged artifact review, and use `status live` when you need the same live gate in machine-readable form.

---
[⬅️ Previous: Access Management](access.md) | [🏠 Home](index.md) | [➡️ Next: Operator Scenarios](scenarios.md)
