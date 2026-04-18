# Datasource Operator Handbook

This guide is for operators who need to inventory Grafana data sources from live Grafana or local bundles, export a masked recovery bundle, replay, diff, or plan that bundle, and make controlled live changes with reviewable dry-runs.

A datasource can look like a simple settings record, but it is the entry point that dashboards, alerts, and queries depend on. Changing a UID, type, URL, or default datasource can move through many dashboards and alert rules. The hard part is that the secrets that make the connection work usually cannot be committed to Git, so backup, replay, and provisioning must not be treated as the same artifact.

Read this chapter by treating datasources as referenced foundation assets. First inspect the live or bundled inventory, then decide whether you need a recovery package, a provisioning projection, a reconcile plan, or a live mutation. After that decision, export/import/diff/plan becomes an operator workflow instead of just moving JSON around.

## Who It Is For

- Operators responsible for backup, replay, and workspace control around Grafana data sources.
- Teams moving data source state into Git, provisioning, or recovery bundles.
- Anyone who needs to understand which fields are safe to store and which credentials must stay masked.

## Primary Goals

- Inventory live data sources or local bundles before exporting or mutating them.
- Build a replayable bundle without leaking sensitive values.
- Use dry-runs and diff views before making live changes.

A good datasource workflow should tell you three things before mutation: who references it, which fields can be stored safely, and which secrets must be recovered through a protected path.

## Before / After

- Before: datasource changes were often treated as one-off config edits with unclear recovery steps.
- After: inventory, masked recovery, provisioning projection, and dry-run checks happen in a repeatable flow.

## What success looks like

- You know which fields belong in the recovery bundle and which ones must stay masked.
- You can validate live inventory or a local export bundle before mutating anything.
- You can explain whether you are working with recovery, provisioning, or direct live mutation.

## Failure checks

- If the replay bundle contains secret values in cleartext, stop and fix the export path before storing it.
- If the import preview does not match the live datasource you expected, check UID and type mapping before applying.
- If the provisioning projection diverges from the recovery bundle, verify which lane you actually need.

> **Goal**: Keep datasource configuration safe to back up, compare, and replay by using a **Masked Recovery** contract that protects sensitive credentials and still leaves enough structure to restore the estate later.

## Datasource Workflow Map

Datasource subcommands differ by whether they read live Grafana, read a local bundle, or prepare a live mutation:

| Job | Start here | Main input | Main output | Next step |
| --- | --- | --- | --- | --- |
| Confirm supported types | `types` | CLI built-in type catalog | Type names and required fields | Choose add / modify fields |
| Inventory live or local state | `list`, `browse` | Live Grafana or `--input-dir` | UID, type, URL, default status | Export, diff, or review |
| Back up and recover | `export`, `import` | Live Grafana or `datasources.json` | Masked recovery bundle / dry-run | Import after review |
| Compare drift | `diff` | Local bundle + live Grafana | Local-vs-live differences | Fix the bundle or import |
| Review reconcile actions | `plan` | Local bundle + live Grafana | Create / update / delete candidates and blockers | Import, re-export, or inspect prune candidates |
| Mutate live state directly | `add`, `modify`, `delete` | Flags + live Grafana | Dry-run or mutation result | Verify with list / diff |
| Generate provisioning projection | `export` | Live Grafana | `provisioning/datasources.yaml` | Hand to Grafana provisioning lane |

`datasources.json` is the source for recovery, diff, and plan. `provisioning/datasources.yaml` is a deployment projection. The first says what can be restored; the second matches Grafana's file provisioning lane. Do not treat the provisioning YAML as the only source of truth.

## What This Area Is For

Use the datasource area when you need to:
- **Inventory**: Audit which datasources exist, their types, and backend URLs from live Grafana or a local bundle.
- **Recovery & Replay**: Maintain a recoverable export of datasource records.
- **Provisioning Projection**: Generate the YAML files required for Grafana's file provisioning.
- **Drift Review**: Compare staged datasource files with live Grafana.
- **Controlled Mutation**: Add, modify, or delete live datasources with dry-run protection.

---

## Workflow Boundaries

Datasource export produces two primary artifacts, each with a specific job:

| Artifact | Purpose | Best Use Case |
| :--- | :--- | :--- |
| `datasources.json` | **Masked Recovery** | The canonical replay contract. Used for restores, replays, and drift comparison. |
| `provisioning/datasources.yaml` | **Provisioning Projection** | Mirrors the disk shape Grafana expects for file-based provisioning. |

**Important**: Treat `datasources.json` as the authoritative recovery source. The provisioning YAML is a secondary projection derived from the recovery bundle.

---

## Inventory: Confirm Type, UID, and Default

Start datasource work with inventory. `datasource types` confirms which types and required fields the CLI understands. `datasource list` shows UID, type, URL, and default status from live Grafana or a local bundle. `datasource browse` is useful when you want to inspect a saved output tree without touching live Grafana.

UID and type are the fields to treat carefully. UID is what dashboards, alert rules, and provisioning refer to as stable identity. Type tells Grafana which plugin handles the record. The default datasource may look like a UI convenience, but many dashboard variables and panel queries can depend on it. If inventory is missing something, check org scope, profile, token permissions, and the local bundle source before adding another datasource.

## Backup and Replay: datasources.json Is The Contract

The main product of `datasource export` is `datasources.json`. It keeps enough structure for diff, dry-run import, and recovery without committing secrets in cleartext. `provisioning/datasources.yaml` is a deployment projection for Grafana's provisioning lane, not the only review source.

Always dry-run before import. The create / update rows in dry-run output describe the real replay impact. A file existing on disk does not mean live Grafana will update the way you expect. If dry-run output shows the wrong UID, name, or type, fix the bundle or mapping before import.

## Diff: Find Drift Before You Decide Who Wins

`datasource diff` answers whether the local bundle and live Grafana still match. If the local bundle is your intended source, diff usually leads to import. If live Grafana has been hotfixed, diff may mean you should re-export or update the review artifact first. Diff is not apply; it only tells you how the two states differ.

Datasource diff is especially useful before dashboard or alert changes. Missing datasource UIDs and default datasource drift often appear only after a dashboard or alert apply fails. Checking datasource drift first catches that environment mismatch earlier.

## Plan: Review Reconcile Actions Before Mutation

`datasource plan` turns local-vs-live differences into action rows. Use it when you need a decision-quality review before deciding whether the bundle should create missing datasources, update existing datasources, leave remote-only datasources alone, or treat remote-only datasources as delete candidates with `--prune`.

Plan is review-only. It does not import, update, or delete. The JSON output keeps stable action IDs and structured hints so CI and future TUI review can consume the same plan model.

## Live Mutation: Treat add / modify / delete As Exceptions

`datasource add`, `modify`, and `delete` touch live Grafana directly. Use them for narrow fixes, break-glass work, or explicit operational changes. The normal path should still be export / diff / import so the change can be reviewed.

If you must mutate live state, start with `--dry-run`, then verify with `list` or `diff`. Before deleting a datasource, confirm that dashboards, alert rules, and provisioning no longer reference that UID. The tool can execute the mutation, but it cannot decide whether every upstream dependency has already moved.

## When To Use Command Reference

This chapter helps you choose the datasource workflow. Once you know which command you need, use the command reference for exact flags, output formats, and complete examples:

- [datasource command overview](../../commands/en/datasource.md)
- [datasource types](../../commands/en/datasource-types.md)
- [datasource browse](../../commands/en/datasource-browse.md)
- [datasource list](../../commands/en/datasource-list.md)
- [export datasource](../../commands/en/export.md)
- [datasource export](../../commands/en/datasource-export.md)
- [datasource import](../../commands/en/datasource-import.md)
- [datasource diff](../../commands/en/datasource-diff.md)
- [datasource plan](../../commands/en/datasource-plan.md)
- [datasource add](../../commands/en/datasource-add.md)
- [datasource modify](../../commands/en/datasource-modify.md)
- [datasource delete](../../commands/en/datasource-delete.md)
- [full command index](../../commands/en/index.md)

---

## Reading Live Inventory

Use `datasource list` to verify the current state of your Grafana plugins and targets.

```bash
# Use datasource list to verify the current state of your Grafana plugins and targets.
grafana-util datasource list \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --table
```

**Validated Output Excerpt:**
```text
UID             NAME        TYPE        URL                     IS_DEFAULT  ORG  ORG_ID
--------------  ----------  ----------  ----------------------  ----------  ---  ------
dehk4kxat5la8b  Prometheus  prometheus  http://prometheus:9090  true             1
```

**How to Read It:**
- **UID**: Stable identity for automation.
- **TYPE**: Identifies the plugin implementation (e.g., prometheus, loki).
- **IS_DEFAULT**: Indicates if this is the default datasource for the organization.
- **URL**: The backend target associated with the record.

---

## Common Commands

| Command | Full Example with Arguments |
| :--- | :--- |
| **List** | `grafana-util datasource list --all-orgs --table` or `grafana-util datasource list --input-dir ./datasources --table` |
| **Export** | `grafana-util datasource export --output-dir ./datasources --overwrite` |
| **Import** | `grafana-util datasource import --input-dir ./datasources --replace-existing --dry-run --table` |
| **Diff** | `grafana-util datasource diff --diff-dir ./datasources` |
| **Plan** | `grafana-util datasource plan --input-dir ./datasources --output-format table` |
| **Add** | `grafana-util datasource add --uid <UID> --name <NAME> --type prometheus --datasource-url <URL> --dry-run --table` |

---

## Operator Examples

### 1. Export Inventory
```bash
# Export datasource inventory and its provisioning projection.
grafana-util export datasource --output-dir ./datasources --overwrite
```
**Output Excerpt:**
```text
Exported datasource inventory -> datasources/datasources.json
Exported metadata            -> datasources/export-metadata.json
Datasource export completed: 3 item(s)
```

### 2. Dry-Run Import Preview
```bash
# Preview whether import would create or update records.
grafana-util datasource import --input-dir ./datasources --replace-existing --dry-run --table
```
**Output Excerpt:**
```text
UID         NAME               TYPE         ACTION   DESTINATION
prom-main   prometheus-main    prometheus   update   existing
loki-prod   loki-prod          loki         create   missing
```
- **ACTION=create**: New datasource record will be created.
- **ACTION=update**: Existing record will be replaced.
- **DESTINATION=missing**: No live datasource currently owns that UID, so the import would create a new record.
- **DESTINATION=existing**: Grafana already has that UID, so the import would replace the current datasource record.

### 3. Direct Live Add (Dry-Run)
```bash
# Dry-run a live add before writing anything to Grafana.
grafana-util datasource add \
  --uid prom-main --name prom-new --type prometheus \
  --datasource-url http://prometheus:9090 --dry-run --table
```
**Output Excerpt:**
```text
INDEX  NAME       TYPE         ACTION  DETAIL
1      prom-new   prometheus   create  would create datasource uid=prom-main
```

### 4. Local Inventory Review
```bash
# Read datasource inventory from the local export bundle.
grafana-util datasource list --input-dir ./datasources --table
```
**Output Excerpt:**
```text
UID             NAME        TYPE        URL                     IS_DEFAULT  ORG  ORG_ID
--------------  ----------  ----------  ----------------------  ----------  ---  ------
dehk4kxat5la8b  Prometheus  prometheus  http://prometheus:9090  true             1
```
- **UID**: Stable identity for automation.
- **TYPE**: Identifies the plugin implementation (e.g., prometheus, loki).
- **IS_DEFAULT**: Indicates if this is the default datasource for the organization.
- **URL**: The backend target associated with the record.

---
[⬅️ Previous: Dashboard Management](dashboard.md) | [🏠 Home](index.md) | [➡️ Next: Alerting Governance](alert.md)
