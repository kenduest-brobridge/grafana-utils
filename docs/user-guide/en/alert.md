# Alert Handbook

This page covers the alerting surface only. It is the operator handbook for building alert desired state, reviewing planned alert changes, applying reviewed plans, and handling older migration-style replay flows.

Use the handbook landing page at [index.md](./index.md) when you need the route map for the full guide. Use this page when you need the alert workflow boundaries in one place.

## What Alert Is For

Use `grafana-util alert ...` when the work is about Grafana alerting resources:

- Build new alert desired state.
- Review changes before they touch live Grafana.
- Apply a reviewed alert plan.
- Export, import, diff, or list alerting resources in the older replay lane.

## What It Is Not For

Do not use the alert surface for:

- Dashboard authoring or migration.
- Datasource inventory or replay.
- Access, org, team, or service-account management.
- Project-wide readiness checks. Use [change / overview / status](./change-overview-status.md) for that.

## Command Families

The alert surface has three operator-facing layers:

| Layer | Commands | Purpose |
| --- | --- | --- |
| Authoring | `init`, `add-rule`, `clone-rule`, `add-contact-point`, `set-route`, `preview-route`, `new-rule`, `new-contact-point`, `new-template` | Build or preview desired alert files without using the legacy replay tree. |
| Review and apply | `plan`, `apply`, `delete` | Review desired state against live Grafana, then apply a reviewed plan or preview a delete. |
| Migration | `export`, `import`, `diff`, `list-rules`, `list-contact-points`, `list-mute-timings`, `list-templates` | Inventory and replay the older `raw/` lane. |

## Workflow Boundaries

Keep these lanes separate:

- The authoring commands write or preview desired-state files. They do not mutate live Grafana directly.
- `plan` and `apply` are the normal live-mutation path for the desired-state lane.
- `export`, `import`, and `diff` stay on the older `raw/` replay lane. Do not mix that lane into the desired-state authoring flow.
- `add-rule` is intentionally limited to simple threshold or classic-condition style authoring. For richer rules, clone an existing desired rule and edit it by hand.
- `preview-route` is a contract preview, not a full Grafana routing simulator.
- `set-route` owns one managed route. Re-running it replaces that route instead of merging a new matcher into the old one.
- `delete` is a preview surface. `policy-tree` is special because Grafana treats it as a reset path, not a normal delete.

## Authoring Lane

Start with the desired tree:

```bash
grafana-util alert init --desired-dir ./alerts/desired
```

Then add or clone resources into that tree:

```bash
grafana-util alert add-contact-point --desired-dir ./alerts/desired --name pagerduty-primary
grafana-util alert add-rule --desired-dir ./alerts/desired --name cpu-high --folder platform-alerts --rule-group cpu --receiver pagerduty-primary --label team=platform --severity critical --expr A --threshold 80 --above --for 5m
grafana-util alert preview-route --desired-dir ./alerts/desired --label team=platform --severity critical
```

Validated output excerpt from the current Docker Grafana `12.4.1` fixture:

```json
{
  "input": {
    "labels": {
      "team": "platform"
    },
    "severity": "critical"
  },
  "matches": []
}
```

That empty match list is expected here. It means the preview contract was evaluated, not that a live alert instance was routed end to end.

## Review And Apply

Use `plan` to compare desired alert files against live Grafana:

```bash
grafana-util alert plan --url http://localhost:3000 --basic-user admin --basic-password admin --desired-dir ./alerts/desired --prune --output json
```

Use `apply` only after the plan has been reviewed:

```bash
grafana-util alert apply --url http://localhost:3000 --basic-user admin --basic-password admin --plan-file ./alert-plan-reviewed.json --approve --output json
```

How to read the plan/apply lane:

- `create` means the desired resource is missing in live Grafana.
- `update` means live Grafana differs from the desired document.
- `noop` means the resource already matches.
- `delete` appears only when `--prune` is enabled.
- `blocked` means the plan found a change but refused to treat it as live-safe.

Validated delete-path excerpt from the current Docker Grafana `12.4.1` fixture:

```json
{
  "summary": {
    "blocked": 0,
    "create": 0,
    "delete": 1,
    "noop": 0,
    "processed": 3,
    "update": 2
  }
}
```

```json
{
  "action": "delete",
  "identity": "cpu-high",
  "kind": "grafana-alert-rule",
  "reason": "missing-from-desired-state"
}
```

The same validation still carried two update rows because of live normalization differences. The important signal is the delete row created by `--prune`.

## Migration Lane

Use the migration lane when you are replaying older alert exports, comparing an export against live Grafana, or listing live alerting inventory.

- `export` writes alert resources into the older export layout.
- `import` replays that export layout back into Grafana.
- `diff` compares exported alert state with live Grafana.
- `list-*` commands are inventory tools. They do not author desired state.

## Where This Fits

- Use [access](./access.md) when the task is about Grafana identities, teams, organizations, or service accounts.
- Use [change / overview / status](./change-overview-status.md) when the task is about project-wide staged change, human project review, or readiness gating.

## Validation Anchor

The current guide documents this alert surface as validated locally on March 30, 2026 against Docker Grafana `12.4.1` at `http://127.0.0.1:43111`. The covered flow includes the desired-tree authoring path, `plan`, `apply`, and the prune/delete path.
