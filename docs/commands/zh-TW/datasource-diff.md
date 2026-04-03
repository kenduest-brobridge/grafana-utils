# datasource diff

## 用途
比較本地 bundle 中的 datasource inventory 與線上 Grafana，並輸出給操作人員看的 diff 報告。

## 何時使用
當您想在匯入前先取得簡潔的線上與本地差異報告時，使用這個指令。

## 重點旗標
- `--diff-dir`：要比對的本地 datasource bundle。
- `--input-format`：選擇 `inventory` 或 `provisioning`。

## 範例
```bash
grafana-util datasource diff --profile prod --diff-dir ./datasources --input-format inventory
grafana-util datasource diff --url http://localhost:3000 --basic-user admin --basic-password admin --diff-dir ./datasources/provisioning --input-format provisioning
grafana-util datasource diff --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --diff-dir ./datasources --input-format inventory
```

## 相關指令
- [datasource export](./datasource-export.md)
- [datasource import](./datasource-import.md)
- [datasource inspect-export](./datasource-inspect-export.md)
