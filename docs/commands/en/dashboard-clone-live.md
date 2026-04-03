# dashboard clone-live

## Purpose
Clone one live dashboard into a local draft with optional overrides.

## When to use
Use this when you want to start from an existing live dashboard but give the local draft a new title, UID, or folder target.

## Key flags
- `--source-uid`: live Grafana dashboard UID to clone.
- `--output`: write the cloned draft to this path.
- `--name`: override the cloned dashboard title.
- `--uid`: override the cloned dashboard UID.
- `--folder-uid`: override the preserved Grafana folder UID.

## Examples
```bash
grafana-util dashboard clone-live --url http://localhost:3000 --basic-user admin --basic-password admin --source-uid cpu-main --output ./cpu-main-clone.json
grafana-util dashboard clone-live --profile prod --source-uid cpu-main --name 'CPU Clone' --uid cpu-main-clone --folder-uid infra --output ./cpu-main-clone.json
```

## Related commands
- [dashboard get](./dashboard-get.md)
- [dashboard patch-file](./dashboard-patch-file.md)
- [dashboard publish](./dashboard-publish.md)
