# dashboard raw-to-prompt

## Purpose
Convert ordinary dashboard JSON or `raw/` lane files into Grafana UI prompt JSON with `__inputs`.

## When to use
Use this when someone gives you a normal Grafana dashboard export, legacy raw JSON, or a `raw/` lane file and you need a prompt-safe file for the Grafana UI `Upload JSON` flow.

## Key flags
- `--input-file`: repeat this for one or more dashboard JSON files.
- `--input-dir`: convert a directory tree. For `raw/` or export roots, the default output is a sibling `prompt/` lane.
- `--output-file`: write one explicit output file. Only valid with one `--input-file`.
- `--output-dir`: write converted files under one output tree.
- `--overwrite`: replace existing prompt files.
- `--datasource-map`: optional JSON or YAML map for repairing datasource references.
- `--resolution`: choose `infer-family`, `exact`, or `strict`.
- `--profile`, `--url`, `--token`, `--basic-user`, `--basic-password`, `--org-id`: optional live datasource lookup inputs used to augment datasource resolution.
- `--dry-run`: show what would be converted without writing files.
- `--progress`, `--verbose`: show progress lines or detailed per-file results.
- `--output-format`: render the final summary as `text`, `table`, `json`, or `yaml`.
- `--log-file`, `--log-format`: write per-item success/fail events to a text log or NDJSON log.
- `--color`: colorize summary output with `auto`, `always`, or `never`.

## Workflow notes
- Single-file mode defaults to a sibling `*.prompt.json` output.
- Repeated `--input-file` flags also default to sibling `*.prompt.json` outputs beside each source file.
- Plain directory input requires `--output-dir` so generated prompt files do not mix into arbitrary source trees.
- `raw/` or combined export roots default to a sibling/generated `prompt/` lane and also write `index.json` plus `export-metadata.json`.
- `prompt/` artifacts are for the Grafana UI `Upload JSON` flow. They are not valid input for `grafana-util dashboard import`, which still expects `raw/` or `provisioning/`.
- If you provide `--profile` or other live auth flags, the command queries the target Grafana datasource inventory and prefers those live matches over staged raw inventory.

## Datasource resolution
- `infer-family` is the default practical mode. It can repair unambiguous families such as Prometheus, Loki, or Flux/Influx from query shape.
- `exact` requires an exact datasource match from embedded data, raw inventory, or `--datasource-map`.
- `exact` can also succeed through optional live datasource lookup when you provide `--profile` or direct live auth flags.
- `strict` fails as soon as a datasource cannot be resolved exactly.
- When a dashboard uses multiple distinguishable datasource references, the command keeps multiple prompt slots instead of merging only by family.
- Ambiguous families such as generic SQL/search/tracing still need better source data or an explicit `--datasource-map`.

## Placeholder model
- `$datasource` is a dashboard variable reference. It means the dashboard or panel is selecting a datasource through a Grafana variable named `datasource`.
- `${DS_PROMETHEUS}` or `${DS_*}` is an external-import input placeholder. It means Grafana should ask for a datasource during `Upload JSON` and then inject the selected value.
- These are related but not identical. A generated prompt file can legitimately contain both:
  - `${DS_*}` in `__inputs` and some typed datasource references
  - `$datasource` in panel-level datasource fields that intentionally keep the dashboard-variable flow
- `raw-to-prompt` tries to preserve that distinction instead of flattening everything into one placeholder style.
- If a dashboard historically used Grafana datasource variables, the migrated prompt may still contain `$datasource` alongside `__inputs`.

## Examples
```bash
# Purpose: Convert ordinary dashboard JSON or `raw/` lane files into Grafana UI prompt JSON with `__inputs`.
grafana-util dashboard raw-to-prompt --input-file ./dashboards/raw/cpu-main.json
```

```bash
# Purpose: Convert ordinary dashboard JSON or `raw/` lane files into Grafana UI prompt JSON with `__inputs`.
grafana-util dashboard raw-to-prompt --input-file ./legacy/cpu.json --input-file ./legacy/logs.json --progress
```

```bash
# Purpose: Convert ordinary dashboard JSON or `raw/` lane files into Grafana UI prompt JSON with `__inputs`.
grafana-util dashboard raw-to-prompt --input-dir ./dashboards/raw --overwrite
```

```bash
# Purpose: Convert ordinary dashboard JSON or `raw/` lane files into Grafana UI prompt JSON with `__inputs`.
grafana-util dashboard raw-to-prompt --input-dir ./legacy-json --output-dir ./converted/prompt --output-format table
```

```bash
# Purpose: Convert ordinary dashboard JSON or `raw/` lane files into Grafana UI prompt JSON with `__inputs`.
grafana-util dashboard raw-to-prompt --input-file ./legacy/cpu.json --datasource-map ./datasource-map.yaml --resolution exact --log-file ./raw-to-prompt.log --log-format json
```

```bash
# Purpose: Convert ordinary dashboard JSON or `raw/` lane files into Grafana UI prompt JSON with `__inputs`.
grafana-util dashboard raw-to-prompt --input-file ./legacy/cpu.json --profile prod --org-id 2 --resolution exact
```

## Related commands
- [dashboard export](./dashboard-export.md)
- [dashboard import](./dashboard-import.md)
- [dashboard analyze (local)](./dashboard-analyze-export.md)
- [dashboard diff](./dashboard-diff.md)
