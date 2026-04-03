# `grafana-util alert list-contact-points`

## 目的

列出目前 Grafana 線上的 alert 聯絡點。

## 使用時機

- 檢視 Grafana 內已設定的通知端點。
- 在文字、表格、CSV、JSON 與 YAML 之間切換輸出格式。

## 主要旗標

- `--org-id` 會列出某個 Grafana org ID 的聯絡點。
- `--all-orgs` 會彙整所有可見 org 的清單。
- `--text`, `--table`, `--csv`, `--json`, `--yaml`, 與 `--output-format` 控制輸出。
- `--no-header` 省略表頭列。

## 說明

- 可重複執行的單一 org 清單查詢優先用 `--profile`。
- `--all-orgs` 最好搭配管理員憑證支援的 `--profile` 或直接 Basic auth，因為 token 權限可能只看到部分資料。

## 範例

```bash
grafana-util alert list-contact-points --profile prod --table
grafana-util alert list-contact-points --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --output-format yaml
grafana-util alert list-contact-points --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --json
```

## 相關命令

- [alert](./alert.md)
- [alert list-rules](./alert-list-rules.md)
- [alert list-templates](./alert-list-templates.md)
