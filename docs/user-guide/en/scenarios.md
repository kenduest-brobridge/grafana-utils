# Operator Scenarios

This chapter translates discrete command families into end-to-end operator workflows. Use this guide when you have a specific problem to solve and need a roadmap rather than a reference page.

---

## 📖 Scenario Map

| Scenario | Goal | Key Surface |
| :--- | :--- | :--- |
| **[1. Environment Verification](#1-environment-verification)** | Proving connectivity before making changes. | `status`, `profile` |
| **[2. Estate-Wide Audit](#2-estate-wide-audit)** | Generating inventory and readiness reports. | `dashboard`, `datasource`, `overview` |
| **[3. Reliable Backups](#3-reliable-backups)** | Exporting assets for version control or DR. | `dashboard export` |
| **[4. Controlled Restore](#4-controlled-restore)** | Replaying exports into live Grafana safely. | `dashboard import` |
| **[5. Alert Governance](#5-alert-governance)** | Managing alerts via the Plan/Apply cycle. | `alert` |
| **[6. Identity Replay](#6-identity-replay)** | Managing Orgs, Users, and Teams. | `access` |
| **[7. Staged Promotion](#7-staged-promotion)** | Handling cross-domain change packages. | `change`, `status` |

---

## 1. Environment Verification

Before any mutation, prove the CLI is pointed at the right target.

**Workflow:**
1. Verify binary version.
2. Initialize or select a Profile.
3. Run a read-only health check.

```bash
grafana-util --version
grafana-util profile list
grafana-util status live --profile prod --output-format table
```

**How to Read the Status:**
- **Overall: status=ready**: Your connection and project health are optimal.
- **Overall: status=blocked**: There are critical errors (blockers) that prevent safe operations.

---

## 2. Estate-Wide Audit

Use this for onboarding, security audits, or pre-change snapshots.

**Workflow:**
1. List all dashboards and their datasource dependencies.
2. Summarize the readiness of the entire project.

```bash
# Inventory all dashboards across all organizations
grafana-util dashboard list --profile prod --all-orgs --with-sources --table

# High-level project snapshot
grafana-util overview live --profile prod
```

---

## 3. Reliable Backups (Dashboard Export)

Export live dashboards into a durable, version-control-friendly tree.

```bash
grafana-util dashboard export \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --export-dir ./backups \
  --overwrite --progress
```

**What to look for:**
- `raw/`: Your primary backup source for later restoration.
- `export-metadata.json`: Summarizes which organizations and dashboards were included.

---

## 4. Controlled Restore (Dashboard Import)

Replay a backup into a live Grafana instance. **Always use Dry-Run.**

```bash
grafana-util dashboard import \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --import-dir ./backups/raw \
  --replace-existing \
  --dry-run \
  --table
```

**How to Read the Import Preview:**
- **ACTION=create**: No existing dashboard with this UID was found; it will be added.
- **ACTION=update**: A dashboard with this UID exists; it will be overwritten.
- **DESTINATION=exists**: Confirms the target UID is already present in Grafana.

---

## 5. Alert Governance (Plan/Apply)

Move alerting changes through a reviewed lifecycle.

**Workflow:**
1. Scaffolding/Editing: `alert add-rule` or manual edits in the desired directory.
2. Review: `alert plan` to generate a delta.
3. Execution: `alert apply` to commit the reviewed changes.

```bash
# Build the change plan
grafana-util alert plan --profile prod --desired-dir ./alerts/desired --prune --output json

# Apply only after review
grafana-util alert apply --profile prod --plan-file ./reviewed-plan.json --approve
```

---

## 6. Identity Replay (Access Management)

Manage users, teams, and service accounts through snapshots.

```bash
# Audit service accounts and their tokens
grafana-util access service-account list --profile prod --table

# Replay user roles and organization memberships
grafana-util access user import --import-dir ./access-users --replace-existing --dry-run
```

---

## 7. Staged Promotion (Change Management)

Handle large, cross-domain change packages (Dashboards + Alerts + Datasources).

**Workflow:**
1. Build staged assets.
2. Run `change summary` for a sanity check.
3. Execute `status staged` for a final readiness gate.

```bash
grafana-util change summary
grafana-util status staged --desired-file ./desired.json --output-format interactive
```

**Why this matters:**
This ensures that the entire "bundle" of changes is consistent and ready before any part of it touches production.

---

## 🔬 Validation Note
All scenarios in this handbook are validated against **Docker Grafana 12.4.1**. Using these patterns ensures consistent, predictable results in production environments.
