# dashboard topology

## Purpose
Build a deterministic dashboard topology graph from JSON artifacts.

## When to use
Use this when you need a graph view of dashboards, folders, variables, datasource links, and optional alert contract data. The command also accepts the `graph` alias.

## Key flags
- `--governance`: dashboard governance JSON input.
- `--queries`: optional dashboard query-report JSON input.
- `--alert-contract`: optional alert contract JSON input.
- `--output-format`: render `text`, `json`, `mermaid`, or `dot`.
- `--output-file`: write the rendered topology to disk.
- `--interactive`: open the interactive terminal browser.

## Examples
```bash
grafana-util dashboard topology --governance ./governance.json --queries ./queries.json --alert-contract ./alert-contract.json --output-format mermaid
grafana-util dashboard graph --governance ./governance.json --queries ./queries.json --alert-contract ./alert-contract.json --output-format dot --output-file ./dashboard-topology.dot
```

## Related commands
- [dashboard inspect-export](./dashboard-inspect-export.md)
- [dashboard governance-gate](./dashboard-governance-gate.md)
- [dashboard screenshot](./dashboard-screenshot.md)

