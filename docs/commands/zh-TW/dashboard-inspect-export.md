# dashboard inspect-export

## 用途
以 operator-summary 與 report-contract 檢視角度分析儀表板匯出目錄。

## 何時使用
當您想讀取本地匯出樹、檢視其結構，或在不連到 Grafana 的情況下產生治理與相依性報告時，使用這個指令。

## 採用前後對照

- **採用前**：匯出樹只是一堆 JSON 檔案，還得自己猜哪些 dashboard、變數或治理檢查比較重要。
- **採用後**：跑一次 inspect，就能把匯出樹整理成維運人員看得懂的檢視，也能直接交給 CI 或後續的 `topology`、`governance-gate`。

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
# 用途：以 operator-summary 與 report-contract 檢視角度分析儀表板匯出目錄。
grafana-util dashboard inspect-export --import-dir ./dashboards/raw --input-format raw --table
```

```bash
# 用途：以 operator-summary 與 report-contract 檢視角度分析儀表板匯出目錄。
grafana-util dashboard inspect-export --import-dir ./dashboards/provisioning --input-format provisioning --report governance-json
```

## 成功判準

- 不必逐一打開 dashboard 檔案，也能說清楚匯出樹裡有哪些內容
- governance 或 dependency 輸出穩定到可以直接交給 CI 或另一位維護者
- 後續要跑 `dashboard topology`、`dashboard impact`、`dashboard governance-gate` 時，可以直接從 inspect 產物開始，不用再重新讀原始匯出樹

## 失敗時先檢查

- 如果匯出樹看起來不完整，先確認你指的是 `raw` 還是 `provisioning` 內容
- 如果後續命令讀不進去，先確認你輸出的是 `governance-json` 還是別的 report 格式
- 如果匯出樹來自較舊的匯出結果，先重跑 `dashboard export`，避免分析到過期檔案

## 相關指令
- [dashboard export](./dashboard-export.md)
- [dashboard diff](./dashboard-diff.md)
- [dashboard governance-gate](./dashboard-governance-gate.md)
