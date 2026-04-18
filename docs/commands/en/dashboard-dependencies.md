# dashboard dependencies

## Purpose
Inspect dashboard export directories through the canonical `dashboard dependencies` command.

## When to use
Use this when you want to read a local export tree, inspect its structure, or render governance and dependency views without contacting Grafana. Prefer `dashboard dependencies --input-dir ...` in new docs and scripts.

## Before / After

- **Before**: an export tree is just a folder of JSON files, and you still have to guess which dashboards, variables, or policy checks matter.
- **After**: one dependency pass turns that tree into operator views you can review, hand to CI, or feed into later checks such as `dependencies` and `policy`.

## Key flags
- `--input-dir`: dashboard export root to inspect.
- `--input-format`: choose `raw`, `provisioning`, or `git-sync`.
- `--input-type`: select `raw` or `source` when the export root has multiple dashboard variants.
- `--output-format`: render `text`, `table`, `csv`, `json`, `yaml`, `tree`, `tree-table`, `dependency`, `dependency-json`, `governance`, `governance-json`, or `queries-json` views.
- `--report-columns`: trim table, csv, or tree-table query output to the selected fields. Use `all` for the full query-column set.
- `--list-columns`: print the supported `--report-columns` values and exit.
- `--interactive`: open the shared dependency workbench.
- `--output-file`: write the result to disk.
- `--no-header`: suppress table-like headers.

## Examples
```bash
# Inspect dashboard export directories through the canonical dashboard dependencies command.
grafana-util dashboard dependencies --input-dir ./dashboards/raw --input-format raw --output-format table
```

```bash
# Inspect dashboard export directories through the canonical dashboard dependencies command.
grafana-util dashboard dependencies --input-dir ./dashboards/provisioning --input-format provisioning --output-format governance-json
```

```bash
grafana-util dashboard dependencies --input-dir ./grafana-oac-repo --input-format git-sync --output-format governance
```

## What success looks like

- you can explain what is inside an export tree without manually opening dozens of dashboard files
- governance or dependency output is stable enough to hand to CI or a second operator
- later checks such as `dashboard dependencies`, `dashboard impact`, or `dashboard policy` can start from the review artifacts instead of re-reading the raw tree

## Failure checks

- if the export tree looks incomplete, confirm whether you are pointing at `raw` or `provisioning` content before you trust the report
- if a later command cannot consume the result, check whether you emitted `governance-json` or another review artifact shape by mistake
- if the tree came from an older export, rerun `dashboard export` first so you do not review stale files

## Related commands
- [dashboard export](./dashboard-export.md)
- [dashboard diff](./dashboard-diff.md)
- [dashboard policy](./dashboard-policy.md)
