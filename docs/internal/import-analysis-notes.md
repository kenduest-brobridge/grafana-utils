# Import Analysis Notes

Date: 2026-03-14

This note captures the current behavior of dashboard import and alert import/export so later feature work can start from a stable baseline without re-analyzing the codebase.

## Dashboard Import

Current `dashboard import` is a batch import workflow rooted at `--import-dir`.

- CLI flags are limited to:
  - `--import-dir`
  - `--replace-existing`
  - `--update-existing-only`
  - `--import-folder-uid`
  - `--ensure-folders`
  - `--import-message`
  - `--dry-run`
  - `--table`
  - `--json`
  - `--no-header`
  - `--progress`
  - `--verbose`
- There is no built-in selector such as:
  - `--dashboard-uid`
  - `--dashboard-title`
  - `--file`
  - `--folder-map`
  - `--dashboard-uid-map`
  - `--datasource-map`

Relevant code:

- [grafana_utils/dashboard_cli.py](/Users/kendlee/work/grafana-utils/grafana_utils/dashboard_cli.py#L382)
- [grafana_utils/dashboards/import_workflow.py](/Users/kendlee/work/grafana-utils/grafana_utils/dashboards/import_workflow.py#L21)
- [grafana_utils/dashboards/export_inventory.py](/Users/kendlee/work/grafana-utils/grafana_utils/dashboards/export_inventory.py#L10)

### Selection Behavior

`dashboard import` discovers all `.json` files under `--import-dir` recursively, excluding metadata/index files.

This means:

- import scope is controlled only by directory layout
- there is no CLI-level filtering for one dashboard
- if an operator wants to import only one dashboard today, they need a directory containing only that file

### Existing/Update Behavior

Dashboard import decisions are keyed by `dashboard.uid`, not by title.

- default mode: `create-only`
  - missing UID: create
  - existing UID: fail
- `--replace-existing`: `create-or-update`
  - missing UID: create
  - existing UID: update
- `--update-existing-only`: `update-or-skip-missing`
  - missing UID: skip
  - existing UID: update

Relevant code:

- [grafana_utils/dashboards/import_support.py](/Users/kendlee/work/grafana-utils/grafana_utils/dashboards/import_support.py#L177)
- [grafana_utils/dashboards/import_support.py](/Users/kendlee/work/grafana-utils/grafana_utils/dashboards/import_support.py#L191)
- [grafana_utils/dashboards/import_support.py](/Users/kendlee/work/grafana-utils/grafana_utils/dashboards/import_support.py#L230)

### Folder Placement

Current folder behavior is:

- exported `meta.folderUid` is used when available
- `--import-folder-uid` overrides destination folder for all imported dashboards
- when updating an existing dashboard with overwrite behavior, the tool preserves the destination Grafana folder by default unless `--import-folder-uid` is explicitly set

This is destination override behavior, not mapping behavior.

### Dashboard Mapping Status

There is currently no mapping feature in `dashboard import` for:

- source dashboard UID to target dashboard UID
- source datasource UID/name to target datasource UID/name
- source folder UID/path to target folder UID/path

The only related concept is `dashboards/prompt/`, which is for Grafana web UI datasource prompts and is not part of CLI `dashboard import`.

## Alert Export and Import

Current `alert export` writes the tool-owned round-trip format under `alerts/raw/`.

Relevant docs:

- [README.md](/Users/kendlee/work/grafana-utils/README.md#L827)
- [DEVELOPER.md](/Users/kendlee/work/grafana-utils/DEVELOPER.md#L262)
- [DEVELOPER.md](/Users/kendlee/work/grafana-utils/DEVELOPER.md#L294)

### Important Constraint

Grafana official alert provisioning `/export` output is intentionally not the import format for this tool.

The tool supports round-trip import for documents emitted by `grafana-utils alert export`.

This is because:

- Grafana's export representation is provisioning-oriented
- Grafana's create/update APIs expect different request shapes
- this project normalizes to a tool-owned raw format for backup/restore and migration

### Supported Imported Resource Types

Current alert import supports:

- alert rules
- contact points
- mute timings
- notification policies
- notification templates

Relevant API summary:

- [DEVELOPER.md](/Users/kendlee/work/grafana-utils/DEVELOPER.md#L398)

### Selection Behavior

`alert import` also works by scanning every `.json` file under `--import-dir` recursively.

There is no dedicated selector such as:

- `--kind`
- `--uid`
- `--name`
- `--rule-group`

Operators can still do partial import by narrowing the directory passed to `--import-dir`, for example a specific subtree under `alerts/raw/rules/` or `alerts/raw/contact-points/`.

Relevant code:

- [grafana_utils/alert_cli.py](/Users/kendlee/work/grafana-utils/grafana_utils/alert_cli.py#L564)
- [grafana_utils/alert_cli.py](/Users/kendlee/work/grafana-utils/grafana_utils/alert_cli.py#L1036)

## Alert Mapping Support

Unlike dashboard import, alert import already supports explicit linkage mapping:

- `--dashboard-uid-map`
- `--panel-id-map`

Relevant CLI definition:

- [grafana_utils/alert_cli.py](/Users/kendlee/work/grafana-utils/grafana_utils/alert_cli.py#L209)

### Meaning of `--dashboard-uid-map`

`--dashboard-uid-map` maps a source dashboard UID to a target dashboard UID for dashboard-linked alert rules.

Example shape:

```json
{
  "old-dashboard-uid": "new-dashboard-uid"
}
```

This does not change dashboard import behavior. It rewrites alert rule references to dashboards during alert import/diff.

### Meaning of `--panel-id-map`

`--panel-id-map` maps a source panel ID within a source dashboard UID to a target panel ID.

Example shape:

```json
{
  "old-dashboard-uid": {
    "7": "19"
  }
}
```

### Dashboard-Linked Alert Rule Export Behavior

For alert rules linked to dashboards, export behavior preserves the original linkage fields:

- `__dashboardUid__`
- `__panelId__`

The export also stores extra dashboard metadata for import-time repair.

Relevant docs:

- [DEVELOPER.md](/Users/kendlee/work/grafana-utils/DEVELOPER.md#L303)

Implication:

- exported alert raw files keep the source environment's original dashboard UID/panel ID linkage
- if the target environment uses different dashboard UIDs or panel IDs, operators need explicit mapping or fallback matching

### Import-Time Linkage Repair Order

Current documented repair behavior:

1. try the original `__dashboardUid__`
2. if `--dashboard-uid-map` is present, apply that mapping
3. if `--panel-id-map` is present, rewrite `__panelId__`
4. if the target Grafana has the mapped or original dashboard UID, stop there
5. otherwise fall back to exported dashboard metadata
6. search target dashboards by exported title, then narrow by folder title and slug
7. rewrite `__dashboardUid__` only if fallback resolves exactly one dashboard

Relevant docs:

- [DEVELOPER.md](/Users/kendlee/work/grafana-utils/DEVELOPER.md#L319)

Current limitation:

- automatic fallback only rewrites `__dashboardUid__`
- `__panelId__` is preserved unless `--panel-id-map` is provided
- there is no heuristic panel-title-based remap

## Practical Baseline For Future Work

If later work adds import-time selection or mapping, current gaps are:

- dashboard import cannot select one dashboard except by directory scoping
- dashboard import has no mapping support
- alert import cannot select subsets by explicit CLI filters except by directory scoping
- alert import does support dashboard/panel linkage remapping

## Candidate Future Features

Possible follow-up features that fit current architecture:

- dashboard import `--dashboard-uid`
- dashboard import `--file`
- dashboard import `--dashboard-uid-map`
- dashboard import datasource remapping
- alert import `--kind`
- alert import `--uid`
- alert import `--name`

These should be designed to preserve current directory-based batch import as the default workflow.
