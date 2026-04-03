# datasource inspect-export

## 用途
在不連線到 Grafana 的情況下，檢查本地的 masked recovery bundle。

## 何時使用
當您想從磁碟讀取 datasource 匯出成品，並以文字、表格、CSV、JSON、YAML 或互動式輸出檢視時，使用這個指令。

## 重點旗標
- `--input-dir`：包含匯出成品的本地目錄。
- `--input-type`：當路徑可能被解讀成 inventory 或 provisioning 兩種型態時，用它指定。
- `--interactive`：開啟本地匯出檢視工作台。
- `--table`、`--csv`、`--text`、`--json`、`--yaml`、`--output-format`：輸出模式控制。

## 範例
```bash
grafana-util datasource inspect-export --input-dir ./datasources --table
grafana-util datasource inspect-export --input-dir ./datasources --json
```

## 相關指令
- [datasource export](./datasource-export.md)
- [datasource import](./datasource-import.md)
- [datasource diff](./datasource-diff.md)
