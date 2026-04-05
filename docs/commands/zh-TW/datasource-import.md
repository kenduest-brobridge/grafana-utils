# datasource import

## 用途
透過 Grafana API 匯入 datasource inventory。

## 何時使用
當您有本地 datasource bundle 或 provisioning 樹，想把它推進 Grafana，無論是實際執行或 dry run，都可以使用這個指令。

## 重點旗標
- `--import-dir`：inventory 或 provisioning 輸入的來源路徑。
- `--input-format`：選擇 `inventory` 或 `provisioning`。
- `--org-id`、`--use-export-org`、`--only-org-id`、`--create-missing-orgs`：控制跨 org 路由。
- `--replace-existing`、`--update-existing-only`、`--require-matching-export-org`：匯入安全與重整控制。
- `--secret-values`：在匯入時解析佔位秘密值。
- `--dry-run`、`--table`、`--json`、`--output-format`、`--no-header`、`--output-columns`、`--progress`、`--verbose`：預覽與回報控制。

## 範例
```bash
# 用途：透過 Grafana API 匯入 datasource inventory。
grafana-util datasource import --profile prod --import-dir ./datasources --dry-run --table
```

```bash
# 用途：透過 Grafana API 匯入 datasource inventory。
grafana-util datasource import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./datasources --use-export-org --only-org-id 2 --create-missing-orgs --dry-run --json
```

```bash
# 用途：透過 Grafana API 匯入 datasource inventory。
grafana-util datasource import --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --import-dir ./datasources --dry-run --table
```

## 採用前後對照

- **採用前**：匯入 datasource bundle 時，常常要先手動整理檔案、org 與 secrets，然後才敢碰 Grafana。
- **採用後**：一個匯入指令就能先預覽計畫、整理 org 路由，再用適當的保護機制推進 bundle。

## 成功判準

- 匯入預覽會清楚顯示哪些 org 與 datasource 會被修改
- provisioning 與 inventory 兩種輸入都能正確路由
- secrets 在 live import 前就已經解開，不會等到送出後才發現問題

## 失敗時先檢查

- 如果匯入碰到錯的 org，先確認路由旗標再重跑
- 如果計畫看起來不完整，先確認 `--input-format` 與 bundle 是 inventory 還是 provisioning
- 如果 secrets 還沒解開，先檢查 placeholder 對照與提供的 secret 值

## 相關指令
- [datasource export](./datasource-export.md)
- [datasource diff](./datasource-diff.md)
- [datasource inspect-export](./datasource-inspect-export.md)
