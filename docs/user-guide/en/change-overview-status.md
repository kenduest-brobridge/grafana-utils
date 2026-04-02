# Change, Overview, and Status Handbook

This guide covers project-surface commands: `change`, `overview`, and `status`.

> **Goal**: Synthesize disparate resources (Dashboards, Alerts, Access) into a single, unified project view for lifecycle management and readiness reporting.

---

## 🛠️ What These Surfaces Are For

Different tasks require different analytical surfaces.

| Surface | Best For | Contract |
| :--- | :--- | :--- |
| **`change`** | Staged workflows and apply intent. | Staged Review Cycle. |
| **`overview`** | Human project review. | Human Snapshot. |
| **`status`** | Readiness gating and automation. | Canonical Readiness Contract. |

---

## 🚧 Workflow Boundaries (The Review Cycle)

Use `change` commands in order when you need a controlled, reviewed-first path for staged assets.

1. **`plan`**: Generate a reviewable delta between local files and live state.
2. **`review`**: Record the operator's decision (Approve/Reject).
3. **`apply`**: Emit the final apply intent after approval.

---

## 📋 Reading Project Status

Use `status live` to verify the health and readiness of your live Grafana estate.

```bash
grafana-util status live --url http://localhost:3000 --basic-user admin --basic-password admin --table
```

**Validated Output Excerpt:**
```text
Project status
Overall: status=partial scope=live domains=6 present=6 blocked=0 blockers=0 warnings=4
Domains:
- dashboard status=ready mode=live-read primary=3 blockers=0 warnings=0
- datasource status=ready mode=live-inventory primary=1 blockers=0 warnings=1
- alert status=ready mode=live-alert-surfaces primary=2 blockers=0 warnings=0
```

**How to Read It:**
- **Overall status**: `ready` (good), `partial` (warnings exist), or `blocked` (errors found).
- **Domains**: Readiness report for each family (Dashboard, Datasource, Alert, etc.).
- **Blockers**: Specific items that must be resolved before the project is considered "ready".

---

## 🚀 Key Commands (Full Argument Reference)

| Command | Full Example with Arguments |
| :--- | :--- |
| **Live Status** | `grafana-util status live --url <URL> --basic-user admin --table` |
| **Staged Status** | `grafana-util status staged --dashboard-export-dir ./dashboards --output json` |
| **Overview** | `grafana-util overview --dashboard-export-dir ./dashboards --output interactive` |
| **Change Plan** | `grafana-util change plan --desired-file <FILE> --live-file <FILE> --output json` |
| **Review** | `grafana-util change review --plan-file <FILE> --reviewed-by admin --approve` |

---

## 🔬 Validated Docker Examples

### 1. Change Plan Excerpt
Preview the intent of a staged change package.
```bash
grafana-util change plan --desired-file ./desired.json --live-file ./live.json --output json
```
**Output Excerpt:**
```json
{
  "summary": { "would_create": 3, "would_update": 0, "would_delete": 0, "noop": 0 },
  "reviewRequired": true
}
```

### 2. Staged Status Contract
Use this in CI to gate deployments based on local file readiness.
```bash
grafana-util status staged --desired-file ./desired.json --output json
```
**Output Excerpt:**
```json
{
  "overall": { "status": "blocked", "domainCount": 6, "blockedCount": 1, "blockerCount": 3 }
}
```
*Note: A 'blocked' status means the local files do not yet meet the project's readiness criteria.*

---

## ⚠️ Operator Rules for Project Surfaces

1.  **Differentiate Surfaces**: Use `overview` for human reviews and `status` for machine contracts or CI gates.
2.  **Staged vs Live**: `status staged` reads local files; `status live` reads Grafana. Do not assume one represents the other.
3.  **Review Chain**: `change review` is mandatory for tracking who approved a change package before it reaches production.
4.  **TUI Navigation**: For complex estate reviews, use `overview --output interactive` to drill down into specific blockers or warnings.

---

## ⏭️ Next Steps
- Review the [**Dashboard**](./dashboard.md) or [**Alert**](./alert.md) handbooks for domain-specific details.
- See the [**Scenarios**](./scenarios.md) for end-to-end examples.
