# dashboard convert export-layout

## Purpose
Repair older dashboard export trees whose `raw/` or `prompt/` files were written under the leaf Grafana folder title instead of the full nested folder path.

## When to use
Use this when an existing export already has correct `raw/folders.json` metadata, but files are laid out like `raw/Infra/CPU__uid.json` instead of `raw/Platform/Team/Infra/CPU__uid.json`. This is an offline artifact repair; it does not contact Grafana.

## Before / After
- **Before**: old exports flatten nested Grafana folders that share the same leaf title.
- **After**: repaired `raw/` and `prompt/` lanes mirror the Grafana folder path recorded in `raw/folders.json`.

## Key flags
- `--input-dir`: existing dashboard export root or variant directory.
- `--output-dir`: write a repaired copy without touching the original export.
- `--in-place`: repair the input tree directly.
- `--backup-dir`: required with `--in-place`; stores changed files before moving them.
- `--variant`: repair only `raw` or `prompt`; repeat the flag to select both. Defaults to both.
- `--raw-dir`: raw lane used as metadata source for prompt-only repair.
- `--folders-file`: explicit folder inventory file.
- `--dry-run`: render the move plan without writing files.
- `--overwrite`: allow existing output files to be replaced.
- `--show-operations`: show each `MOVE`, `SAME`, `BLOCKED`, and `EXTRA` operation when using text output.
- `--output-format`: render text, table, csv, json, or yaml.

## Notes
- The command repairs `raw/` and `prompt/` by default.
- `provisioning/` is intentionally not changed.
- `raw/folders.json` is the source of truth for folder paths.
- Prompt repair uses the matching raw dashboard UID to recover folder identity.
- When raw dashboard JSON lacks `meta.folderUid`, repair falls back to the root export index `folderTitle` only when that title is unique for the org in `raw/folders.json`.
- Blocked items are reported instead of guessed when metadata is missing.
- Text output prints only the summary by default. Add `--show-operations` when you need the per-dashboard operation list.
- Table and CSV output also render summary by default; combine them with `--show-operations` for per-dashboard operation rows.
- JSON and YAML output always include the full plan contract with `summary`, `operations`, and `extraFiles`.
- `--dry-run --output-format json` includes `summary.extraFileCount` and `extraFiles` for files under the repaired lanes that are not listed in the export index. Copy-mode preserves those files; in-place repair leaves them untouched.

## Examples
```bash
# Preview old export layout repairs as a table.
grafana-util dashboard convert export-layout --input-dir ./dashboards --output-dir ./dashboards.fixed --dry-run --output-format table
```

```bash
# Preview only the summary.
grafana-util dashboard convert export-layout --input-dir ./dashboards --output-dir ./dashboards.fixed --dry-run
```

```bash
# Preview every per-dashboard operation.
grafana-util dashboard convert export-layout --input-dir ./dashboards --output-dir ./dashboards.fixed --dry-run --show-operations
```

```bash
# Export operation rows as CSV.
grafana-util dashboard convert export-layout --input-dir ./dashboards --output-dir ./dashboards.fixed --dry-run --output-format csv --show-operations
```

```bash
# Write a repaired copy.
grafana-util dashboard convert export-layout --input-dir ./dashboards --output-dir ./dashboards.fixed --overwrite
```

```bash
# Repair in place after backing up changed files.
grafana-util dashboard convert export-layout --input-dir ./dashboards --in-place --backup-dir ./dashboards.layout-backup --overwrite
```

```bash
# Repair a prompt-only lane using a raw lane for metadata.
grafana-util dashboard convert export-layout --input-dir ./dashboards/prompt --raw-dir ./dashboards/raw --output-dir ./dashboards.fixed/prompt --variant prompt --overwrite
```

## Related commands
- [dashboard export](./dashboard-export.md)
- [dashboard convert raw-to-prompt](./dashboard-convert-raw-to-prompt.md)
- [dashboard import](./dashboard-import.md)
