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
# 用途：在不連線到 Grafana 的情況下，檢查本地的 masked recovery bundle。
grafana-util datasource inspect-export --input-dir ./datasources --table
```

```bash
# 用途：在不連線到 Grafana 的情況下，檢查本地的 masked recovery bundle。
grafana-util datasource inspect-export --input-dir ./datasources --json
```

## 採用前後對照

- **採用前**：要讀 datasource bundle 時，常常得直接打開原始檔案，自己猜哪些內容屬於 inventory、哪些屬於 provisioning。
- **採用後**：一個本地檢視指令就能把秘密值遮蔽起來，並用文字、表格、CSV、JSON、YAML 或互動式方式瀏覽 bundle。

## 成功判準

- 不連線到 Grafana 也能檢查本地 export bundle
- 遮蔽過的秘密值仍然維持遮蔽，但結構還是可讀
- 輸出格式同時適合人工審查與腳本處理

## 失敗時先檢查

- 如果 bundle 打不開，先確認輸入目錄與資料型態是 inventory 還是 provisioning
- 如果遮蔽欄位看起來不對，先確認匯出來源是否真的有那些資料
- 如果互動模式不可用，先退回 text 或 JSON 輸出

## 相關指令
- [datasource export](./datasource-export.md)
- [datasource import](./datasource-import.md)
- [datasource diff](./datasource-diff.md)
