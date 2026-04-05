# dashboard topology

## Purpose
Build a deterministic dashboard topology graph from JSON artifacts.

## When to use
Use this when you need a graph view of dashboards, folders, variables, datasource links, and optional alert contract data. The command also accepts the `graph` alias.

## Before / After

- **Before**: dependency knowledge lives in operator memory, raw JSON, or one-off diagrams that drift out of date.
- **After**: one topology run gives you a reproducible graph you can inspect in text form, hand to Mermaid, or ship to Graphviz.

## Key flags
- `--governance`: dashboard governance JSON input.
- `--queries`: optional dashboard query-report JSON input.
- `--alert-contract`: optional alert contract JSON input.
- `--output-format`: render `text`, `json`, `mermaid`, or `dot`.
- `--output-file`: write the rendered topology to disk.
- `--interactive`: open the interactive terminal browser.

## Examples
```bash
# Purpose: Build a deterministic dashboard topology graph from JSON artifacts.
grafana-util dashboard topology \
  --governance ./governance.json \
  --queries ./queries.json \
  --alert-contract ./alert-contract.json \
  --output-format mermaid
```

```bash
# Purpose: Build a deterministic dashboard topology graph from JSON artifacts.
grafana-util dashboard graph \
  --governance ./governance.json \
  --queries ./queries.json \
  --alert-contract ./alert-contract.json \
  --output-format dot \
  --output-file ./dashboard-topology.dot
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
- [dashboard inspect-export](./dashboard-inspect-export.md)
- [dashboard governance-gate](./dashboard-governance-gate.md)
- [dashboard screenshot](./dashboard-screenshot.md)
