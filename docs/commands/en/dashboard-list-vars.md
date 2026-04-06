# dashboard list-vars

## Purpose
List dashboard templating variables and datasource-like choices from live Grafana, a local dashboard file, or a local export tree.

## When to use
Use this when you need to list variable state, feed a screenshot workflow, debug variable-scoped dashboard URLs, or inspect a rendered local dashboard file.

## Key flags
- `--dashboard-uid` or `--dashboard-url`: choose the dashboard to inspect for variable values.
- `--input`: read one local dashboard JSON file instead of calling Grafana.
- `--input-dir`: read a dashboard from a local export tree instead of calling Grafana.
- `--input-format`: interpret `--input-dir` as `raw` or `provisioning`.
- `--vars-query`: overlay a variable query string such as `var-env=prod&var-host=web01`.
- `--org-id`: scope the inspection to one org.
- `--output-format`: render table, csv, text, json, or yaml.
- `--no-header`: suppress table or CSV headers.
- `--output-file`: write a copy of the output to disk.

## Examples
```bash
# Purpose: List dashboard templating variables and datasource-like choices from live Grafana.
grafana-util dashboard list-vars --dashboard-url 'https://grafana.example.com/d/cpu-main/cpu-overview?var-cluster=prod-a' --profile prod --output-format table
```

```bash
# Purpose: List dashboard templating variables and datasource-like choices from live Grafana.
grafana-util dashboard list-vars --url https://grafana.example.com --dashboard-uid cpu-main --vars-query 'var-cluster=prod-a&var-instance=node01' --basic-user admin --prompt-password --output-format json
```

```bash
# Purpose: List dashboard templating variables and datasource-like choices from a local dashboard file.
grafana-util dashboard list-vars --input ./dashboards/raw/cpu-main.json --output-format yaml
```

## Related commands
- [dashboard screenshot](./dashboard-screenshot.md)
- [dashboard analyze (live)](./dashboard-analyze-live.md)
- [dashboard browse](./dashboard-browse.md)
