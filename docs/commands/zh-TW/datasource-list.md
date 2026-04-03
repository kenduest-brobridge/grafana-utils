# datasource list

## 用途
列出線上 Grafana datasource inventory。

## 何時使用
當您需要一份非互動式的 datasource 清單，不論是目前 org、指定 org，或所有可見 org，都可以使用這個指令。

## 重點旗標
- `--org-id`：列出指定的 Grafana org。
- `--all-orgs`：彙整所有可見 org 的 datasource inventory。需要 Basic auth。
- `--output-format`、`--text`、`--table`、`--csv`、`--json`、`--yaml`：輸出模式控制。
- `--output-columns`：選擇顯示欄位。
- `--no-header`：隱藏表格標頭。

## 範例
```bash
grafana-util datasource list --profile prod --output-format text
grafana-util datasource list --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --output-format yaml
grafana-util datasource list --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --json
```

## 相關指令
- [datasource browse](./datasource-browse.md)
- [datasource export](./datasource-export.md)
- [datasource diff](./datasource-diff.md)
