# Dashboard Operator Handbook

This guide is for operators who need to inventory dashboards, export or import them safely, inspect dependencies, capture screenshots, or review drift before a live change.

## Who It Is For

- SREs and platform engineers responsible for dashboard inventory, promotion, or review.
- Operators who need screenshots, dependency checks, or export trees before a change.
- Teams that want dashboard work to fit into Git, review, and CI flows.

## Primary Goals

- Start from live visibility instead of guessing what exists.
- Understand the export and inspect lanes before replaying files.
- Review drift and dependency shape before import, publish, or delete.

## Before / After

- Before: dashboard work usually started from ad hoc UI clicks, fragile JSON handling, or unclear dependencies.
- After: inventory, inspect, diff, and replay happen in a clearer sequence with reviewable outputs.

## What success looks like

- You know whether the current task belongs in inventory, single-dashboard authoring, export/import replay, inspect, topology, or screenshot.
- You can explain which lane you are using before mutating anything.
- You can prove the dashboard is ready to replay or publish before you touch live state.

## Failure checks

- If the export tree is incomplete, fix the source path before replaying.
- If inspect output shows missing queries or variables, stop and resolve that before import.
- If you cannot explain what a screenshot or topology output is proving, you are probably in the wrong workflow lane.

## Draft authoring workflow

Use the authoring lane when the work starts from one dashboard draft instead of a full export tree.

- Start from a live draft with `dashboard fetch-live` or `dashboard clone-live` when Grafana already has the closest source dashboard.
- Use `dashboard serve` when you want a lightweight local preview browser for one draft file, one draft directory, or one generator script output while you edit.
- Use `dashboard review` before mutation to confirm title, UID, tags, folder UID, and any blocking validation issues.
- Use `dashboard patch-file` when you need to rewrite one local draft in place or write a new patched file.
- Use `dashboard edit-live` when you want to fetch one live dashboard into an external editor, get a review summary with validation blockers, and keep a safe local-draft default instead of immediately mutating Grafana.
- Use `dashboard publish` when the draft is ready to go back through the same import pipeline used by the broader replay path.

Generator-driven teams do not need to stop at an intermediate temp file for every review or publish step.

```bash
# Purpose: Review one generated dashboard from standard input.
jsonnet dashboards/cpu.jsonnet | grafana-util dashboard review --input - --output-format json
```

```bash
# Purpose: Publish one generated dashboard from standard input.
jsonnet dashboards/cpu.jsonnet | grafana-util dashboard publish --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --input - --replace-existing
```

If you are editing one local draft repeatedly, use `dashboard publish --watch` with a file path instead of `--input -`. Watch mode reruns publish or dry-run after each stabilized save and keeps watching even if one iteration fails validation or the API call.

```bash
# Purpose: Re-run dry-run publish after each save while editing one local draft.
grafana-util dashboard publish --url http://localhost:3000 --basic-user admin --basic-password admin --input ./drafts/cpu-main.json --dry-run --watch
```

```bash
# Purpose: Keep one draft open in a lightweight local preview browser while you edit.
grafana-util dashboard serve --input ./drafts/cpu-main.json --port 18080 --open-browser
```

```bash
# Purpose: Fetch one live dashboard into an external editor and keep the result as a local draft by default.
grafana-util dashboard edit-live --profile prod --dashboard-uid cpu-main --output ./drafts/cpu-main.edited.json
```

`dashboard patch-file --input -` requires `--output` because standard input cannot be overwritten in place.
If you target Grafana's built-in General folder, `dashboard publish` normalizes that back to the default root publish path instead of sending a literal `general` folder UID.
`dashboard serve` is intentionally a lightweight preview/document-inspection surface, not a full embedded Grafana renderer.

## History and recovery

When you are trying to recover a known-good dashboard version, use the history lane instead of rebuilding JSON by hand.

- [dashboard history](../../commands/en/dashboard-history.md)
- `dashboard history list` shows the recent revisions for one dashboard UID.
- `dashboard history restore` copies one historical version forward as a new latest Grafana revision entry.
- `dashboard history export` writes a reusable revision-history artifact for review or CI.

Restore is not a destructive overwrite. The selected historical version stays in history, and the restored copy becomes the new current revision.

> **Operator-First Design**: This tool treats dashboards as version-controlled assets. The goal is to move and govern dashboard state safely, with enough visibility to decide whether a file is ready to replay, patch, or promote.

## 🔗 Command Pages

Need the command-by-command surface instead of the workflow guide?

- [dashboard command overview](../../commands/en/dashboard.md)
- [dashboard browse](../../commands/en/dashboard-browse.md)
- [dashboard fetch-live](../../commands/en/dashboard-fetch-live.md)
- [dashboard clone-live](../../commands/en/dashboard-clone-live.md)
- [dashboard list](../../commands/en/dashboard-list.md)
- [dashboard export](../../commands/en/dashboard-export.md)
- [dashboard import](../../commands/en/dashboard-import.md)
- [dashboard raw-to-prompt](../../commands/en/dashboard-raw-to-prompt.md)
- [dashboard patch-file](../../commands/en/dashboard-patch-file.md)
- [dashboard serve](../../commands/en/dashboard-serve.md)
- [dashboard edit-live](../../commands/en/dashboard-edit-live.md)
- [dashboard review](../../commands/en/dashboard-review.md)
- [dashboard publish](../../commands/en/dashboard-publish.md)
- [dashboard delete](../../commands/en/dashboard-delete.md)
- [dashboard diff](../../commands/en/dashboard-diff.md)
- [dashboard analyze (local)](../../commands/en/dashboard-analyze-export.md)
- [dashboard analyze (live)](../../commands/en/dashboard-analyze-live.md)
- [dashboard list-vars](../../commands/en/dashboard-list-vars.md)
- [dashboard history](../../commands/en/dashboard-history.md)
- [dashboard governance-gate](../../commands/en/dashboard-governance-gate.md)
- [dashboard topology](../../commands/en/dashboard-topology.md)
- [dashboard screenshot](../../commands/en/dashboard-screenshot.md)
- [full command index](../../commands/en/index.md)

---

## 🛠️ What This Area Is For

Use the dashboard area for estate-level governance:
- **Inventory**: Understand what exists across one or many organizations.
- **Structured Export**: Move dashboards between environments with dedicated "lanes".
- **Deep Inspection**: Analyze queries and datasource dependencies offline.
- **Screenshots and visual checks**: Produce reproducible dashboard or panel captures for docs, incident notes, and debugging.
- **Drift Review**: Compare staged files against live Grafana before applying.
- **Controlled Mutation**: Import or delete dashboards with mandatory dry-runs.

---

## 🔎 Inspection and screenshot workflows

If your goal is not export or import, but understanding what a dashboard currently looks like, which dependencies it carries, and how variables resolve, start here.

- `dashboard analyze (live)`: analyze one live dashboard's structure, queries, and dependencies.
- `dashboard analyze (local)`: analyze an exported dashboard tree offline.
- `dashboard list-vars`: verify variables, datasource choices, and URL-scoped inputs.
- `dashboard screenshot`: generate a reproducible dashboard or panel capture with a headless browser.
- `dashboard topology`: trace the dashboard's upstream relationships at a glance.

Common cases:

- attaching a screenshot to an incident or runbook
- checking whether one panel resolves the intended variables and datasources
- producing docs or review captures without manual screenshots
- reviewing query/dependency structure before making changes

---

## 🚧 Workflow Boundaries (The Three Lanes)

Dashboard export intentionally produces three different "lanes" because each serves a different operator workflow. **These lanes are not interchangeable.**

| Lane | Purpose | Best Use Case |
| :--- | :--- | :--- |
| `raw/` | **Canonical Replay** | The primary source for `grafana-util dashboard import`. Reversible and API-friendly. |
| `prompt/` | **UI Import** | Compatible with the Grafana UI "Upload JSON" feature. If you only have ordinary or raw dashboard JSON, convert it first with `grafana-util dashboard raw-to-prompt`. |
| `provisioning/` | **File Provisioning** | When Grafana should read dashboards from disk via its internal provisioning system. |

If you add `--include-history` to `dashboard export`, the export tree also gains a `history/` subdirectory for each org scope. In `--all-orgs` mode, that becomes one history tree per exported org root.

Use `dashboard history export` when you need a standalone JSON artifact for one dashboard UID. Use `dashboard export --include-history` when you want history artifacts bundled alongside the export tree.

---

## 🔤 Prompt Placeholder Notes

- `$datasource` is a dashboard variable reference.
- `${DS_*}` is an external-import placeholder created from `__inputs`.
- A prompt dashboard can legitimately contain both forms at once.
- That usually means the dashboard keeps a Grafana datasource-variable workflow while also needing external-import inputs.
- Do not assume `$datasource` automatically means mixed datasource families. In many cases it only means the dashboard is still routing panel selection through one datasource variable.

---

## ⚖️ Staged vs Live: The Operator Logic

- **Staged Work**: Local export trees, validation, offline inspection, and dry-run reviews.
- **Live Work**: Grafana-backed inventory, live diffs, imports, and deletions.

**The Golden Rule**: Start with `list` or `browse` to discover, `export` to a staged tree, `analyze` and `diff` to verify, and only then `import` or `delete` after a matching dry-run.

---

## 📋 Reading Live Inventory

Use `dashboard list` to get a fast picture of the estate.

```bash
# Purpose: Use dashboard list to get a fast picture of the estate.
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
| **Export** | `grafana-util dashboard export --output-dir ./dashboards --overwrite --progress` |
| **Export + History** | `grafana-util dashboard export --output-dir ./dashboards --include-history --overwrite --progress` |
| **Raw to Prompt** | `grafana-util dashboard raw-to-prompt --input-dir ./dashboards/raw --output-dir ./dashboards/prompt --overwrite --progress` |
| **Import** | `grafana-util dashboard import --input-dir ./dashboards/raw --replace-existing --dry-run --table` |
| **Diff** | `grafana-util dashboard diff --input-dir ./dashboards/raw --input-format raw` |
| **Analyze** | `grafana-util dashboard analyze --input-dir ./dashboards/raw --input-format raw --output-format dependency` |
| **Delete** | `grafana-util dashboard delete --uid <UID> --url <URL> --basic-user admin --basic-password admin` |
| **List Vars** | `grafana-util dashboard list-vars --uid <UID> --url <URL> --table` |
| **Patch File** | `grafana-util dashboard patch-file --input <FILE> --name "New Title" --output <FILE>` |
| **Publish** | `grafana-util dashboard publish --input <FILE> --url <URL> --basic-user admin --basic-password admin` |
| **Clone Live** | `grafana-util dashboard clone-live --source-uid <UID> --output <FILE> --url <URL>` |

---

## 🔬 Validated Docker Examples

### 1. Export Progress
Use `--progress` for a clean log during large estate exports.
```bash
# Purpose: Use --progress for a clean log during large estate exports.
grafana-util dashboard export --output-dir ./dashboards --overwrite --progress
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
# Purpose: Always confirm the destination action before mutation.
grafana-util dashboard import --input-dir ./dashboards/raw --dry-run --table
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
- **DESTINATION=missing**: No live dashboard currently owns that UID, so the import would create a new record.
- **DESTINATION=exists**: The UID already exists in Grafana, so the import would target that live dashboard.

### 3. Provisioning-Oriented Comparison
Compare your local provisioning files against live state.
```bash
# Purpose: Compare your local provisioning files against live state.
grafana-util dashboard diff --input-dir ./dashboards/provisioning --input-format provisioning
```
**Output Excerpt:**
```text
--- live/cpu-main
+++ export/cpu-main
-  "title": "CPU Overview"
+  "title": "CPU Overview v2"
```

---
[⬅️ Previous: Architecture & Design](architecture.md) | [🏠 Home](index.md) | [➡️ Next: Datasource Management](datasource.md)
