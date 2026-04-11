# `grafana-util workspace preview`

## Purpose

Show what would change from the current workspace inputs.

## When to use

- Use this when you want the review artifact before apply.
- This is the task-first replacement for the common `plan` step.

## Key flags

- positional workspace path
- `--desired-file`
- `--target-inventory`, `--mapping-file`, `--availability-file`
- `--live-file`
- `--fetch-live`
- `--allow-prune`
- `--trace-id`
- `--output-format`, `--output-file`

## Example

```bash
grafana-util workspace preview ./grafana-oac-repo --fetch-live --profile prod
```

## Related commands

- [workspace](./workspace.md)
- [workspace test](./workspace-test.md)
- [workspace apply](./workspace-apply.md)
