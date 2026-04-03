# `grafana-util access`

## Purpose

Run the access-management command surface for users, orgs, teams, and service accounts.

## When to use

- List or browse access inventory.
- Create, modify, export, import, diff, or delete access resources.
- Manage service-account tokens.

## Key flags

- `--profile`, `--url`, `--token`, `--basic-user`, `--basic-password`
- `--prompt-password`, `--prompt-token`, `--timeout`, `--verify-ssl`, `--insecure`, `--ca-cert`
- Use the nested subcommands for `user`, `org`, `team` or `group`, and `service-account`.

## Auth notes

- Prefer `--profile` for repeatable inventory reads.
- Org, user, team, and service-account lifecycle commands often need admin-level credentials; direct Basic auth is the most predictable fallback.
- Token auth may be too narrow for org-wide administration even when read-only list commands work.

## Examples

```bash
grafana-util access user list --profile prod --json
grafana-util access service-account list --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format text
grafana-util access service-account token add --url http://localhost:3000 --basic-user admin --basic-password admin --name deploy-bot --token-name nightly
```

## Related commands

- [access user](./access-user.md)
- [access org](./access-org.md)
- [access team](./access-team.md)
- [access service-account](./access-service-account.md)
