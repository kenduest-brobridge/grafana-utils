# Operator Scenarios

This chapter turns command families into end-to-end operator workflows so you can move from one-off commands to a repeatable operating sequence.

## Who It Is For

- Operators who already know the job to do, but want the safest order of operations.
- Teams standardizing common workflows for dashboard, access, alert, or recovery work.
- Reviewers who need an end-to-end checklist instead of isolated command help.

## Primary Goals

- Turn separate command namespaces into complete operational paths.
- Show which validation step should happen before the next mutation.
- Reduce guesswork when moving between live, staged, export, and replay work.

## Before / After

- Before: a reader had to infer the operator story from many separate command pages.
- After: each scenario frames the problem, the lane, and the expected evidence before the command sequence starts.

## What success looks like

- You can pick the scenario that matches the problem you are trying to solve.
- You know whether the next step is inventory, replay, review, or troubleshooting.
- You can explain the expected output before you run the flow.

## Failure checks

- If the scenario starts with the wrong lane, stop and switch to the chapter that matches the actual workflow.
- If the expected output does not line up with the stage you are in, resolve the mismatch before continuing.
- If you still need exact flags more than workflow context, switch to the command reference.

For the exact flags behind each workflow, see [dashboard](../../commands/en/dashboard.md), [access](../../commands/en/access.md), [alert](../../commands/en/alert.md), [change](../../commands/en/change.md), [status](../../commands/en/status.md), and [overview](../../commands/en/overview.md).

---

## 1. Environment Verification

Prove connectivity and version alignment before making changes.

**Before**: You are not sure whether the CLI is pointing at the right Grafana, which credentials are active, or whether the live surface is healthy enough to continue.

**After**: You have one verified live target, one readable readiness result, and one browsable overview before any mutation starts.

```bash
# Purpose: Prove connectivity and version alignment before making changes.
grafana-util profile list
```

```bash
# Purpose: Prove connectivity and version alignment before making changes.
grafana-util status live --profile prod --output-format table
```

```bash
# Purpose: Prove connectivity and version alignment before making changes.
grafana-util overview live --profile prod --output-format interactive
```
**Expected Output:**
```text
PROFILES:
  * prod (default) -> http://grafana.internal

OVERALL: status=ready

Project overview
Live status: ready
```
Start with `profile list` to confirm which repo-local defaults are active, then use `status live` for the gate and `overview live --output-format interactive` when you want the same live surface in a browsable TUI.

---

## 2. Estate-Wide Audit

Inventory all assets across all organizations.

**Before**: You know there are dashboards, orgs, and users in the estate, but you cannot quickly answer what exists or who owns what without jumping between screens.

**After**: You have one inventory surface for dashboards and one for access state, so you can explain the current estate before export, replay, or cleanup work.

```bash
# Purpose: Inventory all assets across all organizations.
grafana-util dashboard list --profile prod --all-orgs --with-sources --table
```

```bash
# Purpose: Inventory all assets across all organizations.
grafana-util access org list --basic-user admin --basic-password admin --with-users --output-format yaml
```
**Expected Output:**
```text
UID        TITLE             FOLDER    ORG   SOURCES
cpu-view   CPU Metrics       Metrics   1     prometheus-main
mem-view   Memory Usage      Metrics   5     loki-prod

id: 1
name: Main Org
users:
  - alice@example.com
```
Use the dashboard and access inventory together when you need to answer what exists before you touch anything.

---

## 3. Reliable Backups (Dashboard Export)

Export live dashboards into a durable tree.

**Before**: A "backup" is either a UI export or a one-off file dump with weak review value.

**After**: You have a reviewable export tree that can feed inspection, replay, migration, and Git-based review.

```bash
# Purpose: Export live dashboards into a durable tree.
grafana-util dashboard export --export-dir ./backups --overwrite --progress
```

```bash
# Purpose: Export live dashboards into a durable tree.
grafana-util access org export --export-dir ./access-orgs
```

```bash
# Purpose: Export live dashboards into a durable tree.
grafana-util access service-account export --export-dir ./access-service-accounts
```
**Expected Output:**
```text
Exporting dashboard 1/32: cpu-metrics
Exporting dashboard 2/32: memory-leak-check
...
Export completed: 32 dashboards saved to ./backups/raw

Exported organization inventory -> access-orgs/orgs.json
Exported service account inventory -> access-service-accounts/service-accounts.json
```
Keep dashboard, org, and service-account exports together when the goal is a reproducible estate snapshot.

---

## 4. Controlled Restore (Dashboard Import)

Replay a backup into a live Grafana instance.

**Before**: Restore means hoping the export is complete and finding out too late whether the import will overwrite something important.

**After**: You preview the replay with dry-run tables first, then decide whether the import is safe enough to continue.

```bash
# Purpose: Replay a backup into a live Grafana instance.
grafana-util dashboard import --import-dir ./backups/raw --replace-existing --dry-run --table
```

```bash
# Purpose: Replay a backup into a live Grafana instance.
grafana-util access team import --import-dir ./access-teams --replace-existing --dry-run --table
```
**Expected Output:**
```text
UID        TITLE          ACTION   DESTINATION
cpu-view   CPU Metrics    update   exists
net-view   Network IO     create   missing

LOGIN       ROLE    ACTION   STATUS
dev-admin   Admin   update   existing
ops-user    Viewer  create   missing
```
Use the dry-run tables to check whether the restore is additive or destructive before you commit to the live import.

---

## 5. Alert Governance (Plan/Apply)

Move alerting changes through a reviewed lifecycle.

```bash
# Purpose: Move alerting changes through a reviewed lifecycle.
grafana-util change summary --desired-file ./desired.json
```

```bash
# Purpose: Move alerting changes through a reviewed lifecycle.
grafana-util change preflight --desired-file ./desired.json --output-format json
```

```bash
# Purpose: Move alerting changes through a reviewed lifecycle.
grafana-util alert plan --profile prod --desired-dir ./alerts/desired --output-format json
```
**Expected Output (Snippet):**
```text
CHANGE PACKAGE SUMMARY:
- dashboards: 5 modified, 2 added
- alerts: 3 modified

PREFLIGHT CHECK:
- dashboards: valid (7 files)
- result: 0 errors, 0 blockers

{
  "summary": { "modified": 2, "added": 1, "deleted": 0 },
  "plan_id": "plan-2026-04-02-abc"
}
```
Run `change summary` first when you want to understand the size of the change, then `change preflight` when you need to confirm the staged inputs are structurally sound before alert-specific planning.

---

## 6. Identity Replay (Access Management)

Manage users, teams, and service accounts through snapshots.

```bash
# Purpose: Manage users, teams, and service accounts through snapshots.
grafana-util access user import --import-dir ./access-users --dry-run --table
```

```bash
# Purpose: Manage users, teams, and service accounts through snapshots.
grafana-util access service-account token add --service-account-id 15 --token-name nightly --seconds-to-live 3600 --json
```

```bash
# Purpose: Manage users, teams, and service accounts through snapshots.
grafana-util access service-account token delete --service-account-id 15 --token-name nightly --yes --json
```
**Expected Output:**
```text
LOGIN       ROLE    ACTION   STATUS
dev-admin   Admin   update   existing
ops-user    Viewer  create   missing

{
  "serviceAccountId": "15",
  "name": "nightly",
  "secondsToLive": "3600",
  "key": "eyJ..."
}
```
This workflow is for replaying identity state safely: use import dry-run for users, and use the service-account token commands when you need to rotate automation credentials without guessing at the target account.

---
[⬅️ Previous: Change & Status](change-overview-status.md) | [🏠 Home](index.md) | [➡️ Next: Technical Reference](reference.md)
