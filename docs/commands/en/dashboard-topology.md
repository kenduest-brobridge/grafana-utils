# dashboard topology

## Purpose
Build a deterministic dashboard topology graph directly from live Grafana or a local export tree, with saved analysis artifacts available for advanced reuse.

## When to use
Use this when you need a graph view of dashboards, folders, variables, datasource links, and optional alert contract data. Prefer `--url` or `--input-dir` for the common path; keep artifact inputs for advanced reuse or CI. The command also accepts the `graph` alias.

## Before / After

- **Before**: dependency knowledge lives in operator memory, raw JSON, or one-off diagrams that drift out of date.
- **After**: one topology run gives you a reproducible graph you can inspect in text form, hand to Mermaid, or ship to Graphviz.

## Key flags
- `--url`: analyze live Grafana directly.
- `--input-dir`: analyze a local export tree directly.
- `--input-format`: choose `raw` or `provisioning` when analyzing local exports.
- `--governance`: dashboard governance JSON input (`governance-json` artifact, advanced reuse).
- `--queries`: optional dashboard query-report JSON input (`queries-json` artifact, advanced reuse).
- `--alert-contract`: optional alert contract JSON input.
- `--output-format`: render `text`, `json`, `mermaid`, or `dot`.
- `--output-file`: write the rendered topology to disk.
- `--interactive`: open the interactive terminal browser.

## Examples
```bash
# Purpose: Build a deterministic dashboard topology graph directly from live Grafana.
grafana-util dashboard topology \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --output-format mermaid
```

```bash
# Purpose: Build a deterministic dashboard topology graph from a local export tree.
grafana-util dashboard graph \
  --input-dir ./dashboards/raw \
  --input-format raw \
  --output-format dot \
  --output-file ./dashboard-topology.dot
```

```bash
# Purpose: Advanced reuse: build a deterministic dashboard topology graph from saved artifacts.
grafana-util dashboard topology \
  --governance ./governance.json \
  --queries ./queries.json \
  --alert-contract ./alert-contract.json \
  --output-format mermaid
```

## What success looks like

- you can point at the exact dashboards, panels, variables, and datasource links involved in one export or live snapshot
- the same topology can be reviewed in the terminal or rendered into Mermaid or Graphviz without rewriting data
- alert contract edges are visible early enough that operators can spot routing or dependency surprises before change work starts

## Failure checks

- if the graph looks empty or too small, verify whether the `governance` input was produced from the right export tree or live environment
- if you expected alert edges but none appear, confirm you supplied `--alert-contract`
- if a downstream visual tool rejects the result, double-check whether you emitted `mermaid`, `dot`, `json`, or plain `text`

## Related commands
- [dashboard analyze (local)](./dashboard-analyze-export.md)
- [dashboard governance-gate](./dashboard-governance-gate.md)
- [dashboard screenshot](./dashboard-screenshot.md)
