# `grafana-util alert clone-rule`

## 目的

將既有的暫存 alert 規則複製到新的撰寫目標。

## 使用時機

- 把現有規則當作變體的起點重用。
- 在複製時覆寫資料夾、規則群組、接收器或路由行為。

## 主要旗標

- `--desired-dir` 指向暫存的 alert 樹。
- `--source` 指定要複製的規則。
- `--name` 設定新的規則名稱。
- `--folder`、`--rule-group`、`--receiver` 與 `--no-route` 用來調整複製後的目標。
- `--dry-run` 預覽複製後的輸出。

## 範例

```bash
grafana-util alert clone-rule --desired-dir ./alerts/desired --source cpu-high --name cpu-high-staging --folder staging-alerts --rule-group cpu --receiver slack-platform
grafana-util alert clone-rule --desired-dir ./alerts/desired --source cpu-high --name cpu-high-staging --dry-run
```

## 相關命令

- [alert](./alert.md)
- [alert add-rule](./alert-add-rule.md)
- [alert new-rule](./alert-new-rule.md)
