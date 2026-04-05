# dashboard import

## 用途
透過 Grafana API 匯入儀表板 JSON 檔案。

## 何時使用
當您手上有本地匯出樹或 provisioning 樹，想把儀表板推回 Grafana，無論是實際執行或 dry run，都可以使用這個指令。這個指令只吃 `raw/` 或 `provisioning/` 輸入，不吃 Grafana UI 的 `prompt/` 路徑。

## 採用前後對照
- **採用前**：匯入比較像盲目 replay，folder、org 或 schema 問題往往要打到 live 後才知道。
- **採用後**：匯入會先變成可 preview 的回放步驟，先用 `--dry-run` 看清楚，再決定是否真的動 live。

## 重點旗標
- `--import-dir`：原始或合併匯出輸入的來源目錄。
- `--input-format`：選擇 `raw` 或 `provisioning`。
- `--org-id`、`--use-export-org`、`--only-org-id`、`--create-missing-orgs`：控制跨 org 路由。
- `--import-folder-uid`：強制指定目的資料夾 UID。
- `--ensure-folders`、`--replace-existing`、`--update-existing-only`：控制匯入行為。
- `--require-matching-folder-path`、`--require-matching-export-org`、`--strict-schema`、`--target-schema-version`：安全檢查。
- `--import-message`：儲存在 Grafana 的修訂訊息。
- `--interactive`、`--dry-run`、`--table`、`--json`、`--output-format`、`--output-columns`、`--no-header`、`--progress`、`--verbose`：預覽與回報控制。

## 成功判準
- dry-run 先把 create/update 動作列清楚，再進入 live replay
- 目的 org 與 folder 路由足夠明確，可以先 review
- 這次匯入使用的是正確的輸入 lane：`raw` 或 `provisioning`，不是 `prompt`

## 失敗時先檢查
- 如果 folder 或 org 落點不對，先檢查路由旗標，不要直接重跑 live import
- 如果看起來會刪或覆蓋太多，先停在 `--dry-run` 並回頭檢查匯出樹
- 如果 schema 被擋下來，先確認來源資料是不是需要先正規化再匯入

## 範例
```bash
# 用途：透過 Grafana API 匯入儀表板 JSON 檔案。
grafana-util dashboard import --profile prod --import-dir ./dashboards/raw --replace-existing
```

```bash
# 用途：透過 Grafana API 匯入儀表板 JSON 檔案。
grafana-util dashboard import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards/raw --dry-run --table
```

```bash
# 用途：透過 Grafana API 匯入儀表板 JSON 檔案。
grafana-util dashboard import --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --import-dir ./dashboards/raw --dry-run --table
```

## 相關指令
- [dashboard export](./dashboard-export.md)
- [dashboard raw-to-prompt](./dashboard-raw-to-prompt.md)
- [dashboard diff](./dashboard-diff.md)
- [dashboard publish](./dashboard-publish.md)
