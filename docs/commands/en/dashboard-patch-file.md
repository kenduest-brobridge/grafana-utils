# dashboard patch-file

## Purpose
Patch one local dashboard JSON file in place or write it to a new path.

## When to use
Use this when you need to rewrite dashboard metadata locally before review or publish, without contacting Grafana.

## Key flags
- `--input`: dashboard JSON file to patch. Use `-` to read one wrapped or bare dashboard JSON document from standard input.
- `--output`: write to a different path instead of overwriting the input. Required when `--input -` is used.
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

```bash
# Purpose: Patch one generated dashboard from standard input into an explicit output file.
jsonnet dashboards/cpu.jsonnet | grafana-util dashboard patch-file --input - --output ./drafts/cpu-main.json --folder-uid infra
```

## Related commands
- [dashboard review](./dashboard-review.md)
- [dashboard publish](./dashboard-publish.md)
- [dashboard fetch-live](./dashboard-fetch-live.md)
