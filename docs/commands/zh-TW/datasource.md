# datasource

## 先判斷你現在要做哪一種事

| 你現在想做的事 | 先開哪個命令頁 | 這頁會幫你回答什麼 |
| --- | --- | --- |
| 想先看支援哪些 datasource 類型 | [datasource types](./datasource-types.md) | 先確認這個環境能建立什麼 |
| 想先盤點 live 或本地 datasource 現況 | [datasource list](./datasource-list.md)、[datasource browse](./datasource-browse.md) | 先知道有哪些 datasource 與內容 |
| 想先匯出、匯入、比對或產生 plan | [datasource export](./datasource-export.md)、[datasource diff](./datasource-diff.md)、[datasource plan](./datasource-plan.md)、[datasource import](./datasource-import.md) | 先決定搬移或 review 路徑 |
| 想直接改 live datasource | [datasource add](./datasource-add.md)、[datasource modify](./datasource-modify.md)、[datasource delete](./datasource-delete.md) | 先確認變更面，再動 live |

## 先選哪一條資料路徑

- **live Grafana**：先用 `types`、`list`、`browse` 盤點，再決定要不要 export 或 modify
- **本地 bundle / 匯出樹**：先用 `diff`、`plan`、`import` 看搬移、回放或 prune review 路徑
- **直接 live mutation**：只有在 scope 與輸入都確認過後，才進 `add` / `modify` / `delete`

## 這個入口是做什麼的

`grafana-util datasource` 把 datasource 的生命週期收在同一個入口：從類型查找、瀏覽、讀取 live 或本地 inventory、匯出、匯入、比對、plan，到 live add / modify / delete 都在這裡處理。這頁適合先判斷下一步該走 inventory、bundle、diff、plan，還是 live mutation。

## 重點旗標

- `--url`：Grafana 基底網址。
- `--token`、`--basic-user`、`--basic-password`：共用的線上 Grafana 憑證。
- `--profile`：從 `grafana-util.yaml` 載入 repo 本地預設值。
- `--color`：控制這個指令群組的 JSON 彩色輸出。

## 這一組頁面怎麼讀比較不會亂

1. 先看這頁，判斷你是在做 inventory、bundle、diff、plan，還是 live mutation。
2. 進到子命令頁後，先看 data source 來源是 live 還是本地 bundle。
3. 先跑最短成功路徑，再加進階旗標，不要一開始就帶滿所有 options。
4. 如果是 production 變更，先完成 export / diff，再進 live mutation。

## 採用前後對照

- **採用前**：data source 工作常散在 Grafana UI、API 呼叫或一次性的 shell 指令裡，之後很難回頭審查。
- **採用後**：同一套生命週期集中在一個指令群組裡，browse、export、diff 和 live 修改可以共用同樣的驗證與審查習慣。

## 成功判準

- 在動到 production data source 前，就能先判斷下一步該走 inventory、export / import、diff、plan 還是 live mutation
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

## 各工作流入口

| 工作流 | 入口頁 | 常見延伸頁 |
| --- | --- | --- |
| 盤點 | [datasource types](./datasource-types.md) | [datasource list](./datasource-list.md)、[datasource browse](./datasource-browse.md) |
| 搬移 | [datasource export](./datasource-export.md) | [datasource import](./datasource-import.md)、[datasource diff](./datasource-diff.md)、[datasource plan](./datasource-plan.md) |
| live 變更 | [datasource add](./datasource-add.md) | [datasource modify](./datasource-modify.md)、[datasource delete](./datasource-delete.md) |
