# Workspace Review & Status

Use this chapter when you need to answer two questions before or after a workspace update: is the estate ready, and what exactly will move?

This domain focuses on the review gate: the final layer of validation before and after a local Grafana workspace is applied.

## Who It Is For

- Operators reviewing readiness before a maintenance window or apply path.
- Engineers who need to summarize a local workspace before apply.
- Reviewers who want to separate “is the estate healthy?” from “what will this workspace do?”

## Primary Goals

- Separate live health checks from staged workspace review.
- Catch broken inputs before an apply path starts.
- Give reviewers a stable summary of what changed and what still looks risky.

## Before / After

- Before: live checks, staged reviews, and snapshot-style summaries could feel like separate tools with overlapping names.
- After: live checks, staged reviews, and workspace previews are grouped into one guided path.

## What success looks like

- You can tell whether the task is about readiness, snapshots, or workspace review before apply.
- You know which command to open when a workflow moves from read-only checks into mutation.
- You can explain what should happen before a workspace is applied.

## Failure checks

- If the staged and live surfaces disagree, stop and identify which lane is stale before applying anything.
- If a snapshot or summary does not match your expectation, treat that as a workflow warning, not a cosmetic issue.
- If you cannot say why you need this chapter, you may be in the wrong workflow lane.

## 🔗 Command Pages

Need the command-by-command surface instead of the workflow guide?

Primary lane:

- [workspace](../../commands/en/workspace.md)
- [workspace scan](../../commands/en/workspace-scan.md)
- [workspace test](../../commands/en/workspace-test.md)
- [workspace preview](../../commands/en/workspace-preview.md)
- [workspace apply](../../commands/en/workspace-apply.md)
- [status](../../commands/en/status.md)
- [status staged](../../commands/en/status.md#staged)
- [status live](../../commands/en/status.md#live)
- [status overview](../../commands/en/status.md#overview)
- [status snapshot](../../commands/en/status.md#snapshot)

Supporting pages:

- [snapshot](../../commands/en/snapshot.md)
- [snapshot export](../../commands/en/snapshot.md#export)
- [snapshot review](../../commands/en/snapshot.md#review)
- [config](../../commands/en/config.md)
- [config profile](../../commands/en/profile.md)
- [config profile list](../../commands/en/profile.md#list)
- [config profile show](../../commands/en/profile.md#show)
- [config profile add](../../commands/en/profile.md#add)
- [config profile example](../../commands/en/profile.md#example)
- [config profile init](../../commands/en/profile.md#init)

---

## 🚦 Status Surfaces

We distinguish between **Live** (what is actually running) and **Staged** (what you intend to apply).

### 1. Live Readiness Check
```bash
# Purpose: 1. Live Readiness Check.
grafana-util status live --output-format table
```

```bash
# Purpose: 1. Live Readiness Check.
grafana-util status live --profile prod --sync-summary-file ./sync-summary.json --package-test-file ./workspace-package-test.json --output-format json
```
**Expected Output:**
```text
OVERALL: status=ready

COMPONENT    HEALTH   REASON
Dashboards   ok       32/32 Accessible
Datasources  ok       Secret recovery verified
Alerts       ok       No dangling rules
```
Use `status live` when you want the shared status path to tell you whether Grafana is safe to read from or apply into. The extra staged sync files deepen the live view without changing the command shape.

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

## 📋 Workspace Lifecycle

Manage the transition from Git to production Grafana.

### First-run shortcut

If you are not sure where to start, use this sequence:

1. `workspace scan .`
2. `workspace test .`
3. `workspace preview . --fetch-live --profile <profile>`
4. `workspace apply --preview-file ./workspace-preview.json --approve --execute-live --profile <profile>`

The workspace path is the shortest path because `workspace` will try to discover common staged inputs in the current repo or working tree, including one repo root that mixes Git Sync dashboards, `alerts/raw`, and `datasources/provisioning`. If that does not match your layout, switch to explicit flags such as `--desired-file`, `--dashboard-export-dir`, `--alert-export-dir`, `workspace package`, or `--target-inventory`.

Example mixed workspace tree:

```text
./grafana-oac-repo/
  dashboards/git-sync/raw/
  dashboards/git-sync/provisioning/
  alerts/raw/
  datasources/provisioning/datasources.yaml
```

### 1. Workspace Scan
Get a fast, task-first summary of what the staged package contains.
```bash
# Purpose: Scan the staged package from one mixed repo root.
grafana-util workspace scan ./grafana-oac-repo
```

This same workspace root can contain `dashboards/git-sync/raw`, `dashboards/git-sync/provisioning`, `alerts/raw`, and `datasources/provisioning/datasources.yaml`.

```bash
# Purpose: Scan explicit staged exports as JSON.
grafana-util workspace scan --dashboard-export-dir ./dashboards/raw --alert-export-dir ./alerts/raw --output-format json
```
**Expected Output:**
```text
WORKSPACE PACKAGE SUMMARY:
- dashboards: 5 modified, 2 added
- alerts: 3 modified
- access: 1 added
- total impact: 11 operations
```
Use scan to size the workspace before you fetch live state. If the total is unexpectedly large, stop and review the staged inputs first.

### 2. Workspace Test
Verify staged readiness before you preview or apply anything.
```bash
# Purpose: Check the discovered mixed workspace with staged availability hints.
grafana-util workspace test ./grafana-oac-repo --availability-file ./availability.json
```

```bash
# Purpose: Check the staged package and merge live availability hints.
grafana-util workspace test . --fetch-live --output-format json
```
**Expected Output:**
```text
WORKSPACE TEST:
- dashboards: valid (7 files)
- datasources: valid (1 inventory found)
- result: 0 errors, 0 blockers
```
Use test when you need a readiness gate before preview or apply. A clean test means the inputs are shaped correctly and any requested availability checks passed; it does not mean live Grafana already matches them.

### 3. Workspace Preview
Build the actionable preview that shows what would change.
```bash
# Purpose: Preview the current mixed workspace against live Grafana.
grafana-util workspace preview ./grafana-oac-repo --fetch-live --profile prod
```

```bash
# Purpose: Preview one explicit desired/live pair as JSON.
grafana-util workspace preview --desired-file ./desired.json --live-file ./live.json --output-format json
```

Preview is the task-first replacement for the common `plan` step. It still emits the same reviewable staged contract underneath, but the operator entrypoint is now “preview what would change” instead of “build a plan document.”
That preview contract is also where ordering lives: `ordering.mode`, `operations[].orderIndex` / `orderGroup` / `kindOrder`, and `summary.blocked_reasons` tell reviewers how the preview is sequenced and which operations remain blocked before apply.

If the same mixed workspace root needs to become a handoff package, run `workspace package ./grafana-oac-repo --output-file ./workspace-package.json`, then keep the resulting `workspace-package.json` as the portable review artifact.

---

## 🖥️ Interactive Mode (TUI) Semantics

`status overview live --output-format interactive` opens the live project overview through the shared status overview path.

```bash
# Purpose: status overview live --output-format interactive opens the live project overview through the shared status overview path.
grafana-util status overview live --url http://localhost:3000 --basic-user admin --basic-password admin --output-format interactive
```

The TUI uses the following visual language:
- **🟢 Green**: The component is healthy and fully reachable.
- **🟡 Yellow**: The component is functional but has warnings, such as missing metadata.
- **🔴 Red**: The component is blocked and needs action before deployment.

Use `status overview` without `live` for staged artifact review, and use `status live` when you need the same live gate in machine-readable form.

---
[⬅️ Previous: Access Management](access.md) | [🏠 Home](index.md) | [➡️ Next: Operator Scenarios](scenarios.md)
