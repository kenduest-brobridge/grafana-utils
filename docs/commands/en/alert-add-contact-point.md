# `grafana-util alert add-contact-point`

## Purpose

Author a staged alert contact point from the higher-level authoring surface.

## When to use

- Create a new contact point under a desired-state alert tree.
- Preview the generated file before writing it.

## Key flags

- `--desired-dir` points to the staged alert tree.
- `--name` sets the contact point name.
- `--dry-run` previews the planned output.

## Before / After

- Before: hand-build the contact point file and remember the authoring shape yourself.
- After: create one staged contact point file from the higher-level authoring surface.

## What success looks like

- The contact point file appears in the desired-state tree with the expected name.
- A dry-run shows the receiver details you intended to write.

## Failure checks

- Verify `--desired-dir` before writing into the tree.
- Check for duplicate contact point names if the scaffold looks reused.
- Remember that this command authors the contact point; it does not wire the route tree by itself.

## Examples

```bash
# Purpose: Author a staged alert contact point from the higher-level authoring surface.
grafana-util alert add-contact-point --desired-dir ./alerts/desired --name pagerduty-primary
```

```bash
# Purpose: Author a staged alert contact point from the higher-level authoring surface.
grafana-util alert add-contact-point --desired-dir ./alerts/desired --name pagerduty-primary --dry-run
```

## Related commands

- [alert](./alert.md)
- [alert set-route](./alert-set-route.md)
- [alert new-contact-point](./alert-new-contact-point.md)
