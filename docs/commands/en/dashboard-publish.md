# dashboard publish

## Purpose
Publish one local dashboard JSON file through the existing dashboard import pipeline.

## When to use
Use this when a local draft is ready to go live and you want the command to stage or push it through the same import path used by the CLI.

## Key flags
- `--input`: dashboard JSON file to publish.
- `--replace-existing`: update an existing dashboard when the UID already exists.
- `--folder-uid`: override the destination folder UID.
- `--message`: revision message stored in Grafana.
- `--dry-run`: preview the publish without changing Grafana.
- `--table`, `--json`: dry-run output modes.

## Examples
```bash
grafana-util dashboard publish --url http://localhost:3000 --basic-user admin --basic-password admin --input ./drafts/cpu-main.json --folder-uid infra --message 'Promote CPU dashboard'
grafana-util dashboard publish --url http://localhost:3000 --basic-user admin --basic-password admin --input ./drafts/cpu-main.json --dry-run --table
```

## Related commands
- [dashboard import](./dashboard-import.md)
- [dashboard review](./dashboard-review.md)
- [dashboard patch-file](./dashboard-patch-file.md)

