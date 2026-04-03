# `grafana-util access`

## 目的

執行 access-management 指令介面，涵蓋使用者、組織、團隊與服務帳號。

## 使用時機

- 列出或瀏覽 access 清單。
- 建立、修改、匯出、匯入、比對或刪除 access 資源。
- 管理 service-account token。

## 主要旗標

- `--profile`, `--url`, `--token`, `--basic-user`, `--basic-password`
- `--prompt-password`, `--prompt-token`, `--timeout`, `--verify-ssl`, `--insecure`, `--ca-cert`
- 使用巢狀子命令處理 `user`、`org`、`team` 或 `group`，以及 `service-account`。

## 驗證說明

- 可重複的 inventory 讀取優先用 `--profile`。
- org、user、team 與 service-account lifecycle 往往需要管理員權限，直接 Basic auth 是最穩妥的 fallback。
- 即使 token 能執行部分 list 指令，也不代表足夠支援整個 access 管理面。

## 範例

```bash
grafana-util access user list --profile prod --json
grafana-util access service-account token add --url http://localhost:3000 --basic-user admin --basic-password admin --name deploy-bot --token-name nightly
grafana-util access service-account list --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format text
```

## 相關命令

- [access user](./access-user.md)
- [access org](./access-org.md)
- [access team](./access-team.md)
- [access service-account](./access-service-account.md)
- [access service-account token](./access-service-account-token.md)
