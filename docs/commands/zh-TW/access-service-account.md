# `grafana-util access service-account`

## 目的

列出、建立、匯出、匯入、比對或刪除 Grafana service account，並管理其 token。

## 使用時機

- 檢視 service-account 清單。
- 建立或更新 service-account 套件。
- 產生或刪除 service-account token。

## 主要旗標

- `list`: `--query`, `--page`, `--per-page`, `--table`, `--csv`, `--json`, `--yaml`, `--output-format`
- `add`: `--name`, `--role`, `--disabled`, `--json`
- `export` 與 `diff`: `--export-dir` 或 `--diff-dir`, `--overwrite`, `--dry-run`
- `import`: `--import-dir`, `--replace-existing`, `--dry-run`, `--table`, `--json`, `--output-format`, `--yes`
- `delete`: `--name`, `--yes`, `--json`
- `token add`: `--service-account-id` 或 `--name`, `--token-name`, `--seconds-to-live`, `--json`
- `token delete`: `--service-account-id` 或 `--name`, `--token-name`, `--yes`, `--json`

## 範例

```bash
grafana-util access service-account list --profile prod --output-format text
grafana-util access service-account add --url http://localhost:3000 --basic-user admin --basic-password admin --name deploy-bot --role Editor --json
grafana-util access service-account token add --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --name deploy-bot --token-name nightly
```

## 相關命令

- [access](./access.md)
- [access service-account token](./access-service-account-token.md)
- [access user](./access-user.md)
