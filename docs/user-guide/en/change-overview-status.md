# Project Status & Change Overview

This domain focuses on the governance gate: the final layer of validation before and after making changes.

## 🔗 Command Pages

Need the command-by-command surface instead of the workflow guide?

- [change](../../commands/en/change.md)
- [change summary](../../commands/en/change.md#summary)
- [change plan](../../commands/en/change.md#plan)
- [change review](../../commands/en/change.md#review)
- [change apply](../../commands/en/change.md#apply)
- [change audit](../../commands/en/change.md#audit)
- [change preflight](../../commands/en/change.md#preflight)
- [change assess-alerts](../../commands/en/change.md#assess-alerts)
- [change bundle](../../commands/en/change.md#bundle)
- [change bundle-preflight](../../commands/en/change.md#bundle-preflight)
- [change promotion-preflight](../../commands/en/change.md#promotion-preflight)
- [status](../../commands/en/status.md)
- [status staged](../../commands/en/status.md#staged)
- [status live](../../commands/en/status.md#live)
- [overview](../../commands/en/overview.md)
- [overview live](../../commands/en/overview.md#live)
- [snapshot](../../commands/en/snapshot.md)
- [snapshot export](../../commands/en/snapshot.md#export)
- [snapshot review](../../commands/en/snapshot.md#review)
- [profile](../../commands/en/profile.md)
- [profile list](../../commands/en/profile.md#list)
- [profile show](../../commands/en/profile.md#show)
- [profile add](../../commands/en/profile.md#add)
- [profile example](../../commands/en/profile.md#example)
- [profile init](../../commands/en/profile.md#init)
- [full command index](../../commands/en/index.md)

---

## 🚦 Status Surfaces

We distinguish between **Live** (what is actually running) and **Staged** (what you intend to deploy).

### 1. Live Readiness Check
```bash
grafana-util status live --output table
grafana-util status live --profile prod --sync-summary-file ./sync-summary.json --bundle-preflight-file ./bundle-preflight.json --output json
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
grafana-util status staged --desired-file ./desired.json --output json
grafana-util status staged --dashboard-export-dir ./dashboards/raw --alert-export-dir ./alerts --desired-file ./desired.json --output table
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

### 1. Change Summary
Get a high-level summary of your current change package.
```bash
grafana-util change summary --desired-file ./desired.json
grafana-util change summary --desired-file ./desired.json --output json
```
**Expected Output:**
```text
CHANGE PACKAGE SUMMARY:
- dashboards: 5 modified, 2 added
- alerts: 3 modified
- access: 1 added
- total impact: 11 operations
```
Use the summary to size the change before you inspect the plan. If the total is unexpectedly large, stop and review the staged inputs first.

### 2. Preflight Validation
Verify the structural integrity of your export/import trees.
```bash
grafana-util change preflight --desired-file ./desired.json --availability-file ./availability.json
grafana-util change preflight --desired-file ./desired.json --fetch-live --output json
```
**Expected Output:**
```text
PREFLIGHT CHECK:
- dashboards: valid (7 files)
- datasources: valid (1 inventory found)
- result: 0 errors, 0 blockers
```
Use preflight when you need a structural gate before planning or applying. A clean preflight means the inputs are shaped correctly, not that live Grafana already matches them.

---

## 🖥️ Interactive Mode (TUI) Semantics

`overview live --output interactive` opens the live project overview through the shared status live path.

```bash
grafana-util overview live --url http://localhost:3000 --basic-user admin --basic-password admin --output interactive
```

The TUI uses the following visual language:
- **🟢 Green**: The component is healthy and fully reachable.
- **🟡 Yellow**: The component is functional but has warnings, such as missing metadata.
- **🔴 Red**: The component is blocked and needs action before deployment.

Use `overview` without `live` for staged artifact review, and use `status live` when you need the same live gate in machine-readable form.

---
[⬅️ Previous: Access Management](access.md) | [🏠 Home](index.md) | [➡️ Next: Operator Scenarios](scenarios.md)
