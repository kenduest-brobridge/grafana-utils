# Datasource Operator Handbook

This guide is for operators who need to inventory Grafana data sources from live Grafana or local bundles, export a masked recovery bundle, replay or diff that bundle, and make controlled live changes with reviewable dry-runs.

## Who It Is For

- Operators responsible for backup, replay, and change control around Grafana data sources.
- Teams moving data source state into Git, provisioning, or recovery bundles.
- Anyone who needs to understand which fields are safe to store and which credentials must stay masked.

## Primary Goals

- Inventory live data sources or local bundles before exporting or mutating them.
- Build a replayable bundle without leaking sensitive values.
- Use dry-runs and diff views before making live changes.

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

## 🔗 Command Pages

Need the command-by-command surface instead of the workflow guide?

- [datasource command overview](../../commands/en/datasource.md)
- [datasource types](../../commands/en/datasource-types.md)
- [datasource browse](../../commands/en/datasource-browse.md)
- [datasource list](../../commands/en/datasource-list.md)
- [datasource export](../../commands/en/datasource-export.md)
- [datasource import](../../commands/en/datasource-import.md)
- [datasource diff](../../commands/en/datasource-diff.md)
- [datasource list](../../commands/en/datasource-list.md)
- [datasource add](../../commands/en/datasource-add.md)
- [datasource modify](../../commands/en/datasource-modify.md)
- [datasource delete](../../commands/en/datasource-delete.md)
- [full command index](../../commands/en/index.md)

---

## 🛠️ What This Area Is For

Use the datasource area when you need to:
- **Inventory**: Audit which datasources exist, their types, and backend URLs from live Grafana or a local bundle.
- **Recovery & Replay**: Maintain a recoverable export of datasource records.
- **Provisioning Projection**: Generate the YAML files required for Grafana's file provisioning.
- **Drift Review**: Compare staged datasource files with live Grafana.
- **Controlled Mutation**: Add, modify, or delete live datasources with dry-run protection.

---

## 🚧 Workflow Boundaries

Datasource export produces two primary artifacts, each with a specific job:

| Artifact | Purpose | Best Use Case |
| :--- | :--- | :--- |
| `datasources.json` | **Masked Recovery** | The canonical replay contract. Used for restores, replays, and drift comparison. |
| `provisioning/datasources.yaml` | **Provisioning Projection** | Mirrors the disk shape Grafana expects for file-based provisioning. |

**Important**: Treat `datasources.json` as the authoritative recovery source. The provisioning YAML is a secondary projection derived from the recovery bundle.

---

## 📋 Reading Live Inventory

Use `datasource list` to verify the current state of your Grafana plugins and targets.

```bash
# Purpose: Use datasource list to verify the current state of your Grafana plugins and targets.
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

## 🚀 Key Commands (Full Argument Reference)

| Command | Full Example with Arguments |
| :--- | :--- |
| **List** | `grafana-util datasource list --all-orgs --table` or `grafana-util datasource list --input-dir ./datasources --table` |
| **Export** | `grafana-util datasource export --output-dir ./datasources --overwrite` |
| **Import** | `grafana-util datasource import --input-dir ./datasources --replace-existing --dry-run --table` |
| **Diff** | `grafana-util datasource diff --input-dir ./datasources` |
| **Add** | `grafana-util datasource add --uid <UID> --name <NAME> --type prometheus --datasource-url <URL> --dry-run --table` |

---

## 🔬 Validated Docker Examples

### 1. Export Inventory
```bash
# Purpose: 1. Export Inventory.
grafana-util datasource export --output-dir ./datasources --overwrite
```
**Output Excerpt:**
```text
Exported datasource inventory -> datasources/datasources.json
Exported metadata            -> datasources/export-metadata.json
Datasource export completed: 3 item(s)
```

### 2. Dry-Run Import Preview
```bash
# Purpose: 2. Dry-Run Import Preview.
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
# Purpose: 3. Direct Live Add (Dry-Run).
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
# Purpose: 4. Local Inventory Review.
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
