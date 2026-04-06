# dashboard analyze-export

## Purpose
Analyze dashboard export directories through the canonical `dashboard analyze` command.

## When to use
Use this when you want to read a local export tree, inspect its structure, or render governance and dependency views without contacting Grafana. Prefer `dashboard analyze --input-dir ...` in new docs and scripts.

## Before / After

- **Before**: an export tree is just a folder of JSON files, and you still have to guess which dashboards, variables, or policy checks matter.
- **After**: one analysis pass turns that tree into operator views you can review, hand to CI, or feed into later checks such as `topology` and `governance-gate`.

## Key flags
- `--input-dir`: dashboard export root to analyze.
- `--input-format`: choose `raw` or `provisioning`.
- `--input-type`: select `raw` or `source` when the export root has multiple dashboard variants.
- `--output-format`: render `text`, `table`, `csv`, `json`, `yaml`, `tree`, `tree-table`, `dependency`, `dependency-json`, `governance`, `governance-json`, or `queries-json` views.
- `--interactive`: open the shared analysis workbench.
- `--output-file`: write the result to disk.
- `--no-header`: suppress table-like headers.

## Examples
```bash
# Purpose: Analyze dashboard export directories through the canonical dashboard analyze command.
grafana-util dashboard analyze --input-dir ./dashboards/raw --input-format raw --output-format table
```

```bash
# Purpose: Analyze dashboard export directories through the canonical dashboard analyze command.
grafana-util dashboard analyze --input-dir ./dashboards/provisioning --input-format provisioning --output-format governance-json
```

## What success looks like

- you can explain what is inside an export tree without manually opening dozens of dashboard files
- governance or dependency output is stable enough to hand to CI or a second operator
- later checks such as `dashboard topology`, `dashboard impact`, or `dashboard governance-gate` can start from the inspect artifacts instead of re-reading the raw tree

## Failure checks

- if the export tree looks incomplete, confirm whether you are pointing at `raw` or `provisioning` content before you trust the report
- if a later command cannot consume the result, check whether you emitted `governance-json` or another analysis artifact shape by mistake
- if the tree came from an older export, rerun `dashboard export` first so you do not analyze stale files

## Related commands
- [dashboard export](./dashboard-export.md)
- [dashboard diff](./dashboard-diff.md)
- [dashboard governance-gate](./dashboard-governance-gate.md)
