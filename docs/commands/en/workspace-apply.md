# `grafana-util workspace apply`

## Purpose

Turn a reviewed preview into staged or live apply output.

## When to use

- Use this after preview and review are already complete.
- Add `--execute-live` only when you are ready to mutate Grafana for real.

## Key flags

- `--preview-file`
- `--plan-file`
- `--approve`
- `--execute-live`
- `--approval-reason`, `--apply-note`
- `--output-format`

## Example

```bash
grafana-util workspace apply --preview-file ./workspace-preview.json --approve --execute-live --profile prod
```

## Related commands

- [workspace](./workspace.md)
- [workspace preview](./workspace-preview.md)
- [workspace ci](./workspace.md#ci)

