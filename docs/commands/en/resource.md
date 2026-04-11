# `grafana-util status resource`

## Purpose
Run generic read-only Grafana resource queries through the `status` namespace.

## When to use
Use this when you need a read-only live lookup before a richer workflow exists in `dashboard`, `alert`, `datasource`, `access`, or `workspace`.

## Description
This namespace is intentionally narrower and more generic than the main operator workflows. It exists so you can inspect a supported live Grafana resource kind without waiting for a full domain-specific command surface.

The old top-level `grafana-util resource ...` path is no longer the public entrypoint. Use `grafana-util status resource ...` instead.

Treat this as a read-only utility surface, not as the primary entrypoint for day-to-day mutation work.

## Workflow
- Start with `status resource describe` to see the supported selector patterns and endpoint shape for each kind.
- Use `status resource kinds` to see the currently supported live resource kinds.
- Use `status resource list <kind>` when you need inventory for one kind.
- Use `status resource get <kind>/<identity>` when you need the full live payload for one item.

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
grafana-util status resource describe
```

```bash
# Purpose: Show the currently supported generic resource kinds.
grafana-util status resource kinds
```

```bash
# Purpose: List live dashboard resources from a local Grafana.
grafana-util status resource list dashboards --url http://localhost:3000 --basic-user admin --basic-password admin
```

```bash
# Purpose: Fetch one datasource payload as YAML.
grafana-util status resource get datasources/prom-main --profile prod --output-format yaml
```

## Related commands
- [resource describe](./resource-describe.md)
- [resource kinds](./resource-kinds.md)
- [resource list](./resource-list.md)
- [resource get](./resource-get.md)
- [dashboard](./dashboard.md)
