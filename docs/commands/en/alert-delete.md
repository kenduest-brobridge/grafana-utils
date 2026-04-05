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
- `--output-format` renders delete preview or execution output as `text` or `json`.

## Before / After

- Before: clean up a resource manually through the UI and hope you target the right object.
- After: delete exactly one named alert resource by kind and identity.

## What success looks like

- The target kind and identity are the only things affected.
- A preview or execution result clearly shows the resource you meant to remove.
- Policy-tree resets only happen when `--allow-policy-reset` is present.

## Failure checks

- Confirm the kind matches the identity you intend to remove.
- Check the org/profile context before deleting if the resource is not where you expect.
- If you are touching the policy tree, make sure `--allow-policy-reset` is present and intentional.

## Examples

```bash
# Purpose: Delete one explicit alert resource identity.
grafana-util alert delete --kind rule --identity cpu-main
```

```bash
# Purpose: Delete one explicit alert resource identity.
grafana-util alert delete --url http://localhost:3000 --basic-user admin --basic-password admin --kind policy-tree --identity default --allow-policy-reset
```

## Related commands

- [alert](./alert.md)
- [alert plan](./alert-plan.md)
- [alert apply](./alert-apply.md)
