# `grafana-util access service-account`

## Purpose

List live or local Grafana service accounts, create, export, import, diff, or delete Grafana service accounts, and manage their tokens.

## When to use

- Inspect service-account inventory.
- Inspect service accounts from a live Grafana server or from a local export bundle.
- Create or update service-account bundles.
- Generate or delete service-account tokens.

## Before / After

- **Before**: service-account work often starts with a manual UI lookup and a one-off token action.
- **After**: one namespace covers service-account inventory, bundle management, token creation, and token deletion with repeatable CLI input.

## What success looks like

- service-account changes stay tied to one named identity instead of a loose UI click path
- token operations are explicit enough to review or script
- inventory and bundle output can be passed to later access or change workflows without guesswork

## Failure checks

- if a token add or delete fails, recheck whether the service account name or ID matches the target environment
- if an inventory listing looks incomplete, confirm auth scope and org context before assuming the service account is missing
- if the output is going into another step, pick the exact `--output-format` you want rather than relying on a default

## Key flags

- `list`: `--input-dir`, `--query`, `--page`, `--per-page`, `--table`, `--csv`, `--json`, `--yaml`, `--output-format`
- `add`: `--name`, `--role`, `--disabled`, `--json`
- `export` and `diff`: `--output-dir` or `--diff-dir`, `--overwrite`, `--dry-run`
- `import`: `--input-dir`, `--replace-existing`, `--dry-run`, `--table`, `--json`, `--output-format`, `--yes`
- `delete`: `--name`, `--service-account-id`, `--prompt`, `--yes`, `--json`
- `token add`: `--service-account-id` or `--name`, `--token-name`, `--seconds-to-live`, `--json`
- `token delete`: `--service-account-id` or `--name`, `--token-id` or `--token-name`, `--prompt`, `--yes`, `--json`

## Examples

```bash
# Purpose: Inspect service accounts before creating or deleting a token.
grafana-util access service-account list --url http://localhost:3000 --basic-user admin --basic-password admin --output-format text
```

```bash
# Purpose: Review a saved service-account bundle before replaying it.
grafana-util access service-account list --input-dir ./access-service-accounts --output-format table
```

```bash
# Purpose: Create a service account for repeatable deployment automation.
grafana-util access service-account add --url http://localhost:3000 --basic-user admin --basic-password admin --name deploy-bot --role Editor --json
```

```bash
# Purpose: Issue a named token for one service account.
grafana-util access service-account token add --profile prod --name deploy-bot --token-name nightly
```

```bash
# Purpose: Prompt for one service account, confirm it, and then delete it.
grafana-util access service-account delete --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --prompt
```

## Related commands

- [access](./access.md)
- [access service-account token](./access-service-account-token.md)
- [access user](./access-user.md)
