# `grafana-util alert add-rule`

## 目的

從較高階的撰寫介面建立一個暫存中的 alert 規則。

## 使用時機

- 在目標狀態 alert 樹中建立新的規則。
- 一次加入標籤、註解、嚴重性與閾值邏輯。
- 除非明確略過，否則會一併為規則建立路由。

## 主要旗標

- `--desired-dir` 指向暫存的 alert 樹。
- `--name`、`--folder` 和 `--rule-group` 定義規則放置位置。
- `--receiver` 或 `--no-route` 控制路由撰寫。
- `--label`、`--annotation`、`--severity`、`--for`、`--expr`、`--threshold`、`--above` 與 `--below` 決定規則內容。
- `--dry-run` 預覽即將輸出的檔案。

## 範例

```bash
grafana-util alert add-rule --desired-dir ./alerts/desired --name cpu-high --folder platform-alerts --rule-group cpu --receiver pagerduty-primary --severity critical --expr 'A' --threshold 80 --above --for 5m --label team=platform --annotation summary='CPU high'
grafana-util alert add-rule --desired-dir ./alerts/desired --name cpu-high --folder platform-alerts --rule-group cpu --receiver pagerduty-primary --dry-run
```

## 相關命令

- [alert](./alert.md)
- [alert clone-rule](./alert-clone-rule.md)
- [alert new-rule](./alert-new-rule.md)
