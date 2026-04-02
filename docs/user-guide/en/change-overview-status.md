# Change, Overview, and Status Handbook

This page covers the project-surface commands only: `change`, `overview`, and `status`.

Use the handbook landing page at [index.md](./index.md) when you need the route map for the full guide. Use this page when you need the boundaries between staged change workflows, human project review, and canonical readiness checks.

## What These Surfaces Are For

Use each surface for a different job:

| Surface | Best for | Output style |
| --- | --- | --- |
| `change` | Staged change packaging, review, and apply intent | Text or JSON |
| `overview` | Human-facing project snapshot and operator review | Text, JSON, or interactive |
| `status` | Canonical readiness contract for automation and gating | Text, JSON, or interactive |

## What They Are Not For

Do not use these surfaces for:

- Direct resource CRUD. Use the resource-specific command families instead.
- Alert authoring, alert review, or alert replay. Use [alert](./alert.md).
- Identity or membership management. Use [access](./access.md).
- Dashboard or datasource migration as a replacement for the resource-specific export/import/diff lanes.

## Command Families

### `change`

Use `change` when you want to turn staged inputs into a reviewable project change workflow.

Commands in this family:

- `summary`
- `bundle`
- `bundle-preflight`
- `preflight`
- `assess-alerts`
- `plan`
- `review`
- `apply`

### `overview`

Use `overview` when a human wants one readable project snapshot.

Commands in this family:

- `overview`
- `overview live`

### `status`

Use `status` when another tool or gate needs the canonical readiness contract.

Commands in this family:

- `status staged`
- `status live`

## Workflow Boundaries

Keep these distinctions explicit:

- `change` is the staged workflow. It packages inputs, builds plans, records review, and emits apply intent.
- `change summary` is the lightest read. It summarizes desired resources.
- `change bundle` and `change bundle-preflight` are packaging and preflight steps. They are not project-wide readiness gates by themselves.
- `change plan`, `change review`, and `change apply` form the staged review lifecycle.
- `change assess-alerts` is a focused classifier for alert candidates, plan-only rows, and blocked rows.
- `overview` is the operator view. It is useful for reading the project shape, not for serving as a machine contract.
- `overview live` routes through the shared live status path, but keeps the human-oriented presentation.
- `status` is the canonical contract. Use it for automation, CI, or a readiness gate.
- `status staged` reads staged exports or staged files.
- `status live` reads current Grafana state.

In short:

- Use `change` when the question is "what will change and was it reviewed?"
- Use `overview` when the question is "what does the project look like?"
- Use `status` when the question is "is it ready?"

## Validated Examples

The current guide validates these surfaces against Docker Grafana `12.4.1`. The examples below are the same operator shapes from that validated set.

### `change plan`

```bash
grafana-util change plan --desired-file ./desired-plan.json --live-file ./live.json --output json
```

Validated excerpt:

```json
{
  "kind": "grafana-utils-sync-plan",
  "summary": {
    "would_create": 3,
    "would_update": 0,
    "would_delete": 0,
    "noop": 0
  },
  "reviewRequired": true
}
```

### `change review` and `change apply`

```bash
grafana-util change review --plan-file ./change-plan.json --review-note "docs-reviewed" --reviewed-by docs-user --output json
grafana-util change apply --plan-file ./change-plan-reviewed.json --approve --output json
```

Validated excerpt:

```json
{
  "kind": "grafana-utils-sync-apply-intent",
  "approved": true,
  "reviewed": true,
  "summary": {
    "would_create": 3,
    "would_update": 0,
    "would_delete": 0,
    "noop": 0
  }
}
```

### `overview`

```bash
grafana-util overview \
  --dashboard-export-dir ./dashboards/raw \
  --datasource-export-dir ./datasources \
  --alert-export-dir ./alerts \
  --access-user-export-dir ./access-users \
  --access-team-export-dir ./access-teams \
  --access-org-export-dir ./access-orgs \
  --access-service-account-export-dir ./access-service-accounts \
  --desired-file ./desired.json \
  --output text
```

Validated excerpt:

```text
Project overview
Status: blocked domains=6 present=5 blocked=1 blockers=3 warnings=0 freshness=current oldestAge=222s
Artifacts: 8 total, 1 dashboard export, 1 datasource export, 1 alert export, 1 access user export, 1 access team export, 1 access org export, 1 access service-account export, 1 change summary, 0 bundle preflight, 0 promotion preflight
```

### `status staged`

```bash
grafana-util status staged \
  --dashboard-export-dir ./dashboards/raw \
  --datasource-export-dir ./datasources \
  --alert-export-dir ./alerts \
  --access-user-export-dir ./access-users \
  --access-team-export-dir ./access-teams \
  --access-org-export-dir ./access-orgs \
  --access-service-account-export-dir ./access-service-accounts \
  --desired-file ./desired.json \
  --output json
```

Validated excerpt:

```json
{
  "scope": "staged-only",
  "overall": {
    "status": "blocked",
    "domainCount": 6,
    "blockedCount": 1,
    "blockerCount": 3
  }
}
```

### `status live`

```bash
grafana-util status live --url http://localhost:3000 --basic-user admin --basic-password admin --output json
```

Use this when you need the live readiness contract from current Grafana rather than staged inputs.

## Where This Fits

- Use [alert](./alert.md) when the task is alert authoring or alert replay.
- Use [access](./access.md) when the task is identity or membership management.
