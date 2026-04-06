# dashboard impact

## Purpose
Estimate the blast radius of one datasource directly from live Grafana, a local export tree, or reusable dashboard governance artifacts.

## When to use
Use this when you are about to change, migrate, or troubleshoot one datasource and need to know which dashboards and alert resources depend on it before you touch the live system. Prefer direct live or local inputs for the common path; keep governance artifacts for reuse.

## Before / After

- **Before**: datasource risk is inferred from memory, naming conventions, or manual Grafana searches.
- **After**: one `impact` run tells you which dashboards and alert-linked resources are downstream of a datasource UID.

## Key flags
- `--url`: analyze live Grafana directly.
- `--import-dir`: analyze a local export tree directly.
- `--input-format`: choose `raw` or `provisioning` when analyzing local exports.
- `--governance`: dashboard governance JSON input (`governance-json` artifact).
- `--datasource-uid`: datasource UID to trace.
- `--alert-contract`: optional alert contract JSON input.
- `--output-format`: render `text` or `json`.
- `--interactive`: open the interactive terminal browser.

## Examples
```bash
# Purpose: Estimate the blast radius of one datasource directly from live Grafana.
grafana-util dashboard impact \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --datasource-uid prom-main \
  --output-format text
```

```bash
# Purpose: Estimate the blast radius of one datasource from a local export tree.
grafana-util dashboard impact \
  --import-dir ./dashboards/raw \
  --input-format raw \
  --datasource-uid prom-main \
  --output-format json
```

```bash
# Purpose: Estimate the blast radius of one datasource from reusable dashboard governance artifacts.
grafana-util dashboard impact \
  --governance ./governance.json \
  --datasource-uid prom-main \
  --alert-contract ./alert-contract.json \
  --output-format json
```

## What success looks like

- you can name the dashboards affected by one datasource change before you modify the datasource itself
- alert resources appear in the same report when alert contract data is available
- the result is concrete enough to use in a review ticket, migration plan, or incident response handoff

## Failure checks

- if the result is empty, confirm the datasource UID matches the governance artifact instead of the display name you remember from Grafana
- if alert-linked resources are missing, check whether you supplied `--alert-contract`
- if the JSON is going into CI or an external tool, validate the top-level shape before you assume a zero-impact result is real

## Related commands
- [dashboard analyze (local)](./dashboard-analyze-export.md)
- [dashboard topology](./dashboard-topology.md)
- [dashboard governance-gate](./dashboard-governance-gate.md)
