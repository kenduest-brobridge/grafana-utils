# datasource

## 用途
`grafana-util datasource` 是處理目錄查找、線上瀏覽、匯出 / 匯入、diff，以及線上建立 / 修改 / 刪除工作流程的指令群組。這個指令群組也可用 `grafana-util ds` 呼叫。

## 何時使用
當你想檢查支援的 data source 類型、瀏覽線上清單、匯出 data source bundle、比較本地 bundle 與 Grafana，或建立並維護線上 data source 時，就會用到這個指令群組。

## 說明
如果你的工作是處理整個 data source 生命週期，而不是只做單一修改，先看這一頁最合適。`datasource` 指令群組把維運時常一起出現的工作整理在同一個入口下，例如看支援類型、讀 live inventory、匯出與 diff bundle，以及修正或更新 live Grafana data source 物件。

這頁特別適合需要先判斷下一步該走 inventory、export / import、diff，還是 live add / modify / delete 的維運人員。

## 重點旗標
- `--url`：Grafana 基底網址。
- `--token`、`--basic-user`、`--basic-password`：共用的線上 Grafana 憑證。
- `--profile`：從 `grafana-util.yaml` 載入 repo 本地預設值。
- `--color`：控制這個指令群組的 JSON 彩色輸出。

## 驗證說明
- 可重複執行的 data source 清單與變更工作優先用 `--profile`。
- org 跨越或管理員型 mutation 工作，直接 Basic auth 會更穩定。
- Token 驗證適合權限邊界明確的讀取或 diff 流程。

## 採用前後對照

- **採用前**：data source 工作常散在 Grafana UI、API 呼叫或一次性的 shell 指令裡，之後很難回頭審查。
- **採用後**：同一套生命週期會集中在一個指令群組裡，browse、export、diff 和 live 修改可以共用同樣的驗證與審查習慣。

## 成功判準

- 在動到 production data source 前，就能先判斷下一步該走 inventory、export/import、diff 還是 live mutation
- 可重複的 profile 與驗證設定，讓同一批命令能同時支援日常維運和 CI
- export 與 diff 讓你能先看清楚內容，而不是先改 live data source 再回頭補救

## 失敗時先檢查

- 如果 browse 或 list 看起來不完整，先確認 token 或 profile 是否真的看得到目標 org
- 如果 export 或 diff 結果像是舊資料，先確認是不是指到錯的 Grafana，或用了過期的本地 bundle
- 如果 live mutation 失敗，先把打算送出的輸入和目前 live data source 對照清楚，再決定要不要重跑

## 範例
```bash
# 用途：顯示 datasource 指令群組的 help 與子指令。
grafana-util datasource --help
```

```bash
# 用途：顯示內建且支援的 datasource 類型目錄。
grafana-util datasource types
```

```bash
# 用途：用已儲存的 profile 來瀏覽線上 datasource。
grafana-util datasource browse --profile prod
```

```bash
# 用途：用明確的憑證瀏覽單一 org 的 datasource。
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
