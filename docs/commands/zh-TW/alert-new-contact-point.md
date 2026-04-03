# `grafana-util alert new-contact-point`

## 目的

建立一個較低階的暫存 alert 聯絡點骨架。

## 使用時機

- 在目標狀態樹中先建立新的聯絡點檔案。
- 先從骨架開始，再補齊接收器細節。

## 主要旗標

- `--desired-dir` 指向暫存的 alert 樹。
- `--name` 設定骨架名稱。

## 範例

```bash
grafana-util alert new-contact-point --desired-dir ./alerts/desired --name pagerduty-primary
grafana-util alert add-contact-point --desired-dir ./alerts/desired --name pagerduty-primary
```

## 相關命令

- [alert](./alert.md)
- [alert add-contact-point](./alert-add-contact-point.md)
- [alert set-route](./alert-set-route.md)
