# 🛠️ SRE / 維運角色導讀

這一頁給負責日常維運、健康檢查、回放與故障處理的人。先把 live / staged 兩套流程看懂，再把匯出、匯入與變更前檢查串起來。

## 適用對象

- 值班 SRE、平台維運、Grafana operator
- 需要做健康檢查、盤點、回放或漂移比對的人
- 需要在變更前後做檢查把關的人

## 主要目標

- 先用 `status live` 和 `overview live` 確認現況
- 先在 `change inspect`、`change check`、`change preview` 擋掉不合理的變更包
- 先用 `--dry-run` 看匯入結果，再決定要不要真的套用
- 需要時才用 direct Basic auth 做 break-glass

## 採用前後對照

- 以前：SRE 要靠一串零散指令自己拼出 readiness、scope 與 replay 風險。
- 現在：先用可重複的 profile，接著走 live check、staged review，最後才進 apply。

## 成功判準

- 你能判斷憑證到底看不看得到你要處理的範圍。
- 你能分清楚 live read、staged review 和 apply 是不同路徑。
- 你有一條能穩定處理 dashboard、alert 與 access 的維運路線。

## 失敗時先檢查

- 如果 token scope 比任務還窄，先停下來換一張真的看得到該範圍的憑證。
- 如果 live check 成功、apply 失敗，先檢查寫入權限與 staged input，不要先懷疑 renderer。
- 如果你還說不出任務屬於哪條 lane，先回去看工作流章節，不要直接改 live。

## 典型維運任務

- 維護前先做即時整備度檢查
- 盤點跨 org 的 dashboard、data source 或 alert
- 在 apply 之前先做 staged inspect、check、preview 與 dry-run
- 在備份、漂移檢查或災難復原時做匯出與比對

## 建議的連線與秘密資料處理方式

1. 日常維運優先用 `--profile`，並把密碼或 token 放在環境變數或 secret store。
2. 出現緊急處理或尚未建立 profile 時，再用 direct Basic auth。
3. token 只適合權限範圍很清楚的讀取或單一 org 自動化，不要假設它能完成所有管理動作。
4. 跨 org、管理員層級與 service account 類作業，通常還是需要更完整的管理員憑證。

## 建議先跑的 5 個指令

```bash
# 用途：建議先跑的 5 個指令。
grafana-util status live --profile prod --output-format table
```

```bash
# 用途：建議先跑的 5 個指令。
grafana-util overview live --profile prod --output-format interactive
```

```bash
# 用途：建議先跑的 5 個指令。
grafana-util change inspect --workspace .
```

```bash
# 用途：建議先跑的 5 個指令。
grafana-util change check --workspace . --fetch-live --output-format json
```

```bash
# 用途：建議先跑的 5 個指令。
grafana-util change preview --workspace . --fetch-live --output-format json
```

```bash
# 用途：建議先跑的 5 個指令。
grafana-util dashboard export --output-dir ./backups --overwrite --progress
```

如果你要先處理存取層資產，可以把 `dashboard export` 換成：

```bash
# 用途：如果你要先處理存取層資產，可以把 dashboard export 換成。
grafana-util access org list --table
```

## 接下來先讀哪些章節

- [變更與狀態](change-overview-status.md)
- [Dashboard 管理](dashboard.md)
- [Data source 管理](datasource.md)
- [告警治理](alert.md)
- [Access 管理](access.md)
- [疑難排解與名詞解釋](troubleshooting.md)

## 建議同時開著哪些指令頁

- [status](../../commands/zh-TW/status.md)
- [overview](../../commands/zh-TW/overview.md)
- [change](../../commands/zh-TW/change.md)
- [dashboard](../../commands/zh-TW/dashboard.md)
- [alert](../../commands/zh-TW/alert.md)
- [access](../../commands/zh-TW/access.md)
- [指令詳細說明總索引](../../commands/zh-TW/index.md)

## 常見錯誤與限制

- 不要把 `status live` 當成部署前的唯一檢查；`change check` 和 staged 驗證還是要跑。
- 不要在匯入前略過 `--dry-run`，尤其是會覆寫既有資產時。
- 不要假設 token 看得到所有 org，`--all-orgs` 與管理操作常會受 scope 限制。
- 不要把 `tokens.json` 當一般輸出檔；它包含敏感資訊。
- 不要直接進行破壞性操作，先 inspect、再 check、再 preview，最後才套用。

## 什麼叫做處於良好的維運姿勢

- 你知道目前的 credential 到底能不能看見要處理的 org 或管理範圍
- 你能分清楚即時讀取、staged review 與真正 apply 是三種不同流程
- 重大變更前會先跑 preflight 或 dry-run
- 問題一旦從 status 轉進 dashboard、alert 或 access，你知道要切到哪一頁

## 什麼時候切到更深的文件

- inventory、export/import、inspect、screenshot 類問題，切到 [Dashboard 管理](dashboard.md)
- rule、route、contact point、plan/apply 類問題，切到 [告警治理](alert.md)
- org、user、team、service account 類問題，切到 [Access 管理](access.md)
- 已經知道要跑哪個流程，只差精確旗標時，切到 [指令詳細說明總索引](../../commands/zh-TW/index.md)

## 下一步

- [回到手冊首頁](index.md)
- [先看變更與狀態](change-overview-status.md)
- [再看 Dashboard 管理](dashboard.md)
- [需要指令細節時查看指令詳細說明總索引](../../commands/zh-TW/index.md)
