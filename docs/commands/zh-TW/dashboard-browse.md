# dashboard browse

## 用途
在互動式終端機 UI 中開啟線上儀表板樹狀結構。

## 何時使用
當您想要瀏覽資料夾、選取某個儀表板，或在抓取、比對、匯入、刪除之前先檢視線上樹狀結構時，使用這個指令。

## 重點旗標
- `--path`：從某個資料夾子樹開始，而不是從整棵樹開始。
- `--org-id`：瀏覽指定的 Grafana org。
- `--all-orgs`：彙整所有可見 org 的瀏覽結果。需要 Basic auth。
- 共用線上旗標：`--url`、`--token`、`--basic-user`、`--basic-password`。

## 範例
```bash
grafana-util dashboard browse --profile prod
grafana-util dashboard browse --url http://localhost:3000 --basic-user admin --basic-password admin --path 'Platform / Infra'
grafana-util dashboard browse --url http://localhost:3000 --token "$GRAFANA_API_TOKEN"
```

## 相關指令
- [dashboard list](./dashboard-list.md)
- [dashboard get](./dashboard-get.md)
- [dashboard delete](./dashboard-delete.md)
