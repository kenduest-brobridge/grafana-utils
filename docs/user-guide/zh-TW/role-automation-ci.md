# 🤖 自動化 / CI 角色導讀

這一頁給寫 pipeline、wrapper script、排程工作與機器化檢查的人。目標是讓輸出可預測、秘密可輪替、失敗可判讀。

## 適用對象

- CI owner、平台自動化維護者、腳本作者
- 需要把 `grafana-util` 放進 pipeline 的人
- 需要機器可讀輸出與前置門禁的人

## 主要目標

- 先把連線設定收斂成可重複的 profile
- 先用 JSON 或 table 類輸出做機器判讀
- 先在 `status staged` 與 `change preflight` 擋掉不合格變更
- 只在確定 scope 合理時才用 token 或 service-account token

## 典型自動化任務

- 在 promotion 或 apply 前跑 readiness gate
- 從 staged 或 live 狀態產生可機器判讀的摘要
- 讓多個 job 共用一種 profile 形狀
- 在 auth scope、連線或 staged input 不正確時快速 fail fast

## 建議的驗證與秘密處理

1. CI 內優先用 `--profile` 搭配 `token_env` 或 `password_env`，讓 pipeline 不直接持有明文秘密。
2. direct Basic auth 只留給 bootstrap、除錯或手動救援，不要當成預設路徑。
3. token 只適合單一 org 或權限邊界很明確的自動化，不要拿它去覆蓋管理員 surface。
4. 若流程會產生 service-account token，請把它視為敏感憑證並規劃輪替與撤銷。

## 建議先跑的 5 個指令

```bash
grafana-util profile add ci --url https://grafana.example.com --token-env GRAFANA_CI_TOKEN
grafana-util profile show --profile ci --output-format yaml
grafana-util status staged --desired-file ./desired.json --output json
grafana-util change preflight --desired-file ./desired.json --fetch-live --output json
grafana-util overview live --profile ci --output yaml
```

如果您的 pipeline 只需要驗證某個 live surface，則可把最後一行換成 direct Basic auth 或單一 org token 的等價查詢，但不要把範圍寫得比權限更大。

## 什麼叫做自動化路徑已經穩定

- job 不會卡在 prompt
- 同一種 profile 可重複用在多個檢查步驟
- 輸出夠穩定，可以放心給 parser 或 gate 使用
- 失敗時可以區分是 credential、scope、staged input，還是 connectivity 的問題

## 接下來先讀哪些章節

- [開始使用](getting-started.md)
- [技術參考手冊](reference.md)
- [變更與狀態](change-overview-status.md)
- [Access 管理](access.md)

## 建議同時開著哪些逐指令頁

- [profile](../../commands/zh-TW/profile.md)
- [status](../../commands/zh-TW/status.md)
- [change](../../commands/zh-TW/change.md)
- [overview](../../commands/zh-TW/overview.md)
- [access service-account](../../commands/zh-TW/access-service-account.md)
- [access service-account token](../../commands/zh-TW/access-service-account-token.md)
- [逐指令總索引](../../commands/zh-TW/index.md)

## 常見錯誤與限制

- 不要在 CI log 裡直接印出 token 或 password。
- 不要把 `status staged` 當成 `apply`；它是門禁，不是變更執行器。
- 不要假設 token 或 service-account token 能跨 org 使用。
- 不要依賴互動式輸出做自動化判讀；機器流程應以 JSON、table 或明確的 exit code 為準。
- 不要在 pipeline 裡臨時手刻明文設定檔，應把秘密來源固定在 env 或 secret store。

## 失敗排查提示

- 驗證成功但輸出看起來不完整：
  先懷疑 token scope，不要先懷疑 renderer。
- 本機可跑、CI 跑不起來：
  先檢查 env 注入、profile path 與 CI runner 是否真的有同樣的 secret source。
- staged gate 成功、apply 或 admin 類流程失敗：
  很可能不是語法錯，而是權限範圍不夠。

## 什麼時候切到更深的文件

- output format、exit code、profile secret 規則，切到 [技術參考手冊](reference.md)
- staged gate、preflight、promotion review，切到 [變更與狀態](change-overview-status.md)
- 需要處理 service-account 憑證輪替或管理時，切到 [Access 管理](access.md)
- 只差精確旗標或 namespace 細節時，切到 [逐指令總索引](../../commands/zh-TW/index.md)

## 下一步

- [回到手冊首頁](index.md)
- [先看開始使用](getting-started.md)
- [再看技術參考手冊](reference.md)
- [需要精確旗標時開啟逐指令總索引](../../commands/zh-TW/index.md)
