# dashboard screenshot

## Purpose
Open one dashboard in a headless browser and capture image or PDF output.

## When to use
Use this when you need a reproducible dashboard or panel screenshot, especially for docs, incident notes, or visual debugging.

## Key flags
- `--dashboard-uid` or `--dashboard-url`: choose the dashboard to capture.
- `--output`: destination file for the capture.
- `--panel-id`: capture only one panel through the solo route.
- `--vars-query` and `--var`: pass variable state into the capture.
- `--full-page` and `--full-page-output`: capture the full scrollable page or tiled output.
- `--header-title`, `--header-url`, `--header-captured-at`, `--header-text`: add PNG or JPEG headers.
- `--theme`: choose the browser theme.
- `--output-format`: force PNG, JPEG, or PDF.
- `--width`, `--height`, `--device-scale-factor`, `--wait-ms`, `--browser-path`: rendering controls.

## Examples
```bash
grafana-util dashboard screenshot --dashboard-url 'https://grafana.example.com/d/cpu-main/cpu-overview?var-cluster=prod-a' --profile prod --output ./cpu-main.png --full-page --header-title --header-url --header-captured-at
grafana-util dashboard screenshot --url https://grafana.example.com --dashboard-uid rYdddlPWk --panel-id 20 --vars-query 'var-datasource=prom-main&var-job=node-exporter&var-node=host01:9100' --basic-user admin --prompt-password --output ./panel.png --header-title 'CPU Busy' --header-text 'Solo panel debug capture'
```

## Related commands
- [dashboard inspect-vars](./dashboard-inspect-vars.md)
- [dashboard inspect-live](./dashboard-inspect-live.md)
- [dashboard topology](./dashboard-topology.md)
