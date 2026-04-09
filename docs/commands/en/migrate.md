# migrate

## Purpose
`grafana-util migrate` is the namespace for repair and normalization workflows that turn one Grafana artifact shape into another safer migration artifact.

## When to use
Use this namespace when the job is not day-to-day live dashboard operations, but preparing exported artifacts for reuse, remapping, or safer re-import into another environment.

## Description
Open this page when the work is fundamentally migration-oriented: repair raw exports, normalize datasource placeholders, or prepare files for a later import/review path. The `migrate` namespace keeps these transforms separate from the live/operator-heavy `dashboard` namespace so dashboard browse, review, publish, and history flows stay focused.

## Workflow lanes

- **Repair exported files**: repair one or more raw dashboard files before reimport or UI upload.
- **Normalize migration artifacts**: turn a raw export tree into a sibling `prompt/` lane.
- **Augment resolution**: optionally use a profile or direct live auth to resolve datasource inventory while repairing files.

## Before / After

- **Before**: a raw export is tied to its original environment and may still carry broken datasource references for the next Grafana.
- **After**: the repaired artifact is explicit, reviewable, and ready for the next import or UI upload step.

## What success looks like

- the command makes it obvious this is a migration step, not a live dashboard mutation
- repaired files can move into the next review or import step without hand-editing JSON
- datasource repair and prompt generation stay available without bloating the main `dashboard` namespace

## Failure checks

- if the next step is API import, confirm you still need `raw/` or `provisioning/` instead of `prompt/`
- if datasource repair is ambiguous, add `--datasource-map` or a live profile before retrying
- if a tree conversion would mix generated files into source material, stop and provide `--output-dir`

## Examples
```bash
# Purpose: Inspect the migrate namespace before choosing a repair path.
grafana-util migrate --help
```

```bash
# Purpose: Repair one raw dashboard file into a prompt-safe migration artifact.
grafana-util migrate dashboard raw-to-prompt --input-file ./legacy/cpu-main.json
```

```bash
# Purpose: Convert one raw export root into a sibling prompt/ lane for later UI upload.
grafana-util migrate dashboard raw-to-prompt --input-dir ./dashboards/raw --output-dir ./dashboards/prompt --overwrite
```

## Related commands

- [migrate dashboard raw-to-prompt](./migrate-dashboard-raw-to-prompt.md)
- [dashboard export](./dashboard-export.md)
- [dashboard import](./dashboard-import.md)
- [dashboard review](./dashboard-review.md)
- [change](./change.md)
