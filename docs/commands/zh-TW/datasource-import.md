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
grafana-util datasource import --profile prod --import-dir ./datasources --dry-run --table
grafana-util datasource import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./datasources --use-export-org --only-org-id 2 --create-missing-orgs --dry-run --json
grafana-util datasource import --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --import-dir ./datasources --dry-run --table
```

## 相關指令
- [datasource export](./datasource-export.md)
- [datasource diff](./datasource-diff.md)
- [datasource inspect-export](./datasource-inspect-export.md)
