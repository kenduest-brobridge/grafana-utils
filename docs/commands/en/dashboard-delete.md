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
- `--prompt`: prompt for the selector, preview the delete plan, and confirm in the terminal.
- `--dry-run`, `--table`, `--json`, `--output-format`, `--no-header`: preview output controls.

## Examples
```bash
# Purpose: Delete live dashboards by UID or folder path.
grafana-util dashboard delete --url http://localhost:3000 --basic-user admin --basic-password admin --uid cpu-main --dry-run --json
```

```bash
# Purpose: Delete live dashboards by UID or folder path.
grafana-util dashboard delete --url http://localhost:3000 --basic-user admin --basic-password admin --path 'Platform / Infra' --yes
```

```bash
# Purpose: Prompt for a delete selector, preview the delete plan, and confirm in the terminal.
grafana-util dashboard delete --url http://localhost:3000 --prompt
```

## Related commands
- [dashboard browse](./dashboard-browse.md)
- [dashboard list](./dashboard-list.md)
- [dashboard import](./dashboard-import.md)
