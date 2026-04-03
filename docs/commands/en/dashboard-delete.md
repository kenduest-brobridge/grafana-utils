# dashboard delete

## Purpose
Delete live dashboards by UID or folder path.

## When to use
Use this when you need to remove one dashboard, a folder subtree, or a subtree plus the matched folders themselves.

## Key flags
- `--uid`: delete one dashboard by UID.
- `--path`: delete dashboards under one folder subtree.
- `--delete-folders`: with `--path`, also remove matched folders.
- `--yes`: acknowledge the live delete.
- `--interactive`: preview and confirm interactively.
- `--dry-run`, `--table`, `--json`, `--output-format`, `--no-header`: preview output controls.

## Examples
```bash
grafana-util dashboard delete --url http://localhost:3000 --basic-user admin --basic-password admin --uid cpu-main --dry-run --json
grafana-util dashboard delete --url http://localhost:3000 --basic-user admin --basic-password admin --path 'Platform / Infra' --yes
```

## Related commands
- [dashboard browse](./dashboard-browse.md)
- [dashboard list](./dashboard-list.md)
- [dashboard import](./dashboard-import.md)
