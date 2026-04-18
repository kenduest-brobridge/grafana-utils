# `grafana-util access plan`

## 用途
依據本地 access bundle 與遠端 Grafana live 狀態，產生 review-first 的 access reconcile plan。

## 何時使用
當你想在 import 或 prune access state 前，先用結構化方式確認「會改什麼」，使用這個命令。

目前實作是 user bundle 的垂直切片。它會規劃 `user` resources；非 user 的 resource selector 會明確回報尚未支援，不會假裝已完成 review。

## 重要參數
- `--input-dir`: 要 review 的本地 access export bundle。
- `--resource`: 選擇 `user`、`team`、`org`、`service-account` 或 `all`。目前切片支援 `user`。
- `--prune`: 將遠端多出的 users 顯示成刪除候選。沒有這個參數時只會當成 extra remote review items。
- `--output-format`: 選擇 `text`、`table` 或 `json`。
- `--show-same`: 在 text/table 輸出中包含沒有變更的 row。
- `--output-columns`, `--list-columns`, `--no-header`: 調整 table 輸出。

## 範例
```bash
# 對 user access bundle 產生摘要 plan。
grafana-util access plan --profile prod --input-dir ./access-users
```

```bash
# 用 table 顯示 review rows。
grafana-util access plan --profile prod --input-dir ./access-users --resource user --output-format table
```

```bash
# 將遠端多出的 users 納入刪除候選。
grafana-util access plan --profile prod --input-dir ./access-users --resource user --prune --output-format json
```

## Before / After

- **Before**: access export、import、diff 依 resource type 分散，review user bundle 時仍可能需要人工推理才敢 mutation。
- **After**: `access plan` 會輸出單一 review document，用穩定 action rows 表示 user bundle reconcile 結果。
- JSON 輸出保留穩定 `actionId`、status、changed fields、target details、blocked reason 與 review hints，可供 CI 或後續 TUI 使用。

## 成功狀態

- import 前可看出 user create、update、same 與 remote-only 狀態
- delete candidates 必須明確使用 `--prune`
- JSON output 可作為 review evidence 或 automation input

## 失敗檢查

- 如果命令回報 selected resource 尚未支援，這個切片先改用 `--resource user`
- 如果 plan 指到錯誤 org，確認 profile 或 shared auth flags
- 如果 delete candidates 看起來不合理，先移除 `--prune` 並檢查 extra remote rows

## 相關命令
- [access user](./access-user.md)
- [access org](./access-org.md)
- [access team](./access-team.md)
- [access service-account](./access-service-account.md)
