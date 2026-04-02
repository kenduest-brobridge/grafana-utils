# Datasource Handbook

This page covers `grafana-util datasource` as an operator workflow for datasource inventory, recovery, replay, provisioning projection, and controlled live mutation. It is the datasource counterpart to the dashboard handbook: the focus is on how to move and govern datasource state safely, not on authoring individual datasource definitions by hand, and on how to interpret the live inventory output before you make a change.

## What this area is for

Use the datasource area when you need to understand which datasources exist, back them up, recreate them in another environment, compare staged files with live Grafana, or patch a live datasource deliberately.

This area is especially useful when you need:

- a recoverable export of datasource inventory
- a provisioning file for Grafana file provisioning
- a dry-run preview before any live add, modify, delete, or import
- a multi-org replay path with explicit org routing
- a quick inventory of datasource names, types, URLs, and identities

Do not use this area when you are trying to reason about dashboard panels, dashboard folder structure, or query behavior inside a dashboard. Those concerns belong in the dashboard handbook.

It is not the dashboard query analysis surface. For that work, stay in the dashboard chapter and use dashboard inspection commands.

## Workflow Boundaries

Datasource export produces two different artifacts with different jobs.

- `datasources.json` is the canonical masked recovery and replay contract. Use it when you need to restore, replay, or compare datasource inventory.
- `provisioning/datasources.yaml` is a derived provisioning projection. Use it when Grafana should read datasource configuration from a provisioning file.

The projection is intentionally secondary. Treat `datasources.json` as the primary restore source, and treat `provisioning/` as the disk shape Grafana expects for file provisioning.

Exported datasource state is masked on purpose. That means the bundle is suitable for recovery and replay without exposing secrets as plain export data. The provisioning YAML exists to express the runtime configuration Grafana needs, not to replace the recovery bundle.

Org routing matters here. `--org-id` and `--all-orgs` on datasource list and export are Basic-auth-only because the CLI must switch org context through Grafana admin APIs.

## Staged vs Live

Datasource work also splits cleanly into staged and live halves:

- staged work is export, provisioning projection review, diff, and dry-run import review
- live work is list, add, modify, delete, and import into Grafana

Use the staged path when you need to compare or replay. Use the live path when you are ready to change Grafana.

Good operator habits:

- list first to confirm the current datasource inventory
- export before making changes if you need a recovery point
- review both `datasources.json` and the derived provisioning YAML
- diff staged files against live Grafana before importing
- use `--dry-run` on import and add flows before applying changes
- keep `datasources.json` as the authoritative replay source even when provisioning output is present

## Reading Live Inventory

`datasource list` is the first command to run when you want to confirm what Grafana currently has, which plugin family each datasource belongs to, and whether the datasource is the default one for the org.

```bash
grafana-util datasource list \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --table
```

Validated live output excerpt:

```text
UID             NAME        TYPE        URL                     IS_DEFAULT  ORG  ORG_ID
--------------  ----------  ----------  ----------------------  ----------  ---  ------
dehk4kxat5la8b  Prometheus  prometheus  http://prometheus:9090  true             1

Listed 1 data source(s).
```

Read the row from left to right:

- `UID` is the stable automation identity.
- `NAME` is the human-facing label you will usually recognize in Grafana.
- `TYPE` tells you which plugin family or datasource implementation is in play.
- `URL` shows the backend target associated with the datasource.
- `IS_DEFAULT` tells you whether Grafana will use this datasource when a dashboard does not specify one.
- `ORG` and `ORG_ID` tell you which org owns the datasource record.

If the datasource is default but the URL or type does not match what the estate expects, treat that as a configuration problem before exporting or replaying anything.

## Key Commands

The datasource area is smaller than the dashboard area but the same operator logic applies.

| Command | Best use |
| --- | --- |
| `datasource list` | Live inventory of datasources across one org or many orgs |
| `datasource export` | Produce the masked recovery bundle and provisioning projection |
| `datasource import` | Replay inventory or provisioning-derived definitions into live Grafana |
| `datasource diff` | Compare staged datasource files with live Grafana |
| `datasource add` | Create one live datasource directly in Grafana |
| `datasource modify` | Update an existing live datasource directly |
| `datasource delete` | Remove one live datasource deliberately |

For most operators the cycle is:

1. list the current datasources
2. export the inventory to get `datasources.json`
3. inspect the masked recovery bundle and provisioning projection
4. diff the staged files against live Grafana
5. import or mutate live state with a dry-run first

## Docker Grafana Validated Examples

The examples below were exercised against a local Docker Grafana `12.4.1` instance in the live-smoke path used by the main guide.

### Export inventory

```bash
grafana-util datasource export \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --export-dir ./datasources \
  --overwrite
```

Example output excerpt:

```text
Exported datasource inventory -> datasources/datasources.json
Exported metadata            -> datasources/export-metadata.json
Datasource export completed: 3 item(s)
```

The export also writes `provisioning/datasources.yaml` by default unless you explicitly skip the provisioning lane.

### Dry-run import preview

```bash
grafana-util datasource import \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --import-dir ./datasources \
  --replace-existing \
  --dry-run \
  --table
```

Example output excerpt:

```text
UID         NAME               TYPE         ACTION   DESTINATION
prom-main   prometheus-main    prometheus   update   existing
loki-prod   loki-prod          loki         create   missing
```

How to read it:

- `ACTION=create` means Grafana does not have that datasource yet.
- `ACTION=update` means the live datasource would be replaced.
- `DESTINATION=missing` means no live match was found.
- `DESTINATION=existing` means a live datasource was matched before the importer decided whether to update it.

### Direct live add preview

```bash
grafana-util datasource add \
  --url http://localhost:3000 \
  --token <TOKEN> \
  --uid prom-main \
  --name prometheus-main \
  --type prometheus \
  --access proxy \
  --datasource-url http://prometheus:9090 \
  --basic-auth \
  --basic-auth-user metrics-user \
  --basic-auth-password metrics-pass \
  --dry-run \
  --table
```

Example output excerpt:

```text
INDEX  NAME               TYPE         ACTION  DETAIL
1      prometheus-main    prometheus   create  would create datasource uid=prom-main
```

This is the expected shape for a deliberate live mutation preview. Use the same pattern for `modify` and `delete` before removing `--dry-run`.

## Output Excerpts

Datasource output is easiest to interpret if you keep three identities in mind:

- `UID` is the stable automation key.
- `NAME` is the human-facing label.
- `TYPE` must match the plugin or datasource family you intend to manage.

For routed multi-org imports, dry-run output may also report org-level states such as `exists`, `missing-org`, or `would-create-org` before it evaluates individual datasource rows. That distinction matters when you are replaying a combined export root with `--use-export-org`.

The most important contract decision is simple:

- use `datasources.json` for restore and replay
- use `provisioning/datasources.yaml` for Grafana file provisioning

If those two files disagree with your intent, fix the export or the replay target before mutating live Grafana. When you are reading a table, start with the action or default state, then confirm the UID and org context, then verify the URL or folder-like destination that will be affected.
