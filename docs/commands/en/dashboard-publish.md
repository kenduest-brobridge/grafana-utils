# dashboard publish

## Purpose
Publish one local dashboard JSON file through the existing dashboard import pipeline.

## When to use
Use this when a local draft is ready to go live and you want the command to stage or push it through the same import path used by the CLI.

## Key flags
- `--input`: dashboard JSON file to publish. Use `-` to read one wrapped or bare dashboard JSON document from standard input.
- `--replace-existing`: update an existing dashboard when the UID already exists.
- `--folder-uid`: override the destination folder UID. The built-in General folder is normalized back to the default root publish path instead of sending a literal `general` folder UID.
- `--message`: revision message stored in Grafana.
- `--dry-run`: preview the publish without changing Grafana.
- `--watch`: rerun publish or dry-run whenever the local input file changes. Use this with a local file path, not `--input -`. The watcher reports change detection, transient failures, and reruns, and keeps watching until you stop it.
- `--table`, `--json`: dry-run output modes.

## Examples
```bash
# Purpose: Publish one local dashboard JSON file through the existing dashboard import pipeline.
grafana-util dashboard publish --url http://localhost:3000 --basic-user admin --basic-password admin --input ./drafts/cpu-main.json --folder-uid infra --message 'Promote CPU dashboard'
```

```bash
# Purpose: Publish one local dashboard JSON file through the existing dashboard import pipeline.
grafana-util dashboard publish --url http://localhost:3000 --basic-user admin --basic-password admin --input ./drafts/cpu-main.json --dry-run --table
```

```bash
# Purpose: Publish one generated dashboard from standard input.
jsonnet dashboards/cpu.jsonnet | grafana-util dashboard publish --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --input - --replace-existing
```

```bash
# Purpose: Watch one local draft file and rerun dry-run after each save.
grafana-util dashboard publish --url http://localhost:3000 --basic-user admin --basic-password admin --input ./drafts/cpu-main.json --dry-run --watch
```

## Related commands
- [dashboard import](./dashboard-import.md)
- [dashboard review](./dashboard-review.md)
- [dashboard patch-file](./dashboard-patch-file.md)
