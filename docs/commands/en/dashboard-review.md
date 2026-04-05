# dashboard review

## Purpose
Review one local dashboard JSON file without touching Grafana.

## When to use
Use this when you want a local read-only check of a dashboard draft before publishing or importing it.

## Key flags
- `--input`: dashboard JSON file to review.
- `--output-format`: choose `text`, `table`, `csv`, `json`, or `yaml`.
- `--json`, `--table`, `--csv`, `--yaml`: direct output selectors.

## Examples
```bash
# Purpose: Review one local dashboard JSON file without touching Grafana.
grafana-util dashboard review --input ./drafts/cpu-main.json
```

```bash
# Purpose: Review one local dashboard JSON file without touching Grafana.
grafana-util dashboard review --input ./drafts/cpu-main.json --output-format yaml
```

## Related commands
- [dashboard patch-file](./dashboard-patch-file.md)
- [dashboard publish](./dashboard-publish.md)
- [dashboard inspect-export](./dashboard-inspect-export.md)

