# `grafana-util alert export`

## 目的

將 alert 資源匯出成 `raw/` JSON 檔案。

## 使用時機

- 從 Grafana 擷取 alert 規則、聯絡點、靜音時段、範本與政策。
- 在審閱或匯入前建立本機套件。

## 主要旗標

- `--output-dir` 指定匯出套件的寫入位置，預設為 `alerts`。
- `--flat` 會把資源檔直接寫入各自的資源目錄。
- `--overwrite` 會取代既有的匯出檔。
- 使用 `grafana-util alert` 的共用連線旗標。

## 範例

```bash
grafana-util alert export --profile prod --output-dir ./alerts --overwrite
grafana-util alert export --url http://localhost:3000 --basic-user admin --basic-password admin --output-dir ./alerts --flat
grafana-util alert export --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-dir ./alerts --overwrite
```

## 相關命令

- [alert](./alert.md)
- [alert import](./alert-import.md)
- [alert plan](./alert-plan.md)
