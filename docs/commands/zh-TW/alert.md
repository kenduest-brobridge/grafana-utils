# alert

## 這一頁對應的工作流

| 工作流 | 常用子命令 |
| --- | --- |
| 盤點現況 | `list-rules`、`list-contact-points`、`list-mute-timings`、`list-templates` |
| 匯出 / 匯入 / 比對 | `export`、`import`、`diff` |
| 變更規劃與套用 | `plan`、`apply`、`delete` |
| 規則 / 聯絡點 / 路由撰寫 | `new-rule`、`add-rule`、`clone-rule`、`new-contact-point`、`add-contact-point`、`new-template`、`set-route`、`preview-route` |

## 從這裡開始

- 先盤點：`alert list-rules`、`alert list-contact-points`
- 先看現況再改：`alert export`、`alert diff`
- 先規劃再套用：`alert plan`、`alert apply`
- 先建草稿：`alert new-rule`、`alert new-contact-point`、`alert new-template`
- 先調路由：`alert set-route`、`alert preview-route`

## 說明

`grafana-util alert` 把告警工作流收在同一個入口：從盤點、匯出、比對，到路由設計、草稿撰寫，再到 plan / apply。這頁適合先搞懂規則、通知路由與 contact point 的關係，再決定要往哪個子命令深入。

新的 canonical 分組方式是：

- `alert live ...`
- `alert migrate ...`
- `alert author ...`
- `alert scaffold ...`
- `alert plan/apply ...`

## 採用前後對照

- **採用前**：告警工作常分散在 UI、臨時 export，或不容易重跑的 shell 指令裡。
- **採用後**：同一個命令群組就能把 inventory、撰寫、diff、規劃與套用放在一起。

## 成功判準

- 你在開始前就能判斷這次 alert 變更屬於盤點、撰寫、路由設計，還是 review / apply
- plan 或 export 可以一路走到 review，而不會把 policy 或 routing context 弄丟
- 同一條流程也能在 CI 或事故回顧時重跑

## 失敗時先檢查

- 如果 inventory 指令抓到的東西比預期少，先確認 auth scope 是否涵蓋需要的 org 或 folder
- 如果 review 或 apply 步驟怪怪的，先看 alert plan JSON，再決定是不是 CLI 真有問題
- 如果結果要交給自動化，請把輸出格式寫清楚，讓下游步驟知道 contract

## 主要旗標

- `--profile`、`--url`、`--token`、`--basic-user`、`--basic-password`
- `--prompt-password`、`--prompt-token`、`--timeout`、`--verify-ssl`
- 巢狀子命令主要分成 `live`、`export`、`import`、`diff`、`plan`、`apply`、`author`、`scaffold`

## 範例

```bash
# 先盤點目前有哪些 alert 規則。
grafana-util alert live list-rules --profile prod --json
```

```bash
# 先把現況匯出，再拿去做 diff 或 review。
grafana-util alert migrate export --url http://localhost:3000 --basic-user admin --basic-password admin --output-dir ./alerts --overwrite
```

```bash
# 先試跑規劃，不要直接套用。
grafana-util alert plan --desired-dir ./alerts/desired --output-format json
```

## 相關命令

### 盤點

- [alert list-rules](./alert-list-rules.md)
- [alert list-contact-points](./alert-list-contact-points.md)
- [alert list-mute-timings](./alert-list-mute-timings.md)
- [alert list-templates](./alert-list-templates.md)

### 搬移

- [alert export](./alert-export.md)
- [alert import](./alert-import.md)
- [alert diff](./alert-diff.md)

### 變更前檢查

- [alert plan](./alert-plan.md)
- [alert apply](./alert-apply.md)
- [alert delete](./alert-delete.md)

### 規則與路由撰寫

- [alert add-rule](./alert-add-rule.md)
- [alert clone-rule](./alert-clone-rule.md)
- [alert add-contact-point](./alert-add-contact-point.md)
- [alert set-route](./alert-set-route.md)
- [alert preview-route](./alert-preview-route.md)
- [alert new-rule](./alert-new-rule.md)
- [alert new-contact-point](./alert-new-contact-point.md)
- [alert new-template](./alert-new-template.md)
- [access](./access.md)
