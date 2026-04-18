# access

## 先判斷你現在要做哪一種事

| 你現在想做的事 | 先開哪個命令頁 | 這頁會幫你回答什麼 |
| --- | --- | --- |
| 想先盤點目前有哪些 user、org、team | [access user](./access-user.md)、[access org](./access-org.md)、[access team](./access-team.md) | 先知道目前有哪些主體與關係 |
| 想管理 team member、org member 或使用者狀態 | [access user](./access-user.md)、[access org](./access-org.md)、[access team](./access-team.md) | 先確認你要改的是哪一層 access 面 |
| 想處理 service account 的建立、列表與刪除 | [access service-account](./access-service-account.md) | 先管理非人類身分的生命週期 |
| 想追蹤或建立 service account token | [access service-account token](./access-service-account-token.md) | 先確認 token 所屬 service account 與命名 |
| 想先看本機匯出的 access bundle | [access user](./access-user.md)、[access org](./access-org.md)、[access team](./access-team.md) | 先決定是 live inventory 還是本地 bundle |
| 想先知道 user bundle 套到 live 會改什麼 | [access plan](./access-plan.md) | 先產生 review plan，再決定是否 import 或 prune |

## 這一頁對應的工作流

| 工作流 | 常用子命令 |
| --- | --- |
| 盤點使用者 / org / team / service account | `user`、`org`、`team`、`service-account`、`service-account token` |
| 規劃 access 變更 | `plan` |
| 管理成員與權限 | `user`、`org`、`team` |
| 管理 service account 與 token | `service-account`、`service-account token` |

## 從這裡開始

- 先看現況：`access user list`、`access org list`、`access team list`、`access service-account list`
- 想看本機套件：把 `--input-dir ./access-*` 加到對應的 `list`
- 想先做 user bundle vs live review：使用 `access plan --resource user`
- 要處理 service account：直接進 `access service-account`
- 要追 token：直接進 `access service-account token`
- 要先確認範圍：先看對應的 list，再做新增、修改或刪除

## 先選哪一條資料路徑

- **live Grafana inventory**：先進 `user`、`org`、`team` 或 `service-account` 的 `list`
- **本地 access bundle**：先在對應的 `list` 加 `--input-dir`
- **service account 生命週期**：進 `access service-account`
- **token 生命週期**：進 `access service-account token`

## 這個入口是做什麼的

`grafana-util access` 把身分與存取工作收在同一個入口：`user`、`org`、`team`、`service account` 和 `service-account token` 的生命週期都在這裡處理。`list` 可以直接讀 live Grafana 或本機 bundle；這頁適合先判斷自己應該往哪個操作面走，而不是直接猜一個命令名。

## 主要旗標

- `--profile`、`--url`、`--token`、`--basic-user`、`--basic-password`
- `--prompt-password`、`--prompt-token`、`--timeout`、`--verify-ssl`、`--insecure`、`--ca-cert`
- 巢狀子命令處理 `user`、`org`、`team` 或 `group`，以及 `service-account`

## 這一組頁面怎麼讀比較不會亂

1. 先在這頁判斷你是在查 live inventory、改 membership，還是管 service account / token。
2. 進到對應子頁後，先確認輸入來源是 live Grafana 還是本地 bundle。
3. 先跑 `list` 或 read-only 路徑，確認 scope 沒錯，再做新增、修改或刪除。
4. 涉及 token 時，先確認 service account 名稱與 org 範圍，再做 token 操作。

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
# 先規劃 user bundle 套到 live 會造成什麼變更。
grafana-util access plan --profile prod --input-dir ./access-users --resource user --output-format table
```

```bash
# 先建立或更新 service account token。
grafana-util access service-account token add --url http://localhost:3000 --basic-user admin --basic-password admin --name deploy-bot --token-name nightly
```

```bash
# 先看 service account 與 token 的現況。
grafana-util access service-account list --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format text
```

## 各工作流入口

| 工作流 | 入口頁 | 常見延伸頁 |
| --- | --- | --- |
| 使用者與成員盤點 | [access user](./access-user.md) | [access org](./access-org.md)、[access team](./access-team.md) |
| access 變更規劃 | [access plan](./access-plan.md) | [access user](./access-user.md) |
| org / team 管理 | [access org](./access-org.md) | [access team](./access-team.md)、[access user](./access-user.md) |
| service account 生命週期 | [access service-account](./access-service-account.md) | [access service-account token](./access-service-account-token.md) |
| token 管理 | [access service-account token](./access-service-account-token.md) | [access service-account](./access-service-account.md) |

## 相關命令

### 盤點

- [access plan](./access-plan.md)
- [access user](./access-user.md)
- [access org](./access-org.md)
- [access team](./access-team.md)

### service account 與 token

- [access service-account](./access-service-account.md)
- [access service-account token](./access-service-account-token.md)
