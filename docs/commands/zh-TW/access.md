# access

## 這一頁對應的工作流

| 工作流 | 常用子命令 |
| --- | --- |
| 盤點使用者 / org / team / service account | `user`、`org`、`team`、`service-account`、`service-account token` |
| 管理成員與權限 | `user`、`org`、`team` |
| 管理 service account 與 token | `service-account`、`service-account token` |

## 從這裡開始

- 先看現況：`access user list`、`access org list`、`access team list`、`access service-account list`
- 想看本機套件：把 `--input-dir ./access-*` 加到對應的 `list`
- 要處理 service account：直接進 `access service-account`
- 要追 token：直接進 `access service-account token`
- 要先確認範圍：先看對應的 list，再做新增、修改或刪除

## 說明

`grafana-util access` 把身分與存取工作收在同一個入口：`user`、`org`、`team`、`service account` 和 `service-account token` 的生命週期都在這裡處理。`list` 可以直接讀 live Grafana 或本機 bundle；這頁適合先判斷自己應該往哪個操作面走，而不是直接猜一個命令名。

## 主要旗標

- `--profile`、`--url`、`--token`、`--basic-user`、`--basic-password`
- `--prompt-password`、`--prompt-token`、`--timeout`、`--verify-ssl`、`--insecure`、`--ca-cert`
- 巢狀子命令處理 `user`、`org`、`team` 或 `group`，以及 `service-account`

## 採用前後對照

- **採用前**：成員、org、team 與 token 工作常散在 UI 點擊、一次性的 API 呼叫，或很難重跑的 shell 指令裡。
- **採用後**：同一個 CLI 命令群組能把盤點、生命週期與 token 管理收斂到同一套設定。

## 成功判準

- 你在動手前就能先判斷這件事是屬於 `user`、`org`、`team`，還是 `service-account`
- inventory 讀取會因為 profile 與驗證設定清楚而可重複
- token 與生命週期變更有足夠證據，可以交給另一位維護者或 CI

## 失敗時先檢查

- 如果 list 結果比預期少，先確認是不是需要管理員等級的 Basic auth，而不是較窄權限的 token
- 如果 token 或成員操作失敗，先核對你是不是在正確的 org 與正確的 access 面上操作
- 如果輸出要交給自動化，先確認選了正確的 `--output-format`，讓 parser 知道欄位形狀

## 範例

```bash
# 先盤點目前有哪些 user。
grafana-util access user list --profile prod --json
```

```bash
# 先看本機存好的 org 套件。
grafana-util access org list --input-dir ./access-orgs --output-format table
```

```bash
# 先建立或更新 service account token。
grafana-util access service-account token add --url http://localhost:3000 --basic-user admin --basic-password admin --name deploy-bot --token-name nightly
```

```bash
# 先看 service account 與 token 的現況。
grafana-util access service-account list --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format text
```

## 相關命令

### 盤點

- [access user](./access-user.md)
- [access org](./access-org.md)
- [access team](./access-team.md)

### 服務帳號與 token

- [access service-account](./access-service-account.md)
- [access service-account token](./access-service-account-token.md)
