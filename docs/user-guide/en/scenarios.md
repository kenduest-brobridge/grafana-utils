# Operator Scenarios

This chapter translates discrete command families into end-to-end operator workflows.

For the exact flags behind each workflow, see [dashboard](../../commands/en/dashboard.md), [access](../../commands/en/access.md), [alert](../../commands/en/alert.md), [change](../../commands/en/change.md), [status](../../commands/en/status.md), and [overview](../../commands/en/overview.md).

---

## 1. Environment Verification

Prove connectivity and version alignment before making changes.

```bash
grafana-util profile list
grafana-util status live --profile prod --output table
grafana-util overview live --profile prod --output interactive
```
**Expected Output:**
```text
PROFILES:
  * prod (default) -> http://grafana.internal

OVERALL: status=ready

Project overview
Live status: ready
```
Start with `profile list` to confirm which repo-local defaults are active, then use `status live` for the gate and `overview live --output interactive` when you want the same live surface in a browsable TUI.

---

## 2. Estate-Wide Audit

Inventory all assets across all organizations.

```bash
grafana-util dashboard list --profile prod --all-orgs --with-sources --table
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

```bash
grafana-util dashboard export --export-dir ./backups --overwrite --progress
grafana-util access org export --export-dir ./access-orgs
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

```bash
grafana-util dashboard import --import-dir ./backups/raw --replace-existing --dry-run --table
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
grafana-util change summary --desired-file ./desired.json
grafana-util change preflight --desired-file ./desired.json --output json
grafana-util alert plan --profile prod --desired-dir ./alerts/desired --output json
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
grafana-util access user import --import-dir ./access-users --dry-run --table
grafana-util access service-account token add --service-account-id 15 --token-name nightly --seconds-to-live 3600 --json
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
