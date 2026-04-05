# `grafana-util access`

## Purpose

Run the access-management command surface for users, orgs, teams, and service accounts.

## When to use

- List or browse access inventory.
- Create, modify, export, import, diff, or delete access resources.
- Manage service-account tokens.

## Description
Open this page when the work is about Grafana identity and access as a whole. The `access` namespace is the grouped entrypoint for org, user, team, service account, and service-account token lifecycle work.

This page is for administrators who first need to choose the right access surface. If your task touches membership, org structure, service-account rotation, or access snapshots, start here and then jump into the matching subcommand page.

## Key flags

- `--profile`, `--url`, `--token`, `--basic-user`, `--basic-password`
- `--prompt-password`, `--prompt-token`, `--timeout`, `--verify-ssl`, `--insecure`, `--ca-cert`
- Use the nested subcommands for `user`, `org`, `team` or `group`, and `service-account`.

## Auth notes

- Prefer `--profile` for repeatable inventory reads.
- Org, user, team, and service-account lifecycle commands often need admin-level credentials; direct Basic auth is the most predictable fallback.
- Token auth may be too narrow for org-wide administration even when read-only list commands work.

## Before / After

Before: access work is split across ad hoc UI clicks, one-off API calls, and hard-to-repeat shell commands.
After: the same access surface is grouped into one CLI namespace, so inventory, lifecycle, token work, and snapshots can be repeated with the same settings.

## What success looks like

- you can tell whether the task belongs to `user`, `org`, `team`, or `service-account` before you touch production
- inventory reads are repeatable because profile and auth defaults are explicit
- token and lifecycle changes are auditable enough to hand off to another operator or to CI

## Failure checks

- if a list command returns less than you expect, confirm whether you need admin-level Basic auth instead of a narrower token
- if a token or membership operation fails, verify that you are on the right org and the right access surface before retrying
- if the output is going into automation, check the selected `--output-format` first so the parser knows what shape to expect

## Examples

```bash
# Purpose: Run the access-management command surface for users, orgs, teams, and service accounts.
grafana-util access user list --profile prod --json
```

```bash
# Purpose: Run the access-management command surface for users, orgs, teams, and service accounts.
grafana-util access service-account list --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format text
```

```bash
# Purpose: Run the access-management command surface for users, orgs, teams, and service accounts.
grafana-util access service-account token add --url http://localhost:3000 --basic-user admin --basic-password admin --name deploy-bot --token-name nightly
```

## Related commands

- [access user](./access-user.md)
- [access org](./access-org.md)
- [access team](./access-team.md)
- [access service-account](./access-service-account.md)
- [access service-account token](./access-service-account-token.md)
