# `grafana-util access service-account token`

## Purpose

Add or delete tokens for a Grafana service account.

## When to use

- Create a new service-account token.
- Delete an existing service-account token by service-account name or id.

## Key flags

- `add`: `--service-account-id` or `--name`, `--token-name`, `--seconds-to-live`, `--json`
- `delete`: `--service-account-id` or `--name`, `--token-name`, `--yes`, `--json`

## Examples

```bash
grafana-util access service-account token add --profile prod --name deploy-bot --token-name nightly
grafana-util access service-account token delete --profile prod --name deploy-bot --token-name nightly --yes --json
```

## Related commands

- [access](./access.md)
- [access service-account](./access-service-account.md)
