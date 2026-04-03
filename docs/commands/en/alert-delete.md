# `grafana-util alert delete`

## Purpose

Delete one explicit alert resource identity.

## When to use

- Remove a single rule, contact point, mute timing, policy tree, or template by identity.
- Reset the managed notification policy tree only when you intend to allow it.

## Key flags

- `--kind` selects the resource kind to delete.
- `--identity` provides the explicit resource identity.
- `--allow-policy-reset` permits policy-tree reset.
- `--output` renders delete preview or execution output as `text` or `json`.

## Examples

```bash
grafana-util alert delete --kind rule --identity cpu-main
grafana-util alert delete --url http://localhost:3000 --basic-user admin --basic-password admin --kind policy-tree --identity default --allow-policy-reset
```

## Related commands

- [alert](./alert.md)
- [alert plan](./alert-plan.md)
- [alert apply](./alert-apply.md)
