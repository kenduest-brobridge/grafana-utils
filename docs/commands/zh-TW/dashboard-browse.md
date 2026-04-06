# dashboard browse

## 用途
在互動式終端機 UI 中開啟線上儀表板樹狀結構，或開啟本機匯出樹。

## 何時使用
當您想要瀏覽資料夾、選取某個儀表板、在抓取、比對、匯入、刪除之前先檢視線上樹狀結構，或在不連線 Grafana 的情況下檢視本機匯出樹時，使用這個指令。

## 重點旗標
- `--path`：從某個資料夾子樹開始，而不是從整棵樹開始。
- `--input-dir`：瀏覽本機 raw 匯出根目錄、all-orgs 匯出根目錄，或 provisioning 樹。
- `--input-format`：將本機匯出樹視為 `raw` 或 `provisioning`。
- `--org-id`：瀏覽指定的 Grafana org。
- `--all-orgs`：彙整所有可見 org 的瀏覽結果。需要 Basic auth。
- 共用線上旗標：`--url`、`--token`、`--basic-user`、`--basic-password`。

## 範例
```bash
# 用途：在互動式終端機 UI 中開啟線上儀表板樹狀結構。
grafana-util dashboard browse --profile prod
```

```bash
# 用途：在互動式終端機 UI 中開啟線上儀表板樹狀結構。
grafana-util dashboard browse --url http://localhost:3000 --basic-user admin --basic-password admin --path 'Platform / Infra'
```

```bash
# 用途：在互動式終端機 UI 中開啟本機 raw 匯出樹。
grafana-util dashboard browse --input-dir ./dashboards/raw --path 'Platform / Infra'
```

```bash
# 用途：在互動式終端機 UI 中開啟線上儀表板樹狀結構。
grafana-util dashboard browse --url http://localhost:3000 --token "$GRAFANA_API_TOKEN"
```

## 相關指令
- [dashboard list](./dashboard-list.md)
- [dashboard fetch-live](./dashboard-fetch-live.md)
- [dashboard delete](./dashboard-delete.md)
