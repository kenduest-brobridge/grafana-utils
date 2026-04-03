# datasource add

## 用途
透過 Grafana API 建立一個線上 Grafana datasource。

## 何時使用
當您想直接建立新的 datasource，或在套用前先 dry-run 建立步驟時，使用這個指令。

## 重點旗標
- `--uid`：穩定的 datasource 識別碼。
- `--name`：datasource 名稱。
- `--type`：Grafana datasource plugin type id。
- `--datasource-url`：datasource 目標網址。
- `--access`：proxy 或 direct 存取模式。
- `--default`：標記為預設 datasource。
- `--preset-profile` 與 `--apply-supported-defaults`：產生支援的預設值。
- `--json-data`、`--secure-json-data`、`--secure-json-data-placeholders`、`--secret-values`：設定自訂欄位與秘密值。
- `--dry-run`、`--table`、`--json`、`--output-format`、`--no-header`：預覽輸出控制。

## 範例
```bash
grafana-util datasource add --profile prod --name tempo-main --type tempo --datasource-url http://tempo:3200 --preset-profile full --dry-run --json
grafana-util datasource add --url http://localhost:3000 --basic-user admin --basic-password admin --name prometheus-main --type prometheus --datasource-url http://prometheus:9090 --dry-run --table
grafana-util datasource add --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --name tempo-main --type tempo --datasource-url http://tempo:3200 --preset-profile full --dry-run --json
```

## 相關指令
- [datasource types](./datasource-types.md)
- [datasource modify](./datasource-modify.md)
- [datasource list](./datasource-list.md)
