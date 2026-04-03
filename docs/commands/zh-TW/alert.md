# `grafana-util alert`

## 目的

執行 alerting 指令介面，用來匯出、匯入、比對、規劃、套用、刪除、撰寫與列出 Grafana alert 資源。

## 使用時機

- 從 Grafana 匯出本機 alert 套件。
- 將 alert 套件匯入或與線上 Grafana 狀態做 diff。
- 建立並套用經審閱過的 alert 管理計畫。
- 撰寫暫存的規則、聯絡點、路由與範本。
- 列出線上 alert 規則、聯絡點、靜音時段與範本。

## 主要旗標

- `--profile`, `--url`, `--token`, `--basic-user`, `--basic-password`
- `--prompt-password`, `--prompt-token`, `--timeout`, `--verify-ssl`
- 使用巢狀子命令處理 `export`、`import`、`diff`、`plan`、`apply`、`delete`、`add-rule`、`clone-rule`、`add-contact-point`、`set-route`、`preview-route`、`new-rule`、`new-contact-point`、`new-template`、`list-rules`、`list-contact-points`、`list-mute-timings` 和 `list-templates`。

## 驗證說明

- 一般 alert 檢查與套用流程優先用 `--profile`。
- 需要更廣 org 可見度或管理員盤點時，Basic auth 會更穩定。
- Token 驗證較適合單一 org 或權限範圍已知的自動化。

## 範例

```bash
grafana-util alert list-rules --profile prod --json
grafana-util alert export --url http://localhost:3000 --basic-user admin --basic-password admin --output-dir ./alerts --overwrite
grafana-util alert export --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-dir ./alerts --flat
```

## 相關命令

- [alert export](./alert-export.md)
- [alert import](./alert-import.md)
- [alert diff](./alert-diff.md)
- [alert plan](./alert-plan.md)
- [alert apply](./alert-apply.md)
- [alert delete](./alert-delete.md)
- [access](./access.md)
