# `grafana-util alert add-rule`

## 目的

從較高階的撰寫介面建立一個暫存中的 alert 規則。

## 使用時機

- 在目標狀態 alert 樹中建立新的規則。
- 一次加入標籤、註解、嚴重性與閾值邏輯。
- 除非明確略過，否則會一併為規則建立路由。

## 採用前後對照

- **採用前**：新增一條 alert 規則常常得自己手動拼 YAML、補路由，還要擔心欄位漏掉。
- **採用後**：規則內容、標籤、閾值與路由可以一次在 CLI 裡補齊，之後再進入 review 或 apply 流程。

## 成功判準

- 規則的名稱、folder、rule group、條件與路由會一起產生，不需要另外補手工檔案
- 暫存目錄裡的輸出足夠讓另一位維護者直接 review
- `--dry-run` 能先看出即將輸出的檔案形狀與路由結果

## 失敗時先檢查

- 如果建立規則失敗，先確認 `--desired-dir` 指向的是正確的暫存 alert 樹
- 如果路由沒有建立，先確認是不是用了 `--no-route` 或沒有提供 `--receiver`
- 如果你要把結果交給後續 apply，先看 `--dry-run` 或輸出的檔案內容再繼續

## 主要旗標

- `--desired-dir` 指向暫存的 alert 樹。
- `--name`、`--folder` 和 `--rule-group` 定義規則放置位置。
- `--receiver` 或 `--no-route` 控制路由撰寫。
- `--label`、`--annotation`、`--severity`、`--for`、`--expr`、`--threshold`、`--above` 與 `--below` 決定規則內容。
- `--dry-run` 預覽即將輸出的檔案。

## 範例

```bash
# 用途：從較高階的撰寫介面建立一個暫存中的 alert 規則。
grafana-util alert add-rule --desired-dir ./alerts/desired --name cpu-high --folder platform-alerts --rule-group cpu --receiver pagerduty-primary --severity critical --expr 'A' --threshold 80 --above --for 5m --label team=platform --annotation summary='CPU high'
```

```bash
# 用途：從較高階的撰寫介面建立一個暫存中的 alert 規則。
grafana-util alert add-rule --desired-dir ./alerts/desired --name cpu-high --folder platform-alerts --rule-group cpu --receiver pagerduty-primary --dry-run
```

## 相關命令

- [alert](./alert.md)
- [alert clone-rule](./alert-clone-rule.md)
- [alert new-rule](./alert-new-rule.md)
