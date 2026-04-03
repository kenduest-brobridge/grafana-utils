# datasource types

## 用途
顯示內建且支援的 datasource 類型目錄。

## 何時使用
當您需要查看 CLI 正規化並支援的標準 datasource type id，以便建立流程使用時，使用這個指令。

## 重點旗標
- `--output-format`：將目錄輸出為 text、table、csv、json 或 yaml。

## 範例
```bash
grafana-util datasource types
grafana-util datasource types --output-format yaml
```

## 相關指令
- [datasource add](./datasource-add.md)
- [datasource modify](./datasource-modify.md)
- [datasource list](./datasource-list.md)
