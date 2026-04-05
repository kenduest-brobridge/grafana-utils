# `grafana-util alert new-contact-point`

## 目的

建立一個較低階的暫存 alert 聯絡點骨架。

## 使用時機

- 在目標狀態樹中先建立新的聯絡點檔案。
- 先從骨架開始，再補齊接收器細節。

## 主要旗標

- `--desired-dir` 指向暫存的 alert 樹。
- `--name` 設定骨架名稱。

## 採用前後對照

- 之前：從空白檔案開始，很多聯絡點欄位都得自己記。
- 之後：先生出一個骨架檔，之後再補 receiver 細節。

## 成功判準

- 骨架檔出現在你預期的目標狀態樹裡。
- 產出的檔案是一個乾淨的起點，方便後續補細節。

## 失敗時先檢查

- 如果骨架沒有落在預期位置，先看 `--desired-dir`。
- 如果名稱衝到現有檔案，先換一個聯絡點名稱。

## 範例

```bash
# 用途：建立一個較低階的暫存 alert 聯絡點骨架。
grafana-util alert new-contact-point --desired-dir ./alerts/desired --name pagerduty-primary
```

```bash
# 用途：建立一個較低階的暫存 alert 聯絡點骨架。
grafana-util alert add-contact-point --desired-dir ./alerts/desired --name pagerduty-primary
```

## 相關命令

- [alert](./alert.md)
- [alert add-contact-point](./alert-add-contact-point.md)
- [alert set-route](./alert-set-route.md)
