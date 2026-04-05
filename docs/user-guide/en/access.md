# Access Management (Identity & Access)

Use this chapter when identity and access are the task: org boundaries, users, teams, service accounts, and the tokens that let automation act safely.

## Who It Is For

- Administrators managing org, user, team, or service-account lifecycle work.
- Operators exporting or replaying identity state across environments.
- Teams rotating service-account tokens or auditing access drift.

## Primary Goals

- Clarify which access surface matches the task before running mutations.
- Keep org, user, team, and service-account inventory reviewable.
- Treat token rotation and access replay as controlled workflows instead of one-off edits.

## Before / After

- Before: identity and access work was spread across several command groups with different naming styles.
- After: org, user, team, and service-account workflows share one guide, one glossary, and clearer entry points.

## What success looks like

- You can tell whether the task is org management, user management, team management, or service-account handling.
- You can name the exact command family before you start mutating anything.
- You know when you need a review step before applying a change.

## Failure checks

- If an identity change would affect more orgs than you intended, stop and verify the scope first.
- If you are unsure whether a token or service account can perform the task, check the command page before mutation.
- If the wording still feels split across team/org/user surfaces, return to the glossary and the command reference.

## Command Pages

Need the command-by-command surface instead of the workflow guide?

- [access command overview](../../commands/en/access.md)
- [access user](../../commands/en/access-user.md)
- [access org](../../commands/en/access-org.md)
- [access team](../../commands/en/access-team.md)
- [access service-account](../../commands/en/access-service-account.md)
- [access service-account token](../../commands/en/access-service-account-token.md)
- [full command index](../../commands/en/index.md)

---

## org Management

Use `access org` when you need Basic-auth-backed inventory, export, or replay for orgs, especially when you need to verify which orgs exist before a cross-org change.

### 1. List, Export, and Replay orgs
```bash
# Purpose: 1. List, Export, and Replay orgs.
grafana-util access org list --table
```

```bash
# Purpose: 1. List, Export, and Replay orgs.
grafana-util access org export --export-dir ./access-orgs
```

```bash
# Purpose: 1. List, Export, and Replay orgs.
grafana-util access org import --import-dir ./access-orgs --dry-run
```
**Expected Output:**
```text
ID   NAME        IS_MAIN   QUOTA
1    Main Org    true      -
5    SRE Team    false     10

Exported organization inventory -> access-orgs/orgs.json
Exported organization metadata   -> access-orgs/export-metadata.json

PREFLIGHT IMPORT:
  - would create 0 org(s)
  - would update 1 org(s)
```
Use the list output to confirm the main org, then export/import when you need a repeatable org snapshot.

---

## User and team management

Use `access user` and `access team` for membership changes, snapshots, and drift checks when you need to reconcile who can see or edit what.

### 1. Add, Modify, and Diff Users
```bash
# Add a new user with global admin role
grafana-util access user add --login dev-user --role Admin --prompt-password

# Update an existing user's organization role
grafana-util access user modify --login dev-user --org-id 5 --role Editor

# Compare a saved user snapshot against live Grafana
grafana-util access user diff --diff-dir ./access-users --scope global
```
**Expected Output:**
```text
Created user dev-user -> id=12 orgRole=Editor grafanaAdmin=true

No user differences across 12 user(s).
```
Use `--prompt-password` when you do not want a password in shell history. `--scope global` requires Basic auth.

### 2. Discover and Sync Teams
```bash
# Purpose: 2. Discover and Sync Teams.
grafana-util access team list --org-id 1 --table
```

```bash
# Purpose: 2. Discover and Sync Teams.
grafana-util access team export --export-dir ./access-teams --with-members
```

```bash
# Purpose: 2. Discover and Sync Teams.
grafana-util access team import --import-dir ./access-teams --replace-existing --dry-run --table
```
**Expected Output:**
```text
ID   NAME           MEMBERS   EMAIL
10   Platform SRE   5         sre@company.com

Exported team inventory -> access-teams/teams.json
Exported team metadata   -> access-teams/export-metadata.json

LOGIN       ROLE    ACTION   STATUS
dev-admin   Admin   update   existing
ops-user    Viewer  create   missing
```
Use `--with-members` when the export must preserve membership state, and use `--dry-run --table` before a destructive import.

---

## service account management

Service accounts are the foundation of repeatable automation, CI jobs, and scoped integrations.

### 1. List and Export Service Accounts
```bash
# Purpose: 1. List and Export Service Accounts.
grafana-util access service-account list --json
```

```bash
# Purpose: 1. List and Export Service Accounts.
grafana-util access service-account export --export-dir ./access-sa
```
**Expected Output:**
```text
[
  {
    "id": "15",
    "name": "deploy-bot",
    "role": "Editor",
    "disabled": false,
    "tokens": "1",
    "orgId": "1"
  }
]

Listed 1 service account(s) at http://127.0.0.1:3000

Exported service account inventory -> access-sa/service-accounts.json
Exported service account tokens    -> access-sa/tokens.json
```
`access service-account export` writes both the inventory and the token bundle. Treat `tokens.json` as sensitive.

### 2. Create and Delete Tokens
```bash
# Add a new token to a service account by name
grafana-util access service-account token add --name deploy-bot --token-name nightly --seconds-to-live 3600

# Add a new token by numeric id and capture the one-time secret
grafana-util access service-account token add --service-account-id 15 --token-name ci-deployment-token --json

# Delete an old token after verification
grafana-util access service-account token delete --service-account-id 15 --token-name nightly --yes --json
```
**Expected Output:**
```text
Created service-account token nightly -> serviceAccountId=15

{
  "serviceAccountId": "15",
  "name": "ci-deployment-token",
  "secondsToLive": "3600",
  "key": "eyJ..."
}

{
  "serviceAccountId": "15",
  "tokenId": "42",
  "name": "nightly",
  "message": "Service-account token deleted."
}
```
Use `--json` when you need the one-time `key` field. Plain text is better for logs, not for credential capture.

---

## Drift Detection (Diff)

Compare your local identity snapshots against the live Grafana server.

```bash
# Purpose: Compare your local identity snapshots against the live Grafana server.
grafana-util access user diff --import-dir ./access-users
```

```bash
# Purpose: Compare your local identity snapshots against the live Grafana server.
grafana-util access team diff --diff-dir ./access-teams
```

```bash
# Purpose: Compare your local identity snapshots against the live Grafana server.
grafana-util access service-account diff --diff-dir ./access-sa
```
**Expected Output:**
```text
--- Live Users
+++ Snapshot Users
-  "login": "old-user"
+  "login": "new-user"

No team differences across 4 team(s).
No service account differences across 2 service account(s).
```
Use diff output to decide whether a snapshot is safe to import or whether live Grafana has already drifted.

---
[⬅️ Previous: Alerting Governance](alert.md) | [🏠 Home](index.md) | [➡️ Next: Change & Status](change-overview-status.md)
