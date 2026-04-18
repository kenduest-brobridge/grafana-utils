# dashboard dependencies

## 用途
分析本地 dashboard 匯出樹，整理相依性、治理與查詢結構，讓你在不連到 Grafana 的情況下也能先看清楚內容。

## 何時使用
當你的來源是本地匯出樹，而不是 live Grafana 時，就先開這頁。它適合拿來做離線盤點、產出治理成品、提供 `policy` / `impact` / CI 重用，或在 review 前先確認匯出樹內容。

## 最短成功路徑

1. 先確認手上的資料是 `raw`、`provisioning` 還是 `git-sync`。
2. 指向匯出根目錄：`--input-dir ...`。
3. 先跑一次 `--output-format table` 或 `--output-format governance-json`。
4. 結果可信後，再決定要不要接 `policy`、`impact` 或 CI。

## 你應該選 `dependencies` 還是 `summary`

- 來源是 **本地匯出樹**：用 `dashboard dependencies`
- 來源是 **live Grafana**：改看 [dashboard summary](./dashboard-summary.md)
- 你要追某個 datasource 的牽動範圍：改看 [dashboard impact](./dashboard-impact.md)
- 你只想做 dashboard 草稿比對：先看 `dashboard diff` / `dashboard review`

## 採用前後對照

- **採用前**：匯出樹只是一堆 JSON 檔案，還得自己猜哪些 dashboard、變數或治理檢查比較重要。
- **採用後**：跑一次 `dashboard dependencies`，就能把匯出樹整理成維運人員看得懂的檢視，也能直接交給 CI 或後續的 `dependencies`、`policy`。

## 重點旗標
- `--input-dir`：要分析的儀表板匯出根目錄。
- `--input-format`：選擇 `raw`、`provisioning` 或 `git-sync`。
- `--input-type`：當匯出根目錄包含多種儀表板變體時，選擇 `raw` 或 `source`。
- `--output-format`：輸出 `text`、`table`、`csv`、`json`、`yaml`、`tree`、`tree-table`、`dependency`、`dependency-json`、`governance`、`governance-json` 或 `queries-json` 檢視。
- `--report-columns`：把 table、csv 或 tree-table 的 query 輸出裁成指定欄位。可用 `all` 展開完整 query 欄位集合。
- `--list-columns`：列出支援的 `--report-columns` 值後直接結束。
- `--interactive`：開啟共用分析工作台。
- `--output-file`：將結果寫到磁碟。
- `--no-header`：隱藏表格類輸出的標頭。

## 先決定你要輸出什麼

- `table` / `tree-table`：給人快速盤點匯出樹
- `dependency` / `dependency-json`：給相依性檢視或其他工具接手
- `governance` / `governance-json`：給 `policy`、`impact` 或治理流程重用
- `queries-json`：給查詢層分析
- `interactive`：你已經確認來源正確，想現場往下鑽時再開

## 範例（由淺到深）

```bash
# 先用 table 盤點本地 raw 匯出樹。
grafana-util dashboard dependencies --input-dir ./dashboards/raw --input-format raw --output-format table
```

```bash
# 產生可重用的 governance JSON，留給 policy / impact / CI。
grafana-util dashboard dependencies --input-dir ./dashboards/provisioning --input-format provisioning --output-format governance-json
```

```bash
# 分析 repo-backed Git Sync 樹，保留治理輸出給下一步。
grafana-util dashboard dependencies --input-dir ./grafana-oac-repo --input-format git-sync --output-format governance
```

## 什麼叫做這次跑成功

- 不必逐一打開 JSON，也能說清楚匯出樹裡有哪些 dashboard、查詢與相依關係
- 產物格式足以直接交給下一步，而不是還要重新檢視一次
- 同一份輸出可以支撐 review、治理或影響分析

## 成功判準

- 不必逐一打開 dashboard 檔案，也能說清楚匯出樹裡有哪些內容
- governance 或 dependency 輸出穩定到可以直接交給 CI 或另一位維護者
- 後續要跑 `dashboard dependencies`、`dashboard impact`、`dashboard policy` 時，可以直接從 review 成品開始，不用再重新讀原始匯出樹

## 失敗時先檢查

- 如果匯出樹看起來不完整，先確認你指的是 `raw` 還是 `provisioning` 內容
- 如果後續命令讀不進去，先確認你輸出的是 `governance-json` 還是別的 review 成品格式
- 如果匯出樹來自較舊的匯出結果，先重跑 `dashboard export`，避免檢視到過期檔案

## 相關指令
- [dashboard export](./dashboard-export.md)
- [dashboard diff](./dashboard-diff.md)
- [dashboard policy](./dashboard-policy.md)
