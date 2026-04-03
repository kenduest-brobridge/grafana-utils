# datasource modify

## 用途
透過 Grafana API 修改一個線上 Grafana datasource。

## 何時使用
當某個 datasource 已經存在，而您需要更新它的 URL、驗證、JSON payload 或其他線上設定時，使用這個指令。

## 重點旗標
- `--uid`：要修改的 datasource UID。
- `--set-url`：替換 datasource URL。
- `--set-access`：替換 datasource 存取模式。
- `--set-default`：設定或取消預設 datasource 旗標。
- `--basic-auth`、`--basic-auth-user`、`--basic-auth-password`：更新基本驗證設定。
- `--user`、`--password`、`--with-credentials`、`--http-header`：更新支援的請求設定。
- `--tls-skip-verify`、`--server-name`：更新與 TLS 相關的設定。
- `--json-data`、`--secure-json-data`、`--secure-json-data-placeholders`、`--secret-values`：更新結構化欄位與秘密值。
- `--dry-run`、`--table`、`--json`、`--output-format`、`--no-header`：預覽輸出控制。

## 範例
```bash
grafana-util datasource modify --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --uid prom-main --set-url http://prometheus-v2:9090 --dry-run --json
grafana-util datasource modify --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --uid prom-main --set-default true --dry-run --table
```

## 相關指令
- [datasource add](./datasource-add.md)
- [datasource list](./datasource-list.md)
- [datasource delete](./datasource-delete.md)
