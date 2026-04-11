# 🤖 自動化 / CI 角色導讀

這一頁給寫 pipeline、wrapper script、排程工作與自動化檢查的人。重點是讓輸出穩定、憑證好管理，失敗原因也看得出來。

## 適用對象

- CI owner、平台自動化維護者、腳本作者
- 需要把 `grafana-util` 放進 pipeline 的人
- 需要把輸出接進 CI/CD 或其他自動化流程的人

## 主要目標

- 先把連線設定收斂成可重複的 profile
- 先用 JSON 或 table 類輸出讓腳本穩定判讀
- 先在 `status staged` 與 `workspace test` 擋掉有問題的變更
- 只在確定 scope 合理時才用 token 或 service account token

## 採用前後對照

- 以前：每個 job 都要自己重建同一套驗證與輸出接線。
- 現在：用一個環境變數驅動的 profile，就能讓重複跑的流程更穩定、更好排查。

## 成功判準

- CI 跑起來不會一直等 prompt。
- 輸出穩定到可以讓 script、gate 或 parser 直接處理。
- 失敗時能分清楚是驗證、權限範圍、staged input 還是連線問題。

## 失敗時先檢查

- 如果 job 還會跳 prompt，代表 profile 或 env 接線還沒設好。
- 如果輸出看起來正常，但資料太少，先懷疑 scope，不要先懷疑 renderer。
- 如果唯讀 gate 成功、apply 卻失敗，先確認寫入或管理員權限，不要先改流程。

## 典型自動化任務

- 在 promotion 或 apply 前先跑 readiness gate
- 從 staged 或 live 狀態產生可讓腳本處理的摘要
- 讓多個 job 共用同一套 profile 設定
- 在 auth scope、連線或 staged input 不正確時，提早失敗並讓 pipeline 直接停下來

## 建議的連線與秘密資料處理方式

1. CI 內優先用 `--profile` 搭配 `token_env` 或 `password_env`，讓 pipeline 不直接持有明文秘密。
2. direct Basic auth 只留給 bootstrap、除錯或手動救援，不要當成預設做法。
3. token 只適合單一 org 或權限邊界很明確的自動化，不要拿它去碰管理員操作。
4. 若流程會產生 service account token，請把它視為敏感憑證並規劃輪替與撤銷。

## 建議先跑的 5 個指令

```bash
# 用途：建議先跑的 5 個指令。
grafana-util config profile add ci --url https://grafana.example.com --token-env GRAFANA_CI_TOKEN
```

```bash
# 用途：建議先跑的 5 個指令。
grafana-util config profile show --profile ci --output-format yaml
```

```bash
# 用途：建議先跑的 5 個指令。
grafana-util status staged --desired-file ./desired.json --output-format json
```

```bash
# 用途：建議先跑的 5 個指令。
grafana-util workspace test --desired-file ./desired.json --fetch-live --output-format json
```

```bash
# 用途：建議先跑的 5 個指令。
grafana-util status overview live --profile ci --output-format yaml
```

如果 pipeline 只需要驗證某個 live 操作面，可把最後一行換成 direct Basic auth 或單一 org token 的等價查詢，但不要把查詢範圍開得比權限更大。

## 什麼叫做自動化流程已經穩定

- job 不會卡在 prompt
- 同一種 profile 可重複用在多個檢查步驟
- 輸出夠穩定，可以放心交給 parser 或 gate 使用
- 失敗時可以區分是 credential、scope、staged input，還是 connectivity 的問題

## 接下來先讀哪些章節

- [開始使用](getting-started.md)
- [技術參考手冊](reference.md)
- [Workspace 審查與狀態](status-workspace.md)
- [Access 管理](access.md)

## 建議同時開著哪些指令頁

- [config profile](../../commands/zh-TW/config.md)
- [workspace](../../commands/zh-TW/workspace.md)
- [指令詳細說明總索引](../../commands/zh-TW/index.md)
- [access service-account](../../commands/zh-TW/access-service-account.md)
- [access service-account token](../../commands/zh-TW/access-service-account-token.md)
- [指令詳細說明總索引](../../commands/zh-TW/index.md)

## 常見錯誤與限制

- 不要在 CI log 裡直接印出 token 或 password。
- 不要把 `status staged` 當成 `apply`；它是檢查關卡，不是變更執行器。
- 不要假設 token 或 service account token 能跨 org 使用。
- 不要依賴互動式輸出做自動化判讀；自動化流程應以 JSON、table 或明確的 exit code 為準。
- 不要在 pipeline 裡臨時手刻明文設定檔，應把秘密來源固定在 env 或 secret store。

## 失敗排查提示

- 驗證成功但輸出看起來不完整：
  先懷疑 token scope，不要先懷疑 renderer。
- 本機可跑、CI 跑不起來：
  先檢查 env 注入、profile path 與 CI runner 是否真的有同樣的 secret source。
- staged gate 成功、apply 或 admin 類流程卻失敗：
  很可能不是語法錯，而是權限範圍不夠。

## 什麼時候切到更深的文件

- output format、exit code、profile secret 規則，切到 [技術參考手冊](reference.md)
- staged gate、input test、promotion review，切到 [Workspace 審查與狀態](status-workspace.md)
- 需要處理 service account 憑證輪替或管理時，切到 [Access 管理](access.md)
- 只差精確旗標或 command 細節時，切到 [指令詳細說明總索引](../../commands/zh-TW/index.md)

## 下一步

- [回到手冊首頁](index.md)
- [先看開始使用](getting-started.md)
- [再看技術參考手冊](reference.md)
- [需要精確旗標時查看指令詳細說明總索引](../../commands/zh-TW/index.md)
