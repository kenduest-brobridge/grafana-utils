# datasource types

## 用途
顯示內建且支援的 datasource 類型目錄。

## 何時使用
當您需要查看 CLI 正規化並支援的標準 datasource type id，以便建立流程使用時，使用這個指令。

## 重點旗標
- `--output-format`：將目錄輸出為 text、table、csv、json 或 yaml。

## 範例
```bash
# 用途：顯示內建且支援的 datasource 類型目錄。
grafana-util datasource types
```

```bash
# 用途：顯示內建且支援的 datasource 類型目錄。
grafana-util datasource types --output-format yaml
```

## 採用前後對照

- **採用前**：常常只能從 UI 標籤、舊範例或零散筆記去猜 plugin type id。
- **採用後**：一份目錄就列出 CLI 會正規化並支援的 datasource type id，建立流程可以直接對照。

## 成功判準

- 在建立或修改 datasource 之前，就能選對 type id
- 清單夠短，適合快速瀏覽，也夠明確，適合自動化判斷

## 失敗時先檢查

- 如果少了某個 type，先確認這個 plugin 是否真的被目前版本支援
- 如果目錄跟您預期的 Grafana 不一致，先確認是不是正在看舊版 binary

## 相關指令
- [datasource add](./datasource-add.md)
- [datasource modify](./datasource-modify.md)
- [datasource list](./datasource-list.md)
