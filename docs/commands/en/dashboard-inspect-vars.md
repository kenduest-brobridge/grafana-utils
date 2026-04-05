# dashboard inspect-vars

## Purpose
List dashboard templating variables and datasource-like choices from live Grafana.

## When to use
Use this when you need to inspect variable state, feed a screenshot workflow, or debug variable-scoped dashboard URLs.

## Key flags
- `--dashboard-uid` or `--dashboard-url`: choose the dashboard to inspect.
- `--vars-query`: overlay a variable query string such as `var-env=prod&var-host=web01`.
- `--org-id`: scope the inspection to one org.
- `--output-format`: render table, csv, text, json, or yaml.
- `--no-header`: suppress table or CSV headers.
- `--output-file`: write a copy of the output to disk.

## Examples
```bash
# Purpose: List dashboard templating variables and datasource-like choices from live Grafana.
grafana-util dashboard inspect-vars --dashboard-url 'https://grafana.example.com/d/cpu-main/cpu-overview?var-cluster=prod-a' --profile prod --output-format table
```

```bash
# Purpose: List dashboard templating variables and datasource-like choices from live Grafana.
grafana-util dashboard inspect-vars --url https://grafana.example.com --dashboard-uid cpu-main --vars-query 'var-cluster=prod-a&var-instance=node01' --basic-user admin --prompt-password --output-format json
```

## Related commands
- [dashboard screenshot](./dashboard-screenshot.md)
- [dashboard inspect-live](./dashboard-inspect-live.md)
- [dashboard browse](./dashboard-browse.md)
