# resource list

## Purpose
List one supported live Grafana resource kind.

## When to use
Use this when you need a read-only live inventory for one supported resource kind but do not yet need a higher-level domain workflow.

## Key flags
- positional `KIND`: one supported resource kind such as `dashboards`, `folders`, `datasources`, `alert-rules`, or `orgs`
- `--profile`, `--url`, `--token`, `--basic-user`, `--basic-password`: live Grafana connection settings
- `--output-format`: choose `text`, `table`, `json`, or `yaml`

## Examples
```bash
# Purpose: List dashboards as a table from a local Grafana.
grafana-util resource list dashboards --url http://localhost:3000 --basic-user admin --basic-password admin
```

```bash
# Purpose: List folders as YAML.
grafana-util resource list folders --profile prod --output-format yaml
```

```bash
# Purpose: List alert rules as JSON.
grafana-util resource list alert-rules --profile prod --output-format json
```

## Related commands
- [resource](./resource.md)
- [resource describe](./resource-describe.md)
- [resource kinds](./resource-kinds.md)
- [resource get](./resource-get.md)
