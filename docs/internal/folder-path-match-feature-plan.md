# Folder Path Match Import Feature Plan

Date: 2026-03-14

This file is a non-invasive implementation plan for a proposed dashboard import guard:

- `--require-matching-folder-path`

The plan is intentionally recorded in a new file first so active in-flight edits to existing implementation files are not disturbed.

## Goal

Add an optional dashboard import safeguard that only allows updating an existing dashboard when:

- the source raw dashboard folder path
- and the destination Grafana dashboard folder path

match exactly.

If the flag is not set, dashboard import keeps the current behavior unchanged.

## Intended Operator Semantics

Without `--require-matching-folder-path`:

- current import behavior remains unchanged
- create/update decisions are still keyed by dashboard `uid`
- folder path mismatch does not block import

With `--require-matching-folder-path`:

- missing destination dashboard:
  - `create-only` or `create-or-update`: still allow create
  - `update-existing-only`: still follow current skip-missing behavior
- existing destination dashboard:
  - compare source folder full path vs destination folder full path
  - if equal: allow update
  - if different: skip import for that dashboard

## Why Path Match Instead Of UID Match

The requirement is about folder path identity, not raw folder UID equality.

Reasons:

- source and destination folder UIDs may differ across Grafana environments
- a full path comparison better represents human intent
- matching only the last folder title is ambiguous in nested structures

Example:

- `Platform / Infra`
- `Legacy / Infra`

These should not be treated as equivalent.

## Scope Rules

The first implementation should:

- compare complete folder paths
- treat `General` as a normal valid path
- only gate updates to existing dashboards
- not block creates for missing destination dashboards
- not introduce any mapping behavior

## Proposed CLI Contract

New flag:

- `--require-matching-folder-path`

Suggested help text:

`Only update an existing dashboard when the source raw folder path matches the destination Grafana folder path exactly. Missing dashboards still follow the active create/skip mode.`

## Flag Interactions

Recommended constraints:

- compatible with:
  - default create-only mode
  - `--replace-existing`
  - `--update-existing-only`
  - `--dry-run`
  - `--table`
  - `--json`
  - `--progress`
  - `--verbose`
- reject when combined with:
  - `--import-folder-uid`

Reason:

- `--import-folder-uid` forces a destination
- `--require-matching-folder-path` validates the current destination path
- combining both creates conflicting operator intent

## Proposed Dry-Run Action Extension

Current action labels:

- `would-create`
- `would-update`
- `would-fail-existing`
- `would-skip-missing`

Proposed new action:

- `would-skip-folder-mismatch`

Recommended visible fields in dry-run table/json:

- `uid`
- `destination`
- `action`
- `folder_path`
- `source_folder_path`
- `destination_folder_path`
- `file`

If keeping output minimal for the first pass, at minimum include:

- action
- source folder path
- destination folder path

## Proposed Live Import Behavior

When the flag is active and an existing dashboard's folder path does not match:

- do not call the dashboard import API
- record the dashboard as skipped
- surface the reason in verbose/progress/dry-run output
- include mismatch skip counts in final summary

Suggested live summary shape:

- `Imported X dashboard files from ...; skipped Y missing dashboards; skipped Z folder-mismatched dashboards`

## Source Path Resolution

Source folder path should use the same semantics already used by current import rendering:

1. prefer exported folder inventory when `meta.folderUid` maps cleanly through `raw/folders.json`
2. fall back to the dashboard file's relative directory under `--import-dir`
3. treat the built-in General folder consistently

This keeps behavior aligned with existing import/dry-run path reporting.

## Destination Path Resolution

Destination folder path should be resolved only for existing dashboards.

Suggested resolution order:

1. fetch existing dashboard by `uid`
2. inspect destination dashboard metadata for current folder UID
3. resolve that folder UID to a full path using live folder inventory helpers

If the live folder cannot be resolved cleanly:

- prefer a conservative result
- either treat the destination path as unknown and skip
- or surface a clear error if the implementation cannot make a reliable decision

For the first implementation, conservative skip is safer than blind update.

## Edge Cases

### Missing UID

If a source dashboard has no `uid`:

- preserve current behavior
- the matching-folder-path guard should not fabricate a destination comparison

### Missing Source Folder Metadata

If source folder path cannot be resolved:

- do not silently assume a match
- prefer treating the path as unknown

For first implementation, recommended behavior:

- existing destination dashboard + unknown source path + flag enabled => skip with mismatch/unknown-path style reason

### Existing Dashboard In General

If destination folder is Grafana's built-in `General`:

- compare against source path `General`
- exact match should pass

## Suggested Internal Shape

Possible helper result structure:

```python
{
    "matches": True,
    "source_folder_path": "Platform / Infra",
    "destination_folder_path": "Platform / Infra",
    "reason": "",
}
```
```

Or for mismatch:

```python
{
    "matches": False,
    "source_folder_path": "Platform / Infra",
    "destination_folder_path": "Legacy / Infra",
    "reason": "folder-path-mismatch",
}
```
```

## Suggested Integration Points

Expected existing-file changes later, once approved:

- `grafana_utils/dashboard_cli.py`
  - add the new CLI flag
- `grafana_utils/dashboards/import_support.py`
  - add folder-path guard helpers
  - extend action labeling
  - extend dry-run record structure if needed
- `grafana_utils/dashboards/import_workflow.py`
  - apply the guard before live update calls
  - apply the guard during dry-run prediction
  - track mismatch skip counts
- `grafana_utils/dashboards/folder_support.py`
  - add helper(s) if needed for stable source/destination full-path resolution
- `tests/test_python_dashboard_cli.py`
  - parser/help coverage
  - dry-run coverage
  - live import skip coverage
- Rust parity files if the repo wants Python/Rust CLI behavior to stay aligned

## Test Matrix

Minimum cases:

1. existing dashboard, same full path => update allowed
2. existing dashboard, different full path => `would-skip-folder-mismatch`
3. missing dashboard => create still allowed
4. `--update-existing-only` plus mismatch => skipped
5. live import mismatch => import API not called
6. `--import-folder-uid` plus new flag => explicit validation error
7. dry-run table/json include mismatch action and path details
8. `General` vs `General` => allowed
9. nested path mismatch with same leaf folder name => skipped

## Recommendation

Implement the first version as:

- one opt-in flag
- mismatch behavior is skip, not fail-fast
- exact full-path comparison
- conflict with `--import-folder-uid`

This keeps the feature safe, predictable, and easy to explain while preserving current default behavior completely.
