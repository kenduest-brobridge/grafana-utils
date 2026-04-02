# Access Operator Handbook

This guide covers Grafana identity and membership management: Orgs, Users, Teams, Service Accounts, and Tokens.

> **Goal**: Manage who can access your Grafana estate, their organizational roles, and their automation identities with clear inventory and replay capabilities.

---

## 🛠️ What Access Is For

Use `grafana-util access ...` when you need to:
- **Inventory Identities**: Audit users, teams, and service accounts across the estate.
- **Direct Live Mutation**: Create, modify, or delete identities in a specific organization.
- **Snapshot & Replay**: Export identity state into a snapshot for review or cross-environment replay.
- **Token Management**: Lifecycle control for service-account tokens.

---

## 🚧 Workflow Boundaries

| Family | Purpose | Common Operations |
| :--- | :--- | :--- |
| **Org** | Organization lifecycle. | `list`, `add`, `modify`, `export`, `import` |
| **User** | Human accounts. | `list`, `add`, `modify`, `export`, `import`, `diff` |
| **Team** | Membership groups. | `list`, `add`, `modify`, `export`, `import`, `diff` |
| **Service Account** | Automation identities. | `list`, `add`, `token add`, `token delete`, `export`, `import` |

---

## 📋 Reading Live Identity Inventory

Use `access user list` to verify human accounts and their organization roles.

```bash
grafana-util access user list --scope global --table
```

**Validated Output Excerpt:**
```text
ID   LOGIN      EMAIL                NAME             ORG_ROLE   GRAFANA_ADMIN
1    admin      admin@example.com    Grafana Admin    Admin      true
7    svc-ci     ci@example.com       CI Service       Editor     false
```

**How to Read It:**
- **LOGIN**: The unique username for signing in.
- **ORG_ROLE**: The role within the current organization (Admin, Editor, Viewer).
- **GRAFANA_ADMIN**: Indicates if the user has server-wide administrative privileges.

---

## 🚀 Key Commands (Full Argument Reference)

| Command | Full Example with Arguments |
| :--- | :--- |
| **List Users** | `grafana-util access user list --scope global --table` |
| **Add User** | `grafana-util access user add --login dev-user --email dev@example.com --password <PASS>` |
| **Export Teams** | `grafana-util access team export --output-dir ./access/teams --overwrite` |
| **Token Add** | `grafana-util access service-account token add --id <SA_ID> --name ci-token` |
| **Org List** | `grafana-util access org list --all-orgs` |

---

## 🔬 Validated Docker Examples

### 1. Team Import (Dry-Run Replay)
Preview how local team files would affect your live Grafana estate.
```bash
grafana-util access team import --import-dir ./access/teams --replace-existing --dry-run --table
```
**Output Excerpt:**
```text
INDEX  IDENTITY         ACTION       DETAIL
1      platform-team    skip         existing and --replace-existing was not set.
2      sre-team         create       would create team
3      edge-team        add-member   would add team member alice@example.com
```

### 2. Service Account Snapshot
Export service accounts for backup or migration.
```bash
grafana-util access service-account export --output-dir ./access/service-accounts --overwrite
```
**Output Excerpt:**
```text
Exported 3 service-account(s) -> access/service-accounts/service-accounts.json
```

---

## ⚠️ Operator Rules for Access

1.  **Scope Control**: Use `--scope global` for server-wide user audits, or default to the current organization context.
2.  **Destructive Actions**: Commands like `delete` or imports that remove items require the `--yes` acknowledgement flag.
3.  **Token Security**: Tokens are only visible once during creation. `grafana-util` does not store or manage plain-text tokens after they are generated.
4.  **Admin Privileges**: Be cautious when using `access org` or `access user list --all-orgs`, as these require Basic Auth or a server-wide admin token.

---

## ⏭️ Next Steps
- Learn about [**Project Status and Overview**](./change-overview-status.md).
- Follow the [**Scenarios**](./scenarios.md) for end-to-end workflows.
