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
# 用途：透過 Grafana API 刪除一個線上 Grafana datasource。
grafana-util datasource delete --profile prod --uid prom-main --dry-run --json
```

```bash
# 用途：透過 Grafana API 刪除一個線上 Grafana datasource。
grafana-util datasource delete --url http://localhost:3000 --basic-user admin --basic-password admin --uid prom-main --yes
```

```bash
# 用途：透過 Grafana API 刪除一個線上 Grafana datasource。
grafana-util datasource delete --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --uid prom-main --dry-run --json
```

## 採用前後對照

- **採用前**：刪除 datasource 常常只能靠記憶 UI 裡的那一列，或是臨時手打名稱。
- **採用後**：一個指令就能以 UID 或名稱定位目標、先做 dry run，再明確執行 live 刪除。

## 成功判準

- 在任何 live 刪除前，目標 datasource 都已經很明確
- `--dry-run` 讓這個動作變得可審查
- 只有在操作人員真的要刪除時，才使用 `--yes`

## 失敗時先檢查

- 如果指到錯的 datasource，先確認 UID 或名稱再重跑
- 如果 dry-run 是空的，先檢查 org 範圍是否真的看得到目標 datasource
- 如果 live 刪除被拒絕，先確認帳號是否有該 org 的刪除權限

## 相關指令
- [datasource browse](./datasource-browse.md)
- [datasource modify](./datasource-modify.md)
- [datasource list](./datasource-list.md)
