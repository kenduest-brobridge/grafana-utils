# datasource

## 用途
`grafana-util datasource` 是處理目錄查找、線上瀏覽、匯出/匯入、diff，以及線上建立/修改/刪除工作流程的命名空間。這個命名空間也可用 `grafana-util ds` 呼叫。

## 何時使用
當您想檢查支援的 datasource 類型、瀏覽線上清單、匯出 datasource bundle、比較本地 bundle 與 Grafana，或建立並維護線上 datasource 時，使用這個命名空間。

## 重點旗標
- `--url`：Grafana 基底網址。
- `--token`、`--basic-user`、`--basic-password`：共用的線上 Grafana 憑證。
- `--profile`：從 `grafana-util.yaml` 載入 repo 本地預設值。
- `--color`：控制這個命名空間的 JSON 彩色輸出。

## 驗證說明
- 可重複執行的 datasource 清單與變更工作優先用 `--profile`。
- org 跨越或管理員型 mutation 工作，直接 Basic auth 會更穩定。
- Token 驗證適合權限邊界明確的讀取或 diff 流程。

## 範例
```bash
grafana-util datasource --help
grafana-util datasource types
grafana-util datasource browse --profile prod
grafana-util datasource browse --url http://localhost:3000 --basic-user admin --basic-password admin
```

## 相關指令
- [datasource types](./datasource-types.md)
- [datasource list](./datasource-list.md)
- [datasource browse](./datasource-browse.md)
- [datasource inspect-export](./datasource-inspect-export.md)
- [datasource export](./datasource-export.md)
- [datasource import](./datasource-import.md)
- [datasource diff](./datasource-diff.md)
- [datasource add](./datasource-add.md)
- [datasource modify](./datasource-modify.md)
- [datasource delete](./datasource-delete.md)
