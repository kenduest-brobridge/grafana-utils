# Access Handbook

This page covers Grafana identity and membership management: orgs, users, teams, service accounts, and service-account tokens.

Use the handbook landing page at [index.md](./index.md) when you need the route map for the full guide. Use this page when you need the workflow boundaries for access operations in one place.

## What Access Is For

Use `grafana-util access ...` when the work is about who can sign in, which teams exist, which service accounts exist, or how org membership is represented:

- Inspect users, teams, orgs, and service accounts.
- Create, modify, or delete identities.
- Export, import, or diff snapshots for replay and reconciliation.
- Create and delete service-account tokens.

## What It Is Not For

Do not use the access surface for:

- Dashboard, datasource, or alert migration.
- Project-level readiness or cross-domain staging. Use [change / overview / status](./change-overview-status.md) for that.
- Building alert desired state. Use [alert](./alert.md) for that.

## Command Families

The access surface spans four identity families:

| Family | Commands | Typical use |
| --- | --- | --- |
| Org | `access org list`, `add`, `modify`, `delete`, `export`, `import` | Manage the org itself when the task is org-scoped rather than user-scoped. |
| User | `access user list`, `add`, `modify`, `delete`, `export`, `import`, `diff` | Manage human accounts and their org role state. |
| Team | `access team list`, `add`, `modify`, `delete`, `export`, `import`, `diff` | Manage team membership and admin membership. |
| Service account | `access service-account list`, `add`, `export`, `import`, `diff`, `delete`, `token add`, `token delete` | Manage automation identities and their tokens. |

The current guide treats `access team` as the canonical team path.

## Workflow Boundaries

Keep these rules straight:

- `list` commands are inventory and discovery only.
- `add`, `modify`, and `delete` mutate live Grafana.
- `export`, `import`, and `diff` are snapshot/replay tools.
- `--scope` matters for user workflows. Use it deliberately when you need org-local versus global identity state.
- `--replace-existing` is the replay switch for import flows. Use it when you expect reconciliation rather than create-only behavior.
- `--yes` is the destructive-action acknowledgement flag for delete and removal-heavy import paths.
- Service-account token management lives only under `access service-account token ...`.

## Org Workflows

Use `access org` when the unit of work is the org itself, not the people or automation identities inside it.

That family is for org lifecycle and org snapshot replay. If your task is really about a person, a team, or a service account inside one org, switch to the matching family instead of treating it as an org change.

## User Workflows

Use user commands when the work is about an individual account:

- `access user list` for discovery and audits.
- `access user add` for provisioning a new account.
- `access user modify` for role, email, login, or password updates.
- `access user delete` for account removal.
- `access user export`, `import`, and `diff` for snapshot reconciliation.

Validated output excerpt from the current guide:

```text
ID   LOGIN      EMAIL                NAME             ORG_ROLE   GRAFANA_ADMIN
1    admin      admin@example.com    Grafana Admin    Admin      true
7    svc-ci     ci@example.com       CI Service       Editor     false
9    alice      alice@example.com    Alice Chen       Viewer     false
```

That output is from `access user list --scope global --table`. It is the fastest way to confirm the current identity shape before you start changing anything.

## Team Workflows

Use team commands when the work is about group membership:

- `access team list` for discovery.
- `access team add` and `access team modify` for membership changes.
- `access team delete` for removal.
- `access team export`, `import`, and `diff` for migration and reconciliation.

Validated dry-run excerpt from the current guide:

```text
INDEX  IDENTITY         ACTION       DETAIL
1      platform-team    skip         existing and --replace-existing was not set.
2      sre-team         create       would create team
3      edge-team        add-member   would add team member alice@example.com
4      edge-team        remove-member would remove team member bob@example.com

Import summary: processed=4 created=1 updated=1 skipped=1 source=./access-teams
```

This is the replay review surface. It shows what the import would do before any live change is allowed through.

## Service Account Workflows

Use service-account commands for automation identities and token lifecycle:

- `access service-account list` for inventory.
- `access service-account add` for provisioning.
- `access service-account export`, `import`, and `diff` for snapshot replay.
- `access service-account delete` for removal.
- `access service-account token add` and `token delete` for token lifecycle.

Validated output excerpt from the current guide:

```text
Exported 3 service-account(s) from http://localhost:3000 -> access-service-accounts/service-accounts.json and access-service-accounts/export-metadata.json
```

The current guide also records this live-note boundary:

- This snapshot flow is covered by `make test-access-live` against Grafana `12.4.1`, including export, diff, dry-run import, live replay, delete, and token lifecycle commands.

## Validation Anchor

The access surface in the current guide includes Docker Grafana `12.4.1` live-smoke coverage for export, diff, dry-run import, live replay, delete, and token lifecycle behavior. Use the excerpted commands above as the operator-facing shape, and treat the live note as the validation boundary.

## Where This Fits

- Use [alert](./alert.md) for alert desired state, alert planning, and alert replay.
- Use [change / overview / status](./change-overview-status.md) for project-wide change staging and readiness.
