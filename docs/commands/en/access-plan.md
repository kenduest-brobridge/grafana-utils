# `grafana-util access plan`

## Purpose
Build a review-first access reconcile plan from a local access bundle against live Grafana.

## When to use
Use this before importing or pruning access state when you want a structured answer to "what would change?" without mutating Grafana.

The current implementation is a user-bundle vertical slice. It plans `user` resources and reports non-user resource selectors as unsupported instead of pretending to review them.

## Key flags
- `--input-dir`: local access export bundle to review.
- `--resource`: choose `user`, `team`, `org`, `service-account`, or `all`. This slice currently supports `user`.
- `--prune`: show remote-only users as delete candidates. Without this flag they remain extra remote review items.
- `--output-format`: choose `text`, `table`, or `json`.
- `--show-same`: include unchanged rows in text and table output.
- `--output-columns`, `--list-columns`, `--no-header`: tune table output.

## Examples
```bash
# Build a concise user access plan.
grafana-util access plan --profile prod --input-dir ./access-users
```

```bash
# Render a table for review.
grafana-util access plan --profile prod --input-dir ./access-users --resource user --output-format table
```

```bash
# Include remote-only users as delete candidates.
grafana-util access plan --profile prod --input-dir ./access-users --resource user --prune --output-format json
```

## Before / After

- **Before**: access export, import, and diff flows were split by resource type, so a user bundle review could still require manual reasoning before mutation.
- **After**: `access plan` emits a single review document with stable action rows for user bundle reconciliation.
- JSON output is structured for CI and future TUI review. Rows include stable `actionId`, status, changed fields, target details, blocked reason, and review hints.

## What success looks like

- user create, update, same, and remote-only states are visible before import
- delete candidates require an explicit `--prune` review mode
- JSON output can be saved as review evidence or loaded by automation

## Failure checks

- if the command says the selected resource is unsupported, switch to `--resource user` for this slice
- if the plan points at the wrong org, confirm the profile or shared auth flags
- if delete candidates appear unexpectedly, rerun without `--prune` and inspect the extra remote rows first

## Related commands
- [access user](./access-user.md)
- [access org](./access-org.md)
- [access team](./access-team.md)
- [access service-account](./access-service-account.md)
