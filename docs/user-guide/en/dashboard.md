# Dashboard Operator Handbook

This guide covers `grafana-util dashboard` as an operator workflow for inventory, export/import, drift review, and dashboard analysis. 

> **Operator-First Design**: This tool treats dashboards as version-controlled assets. The goal is to move and govern dashboard state safely, providing clear visibility into changes before they touch live Grafana.

---

## 🛠️ What This Area Is For

Use the dashboard area for estate-level governance:
- **Inventory**: Understand what exists across one or many organizations.
- **Structured Export**: Move dashboards between environments with dedicated "lanes".
- **Deep Inspection**: Analyze queries and datasource dependencies offline.
- **Drift Review**: Compare staged files against live Grafana before applying.
- **Controlled Mutation**: Import or delete dashboards with mandatory dry-runs.

---

## 🚧 Workflow Boundaries (The Three Lanes)

Dashboard export intentionally produces three different "lanes" because each serves a different operator workflow. **These lanes are not interchangeable.**

| Lane | Purpose | Best Use Case |
| :--- | :--- | :--- |
| `raw/` | **Canonical Replay** | The primary source for `grafana-util import`. Reversible and API-friendly. |
| `prompt/` | **UI Import** | Compatible with the Grafana UI "Upload JSON" feature. |
| `provisioning/` | **File Provisioning** | When Grafana should read dashboards from disk via its internal provisioning system. |

---

## ⚖️ Staged vs Live: The Operator Logic

- **Staged Work**: Local export trees, validation, offline inspection, and dry-run reviews.
- **Live Work**: Grafana-backed inventory, live diffs, imports, and deletions.

**The Golden Rule**: Start with `list` or `browse` to discover, `export` to a staged tree, `inspect` and `diff` to verify, and only then `import` or `delete` after a matching dry-run.

---

## 📋 Reading Live Inventory

Use `dashboard list` to get a fast picture of the estate.

```bash
grafana-util dashboard list \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --table
```

**Validated Output Excerpt:**
```text
UID                      NAME                                      FOLDER  FOLDER_UID      FOLDER_PATH  ORG        ORG_ID
-----------------------  ----------------------------------------  ------  --------------  -----------  ---------  ------
rYdddlPWl                Node Exporter Full for Host               Demo    ffhrmit0usjk0b  Demo         Main Org.  1
spring-jmx-node-unified  Spring JMX + Node Unified Dashboard (VM)  Demo    ffhrmit0usjk0b  Demo         Main Org.  1
```

**How to Read It:**
- **UID**: Stable identity for automation and deletion.
- **FOLDER_PATH**: Where the dashboard is organized.
- **ORG/ORG_ID**: Confirms which organization owns the object.

---

## 🚀 Key Commands (Full Argument Reference)

| Command | Full Example with Arguments |
| :--- | :--- |
| **List** | `grafana-util dashboard list --all-orgs --with-sources --table` |
| **Export** | `grafana-util dashboard export --export-dir ./dashboards --overwrite --progress` |
| **Import** | `grafana-util dashboard import --import-dir ./dashboards/raw --replace-existing --dry-run --table` |
| **Diff** | `grafana-util dashboard diff --import-dir ./dashboards/raw --input-format raw` |
| **Inspect** | `grafana-util dashboard inspect-export --import-dir ./dashboards/raw --output-format report-table` |
| **Delete** | `grafana-util dashboard delete --uid <UID> --url <URL> --basic-user admin --basic-password admin` |
| **Inspect Vars** | `grafana-util dashboard inspect-vars --uid <UID> --url <URL> --table` |
| **Patch File** | `grafana-util dashboard patch-file --input <FILE> --title "New Title" --output <FILE>` |
| **Publish** | `grafana-util dashboard publish --input <FILE> --url <URL> --basic-user admin --basic-password admin` |
| **Clone Live** | `grafana-util dashboard clone-live --uid <UID> --output <FILE> --url <URL>` |

---

## 🔬 Validated Docker Examples

### 1. Export Progress
Use `--progress` for a clean log during large estate exports.
```bash
grafana-util dashboard export --export-dir ./dashboards --overwrite --progress
```
**Output Excerpt:**
```text
Exporting dashboard 1/7: mixed-query-smoke
Exporting dashboard 2/7: smoke-prom-only
...
Exporting dashboard 7/7: two-prom-query-smoke
```

### 2. Dry-Run Import Preview
Always confirm the destination action before mutation.
```bash
grafana-util dashboard import --import-dir ./dashboards/raw --dry-run --table
```
**Output Excerpt:**
```text
UID                    DESTINATION  ACTION  FOLDER_PATH                    FILE
---------------------  -----------  ------  -----------------------------  --------------------------------------
mixed-query-smoke      exists       update  General                        ./dashboards/raw/Mixed_Query_Dashboard.json
subfolder-chain-smoke  missing      create  Platform / Team / Apps / Prod  ./dashboards/raw/Subfolder_Chain.json
```
- **ACTION=create**: New dashboard will be added.
- **ACTION=update**: Existing live dashboard will be replaced.

### 3. Provisioning-Oriented Comparison
Compare your local provisioning files against live state.
```bash
grafana-util dashboard diff --import-dir ./dashboards/provisioning --input-format provisioning
```
**Output Excerpt:**
```text
--- live/cpu-main
+++ export/cpu-main
-  "title": "CPU Overview"
+  "title": "CPU Overview v2"
```

---

## ⏭️ Next Steps
- Learn about [**Datasource Management**](./datasource.md).
- Explore [**Alerting Workflows**](./alert.md).
