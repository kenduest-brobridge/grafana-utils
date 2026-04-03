# dashboard import

## 用途
透過 Grafana API 匯入儀表板 JSON 檔案。

## 何時使用
當您手上有本地匯出樹或 provisioning 樹，想把儀表板推回 Grafana，無論是實際執行或 dry run，都可以使用這個指令。這個指令只吃 `raw/` 或 `provisioning/` 輸入，不吃 Grafana UI 的 `prompt/` 路徑。

## 重點旗標
- `--import-dir`：原始或合併匯出輸入的來源目錄。
- `--input-format`：選擇 `raw` 或 `provisioning`。
- `--org-id`、`--use-export-org`、`--only-org-id`、`--create-missing-orgs`：控制跨 org 路由。
- `--import-folder-uid`：強制指定目的資料夾 UID。
- `--ensure-folders`、`--replace-existing`、`--update-existing-only`：控制匯入行為。
- `--require-matching-folder-path`、`--require-matching-export-org`、`--strict-schema`、`--target-schema-version`：安全檢查。
- `--import-message`：儲存在 Grafana 的修訂訊息。
- `--interactive`、`--dry-run`、`--table`、`--json`、`--output-format`、`--output-columns`、`--no-header`、`--progress`、`--verbose`：預覽與回報控制。

## 範例
```bash
grafana-util dashboard import --profile prod --import-dir ./dashboards/raw --replace-existing
grafana-util dashboard import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards/raw --dry-run --table
grafana-util dashboard import --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --import-dir ./dashboards/raw --dry-run --table
```

## 相關指令
- [dashboard export](./dashboard-export.md)
- [dashboard raw-to-prompt](./dashboard-raw-to-prompt.md)
- [dashboard diff](./dashboard-diff.md)
- [dashboard publish](./dashboard-publish.md)
