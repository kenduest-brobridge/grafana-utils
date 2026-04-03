# dashboard diff

## 用途
比較本地儀表板檔案與線上 Grafana 儀表板的差異。

## 何時使用
當您想在匯入或發佈儀表板 bundle 之前先看出會變更哪些內容時，使用這個指令。

## 重點旗標
- `--import-dir`：拿這個匯出目錄與 Grafana 比對。
- `--input-format`：選擇 `raw` 或 `provisioning`。
- `--import-folder-uid`：覆寫比對時的目的資料夾 UID。
- `--context-lines`：統一 diff 的上下文行數。

## 範例
```bash
grafana-util dashboard diff --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards/raw
grafana-util dashboard diff --url http://localhost:3000 --basic-user admin --basic-password admin --org-id 2 --import-dir ./dashboards/raw --json
```

## 相關指令
- [dashboard export](./dashboard-export.md)
- [dashboard import](./dashboard-import.md)
- [dashboard inspect-export](./dashboard-inspect-export.md)
