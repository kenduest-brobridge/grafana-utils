# dashboard export

## 用途
將儀表板匯出成 `raw/`、`prompt/` 與 `provisioning/` 檔案。

## 何時使用
當您需要一個本地匯出樹，供後續匯入、檢視、比對或檔案 provisioning 工作流程使用時，使用這個指令。`prompt/` 路徑是給 Grafana UI 匯入用，不是給 dashboard API 匯入用；如果您只有一般或 raw 的 dashboard JSON，需要先轉成 prompt JSON，請使用 `dashboard raw-to-prompt`。

## 重點旗標
- `--export-dir`：匯出樹的目標目錄。
- `--org-id`：匯出指定的 Grafana org。
- `--all-orgs`：把每個可見 org 匯出到各自的子目錄。建議使用 Basic auth。
- `--flat`：直接把檔案寫入各個匯出變體目錄。
- `--overwrite`：取代既有的匯出檔案。
- `--without-dashboard-raw`、`--without-dashboard-prompt`、`--without-dashboard-provisioning`：略過某個變體。
- `--provisioning-provider-name`、`--provisioning-provider-org-id`、`--provisioning-provider-path`：自訂產生的 provisioning provider 檔案。
- `--provisioning-provider-disable-deletion`、`--provisioning-provider-allow-ui-updates`、`--provisioning-provider-update-interval-seconds`：調整 provisioning 行為。
- `--dry-run`：預覽會寫出哪些內容。

## 說明
- 一般單一 org 匯出可優先用 `--profile`。
- `--all-orgs` 最好搭配管理員憑證支援的 `--profile` 或直接 Basic auth，因為 token 的可見範圍可能不足以涵蓋所有 org。

## 範例
```bash
grafana-util dashboard export --profile prod --export-dir ./dashboards --overwrite
grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --export-dir ./dashboards --overwrite
grafana-util dashboard export --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --export-dir ./dashboards --overwrite
```

## 相關指令
- [dashboard inspect-export](./dashboard-inspect-export.md)
- [dashboard import](./dashboard-import.md)
- [dashboard diff](./dashboard-diff.md)
- [dashboard raw-to-prompt](./dashboard-raw-to-prompt.md)
