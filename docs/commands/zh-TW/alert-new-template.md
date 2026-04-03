# `grafana-util alert new-template`

## 目的

建立一個較低階的暫存 alert 範本骨架。

## 使用時機

- 在目標狀態樹中先建立新的通知範本檔案。
- 先從骨架開始，再補上範本內容。

## 主要旗標

- `--desired-dir` 指向暫存的 alert 樹。
- `--name` 設定骨架名稱。

## 範例

```bash
grafana-util alert new-template --desired-dir ./alerts/desired --name sev1-notification
```

## 相關命令

- [alert](./alert.md)
- [alert list-templates](./alert-list-templates.md)
