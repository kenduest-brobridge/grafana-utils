# `grafana-util access`

## 目的

執行 access-management 指令介面，涵蓋使用者、組織、團隊與服務帳號。

## 使用時機

- 列出或瀏覽 access 清單。
- 建立、修改、匯出、匯入、比對或刪除 access 資源。
- 管理 service-account token。

## 說明
如果你現在處理的是 Grafana 身分與存取整體工作，而不是單一 user 命令，先看這一頁最合適。`access` 指令群組不是只管使用者，而是把 org、user、team、service account 和 service-account token 的整套生命週期放在一起。

這頁是給管理者先判斷自己應該往哪個操作面走。只要你的工作牽涉成員、org 結構、service account 輪替，或 access 快照與比對，就先從這裡進來。

## 主要旗標

- `--profile`, `--url`, `--token`, `--basic-user`, `--basic-password`
- `--prompt-password`, `--prompt-token`, `--timeout`, `--verify-ssl`, `--insecure`, `--ca-cert`
- 使用巢狀子命令處理 `user`、`org`、`team` 或 `group`，以及 `service-account`。

## 驗證說明

- 可重複的 inventory 讀取優先用 `--profile`。
- org、user、team 與 service-account lifecycle 往往需要管理員權限，直接 Basic auth 是最穩妥的 fallback。
- 即使 token 能執行部分 list 指令，也不代表足夠支援整個 access 管理面。

## 採用前後對照

- **採用前**：access 工作常常散在 UI 點擊、一次性的 API 呼叫，或不容易重跑的 shell 指令裡。
- **採用後**：同一個 access 面被整理成一個 CLI 命令群組，inventory、生命週期、token 與快照都能用一致設定重複執行。

## 成功判準

- 你能在動到 production 前，先判斷這件事是屬於 `user`、`org`、`team` 還是 `service-account`
- inventory 讀取會因為 profile 與驗證設定清楚而可重複
- token 與生命週期變更有足夠證據，可以交給另一位維護者或 CI

## 失敗時先檢查

- 如果 list 結果比預期少，先確認是不是需要管理員等級的 Basic auth，而不是較窄權限的 token
- 如果 token 或成員操作失敗，先核對你是不是在正確的 org 與正確的 access 面上操作
- 如果輸出要交給自動化，先確認選了正確的 `--output-format`，讓 parser 知道欄位形狀

## 範例

```bash
# 用途：執行 access-management 指令介面，涵蓋使用者、組織、團隊與服務帳號。
grafana-util access user list --profile prod --json
```

```bash
# 用途：執行 access-management 指令介面，涵蓋使用者、組織、團隊與服務帳號。
grafana-util access service-account token add --url http://localhost:3000 --basic-user admin --basic-password admin --name deploy-bot --token-name nightly
```

```bash
# 用途：執行 access-management 指令介面，涵蓋使用者、組織、團隊與服務帳號。
grafana-util access service-account list --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format text
```

## 相關命令

- [access user](./access-user.md)
- [access org](./access-org.md)
- [access team](./access-team.md)
- [access service-account](./access-service-account.md)
- [access service-account token](./access-service-account-token.md)
