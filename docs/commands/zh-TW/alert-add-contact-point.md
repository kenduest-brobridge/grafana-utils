# `grafana-util alert add-contact-point`

## 目的

從較高階的撰寫介面建立一個暫存中的 alert 聯絡點。

## 使用時機

- 在目標狀態 alert 樹中建立新的聯絡點。
- 在寫入前先預覽即將產生的檔案。

## 主要旗標

- `--desired-dir` 指向暫存的 alert 樹。
- `--name` 設定聯絡點名稱。
- `--dry-run` 預覽規劃後的輸出。

## 範例

```bash
grafana-util alert add-contact-point --desired-dir ./alerts/desired --name pagerduty-primary
grafana-util alert add-contact-point --desired-dir ./alerts/desired --name pagerduty-primary --dry-run
```

## 相關命令

- [alert](./alert.md)
- [alert set-route](./alert-set-route.md)
- [alert new-contact-point](./alert-new-contact-point.md)
