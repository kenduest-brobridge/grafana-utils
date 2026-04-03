# 🛠️ SRE / 維運角色導讀

這一頁給負責日常維運、健康檢查、回放與故障處理的人。重點是先把 live/staged 兩條路徑看懂，再把匯出、匯入與前置驗證串起來。

## 適用對象

- On-call SRE、平台維運、Grafana operator
- 需要做健康檢查、盤點、回放或漂移比對的人
- 需要在變更前後做門禁的人

## 主要目標

- 先用 `status live` 和 `overview live` 確認現況
- 先在 `change summary` 與 `change preflight` 擋掉不合理的變更包
- 先用 `--dry-run` 看匯入結果，再決定要不要真的套用
- 需要時才用 direct Basic auth 做 break-glass

## 典型維運任務

- 維護前先做 live readiness 檢查
- 盤點跨 org 的 dashboards、datasources 或 alerts
- 在 apply 之前先做 staged summary、preflight 與 dry-run
- 在備份、漂移檢查或災難復原時做匯出與比對

## 建議的驗證與秘密處理

1. 日常維運優先用 `--profile`，並把密碼或 token 放在環境變數或 secret store。
2. 出現緊急處理或尚未建立 profile 時，再用 direct Basic auth。
3. token 只適合權限範圍很清楚的讀取或單一 org 自動化，不要假設它能完成所有管理動作。
4. 跨 org、管理員層級與 service-account 類作業，通常仍需要更完整的管理員憑證。

## 建議先跑的 5 個指令

```bash
grafana-util status live --profile prod --output table
grafana-util overview live --profile prod --output interactive
grafana-util change summary --desired-file ./desired.json
grafana-util change preflight --desired-file ./desired.json --fetch-live --output json
grafana-util dashboard export --export-dir ./backups --overwrite --progress
```

如果您要先處理存取層資產，則可把 `dashboard export` 換成：

```bash
grafana-util access org list --table
```

## 接下來先讀哪些章節

- [變更與狀態](change-overview-status.md)
- [Dashboard 管理](dashboard.md)
- [Datasource 管理](datasource.md)
- [告警治理](alert.md)
- [Access 管理](access.md)
- [疑難排解與名詞解釋](troubleshooting.md)

## 建議同時開著哪些逐指令頁

- [status](../../commands/zh-TW/status.md)
- [overview](../../commands/zh-TW/overview.md)
- [change](../../commands/zh-TW/change.md)
- [dashboard](../../commands/zh-TW/dashboard.md)
- [alert](../../commands/zh-TW/alert.md)
- [access](../../commands/zh-TW/access.md)
- [逐指令總索引](../../commands/zh-TW/index.md)

## 常見錯誤與限制

- 不要把 `status live` 當成部署前的唯一檢查；`change preflight` 和 staged 驗證還是要跑。
- 不要在匯入前略過 `--dry-run`，尤其是會覆寫既有資產時。
- 不要假設 token 看得到所有 org，`--all-orgs` 與管理操作常會受 scope 限制。
- 不要把 `tokens.json` 當一般輸出檔；它包含敏感資訊。
- 不要直接進行破壞性操作，先看摘要、再看 preflight、最後才套用。

## 什麼叫做處於良好的維運姿勢

- 您知道目前的 credential 到底能不能看見您要處理的 org 或管理範圍
- 您能分清楚 live read、staged review 與真正 apply 三種不同路徑
- 重大變更前會先跑 preflight 或 dry-run
- 問題一旦從 status 轉進 dashboard、alert 或 access，您知道要切到哪一頁

## 什麼時候切到更深的文件

- inventory、export/import、inspect、screenshot 類問題，切到 [Dashboard 管理](dashboard.md)
- rule、route、contact point、plan/apply 類問題，切到 [告警治理](alert.md)
- org、user、team、service-account 類問題，切到 [Access 管理](access.md)
- 已經知道要跑哪個流程，只差精確旗標時，切到 [逐指令總索引](../../commands/zh-TW/index.md)

## 下一步

- [回到手冊首頁](index.md)
- [先看變更與狀態](change-overview-status.md)
- [再看 Dashboard 管理](dashboard.md)
- [需要逐指令細節時開啟逐指令總索引](../../commands/zh-TW/index.md)
