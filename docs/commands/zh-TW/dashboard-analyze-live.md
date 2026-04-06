# dashboard analyze-live

## 用途
透過 canonical 的 `dashboard analyze` 指令分析 live Grafana。

## 何時使用
當您需要和本地匯出樹相同的分析方式，但來源是線上 Grafana 而不是本地匯出樹時，使用這個頁面。新文件與腳本請優先使用 `grafana-util dashboard analyze --url ...`。

## 重點旗標
- `--page-size`：儀表板搜尋的每頁筆數。
- `--concurrency`：最大平行抓取工作數。
- `--org-id`：分析指定的 Grafana org。
- `--all-orgs`：跨所有可見 org 分析。
- `--output-format`、`--output-file`、`--interactive`、`--no-header`：輸出控制。
- `--progress`：顯示抓取進度。

## 範例
```bash
# 用途：透過 canonical 的 dashboard analyze 指令分析 live Grafana。
grafana-util dashboard analyze --profile prod --output-format governance
```

```bash
# 用途：透過 canonical 的 dashboard analyze 指令分析 live Grafana。
grafana-util dashboard analyze --url http://localhost:3000 --basic-user admin --basic-password admin --interactive
```

```bash
# 用途：透過 canonical 的 dashboard analyze 指令分析 live Grafana。
grafana-util dashboard analyze --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format governance
```

## 相關指令
- [dashboard analyze（本地）](./dashboard-analyze-export.md)
- [dashboard list-vars](./dashboard-list-vars.md)
- [dashboard governance-gate](./dashboard-governance-gate.md)
