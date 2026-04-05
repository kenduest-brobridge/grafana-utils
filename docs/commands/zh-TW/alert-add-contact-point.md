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

## 採用前後對照

- 之前：自己手刻 contact point 檔案，還要記得 alert 撰寫結構。
- 之後：從較高階的撰寫介面直接產生一個暫存聯絡點檔案。

## 成功判準

- 聯絡點檔案出現在你預期的目標狀態樹裡。
- dry-run 會顯示你想寫入的 receiver 內容。

## 失敗時先檢查

- 先確認 `--desired-dir` 是不是正確的目標狀態樹。
- 如果名稱重複，先確認是不是已經有同名聯絡點。
- 這個命令只負責撰寫聯絡點，route 還要另外處理。

## 範例

```bash
# 用途：從較高階的撰寫介面建立一個暫存中的 alert 聯絡點。
grafana-util alert add-contact-point --desired-dir ./alerts/desired --name pagerduty-primary
```

```bash
# 用途：從較高階的撰寫介面建立一個暫存中的 alert 聯絡點。
grafana-util alert add-contact-point --desired-dir ./alerts/desired --name pagerduty-primary --dry-run
```

## 相關命令

- [alert](./alert.md)
- [alert set-route](./alert-set-route.md)
- [alert new-contact-point](./alert-new-contact-point.md)
