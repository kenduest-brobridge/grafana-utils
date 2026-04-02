# Datasource Operator Handbook

This guide covers `grafana-util datasource` as an operator workflow for inventory, recovery, replay, and controlled live mutation. 

> **Goal**: Ensure datasource configuration can be backed up, compared, and replayed safely using a **Masked Recovery** contract that protects sensitive credentials.

---

## 🛠️ What This Area Is For

Use the datasource area when you need to:
- **Inventory**: Audit which datasources exist, their types, and backend URLs.
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
| **List** | `grafana-util datasource list --all-orgs --table` |
| **Export** | `grafana-util datasource export --export-dir ./datasources --overwrite` |
| **Import** | `grafana-util datasource import --import-dir ./datasources --replace-existing --dry-run --table` |
| **Diff** | `grafana-util datasource diff --import-dir ./datasources` |
| **Add** | `grafana-util datasource add --uid <UID> --name <NAME> --type prometheus --datasource-url <URL> --dry-run --table` |

---

## 🔬 Validated Docker Examples

### 1. Export Inventory
```bash
grafana-util datasource export --export-dir ./datasources --overwrite
```
**Output Excerpt:**
```text
Exported datasource inventory -> datasources/datasources.json
Exported metadata            -> datasources/export-metadata.json
Datasource export completed: 3 item(s)
```

### 2. Dry-Run Import Preview
```bash
grafana-util datasource import --import-dir ./datasources --replace-existing --dry-run --table
```
**Output Excerpt:**
```text
UID         NAME               TYPE         ACTION   DESTINATION
prom-main   prometheus-main    prometheus   update   existing
loki-prod   loki-prod          loki         create   missing
```
- **ACTION=create**: New datasource record will be created.
- **ACTION=update**: Existing record will be replaced.

### 3. Direct Live Add (Dry-Run)
```bash
grafana-util datasource add \
  --uid prom-main --name prom-new --type prometheus \
  --datasource-url http://prometheus:9090 --dry-run --table
```
**Output Excerpt:**
```text
INDEX  NAME       TYPE         ACTION  DETAIL
1      prom-new   prometheus   create  would create datasource uid=prom-main
```

---

## ⏭️ Next Steps
- Learn about [**Dashboard Management**](./dashboard.md).
- Explore [**Alerting Workflows**](./alert.md).
