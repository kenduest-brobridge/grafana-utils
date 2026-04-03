# Alert Operator Handbook

This guide covers `grafana-util alert` as an operator workflow for alert desired-state authoring, review-first mutation, and migration-style replay flows.

> **Operator Principle**: Change alerts deliberately through a **Plan -> Review -> Apply** cycle to prevent accidental mutations in live environments.

## 🔗 Command Pages

Need the command-by-command surface instead of the workflow guide?

- [alert command overview](../../commands/en/alert.md)
- [alert plan](../../commands/en/alert-plan.md)
- [alert apply](../../commands/en/alert-apply.md)
- [alert export](../../commands/en/alert-export.md)
- [alert preview-route](../../commands/en/alert-preview-route.md)
- [full command index](../../commands/en/index.md)

---

## 🛠️ What This Area Is For

Use the alert area when the work is about Grafana alerting resources:
- **Desired State**: Build new alert configurations locally without touching live Grafana.
- **Review delta**: Compare your desired state against the live estate before approving changes.
- **Controlled Apply**: Execute only reviewed plans.
- **Migration & Replay**: Use the legacy `raw/` lane for inventory snapshots and environment-wide moves.

---

## 🚧 Workflow Boundaries (Two Lanes)

Alerting is split into two distinct operational lanes. **Do not mix these lanes.**

| Lane | Purpose | Common Commands |
| :--- | :--- | :--- |
| **Authoring Lane** | Desired-state files for review/apply. | `init`, `add-rule`, `add-contact-point`, `plan`, `apply` |
| **Migration Lane** | Inventory snapshots and raw replay. | `export`, `import`, `diff`, `list-rules` |

---

## 📋 Authoring Desired State

Start by scaffolding a desired-state tree. This creates local files that represent your "intent".

```bash
# Initialize a desired-state tree
grafana-util alert init --desired-dir ./alerts/desired

# Add a rule to your local files (does not touch Grafana yet)
grafana-util alert add-rule \
  --desired-dir ./alerts/desired \
  --name cpu-high --folder platform-alerts \
  --receiver pagerduty-primary --threshold 80 --above --for 5m
```

---

## 🔬 Review and Apply (The Review Cycle)

Use `plan` to build a preview of the delta between your local files and live Grafana.

```bash
# Generate a plan for review
grafana-util alert plan \
  --url http://localhost:3000 \
  --basic-user admin --basic-password admin \
  --desired-dir ./alerts/desired --prune --output json
```

**How to Read the Plan Output:**
- **create**: Desired resource is missing in live Grafana.
- **update**: Live Grafana differs from your desired file.
- **delete**: Triggered by `--prune` when a live resource is not in your files.

**Validated Apply Step:**
Only execute after the plan has been reviewed and saved.
```bash
grafana-util alert apply \
  --plan-file ./alert-plan-reviewed.json \
  --approve --output json
```

---

## 🚀 Key Commands (Full Argument Reference)

| Command | Full Example with Arguments |
| :--- | :--- |
| **List Rules** | `grafana-util alert list-rules --all-orgs --table` |
| **Export** | `grafana-util alert export --export-dir ./alerts --overwrite` |
| **Plan** | `grafana-util alert plan --desired-dir ./alerts/desired --prune --output json` |
| **Apply** | `grafana-util alert apply --plan-file ./plan.json --approve` |
| **Set Route** | `grafana-util alert set-route --desired-dir ./alerts/desired --receiver pagerduty` |
| **New Rule** | `grafana-util alert new-rule --name <NAME> --folder <FOLDER> --output <FILE>` |
| **New Contact** | `grafana-util alert new-contact-point --name <NAME> --type <TYPE> --output <FILE>` |
| **New Template** | `grafana-util alert new-template --name <NAME> --template <CONTENT> --output <FILE>` |

---

## 🔬 Validated Docker Examples

### 1. Alert Plan Excerpt
```bash
grafana-util alert plan --desired-dir ./alerts/desired --prune --output json
```
**Output Excerpt:**
```json
{
  "summary": {
    "create": 1,
    "update": 2,
    "delete": 1,
    "noop": 0,
    "blocked": 0
  }
}
```

### 2. Route Preview
Verify your routing logic locally before applying.
```bash
grafana-util alert preview-route --desired-dir ./alerts/desired --label team=platform --severity critical
```
**Output Excerpt:**
```json
{
  "input": { "labels": { "team": "platform" }, "severity": "critical" },
  "matches": []
}
```
*Note: A blank match list means the contract was evaluated successfully, not necessarily that a live alert exists.*

---
[⬅️ Previous: Datasource Management](datasource.md) | [🏠 Home](index.md) | [➡️ Next: Access Management](access.md)
