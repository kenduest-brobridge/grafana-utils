# `grafana-util alert new-contact-point`

## Purpose

Create a low-level staged alert contact point scaffold.

## When to use

- Seed a new contact point file in a desired-state tree.
- Start from a scaffold before filling in receiver details.

## Key flags

- `--desired-dir` points to the staged alert tree.
- `--name` seeds the scaffold name.

## Before / After

- Before: start from an empty file and remember every alert contact point field yourself.
- After: generate a scaffold with the right file name and place to fill in receiver details.

## What success looks like

- The scaffold file exists where you expected it in the desired-state tree.
- The generated file is a clean starting point for adding receiver details.

## Failure checks

- Check `--desired-dir` if the scaffold does not land in the tree you expect.
- Verify the name if the scaffold collides with an existing contact point.

## Examples

```bash
# Purpose: Create a low-level staged alert contact point scaffold.
grafana-util alert new-contact-point --desired-dir ./alerts/desired --name pagerduty-primary
```

```bash
# Purpose: Create a low-level staged alert contact point scaffold.
grafana-util alert add-contact-point --desired-dir ./alerts/desired --name pagerduty-primary
```

## Related commands

- [alert](./alert.md)
- [alert add-contact-point](./alert-add-contact-point.md)
- [alert set-route](./alert-set-route.md)
