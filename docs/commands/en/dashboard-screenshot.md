# dashboard screenshot

## Purpose
Open one dashboard in a headless browser and capture image or PDF output.

## When to use
Use this when you need a reproducible dashboard or panel screenshot, especially for docs, incident notes, or visual debugging.

## Description
This page covers the visual-capture workflow under the `dashboard` namespace. Use it when a text export is not enough and you need a reproducible image or PDF artifact that preserves the rendered Grafana state, variable selection, and panel layout.

It is most useful for operators and responders who need screenshots for runbooks, event timelines, visual verification, or before-and-after evidence during debugging and change review.

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
# Purpose: Open one dashboard in a headless browser and capture image or PDF output.
grafana-util dashboard screenshot --dashboard-url 'https://grafana.example.com/d/cpu-main/cpu-overview?var-cluster=prod-a' --profile prod --output ./cpu-main.png --full-page --header-title --header-url --header-captured-at
```

```bash
# Purpose: Open one dashboard in a headless browser and capture image or PDF output.
grafana-util dashboard screenshot --url https://grafana.example.com --dashboard-uid rYdddlPWk --panel-id 20 --vars-query 'var-datasource=prom-main&var-job=node-exporter&var-node=host01:9100' --basic-user admin --prompt-password --output ./panel.png --header-title 'CPU Busy' --header-text 'Solo panel debug capture'
```

## Related commands
- [dashboard list-vars](./dashboard-list-vars.md)
- [dashboard analyze (live)](./dashboard-analyze-live.md)
- [dashboard topology](./dashboard-topology.md)
