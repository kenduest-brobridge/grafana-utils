# `grafana-util workspace test`

## Purpose

Validate whether the local workspace is structurally safe to continue.

## When to use

- Use this when you need a readiness gate before preview or apply.
- Use this in CI when you want a fast decision without building a full preview yet.

## Key flags

- 
- `--availability-file`
- `--target-inventory`, `--mapping-file`
- `--fetch-live`
- `--output-format`

## Example

```bash
grafana-util workspace test ./grafana-oac-repo --fetch-live --output-format json
```

## Related commands

- [workspace](./workspace.md)
- [workspace scan](./workspace-scan.md)
- [workspace preview](./workspace-preview.md)
- [status staged](./status.md#staged)

