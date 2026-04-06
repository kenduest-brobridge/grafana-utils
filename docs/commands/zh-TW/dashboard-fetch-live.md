# dashboard fetch-live

## 用途
將一個線上儀表板抓取成 API 安全的本地 JSON 草稿。

## 何時使用
當您需要把某個線上儀表板複製成本地版本，方便檢視、修補、複製或後續發佈時，使用這個指令。

## 重點旗標
- `--dashboard-uid`：要抓取的線上 Grafana 儀表板 UID。
- `--output`：將抓回來的草稿寫到這個路徑。
- 共用線上旗標：`--url`、`--token`、`--basic-user`、`--basic-password`、`--profile`。

## 範例
```bash
# 用途：將一個線上儀表板抓取成 API 安全的本地 JSON 草稿。
grafana-util dashboard fetch-live --profile prod --dashboard-uid cpu-main --output ./cpu-main.json
```

```bash
# 用途：將一個線上儀表板抓取成 API 安全的本地 JSON 草稿。
grafana-util dashboard fetch-live --url http://localhost:3000 --basic-user admin --basic-password admin --dashboard-uid cpu-main --output ./cpu-main.json
```

```bash
# 用途：將一個線上儀表板抓取成 API 安全的本地 JSON 草稿。
grafana-util dashboard fetch-live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --dashboard-uid cpu-main --output ./cpu-main.json
```

## 相關指令
- [dashboard clone-live](./dashboard-clone-live.md)
- [dashboard patch-file](./dashboard-patch-file.md)
- [dashboard review](./dashboard-review.md)
