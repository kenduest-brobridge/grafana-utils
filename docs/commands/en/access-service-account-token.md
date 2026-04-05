# `grafana-util access service-account token`

## Purpose

Add or delete tokens for a Grafana service account.

## When to use

- Create a new service-account token.
- Delete an existing service-account token by service-account name or id.

## Before / After

- **Before**: service-account token work often happens ad hoc in the Grafana UI, with no easy way to repeat the same action in another environment.
- **After**: token creation and cleanup become explicit CLI steps that can be reviewed, scripted, and repeated for the same service account.

## What success looks like

- token creation is tied to one named service account instead of relying on a manual UI lookup
- token cleanup is deliberate and auditable, especially when you script deletion with `--yes`
- automation can capture the JSON result when it needs to hand the token or delete confirmation to another step

## Failure checks

- if token creation fails, confirm whether you targeted the right service account by `--name` or `--service-account-id`
- if deletion looks like a no-op, recheck the token name and whether you are pointing at the correct Grafana org or environment
- if you plan to pass the result into automation, use `--json` and validate the response shape before storing or forwarding it

## Key flags

- `add`: `--service-account-id` or `--name`, `--token-name`, `--seconds-to-live`, `--json`
- `delete`: `--service-account-id` or `--name`, `--token-name`, `--yes`, `--json`

## Examples

```bash
# Purpose: Create a new token for one service account.
grafana-util access service-account token add --profile prod --name deploy-bot --token-name nightly
```

```bash
# Purpose: Delete a token after review.
grafana-util access service-account token delete --profile prod --name deploy-bot --token-name nightly --yes --json
```

## Related commands

- [access](./access.md)
- [access service-account](./access-service-account.md)
