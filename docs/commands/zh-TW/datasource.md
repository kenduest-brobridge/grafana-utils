# datasource

## 這一頁對應的工作流

| 工作流 | 常用子命令 |
| --- | --- |
| 盤點與瀏覽 data source | `types`、`list`、`browse` |
| 匯出 / 匯入 / 比對 | `export`、`import`、`diff` |
| 新增 / 修改 / 刪除 | `add`、`modify`、`delete` |

## 從這裡開始

- 新環境先看支援類型：`datasource types`
- 要盤點線上現況，或看本地匯出內容：`datasource list`、`datasource browse`
- 要先做草稿或搬移：`datasource export`、`datasource diff`
- 要直接改 live data source：`datasource add`、`datasource modify`、`datasource delete`

## 說明

`grafana-util datasource` 把 data source 的生命週期收在同一個入口：從類型查找、瀏覽、讀取 live 或本地 inventory、匯出、匯入、比對，到 live add / modify / delete 都在這裡處理。這頁適合先判斷下一步該走 inventory、bundle、diff，還是 live mutation。

## 重點旗標

- `--url`：Grafana 基底網址。
- `--token`、`--basic-user`、`--basic-password`：共用的線上 Grafana 憑證。
- `--profile`：從 `grafana-util.yaml` 載入 repo 本地預設值。
- `--color`：控制這個指令群組的 JSON 彩色輸出。

## 採用前後對照

- **採用前**：data source 工作常散在 Grafana UI、API 呼叫或一次性的 shell 指令裡，之後很難回頭審查。
- **採用後**：同一套生命週期集中在一個指令群組裡，browse、export、diff 和 live 修改可以共用同樣的驗證與審查習慣。

## 成功判準

- 在動到 production data source 前，就能先判斷下一步該走 inventory、export / import、diff 還是 live mutation
- 可重複的 profile 與驗證設定，讓同一批命令能同時支援日常維運和 CI
- export 與 diff 讓你能先看清楚內容，而不是先改 live data source 再回頭補救

## 失敗時先檢查

- 如果 browse 或 list 看起來不完整，先確認 token 或 profile 是否真的看得到目標 org
- 如果 export 或 diff 結果像是舊資料，先確認是不是指到錯的 Grafana，或用了過期的本地 bundle
- 如果 live mutation 失敗，先把打算送出的輸入和目前 live data source 對照清楚，再決定要不要重跑

## 範例

```bash
# 先看這個環境支援哪些 data source 類型。
grafana-util datasource types
```

```bash
# 先盤點線上 data source，再決定要不要 export 或修改。
grafana-util datasource browse --profile prod
```

```bash
# 先匯出成 bundle，再拿去做 diff 或搬移。
grafana-util datasource export --profile prod --output-dir ./datasources
```

## 相關命令

### 盤點

- [datasource types](./datasource-types.md)
- [datasource list](./datasource-list.md)
- [datasource browse](./datasource-browse.md)

### 搬移

- [datasource export](./datasource-export.md)
- [datasource import](./datasource-import.md)
- [datasource diff](./datasource-diff.md)

### 變更前檢查

- [datasource add](./datasource-add.md)
- [datasource modify](./datasource-modify.md)
- [datasource delete](./datasource-delete.md)
