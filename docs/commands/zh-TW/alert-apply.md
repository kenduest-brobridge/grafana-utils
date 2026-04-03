# `grafana-util alert apply`

## 目的

套用一份已審閱過的 alert 管理計畫。

## 使用時機

- 執行已在離線環境審閱完成的計畫。
- 在碰觸 Grafana 之前要求明確確認。

## 主要旗標

- `--plan-file` 指向已審閱的計畫文件。
- `--approve` 是允許執行前的必要確認。
- `--output` 可將套用輸出呈現為 `text` 或 `json`。

## 範例

```bash
grafana-util alert apply --profile prod --plan-file ./alert-plan-reviewed.json --approve
grafana-util alert apply --url http://localhost:3000 --basic-user admin --basic-password admin --plan-file ./alert-plan-reviewed.json --approve
grafana-util alert apply --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --plan-file ./alert-plan-reviewed.json --approve
```

## 相關命令

- [alert](./alert.md)
- [alert plan](./alert-plan.md)
- [alert delete](./alert-delete.md)
