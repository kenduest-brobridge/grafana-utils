# `grafana-util access service-account token`

## 目的

為 Grafana service account 新增或刪除 token。

## 使用時機

- 建立新的 service-account token。
- 依 service-account 名稱或 id 刪除既有 token。

## 主要旗標

- `add`: `--service-account-id` 或 `--name`, `--token-name`, `--seconds-to-live`, `--json`
- `delete`: `--service-account-id` 或 `--name`, `--token-name`, `--yes`, `--json`

## 範例

```bash
grafana-util access service-account token add --profile prod --name deploy-bot --token-name nightly
grafana-util access service-account token delete --url http://localhost:3000 --basic-user admin --basic-password admin --name deploy-bot --token-name nightly --yes --json
grafana-util access service-account token add --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --name deploy-bot --token-name nightly
```

## 相關命令

- [access](./access.md)
- [access service-account](./access-service-account.md)
