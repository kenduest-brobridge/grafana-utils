# dashboard inspect-export

## 用途
以 operator-summary 與 report-contract 檢視角度分析儀表板匯出目錄。

## 何時使用
當您想讀取本地匯出樹、檢視其結構，或在不連到 Grafana 的情況下產生治理與相依性報告時，使用這個指令。

## 重點旗標
- `--import-dir`：要檢查的儀表板匯出根目錄。
- `--input-format`：選擇 `raw` 或 `provisioning`。
- `--input-type`：當匯出根目錄包含多種儀表板變體時，選擇 `raw` 或 `source`。
- `--report`：輸出 `table`、`csv`、`json`、`tree`、`tree-table`、`dependency`、`dependency-json`、`governance` 或 `governance-json` 檢視。
- `--output-format`：單一旗標的輸出選擇器。
- `--interactive`：開啟共用 inspect 工作台。
- `--output-file`：將結果寫到磁碟。
- `--no-header`：隱藏表格類輸出的標頭。

## 範例
```bash
grafana-util dashboard inspect-export --import-dir ./dashboards/raw --input-format raw --table
grafana-util dashboard inspect-export --import-dir ./dashboards/provisioning --input-format provisioning --report governance-json
```

## 相關指令
- [dashboard export](./dashboard-export.md)
- [dashboard diff](./dashboard-diff.md)
- [dashboard governance-gate](./dashboard-governance-gate.md)
