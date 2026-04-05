# `grafana-util alert new-rule`

## 目的

建立一個較低階的暫存 alert 規則骨架。

## 使用時機

- 在目標狀態樹中先建立新的規則檔案。
- 先從簡單骨架開始，再補上規則細節。

## 主要旗標

- `--desired-dir` 指向暫存的 alert 樹。
- `--name` 設定骨架名稱。

## 範例

```bash
# 用途：建立一個較低階的暫存 alert 規則骨架。
grafana-util alert new-rule --desired-dir ./alerts/desired --name cpu-main
```

```bash
# 用途：建立一個較低階的暫存 alert 規則骨架。
grafana-util alert add-rule --desired-dir ./alerts/desired --name cpu-main --folder platform-alerts --rule-group cpu --receiver pagerduty-primary
```

## 相關命令

- [alert](./alert.md)
- [alert add-rule](./alert-add-rule.md)
- [alert clone-rule](./alert-clone-rule.md)
