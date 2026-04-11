# `grafana-util access service-account`

## 目的

列出 live 或本機的 Grafana service account、建立、匯出、匯入、比對或刪除 Grafana service account，並管理其 token。

## 使用時機

- 檢視 service-account 清單。
- 從 live Grafana 或本機匯出套件中檢視 service account。
- 建立或更新 service-account 套件。
- 產生或刪除 service-account token。

## 採用前後對照

- **採用前**：service-account 工作常常先從 UI 手動找，再臨時做一次 token 動作。
- **採用後**：同一個命名空間可以處理 service-account inventory、套件管理、token 建立與刪除，而且輸入方式可重複。

## 成功判準

- service-account 變更會明確綁定到一個命名識別，而不是依賴一個模糊的 UI 點擊路徑
- token 操作足夠明確，可以被審查或腳本化
- inventory 與套件輸出能不費力地交給後續 access 或 workspace 工作流程

## 失敗時先檢查

- 如果 token add 或 delete 失敗，先確認 service account 名稱或 ID 是否對應到正確的目標環境
- 如果 inventory 看起來不完整，先核對認證範圍與 org context，再判斷是不是 service account 不存在
- 如果輸出要接給下一步，請先選好明確的 `--output-format`，不要依賴預設值

## 主要旗標

- `list`: `--input-dir`, `--query`, `--output-columns`, `--list-columns`, `--page`, `--per-page`, `--table`, `--csv`, `--json`, `--yaml`, `--output-format`
- `add`: `--name`, `--role`, `--disabled`, `--json`
- `export` 與 `diff`: `--output-dir` 或 `--diff-dir`, `--overwrite`, `--dry-run`
- `import`: `--input-dir`, `--replace-existing`, `--dry-run`, `--table`, `--json`, `--output-format`, `--yes`
- `delete`: `--name`, `--service-account-id`, `--prompt`, `--yes`, `--json`
- `token add`: `--service-account-id` 或 `--name`, `--token-name`, `--seconds-to-live`, `--json`
- `token delete`: `--service-account-id` 或 `--name`, `--token-id` 或 `--token-name`, `--prompt`, `--yes`, `--json`

## 範例

```bash
# 用途：在建立或刪除 token 前，先看清楚 service account。
grafana-util access service-account list --profile prod --output-format text
```

```bash
# 用途：先看本機存好的 service-account 套件。
grafana-util access service-account list --input-dir ./access-service-accounts --output-format table
```

```bash
# 用途：建立一個可重複使用在部署自動化裡的 service account。
grafana-util access service-account add --url http://localhost:3000 --basic-user admin --basic-password admin --name deploy-bot --role Editor --json
```

```bash
# 用途：替單一 service account 建立一個有名稱的 token。
grafana-util access service-account token add --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --name deploy-bot --token-name nightly
```

```bash
# 用途：在終端機中選一個 service account、確認目標，然後刪除。
grafana-util access service-account delete --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --prompt
```

## 相關命令

- [access](./access.md)
- [access service-account token](./access-service-account-token.md)
- [access user](./access-user.md)
