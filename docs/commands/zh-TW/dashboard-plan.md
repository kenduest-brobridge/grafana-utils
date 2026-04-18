# `grafana-util dashboard plan`

## 用途
依據本地 dashboard export tree 與遠端 Grafana live 狀態，產生 review-first 的 dashboard reconcile plan。

## 何時使用
當你想先確認一份 dashboard export 套到目標 Grafana 會造成什麼結果，再決定是否 import、重新 export、prune 遠端多出的 dashboards，或檢查 dependency warnings 時，使用這個命令。

`dashboard plan` 不會修改 Grafana。它會把 local-vs-live dashboard 差異轉成 operator action，例如 `same`、`would-create`、`would-update`、`extra-remote`、`would-delete`、`blocked-target`。
它仍然是 read-only 的 review surface。`--use-export-org` 會把 combined multi-org export root 對應回相同的 target org ID，且需要 Basic auth，讓命令可以解析 live org routing。

## 重要參數
- `--input-dir`: 本地 dashboard export root 或 dashboard variant directory。
- `--input-type`: 選擇 `raw` 或 `source`。prompt-lane exports 使用 `source`。
- `--org-id`: 對指定 Grafana org 做 plan。
- `--use-export-org`: 依 export 出來的 org ID 來路由 combined multi-org export root，而不是只對單一 target org。
- `--only-org-id`: 只針對一個或多個 exported source org ID 做 plan。
- `--create-missing-orgs`: 將缺少的 destination org 保留成 review-only 的 `would-create` 項目。
- `--prune`: 將遠端多出的 dashboards 顯示成 `would-delete` candidates。沒有這個參數時只顯示為 `extra-remote`。
- `--output-format`: 選擇 `text`、`table` 或 `json`。
- `--show-same`: 在 text/table 輸出中包含沒有變更的 row。
- `--output-columns`, `--list-columns`, `--no-header`: 調整 table 輸出。

## 範例
```bash
# 對 raw dashboard export 產生摘要 plan。
grafana-util dashboard plan --profile prod --input-dir ./dashboards/raw
```

```bash
# 用 table 顯示指定 review columns。
grafana-util dashboard plan --profile prod --input-dir ./dashboards/raw --output-format table --output-columns actionId,dashboardTitle,folderPath,status
```

```bash
# Review prompt/source export tree。
grafana-util dashboard plan --profile prod --input-dir ./dashboards/prompt --input-type source --output-format json
```

```bash
# 對 combined all-org export root 做 plan，並對應到相同的 target org ID。
grafana-util dashboard plan --profile prod --input-dir ./dashboards --use-export-org --output-format json
```

```bash
# 只針對某個 exported source org 做 plan。
grafana-util dashboard plan --profile prod --input-dir ./dashboards --use-export-org --only-org-id 42 --output-format table
```

```bash
# 缺少 destination org 時，以 review-only 的 would-create 顯示。
grafana-util dashboard plan --profile prod --input-dir ./dashboards --use-export-org --only-org-id 42 --create-missing-orgs --output-format json
```

```bash
# 將遠端多出的 dashboards 納入刪除候選。
grafana-util dashboard plan --profile prod --input-dir ./dashboards/raw --prune --output-format json
```

## Before / After

- **Before**: dashboard import 與 diff 能看到部分 local-vs-live state，但沒有一個 dashboard-specific 的 reconcile review document。
- **After**: `dashboard plan` 用同一個 review model 顯示 create、update、remote-only、delete-candidate、blocked 與 warning rows。
- JSON 輸出保留穩定 `actionId`、status、changed fields、target evidence、dependency hints 與 review hints，可供 CI 或後續 TUI 使用。
- multi-org export root 會根據 exported org ID 來路由，不是只看檔案目錄名稱。

## 成功狀態

- import 前可看出 create 與 update candidates
- 遠端多出的 dashboards 預設只被標示，不會被刪除
- provisioning 或 managed target 會先被 blocked
- unresolved datasource references 與 folder 問題會顯示為 review hints
- `--create-missing-orgs` 只影響 plan 呈現，不會真的建立 org

## 失敗檢查

- 如果 plan 指到錯誤 org，確認 `--org-id` 或 selected profile
- 如果需要 `--use-export-org`，請確認 export root 是 combined multi-org export，而且已設定 Basic auth
- 如果有 `--only-org-id`，確認 export metadata 裡真的有那些 source org ID
- 如果 delete candidates 看起來不合理，先移除 `--prune` 並檢查 `extra-remote`
- 如果出現 dependency hints，確認目標 Grafana 有預期的 datasource inventory 與 folder structure

## 相關命令
- [dashboard export](./dashboard-export.md)
- [dashboard import](./dashboard-import.md)
- [dashboard diff](./dashboard-diff.md)
- [dashboard dependencies](./dashboard-dependencies.md)
