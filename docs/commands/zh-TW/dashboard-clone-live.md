# dashboard clone-live

## 用途
將一個線上儀表板複製成本地草稿，並可選擇覆寫部分欄位。

## 何時使用
當您想以既有的線上儀表板為起點，但要替本地草稿指定新的標題、UID 或資料夾目標時，使用這個指令。

## 重點旗標
- `--source-uid`：要複製的線上 Grafana 儀表板 UID。
- `--output`：將複製出的草稿寫到這個路徑。
- `--name`：覆寫複製後的儀表板標題。
- `--uid`：覆寫複製後的儀表板 UID。
- `--folder-uid`：覆寫保留的 Grafana 資料夾 UID。

## 範例
```bash
# 用途：將一個線上儀表板複製成本地草稿，並可選擇覆寫部分欄位。
grafana-util dashboard clone-live --profile prod --source-uid cpu-main --output ./cpu-main-clone.json
```

```bash
# 用途：將一個線上儀表板複製成本地草稿，並可選擇覆寫部分欄位。
grafana-util dashboard clone-live --url http://localhost:3000 --basic-user admin --basic-password admin --source-uid cpu-main --name 'CPU Clone' --uid cpu-main-clone --folder-uid infra --output ./cpu-main-clone.json
```

```bash
# 用途：將一個線上儀表板複製成本地草稿，並可選擇覆寫部分欄位。
grafana-util dashboard clone-live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --source-uid cpu-main --output ./cpu-main-clone.json
```

## 相關指令
- [dashboard fetch-live](./dashboard-fetch-live.md)
- [dashboard patch-file](./dashboard-patch-file.md)
- [dashboard publish](./dashboard-publish.md)
