# datasource delete

## 用途
透過 Grafana API 刪除一個線上 Grafana datasource。

## 何時使用
當某個 datasource 應該被依 UID 或名稱移除時，無論是 dry run 或已確認的線上刪除，都可以使用這個指令。

## 重點旗標
- `--uid`：要刪除的 datasource UID。
- `--name`：當沒有 UID 可用時，改用名稱刪除。
- `--yes`：確認這次線上刪除。
- `--dry-run`、`--table`、`--json`、`--output-format`、`--no-header`：預覽輸出控制。

## 範例
```bash
grafana-util datasource delete --profile prod --uid prom-main --dry-run --json
grafana-util datasource delete --url http://localhost:3000 --basic-user admin --basic-password admin --uid prom-main --yes
grafana-util datasource delete --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --uid prom-main --dry-run --json
```

## 相關指令
- [datasource browse](./datasource-browse.md)
- [datasource modify](./datasource-modify.md)
- [datasource list](./datasource-list.md)
