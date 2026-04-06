# `grafana-util change inspect`

## Purpose

Inspect the staged change package from discovered or explicit inputs.

## When to use

- Start here when you need the shortest path to understand what the staged package contains.
- Use this before `change check` or `change preview` when you want to size the package first.
- Prefer this over lower-level `summary` when the staged inputs may come from mixed workspace artifacts instead of only one `desired.json`.

## Before / After

- **Before**: the change package is still just a set of directories, files, or staged contracts on disk.
- **After**: you have one overview-style document that tells you which sections were discovered and how large the change appears to be.

## Key flags

- `--workspace`: auto-discover common staged inputs from the current repo, export tree, or provisioning tree.
- `--desired-file`: inspect one explicit desired change file.
- `--dashboard-export-dir`, `--dashboard-provisioning-dir`: inspect dashboard staged inputs directly.
- `--alert-export-dir`, `--datasource-provisioning-file`: add alert and datasource staged inputs explicitly.
- `--source-bundle`: inspect an existing source bundle instead of per-surface directories.
- `--output-format`: render as `text` or `json`.
- `--output-file`, `--also-stdout`: persist the rendered output if you want a review artifact.

## Examples

```bash
# Purpose: Inspect the staged package from repo-local inputs.
grafana-util change inspect --workspace .
```

**Expected Output:**
```text
CHANGE PACKAGE SUMMARY:
- dashboards: 5 modified, 2 added
- alerts: 3 modified
- datasources: 1 referenced inventory
- total impact: 11 operations
```

```bash
# Purpose: Inspect the staged package directly from an export tree.
grafana-util change inspect --workspace ./dashboards/raw --output-format json
```

**Expected Output:**
```json
{
  "kind": "grafana-utils-overview",
  "schemaVersion": 1,
  "sections": [
    {
      "title": "Dashboards"
    }
  ],
  "projectStatus": {}
}
```

This confirms that inspect found staged inputs and rendered the shared overview-style document instead of failing discovery.

## What success looks like

- the command tells you what staged surfaces were discovered
- the package size looks plausible before you spend time on preview or apply
- the JSON output is stable enough to hand to another operator or attach to review notes

## Failure checks

- if discovery finds nothing, retry with an explicit input flag before assuming the files are broken
- if the package looks unexpectedly small or large, verify that `--workspace` points at the right export tree or repo root
- if JSON consumers read the result, validate `kind` and `schemaVersion` before deeper parsing

## Related commands

- [change](./change.md)
- [change check](./change-check.md)
- [change preview](./change-preview.md)
- [overview](./overview.md)
