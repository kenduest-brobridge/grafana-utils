# `grafana-util access service-account`

## Purpose

List, create, export, import, diff, or delete Grafana service accounts, and manage their tokens.

## When to use

- Inspect service-account inventory.
- Create or update service-account bundles.
- Generate or delete service-account tokens.

## Key flags

- `list`: `--query`, `--page`, `--per-page`, `--table`, `--csv`, `--json`, `--yaml`, `--output-format`
- `add`: `--name`, `--role`, `--disabled`, `--json`
- `export` and `diff`: `--export-dir` or `--diff-dir`, `--overwrite`, `--dry-run`
- `import`: `--import-dir`, `--replace-existing`, `--dry-run`, `--table`, `--json`, `--output-format`, `--yes`
- `delete`: `--name`, `--yes`, `--json`
- `token add`: `--service-account-id` or `--name`, `--token-name`, `--seconds-to-live`, `--json`
- `token delete`: `--service-account-id` or `--name`, `--token-name`, `--yes`, `--json`

## Examples

```bash
grafana-util access service-account list --url http://localhost:3000 --basic-user admin --basic-password admin --output-format text
grafana-util access service-account add --url http://localhost:3000 --basic-user admin --basic-password admin --name deploy-bot --role Editor --json
grafana-util access service-account token add --profile prod --name deploy-bot --token-name nightly
```

## Related commands

- [access](./access.md)
- [access service-account token](./access-service-account-token.md)
- [access user](./access-user.md)
