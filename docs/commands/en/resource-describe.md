# resource describe

## Purpose
Describe the supported live Grafana resource kinds and selector patterns.

## When to use
Use this when you want to know how the generic read-only resource surface is shaped before you pick `resource list` or `resource get`.

## Key flags
- optional positional `KIND`: limit the output to one supported resource kind such as `dashboards`, `folders`, `datasources`, `alert-rules`, or `orgs`
- `--output-format`: choose `text`, `table`, `json`, or `yaml`

## Notes
- This command is descriptive only. It does not discover live schemas from Grafana.
- Use it when you need the selector format, list endpoint, or get endpoint for a supported kind.

## Examples
```bash
# Purpose: Describe every supported live resource kind.
grafana-util resource describe
```

```bash
# Purpose: Describe one supported resource kind as JSON.
grafana-util resource describe dashboards --output-format json
```

```bash
# Purpose: Describe one supported resource kind as a table.
grafana-util resource describe orgs --output-format table
```

## Related commands
- [resource](./resource.md)
- [resource kinds](./resource-kinds.md)
- [resource list](./resource-list.md)
- [resource get](./resource-get.md)
