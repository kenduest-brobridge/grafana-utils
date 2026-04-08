# `grafana-util access service-account token`

## 目的

為 Grafana service account 新增或刪除 token。

## 使用時機

- 建立新的 service-account token。
- 依 service-account 名稱或 id 刪除既有 token。

## 採用前後對照

- **採用前**：service-account token 常常是在 Grafana UI 裡臨時建立，之後很難在另一個環境重做同樣操作。
- **採用後**：建立與清理 token 都會變成明確的 CLI 步驟，方便審查、腳本化，也能重複套用到同一個 service account。

## 成功判準

- token 建立會明確綁定到某個 service account，而不是只靠人工在 UI 裡找
- token 清理是刻意執行的步驟，尤其搭配 `--yes` 時更容易納入自動化
- 如果後續流程要接結果，能用 `--json` 取得可程式判讀的輸出

## 失敗時先檢查

- 如果建立 token 失敗，先確認指定的是正確的 `--name` 或 `--service-account-id`
- 如果刪除看起來沒作用，先核對 token 名稱，以及目前連到的是不是正確的 Grafana org 或環境
- 如果結果要交給自動化，請加 `--json`，並先驗證回傳 shape 再存檔或往下傳

## 主要旗標

- `add`: `--service-account-id` 或 `--name`, `--token-name`, `--seconds-to-live`, `--json`
- `delete`: `--service-account-id` 或 `--name`, `--token-id` 或 `--token-name`, `--prompt`, `--yes`, `--json`

## 範例

```bash
# 用途：替單一 service account 建立新的 token。
grafana-util access service-account token add --profile prod --name deploy-bot --token-name nightly
```

```bash
# 用途：審核後刪除一個 token。
grafana-util access service-account token delete --url http://localhost:3000 --basic-user admin --basic-password admin --name deploy-bot --token-name nightly --yes --json
```

```bash
# 用途：在終端機中先選 service account，再選 token，最後確認刪除。
grafana-util access service-account token delete --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --prompt
```

```bash
# 用途：用環境變數中的 token 建立新的 service-account token。
grafana-util access service-account token add --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --name deploy-bot --token-name nightly
```

## 相關命令

- [access](./access.md)
- [access service-account](./access-service-account.md)
