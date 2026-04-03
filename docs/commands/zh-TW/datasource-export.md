# datasource export

## 用途
將線上 Grafana datasource inventory 匯出成標準化 JSON 與 provisioning 檔案。

## 何時使用
當您需要一個本地 datasource bundle，供後續檢查、比對或匯入時，使用這個指令。

## 重點旗標
- `--export-dir`：匯出樹的目標目錄。
- `--org-id`：匯出指定的 Grafana org。
- `--all-orgs`：把每個可見 org 匯出到各自的子目錄。需要 Basic auth。
- `--overwrite`：取代既有檔案。
- `--without-datasource-provisioning`：略過 provisioning 變體。
- `--dry-run`：預覽會寫出哪些內容。

## 範例
```bash
grafana-util datasource export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./datasources --overwrite
grafana-util datasource export --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --export-dir ./datasources --overwrite
```

## 相關指令
- [datasource inspect-export](./datasource-inspect-export.md)
- [datasource import](./datasource-import.md)
- [datasource diff](./datasource-diff.md)
