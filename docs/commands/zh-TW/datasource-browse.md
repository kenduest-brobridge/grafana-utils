# datasource browse

## 用途
在 Grafana 上開啟線上 datasource 瀏覽器，並可在同一介面進行修改與刪除。

## 何時使用
當您想用互動式清單視圖來檢視、編輯或刪除線上 datasource 時，使用這個指令。

## 重點旗標
- `--org-id`：瀏覽指定的 Grafana org。
- `--all-orgs`：彙整所有可見 org 的 datasource 瀏覽結果。需要 Basic auth。
- 共用線上旗標：`--url`、`--token`、`--basic-user`、`--basic-password`。

## 範例
```bash
grafana-util datasource browse --profile prod
grafana-util datasource browse --url http://localhost:3000 --basic-user admin --basic-password admin --org-id 2
grafana-util datasource browse --url http://localhost:3000 --token "$GRAFANA_API_TOKEN"
```

## 相關指令
- [datasource list](./datasource-list.md)
- [datasource modify](./datasource-modify.md)
- [datasource delete](./datasource-delete.md)
