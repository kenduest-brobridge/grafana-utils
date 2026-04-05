# `grafana-util alert`

## 目的

執行 alerting 指令介面，用來匯出、匯入、比對、規劃、套用、刪除、撰寫與列出 Grafana alert 資源。

## 使用時機

- 從 Grafana 匯出本機 alert 套件。
- 將 alert 套件匯入或與線上 Grafana 狀態做 diff。
- 建立並套用經審閱過的 alert 管理計畫。
- 撰寫暫存的規則、聯絡點、路由與範本。
- 列出線上 alert 規則、聯絡點、靜音時段與範本。

## 說明
如果你現在處理的是整個 Grafana 告警工作流，而不是單一命令，先看這一頁最合適。`alert` 指令群組把唯讀盤點、本地編修、diff 與 review 路徑，以及 plan / apply 這條正式變更流程放在一起。

這頁比較像告警治理的入口頁，適合 SRE、平台維運，或任何要先搞懂規則、通知路由與 contact point 之間關係，再決定往哪個子命令深入的人。

## 主要旗標

- `--profile`, `--url`, `--token`, `--basic-user`, `--basic-password`
- `--prompt-password`, `--prompt-token`, `--timeout`, `--verify-ssl`
- 使用巢狀子命令處理 `export`、`import`、`diff`、`plan`、`apply`、`delete`、`add-rule`、`clone-rule`、`add-contact-point`、`set-route`、`preview-route`、`new-rule`、`new-contact-point`、`new-template`、`list-rules`、`list-contact-points`、`list-mute-timings` 和 `list-templates`。

## 驗證說明

- 一般 alert 檢查與套用流程優先用 `--profile`。
- 需要更廣 org 可見度或管理員盤點時，Basic auth 會更穩定。
- Token 驗證較適合單一 org 或權限範圍已知的自動化。

## 採用前後對照

- **採用前**：alert 工作常常分散在 list、export 或 UI 手動修改，沒有一條從盤點到審核套用的完整路徑。
- **採用後**：`alert` 命令群組把 inventory、撰寫、diff、規劃與套用放在一起，先 review 再改動會比較一致。

## 成功判準

- 你在開始前就能判斷這次 alert 變更屬於 export/import、撰寫、路由設計，還是 review/apply
- plan 或 export 可以一路走到 review，而不會把 policy 或 routing context 弄丟
- 同一條流程也能在 CI 或事故回顧時重跑

## 失敗時先檢查

- 如果 inventory 指令抓到的東西比預期少，先確認 auth scope 是否涵蓋需要的 org 或 folder
- 如果 review 或 apply 步驟怪怪的，先看 alert plan JSON，再決定是不是 CLI 真有問題
- 如果結果要交給自動化，請把輸出格式寫清楚，讓下游步驟知道 contract

## 範例

```bash
# 用途：執行 alerting 指令介面，用來匯出、匯入、比對、規劃、套用、刪除、撰寫與列出 Grafana alert 資源。
grafana-util alert list-rules --profile prod --json
```

```bash
# 用途：執行 alerting 指令介面，用來匯出、匯入、比對、規劃、套用、刪除、撰寫與列出 Grafana alert 資源。
grafana-util alert export --url http://localhost:3000 --basic-user admin --basic-password admin --output-dir ./alerts --overwrite
```

```bash
# 用途：執行 alerting 指令介面，用來匯出、匯入、比對、規劃、套用、刪除、撰寫與列出 Grafana alert 資源。
grafana-util alert export --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-dir ./alerts --flat
```

## 相關命令

- [alert export](./alert-export.md)
- [alert import](./alert-import.md)
- [alert diff](./alert-diff.md)
- [alert plan](./alert-plan.md)
- [alert apply](./alert-apply.md)
- [alert delete](./alert-delete.md)
- [alert add-rule](./alert-add-rule.md)
- [alert clone-rule](./alert-clone-rule.md)
- [alert add-contact-point](./alert-add-contact-point.md)
- [alert set-route](./alert-set-route.md)
- [alert preview-route](./alert-preview-route.md)
- [alert new-rule](./alert-new-rule.md)
- [alert new-contact-point](./alert-new-contact-point.md)
- [alert new-template](./alert-new-template.md)
- [alert list-rules](./alert-list-rules.md)
- [alert list-contact-points](./alert-list-contact-points.md)
- [alert list-mute-timings](./alert-list-mute-timings.md)
- [alert list-templates](./alert-list-templates.md)
- [access](./access.md)
