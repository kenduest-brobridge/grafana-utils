# datasource plan

## 用途
依據本地 datasource bundle 與遠端 Grafana live 狀態，產生 review-first 的 reconcile plan。

## 何時使用
當你想先確認 datasource bundle 套到目標 Grafana 會造成什麼結果，再決定是否 import、重新 export，或處理遠端多出的 datasource 時，使用這個命令。

`datasource plan` 不會修改 Grafana。它會把 bundle-vs-live 差異轉成 operator action，例如 `would-create`、`would-update`、`extra-remote`、`would-delete`、`blocked-read-only`。

## 重要參數
- `--input-dir`: 本地 datasource bundle、workspace root、provisioning 目錄，或具體 provisioning YAML 檔案。
- `--input-format`: 選擇 `inventory` 或 `provisioning`。
- `--org-id`: 對指定 Grafana org 做 plan。
- `--use-export-org`, `--only-org-id`, `--create-missing-orgs`: 將 all-org datasource export 依匯出 org route 回目標 org。
- `--prune`: 將遠端多出的 datasource 顯示成 `would-delete` candidate。沒有這個參數時只顯示為 `extra-remote`。
- `--output-format`: 選擇 `text`、`table` 或 `json`。
- `--show-same`: 在 text/table 輸出中包含沒有變更的 row。
- `--output-columns`, `--list-columns`, `--no-header`: 調整 table 輸出。

## 範例
```bash
# 對一份 datasource bundle 產生摘要 plan。
grafana-util datasource plan --profile prod --input-dir ./datasources
```

```bash
# 用 table 顯示 action rows。
grafana-util datasource plan --profile prod --input-dir ./datasources --output-format table
```

```bash
# 將遠端多出的 datasource 納入刪除候選。
grafana-util datasource plan --profile prod --input-dir ./datasources --prune --output-format json
```

```bash
# 對 all-org export 依來源 org route 回目標 org 做 plan。
grafana-util datasource plan --url http://localhost:3000 --basic-user admin --basic-password admin --input-dir ./datasources --use-export-org --output-format table
```

## Before / After

- **Before**: `datasource diff` 只顯示 local-vs-live 差異，`datasource import --dry-run` 只 preview import records，沒有一個完整 reconcile review surface。
- **After**: `datasource plan` 用同一個 review model 顯示 create、update、remote-only、delete-candidate 與 blocked actions。
- JSON 輸出保留穩定 `actionId`、status、target evidence、changed fields 與 review hints，可供 CI 或後續 TUI 使用。

## 成功狀態

- import 前可看出 create 與 update candidates
- 遠端多出的 datasource 預設只被標示，不會被刪除
- read-only 或 provisioning 管理的 target 會先被 blocked
- JSON output 可作為 review evidence 或 automation input

## 失敗檢查

- 如果 plan 指到錯誤 org，確認 `--org-id` 或 `--use-export-org`
- 如果 `--use-export-org` 失敗，確認 input 是 combined inventory export root，且 credentials 可以列舉 org
- 如果 delete candidates 看起來不合理，先移除 `--prune` 並檢查 `extra-remote`
- 如果 secret 相關差異無法完全判斷，記得 Grafana live API 不會回傳 plaintext datasource secrets

## 相關命令
- [datasource diff](./datasource-diff.md)
- [datasource import](./datasource-import.md)
- [datasource export](./datasource-export.md)
- [datasource delete](./datasource-delete.md)
