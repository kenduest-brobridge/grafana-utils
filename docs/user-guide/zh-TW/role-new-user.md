# 👤 新使用者角色導讀

這一頁給第一次接觸 `grafana-util` 的讀者。目標不是一次學完所有指令，而是先把連線、profile 與唯讀檢查跑順。

## 適用對象

- 第一次使用這組 CLI 的工程師或維運人員
- 需要先確認連線、版本與 profile 是否正常的人
- 還不需要做匯入、套用或跨組織作業的人

## 主要目標

- 先建立一個可重複使用的 profile
- 先學會看 `status live` 與 `overview live`
- 先分清楚何時用 profile、何時才退回 direct Basic auth
- 先知道 token 只適合範圍很明確的單一 org 自動化

## 第一天最常做的事

- 確認 binary 已經在 `PATH` 上
- 為本機 lab 或 dev Grafana 建一個 repo-local profile
- 跑一次安全的 live read，分清楚 `status live` 和 `overview live`
- 知道下一步該往 dashboards、alerts、access 還是 automation 哪條路走

## 建議的驗證與秘密處理

1. 先用 `--profile`。這是日常操作最穩定的路徑，也最不容易把秘密重複貼在命令列上。
2. 沒有 profile 時，才用 direct Basic auth 做 bootstrap 或臨時檢查，並優先搭配 `--prompt-password`。
3. 只有在您很清楚 token scope 只需要涵蓋單一 org 或狹窄權限時，才改用 token。
4. 密碼與 token 盡量放在 `password_env`、`token_env` 或 secret store，不要寫進明文命令列。

## 建議先跑的 5 個指令

```bash
grafana-util profile init --overwrite
grafana-util profile add dev --url http://127.0.0.1:3000 --basic-user admin --prompt-password
grafana-util profile list
grafana-util profile show --profile dev --output-format yaml
grafana-util status live --profile dev --output yaml
```

如果您暫時還沒有 profile，才改用這個 bootstrap 入口：

```bash
grafana-util status live --url http://localhost:3000 --basic-user admin --prompt-password --output yaml
```

如果您手上已經有一個範圍很明確的 token，也可以做一次等價的唯讀檢查：

```bash
grafana-util overview live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output json
```

## 什麼叫做已經上手

當您符合下面幾點時，就可以離開新手路徑：

- `grafana-util --version` 在平常使用的 shell 可以正常執行
- `profile show --profile dev` 解析出來的欄位是您預期的
- `status live --profile dev` 可以穩定回傳可讀輸出
- 您已經知道下一步要進 dashboards、alerts、access，還是 CI / automation

## 接下來先讀哪些章節

- [開始使用](getting-started.md)
- [技術參考手冊](reference.md)
- [疑難排解與名詞解釋](troubleshooting.md)

## 建議同時開著哪些逐指令頁

- [profile](../../commands/zh-TW/profile.md)
- [status](../../commands/zh-TW/status.md)
- [overview](../../commands/zh-TW/overview.md)
- [逐指令總索引](../../commands/zh-TW/index.md)

## 常見錯誤與限制

- 不要一開始就把 `--output-format` 和 `--output` 混著用；這兩個旗標是不同層級的輸出控制。
- 不要把明文密碼直接寫進 `grafana-util.yaml`，除非您只是在做一次性的 lab 或 demo。
- 不要期待窄權限 token 能做所有事，尤其是跨 org 盤點或管理類操作。
- 不要先碰匯入或套用流程；先把 profile、status、overview 的讀取路徑跑通。

## 什麼時候切到更深的文件

- 需要理解整個工作流故事時，切到 handbook 章節
- 已經知道要跑哪個流程、只差精確旗標時，切到逐指令頁
- 語法沒錯但 scope、auth 或輸出結果不符合預期時，切到疑難排解

## 下一步

- [回到手冊首頁](index.md)
- [先看開始使用](getting-started.md)
- [再看技術參考手冊](reference.md)
- [需要精確旗標時開啟逐指令總索引](../../commands/zh-TW/index.md)
