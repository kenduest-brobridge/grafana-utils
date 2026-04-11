# `grafana-util workspace scan`

## Purpose

Discover what is in one local workspace or staged package.

## When to use

- Start here when you want the first-pass summary before validation or preview.
- Use this before `workspace test` or `workspace preview` when you need to size the package first.

## Key flags

- positional workspace path
- `--desired-file`
- `--dashboard-export-dir`, `--dashboard-provisioning-dir`
- `--alert-export-dir`, `--datasource-provisioning-file`
- `--output-format`
- `--output-file`, `--also-stdout`

## Example

```bash
grafana-util workspace scan ./grafana-oac-repo
```

## Related commands

- [workspace](./workspace.md)
- [workspace test](./workspace-test.md)
- [workspace preview](./workspace-preview.md)
- [status](./status.md)
