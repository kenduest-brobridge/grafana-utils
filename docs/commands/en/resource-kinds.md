# resource kinds

## Purpose
List the live resource kinds supported by the generic read-only `resource` query surface.

## When to use
Use this when you need to confirm whether the generic resource query surface already supports the live Grafana resource kind you want to inspect. If you need the selector pattern or endpoint shape, use `resource describe` instead.

## Key flags
- `--output-format`: choose `text`, `table`, `json`, or `yaml`.

## Examples
```bash
# Purpose: Show supported resource kinds as a table.
grafana-util resource kinds
```

```bash
# Purpose: Render the same supported resource kinds as JSON.
grafana-util resource kinds --output-format json
```

## Related commands
- [resource](./resource.md)
- [resource describe](./resource-describe.md)
- [resource list](./resource-list.md)
- [resource get](./resource-get.md)
