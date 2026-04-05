# dashboard patch-file

## Purpose
Patch one local dashboard JSON file in place or write it to a new path.

## When to use
Use this when you need to rewrite dashboard metadata locally before review or publish, without contacting Grafana.

## Key flags
- `--input`: dashboard JSON file to patch.
- `--output`: write to a different path instead of overwriting the input.
- `--name`: replace the dashboard title.
- `--uid`: replace the dashboard UID.
- `--folder-uid`: set the preserved folder UID.
- `--message`: store a note in the patched file metadata.
- `--tag`: replace dashboard tags; repeat the flag for multiple tags.

## Examples
```bash
# Purpose: Patch one local dashboard JSON file in place or write it to a new path.
grafana-util dashboard patch-file --input ./dashboards/raw/cpu-main.json --name 'CPU Overview' --folder-uid infra --tag prod --tag sre
```

```bash
# Purpose: Patch one local dashboard JSON file in place or write it to a new path.
grafana-util dashboard patch-file --input ./drafts/cpu-main.json --output ./drafts/cpu-main-patched.json --uid cpu-main --message 'Add folder metadata before publish'
```

## Related commands
- [dashboard review](./dashboard-review.md)
- [dashboard publish](./dashboard-publish.md)
- [dashboard get](./dashboard-get.md)

