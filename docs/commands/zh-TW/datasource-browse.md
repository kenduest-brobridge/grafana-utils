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
# 用途：在 Grafana 上開啟線上 datasource 瀏覽器，並可在同一介面進行修改與刪除。
grafana-util datasource browse --profile prod
```

```bash
# 用途：在 Grafana 上開啟線上 datasource 瀏覽器，並可在同一介面進行修改與刪除。
grafana-util datasource browse --url http://localhost:3000 --basic-user admin --basic-password admin --org-id 2
```

```bash
# 用途：在 Grafana 上開啟線上 datasource 瀏覽器，並可在同一介面進行修改與刪除。
grafana-util datasource browse --url http://localhost:3000 --token "$GRAFANA_API_TOKEN"
```

## 採用前後對照

- **採用前**：要檢視一個 datasource 時，常常得在清單、編輯視窗與刪除確認之間反覆跳轉。
- **採用後**：一個瀏覽器畫面就能同時看見 live inventory 與對應的修改、刪除動作，減少上下文切換。

## 成功判準

- 您可以在不離開清單的情況下檢視 live datasource
- 修改與刪除動作都貼近正在看的那一列
- 在變更前，org 範圍已經很清楚

## 失敗時先檢查

- 如果瀏覽器開出來少了資料，先確認 org 範圍與驗證資訊是否正確
- 如果看不到修改或刪除動作，先確認帳號是否真的有變更 datasource 的權限
- 如果 org 切換看起來不對，先確認是不是刻意使用了 `--all-orgs` 或 `--org-id` 

## 相關指令
- [datasource list](./datasource-list.md)
- [datasource modify](./datasource-modify.md)
- [datasource delete](./datasource-delete.md)
