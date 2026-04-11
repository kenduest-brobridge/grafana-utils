# resource

## Purpose
Read a small set of live Grafana resources through a generic read-only query surface.

## When to use
Use this when you need a read-only live lookup before a richer workflow exists in `dashboard`, `alert`, `datasource`, `access`, or `workspace`.

## Description
This namespace is intentionally narrower and more generic than the main operator workflows. It exists so you can inspect a few supported live Grafana resource kinds without waiting for a full domain-specific command surface.

Treat it as a read-only utility surface, not as the primary entrypoint for day-to-day mutation work.

## Workflow
- Start with `resource describe` to see the supported selector patterns and endpoint shape for each kind.
- Use `resource kinds` to see the currently supported live resource kinds.
- Use `resource list <kind>` when you need inventory for one kind.
- Use `resource get <kind>/<identity>` when you need the full live payload for one item.

## Supported kinds
- `dashboards`
- `folders`
- `datasources`
- `alert-rules`
- `orgs`

## Output
- `kinds` supports `text`, `table`, `json`, and `yaml`
- `list` supports `text`, `table`, `json`, and `yaml`
- `get` supports `text`, `table`, `json`, and `yaml`

## Examples
```bash
# Purpose: Describe the supported live resource kinds and selector patterns.
grafana-util resource describe
```

```bash
# Purpose: Show the currently supported generic resource kinds.
grafana-util resource kinds
```

```bash
# Purpose: List live dashboard resources from a local Grafana.
grafana-util resource list dashboards --url http://localhost:3000 --basic-user admin --basic-password admin
```

```bash
# Purpose: Fetch one datasource payload as YAML.
grafana-util resource get datasources/prom-main --profile prod --output-format yaml
```

## Related commands
- [resource describe](./resource-describe.md)
- [resource kinds](./resource-kinds.md)
- [resource list](./resource-list.md)
- [resource get](./resource-get.md)
- [dashboard](./dashboard.md)
