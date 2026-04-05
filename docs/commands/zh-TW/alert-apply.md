# `grafana-util alert apply`

## 目的

套用一份已審閱過的 alert 管理計畫。

## 使用時機

- 執行已在離線環境審閱完成的計畫。
- 在碰觸 Grafana 之前要求明確確認。

## 採用前後對照

- **採用前**：即使 alert plan 已審閱完成，真正碰到 live Grafana 的最後一步仍然很容易變成不透明的人工操作。
- **採用後**：`alert apply` 會把最後一步變成明確命令，保留核准、可重現驗證與可機器判讀的輸出。

## 主要旗標

- `--plan-file` 指向已審閱的計畫文件。
- `--approve` 是允許執行前的必要確認。
- `--output-format` 可將套用輸出呈現為 `text` 或 `json`。

## 範例

```bash
# 用途：套用一份已審閱過的 alert 管理計畫。
grafana-util alert apply --profile prod --plan-file ./alert-plan-reviewed.json --approve
```

```bash
# 用途：套用一份已審閱過的 alert 管理計畫。
grafana-util alert apply --url http://localhost:3000 --basic-user admin --basic-password admin --plan-file ./alert-plan-reviewed.json --approve
```

```bash
# 用途：套用一份已審閱過的 alert 管理計畫。
grafana-util alert apply --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --plan-file ./alert-plan-reviewed.json --approve
```

## 成功判準

- 已審閱的 alert plan 能直接套用，不必再手改 YAML 或重做一串 UI 操作
- live apply 步驟會保留明確的核准，不會只藏在 shell history 裡
- JSON 輸出穩定到足以交給 CI、變更紀錄或套用後驗證流程

## 失敗時先檢查

- 如果 apply 不肯執行，先確認你給的是 reviewed plan，而且有帶 `--approve`
- 如果 live 結果和預期 plan 差很多，先核對認證、org scope，以及 reviewed plan 是否仍對應目前目標 Grafana
- 如果自動化要吃輸出，建議用 `--output-format json`，並先驗證結果 shape 再判定成功

## 相關命令

- [alert](./alert.md)
- [alert plan](./alert-plan.md)
- [alert delete](./alert-delete.md)
