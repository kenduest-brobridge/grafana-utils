# Alert Operator Handbook

This guide is for operators who need to author alerting changes locally, review the delta before applying it, or replay legacy alert bundles without guessing at the live effect.

This guide covers `grafana-util alert` as an operator workflow for alert desired-state authoring, review-first mutation, and migration-style replay flows.

## Who It Is For

- Operators maintaining Grafana Alerting rules, routes, contact points, and templates.
- Reviewers who need to understand an alert bundle before apply.
- Teams moving alerting state through Git, review, or CI workflows.

## Primary Goals

- Build or import alert desired state without touching live Grafana first.
- Review plan output before an apply path.
- Use replay and migration flows without guessing what live resources will change.

## Before / After

- Before: alert changes often lived in a mixed UI and YAML path with little review context.
- After: authoring, plan output, and apply become separate checkpoints with clearer evidence.

## What success looks like

- You can tell whether you are editing desired state, reviewing a plan, or applying a real change.
- You can explain which part of the alerting chain is affected before touching live state.
- You can read the output and know whether the plan is safe to proceed.

## Failure checks

- If the plan output is missing a contact point or route you expected, stop and verify the staged input first.
- If the apply path would touch more than you intended, treat that as a review failure, not a rendering issue.
- If you cannot explain the alert lane you are in, return to the workflow chapter before mutating anything.

> **Operator Principle**: Change alerts deliberately through a **Plan -> Review -> Apply** cycle to prevent accidental mutations in live environments.

## 🔗 Command Pages

Need the command-by-command surface instead of the workflow guide?

- [advanced alert command overview](../../commands/en/advanced.md)
- [advanced alert migrate export](../../commands/en/advanced.md)
- [advanced alert migrate import](../../commands/en/advanced.md)
- [advanced alert migrate diff](../../commands/en/advanced.md)
- [advanced alert change plan](../../commands/en/advanced.md)
- [advanced alert change apply](../../commands/en/advanced.md)
- [advanced alert live list-rules](../../commands/en/advanced.md)
- [advanced alert live list-contact-points](../../commands/en/advanced.md)
- [advanced alert live list-mute-timings](../../commands/en/advanced.md)
- [advanced alert live list-templates](../../commands/en/advanced.md)
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
  grafana-util advanced alert change plan \
  --url http://localhost:3000 \
  --basic-user admin --basic-password admin \
  --desired-dir ./alerts/desired --prune --output-format json
```

**How to Read the Plan Output:**
- **create**: Desired resource is missing in live Grafana.
- **update**: Live Grafana differs from your desired file.
- **delete**: Triggered by `--prune` when a live resource is not in your files.

**Validated Apply Step:**
Only execute after the plan has been reviewed and saved.
```bash
# Purpose: Only execute after the plan has been reviewed and saved.
  grafana-util advanced alert change apply \
  --plan-file ./alert-plan-reviewed.json \
  --approve --output-format json
```

---

## 🚀 Key Commands (Full Argument Reference)

| Command | Full Example with Arguments |
| :--- | :--- |
| **List Rules** | `grafana-util advanced alert live list-rules --all-orgs --table` |
| **Export** | `grafana-util advanced alert migrate export --output-dir ./alerts --overwrite` |
| **Import** | `grafana-util advanced alert migrate import --input-dir ./alerts/raw --replace-existing --dry-run --json` |
| **Diff** | `grafana-util advanced alert migrate diff --diff-dir ./alerts/raw --output-format json` |
| **Plan** | `grafana-util advanced alert change plan --desired-dir ./alerts/desired --prune --output-format json` |
| **Apply** | `grafana-util advanced alert change apply --plan-file ./plan.json --approve` |
| **Set Route** | `grafana-util advanced alert author route set --desired-dir ./alerts/desired --receiver pagerduty` |
| **Preview Route** | `grafana-util advanced alert author route preview --desired-dir ./alerts/desired --label team=platform --severity critical` |

---

## 🔬 Validated Docker Examples

### 1. Alert Plan Excerpt
```bash
# Purpose: 1. Alert Plan Excerpt.
grafana-util advanced alert change plan --desired-dir ./alerts/desired --prune --output-format json
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
# Purpose: Verify your routing logic locally before applying.
grafana-util advanced alert author route preview --desired-dir ./alerts/desired --label team=platform --severity critical
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
