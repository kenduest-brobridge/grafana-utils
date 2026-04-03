# Access Management (Identity & Orgs)

Manage the identity layer of your Grafana estate: organizations, users, teams, and service accounts.

## 🔗 Command Pages

Need the command-by-command surface instead of the workflow guide?

- [access command overview](../../commands/en/access.md)
- [access user](../../commands/en/access-user.md)
- [access org](../../commands/en/access-org.md)
- [access team](../../commands/en/access-team.md)
- [access service-account](../../commands/en/access-service-account.md)
- [full command index](../../commands/en/index.md)

---

## 🏢 Organization Management

Use `access org` when you need Basic-auth-backed inventory, export, or replay for organizations.

### 1. List, Export, and Replay Organizations
```bash
grafana-util access org list --table
grafana-util access org export --export-dir ./access-orgs
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

## 👤 User & Team Management

Use `access user` and `access team` for membership changes, snapshots, and drift checks.

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
grafana-util access team list --org-id 1 --table
grafana-util access team export --export-dir ./access-teams --with-members
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

## 🤖 Service Account Management

Service accounts are the foundation of automated pipelines.

### 1. List and Export Service Accounts
```bash
grafana-util access service-account list --json
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

## 🔍 Drift Detection (Diff)

Compare your local identity snapshots against the live Grafana server.

```bash
grafana-util access user diff --import-dir ./access-users
grafana-util access team diff --diff-dir ./access-teams
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
