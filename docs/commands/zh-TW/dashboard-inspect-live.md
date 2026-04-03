# dashboard inspect-live

## 用途
透過暫時的 raw export 快照分析線上 Grafana 儀表板。

## 何時使用
當您需要和 `inspect-export` 相同的檢視方式，但來源是線上 Grafana 而不是本地匯出樹時，使用這個指令。

## 重點旗標
- `--page-size`：儀表板搜尋的每頁筆數。
- `--concurrency`：最大平行抓取工作數。
- `--org-id`：檢查指定的 Grafana org。
- `--all-orgs`：跨所有可見 org 檢查。
- `--report`、`--output-format`、`--output-file`、`--interactive`、`--no-header`：輸出控制。
- `--progress`：顯示抓取進度。

## 範例
```bash
grafana-util dashboard inspect-live --profile prod --output-format governance-json
grafana-util dashboard inspect-live --url http://localhost:3000 --basic-user admin --basic-password admin --interactive
grafana-util dashboard inspect-live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format governance-json
```

## 相關指令
- [dashboard inspect-export](./dashboard-inspect-export.md)
- [dashboard inspect-vars](./dashboard-inspect-vars.md)
- [dashboard governance-gate](./dashboard-governance-gate.md)
