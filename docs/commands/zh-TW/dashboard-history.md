# dashboard history

## 用途
列出、還原或匯出單一 dashboard UID 的版本歷史，來源可以是 live Grafana，也可以是本機 history 成品。

## 何時使用
當您需要檢查較早的 dashboard 版本、找回一個已知可用的版本，或把 dashboard 歷史匯出成可重用的成品給審查或 CI 時，使用這個指令。您也可以直接從單一匯出成品或包含 history 的 export tree 讀回同一份歷史。

還原時會建立一個新的最新 revision，不會直接覆蓋您選到的歷史版本。原本的舊版本仍然會留在 history 裡。

## history list 的來源

`dashboard history list` 可以直接讀 live Grafana，也可以讀本機成品：

- live：使用 `--url` 和 `--dashboard-uid`
- 單一 local 成品：使用 `dashboard history export` 產生的 `--input <history.json>`
- export tree：使用 `dashboard export --include-history` 產生的 `--input-dir <export-root>`

`dashboard history restore` 仍然只支援 live。

## 重點旗標
- `--dashboard-uid`：要檢視版本歷史的 dashboard UID。做 live history list 與 restore 時必填；讀取 local export tree 時也可用來過濾特定 dashboard。
- `--input`：讀取 `dashboard history export` 產生的一份可重用 history 成品。
- `--input-dir`：讀取 `dashboard export --include-history` 產生的 export tree。
- `--limit`：list 或 export 要包含多少個最近版本。
- `--version`：要還原的歷史版本號。
- `--message`：新還原 revision 要附帶的版本訊息。
- `--dry-run`：預覽還原，但不會真的變更 Grafana。
- `--yes`：確認真的執行還原。
- `--output-format`：把 list 或 restore 輸出成 text、table、json 或 yaml。
- `--output`：把匯出的歷史成品寫到 JSON 檔。
- `--overwrite`：覆蓋既有的匯出成品檔。

## 還原語意

- 被選到的歷史版本會被複製成新的最新 revision。
- 原本那個歷史版本仍然會保留在 dashboard history 中。
- `--dry-run` 只會顯示還原意圖，不會真的變更 Grafana。
- 真正要還原時，必須加上 `--yes`。

## 給 CI 用的 JSON contract

當自動化流程需要穩定判斷輸出文件時，直接用內建 schema help：

- `grafana-util dashboard history --help-schema`
- `grafana-util dashboard history list --help-schema`
- `grafana-util dashboard history restore --help-schema`
- `grafana-util dashboard history export --help-schema`

判斷順序建議固定成：

1. 先看 `kind`
2. 再確認 `schemaVersion`
3. 最後才往下讀巢狀欄位

常見對應：

- `dashboard history list --output-format json` -> `grafana-util-dashboard-history-list`
- `dashboard history list --input-dir ./dashboards --output-format json` -> 如果沒有再用 `--dashboard-uid` 縮小，會是 `grafana-util-dashboard-history-inventory`
- `dashboard history restore --dry-run --output-format json` -> `grafana-util-dashboard-history-restore`
- `dashboard history restore --output-format json` -> 同一種 contract，但 live 執行仍會建立新的 latest revision
- `dashboard history export --output ./cpu-main.history.json` -> `grafana-util-dashboard-history-export`

幾個值得先記住的 top-level 欄位：

- list -> `kind`、`schemaVersion`、`toolVersion`、`dashboardUid`、`versionCount`、`versions`
- list inventory -> `kind`、`schemaVersion`、`toolVersion`、`artifactCount`、`artifacts`
- restore -> `kind`、`schemaVersion`、`toolVersion`、`mode`、`dashboardUid`、`currentVersion`、`restoreVersion`、`currentTitle`、`restoredTitle`、可選的 `targetFolderUid`、`createsNewRevision`、`message`
- export -> `kind`、`schemaVersion`、`toolVersion`、`dashboardUid`、`currentVersion`、`currentTitle`、`versionCount`、`versions`

## 範例
```bash
# 用途：列出最近 20 個 dashboard revision，方便審查。
grafana-util dashboard history list --url http://localhost:3000 --basic-user admin --basic-password admin --dashboard-uid cpu-main --limit 20 --output-format table
```

```bash
# 用途：直接讀取一份匯出的 dashboard history 成品，並在本機列出版本。
grafana-util dashboard history list --input ./cpu-main.history.json --output-format table
```

```bash
# 用途：讀取包含 history 的 dashboard export tree，並列出指定 dashboard 的版本。
grafana-util dashboard history list --input-dir ./dashboards --dashboard-uid cpu-main --output-format table
```

```bash
# 用途：把某個歷史 dashboard revision 還原成新的最新 Grafana 版本。
grafana-util dashboard history restore --url http://localhost:3000 --basic-user admin --basic-password admin --dashboard-uid cpu-main --version 17 --message "Restore known good CPU dashboard after regression" --dry-run --output-format table
```

```bash
# 用途：把最近的 dashboard history 匯出成可重用的 JSON 成品。
grafana-util dashboard history export --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --dashboard-uid cpu-main --limit 20 --output ./cpu-main.history.json
```

## 採用前後對照

- **採用前**：dashboard 還原常常得先猜哪個舊版本可用，再手動重建，或直接改 JSON。
- **採用後**：同一個 history 指令群組就能列出、還原與匯出，讓操作人員先看清楚，再把同一份成品交給審查或 CI。

## 成功判準

- list 輸出會列出您預期的版本號與訊息
- restore 的 dry-run 會清楚顯示哪個版本將成為新的最新 revision
- 真正還原後，舊版本仍在 history 裡，而新的 current revision 會新增出來
- export 會寫出可重複使用的 JSON 成品，之後不連 Grafana 也能檢查

## 失敗時先檢查

- 如果 list 是空的，先確認 dashboard UID 與驗證資訊是否真的看得到那個 dashboard
- 如果 restore 失敗，先確認目標版本是否存在，以及您是否已經加上 `--yes`
- 如果 export 寫錯檔案或看起來像舊資料，先確認輸出路徑與 `--overwrite` 是否是您刻意要用的

## 相關指令
- [dashboard list](./dashboard-list.md)
- [dashboard analyze（即時）](./dashboard-analyze-live.md)
- [dashboard analyze（本地）](./dashboard-analyze-export.md)
- [dashboard review](./dashboard-review.md)
- [dashboard export](./dashboard-export.md)
