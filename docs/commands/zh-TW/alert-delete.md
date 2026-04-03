# `grafana-util alert delete`

## 目的

刪除一個明確指定的 alert 資源識別。

## 使用時機

- 依識別刪除單一規則、聯絡點、靜音時段、政策樹或範本。
- 只有在確定要這麼做時，才重設由工具管理的通知政策樹。

## 主要旗標

- `--kind` 選擇要刪除的資源種類。
- `--identity` 提供明確的資源識別。
- `--allow-policy-reset` 允許重設政策樹。
- `--output` 可將刪除預覽或執行結果呈現為 `text` 或 `json`。

## 範例

```bash
grafana-util alert delete --profile prod --kind rule --identity cpu-main
grafana-util alert delete --url http://localhost:3000 --basic-user admin --basic-password admin --kind policy-tree --identity default --allow-policy-reset
grafana-util alert delete --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --kind rule --identity cpu-main
```

## 相關命令

- [alert](./alert.md)
- [alert plan](./alert-plan.md)
- [alert apply](./alert-apply.md)
