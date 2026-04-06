# dashboard list-vars

## 用途
列出線上 Grafana 的儀表板模板變數與類似 datasource 的選項。

## 何時使用
當您需要列出變數狀態、供截圖工作流程使用，或排查受變數範圍影響的儀表板 URL 時，使用這個指令。

## 重點旗標
- `--dashboard-uid` 或 `--dashboard-url`：選擇要列出變數的儀表板。
- `--vars-query`：疊加變數查詢字串，例如 `var-env=prod&var-host=web01`。
- `--org-id`：將檢查限制在單一 org。
- `--output-format`：輸出 `table`、`csv`、`text`、`json` 或 `yaml`。
- `--no-header`：隱藏表格或 CSV 標頭。
- `--output-file`：將輸出複本寫到磁碟。

## 範例
```bash
# 用途：列出線上 Grafana 的儀表板模板變數與類似 datasource 的選項。
grafana-util dashboard list-vars --profile prod --dashboard-uid cpu-main --vars-query 'var-cluster=prod-a&var-instance=node01' --output-format json
```

```bash
# 用途：列出線上 Grafana 的儀表板模板變數與類似 datasource 的選項。
grafana-util dashboard list-vars --url https://grafana.example.com --dashboard-uid cpu-main --vars-query 'var-cluster=prod-a&var-instance=node01' --basic-user admin --prompt-password --output-format json
```

```bash
# 用途：列出線上 Grafana 的儀表板模板變數與類似 datasource 的選項。
grafana-util dashboard list-vars --dashboard-url 'https://grafana.example.com/d/cpu-main/cpu-overview?var-cluster=prod-a' --token "$GRAFANA_API_TOKEN" --output-format table
```

## 相關指令
- [dashboard screenshot](./dashboard-screenshot.md)
- [dashboard analyze（即時）](./dashboard-analyze-live.md)
- [dashboard browse](./dashboard-browse.md)
