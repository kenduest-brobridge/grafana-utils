# dashboard export

## 用途
將儀表板匯出成 `raw/`、`prompt/` 與 `provisioning/` 檔案。

## 何時使用
當您需要一個本地匯出樹，供後續匯入、檢視、比對或檔案 provisioning 工作流程使用時，使用這個指令。`prompt/` 路徑是給 Grafana UI 匯入用，不是給 dashboard API 匯入用；如果您只有一般或 raw 的 dashboard JSON，需要先轉成 prompt JSON，請使用 `dashboard raw-to-prompt`。

## 採用前後對照
- **採用前**：匯出比較像一次性的備份動作，之後能不能 review、inspect 或 replay，通常要走到下一步才知道。
- **採用後**：匯出會變成整條工作流的第一份 artifact，後面可以接 inspect、diff、dry-run import 與 Git review。

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

## 成功判準
- 產生出可供 API replay 與進一步 inspect 的 `raw/` 樹
- 如果需要較乾淨的 handoff，也有對應的 `prompt/` 樹
- 匯出結果足夠穩定，可直接拿去比對、審查或納入版本控制

## 失敗時先檢查
- 如果 dashboard 數量不對，先檢查 org 範圍，不要先懷疑 exporter
- 如果 `--all-orgs` 的輸出看起來不完整，先確認憑證是否真的看得到所有 org
- 如果下一步是匯入，先確認這次該沿用 `raw/` 還是 `prompt/`

## 範例
```bash
# 用途：將儀表板匯出成 `raw/`、`prompt/` 與 `provisioning/` 檔案。
grafana-util dashboard export --profile prod --export-dir ./dashboards --overwrite
```

```bash
# 用途：將儀表板匯出成 `raw/`、`prompt/` 與 `provisioning/` 檔案。
grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --export-dir ./dashboards --overwrite
```

```bash
# 用途：將儀表板匯出成 `raw/`、`prompt/` 與 `provisioning/` 檔案。
grafana-util dashboard export --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --export-dir ./dashboards --overwrite
```

## 相關指令
- [dashboard inspect-export](./dashboard-inspect-export.md)
- [dashboard import](./dashboard-import.md)
- [dashboard diff](./dashboard-diff.md)
- [dashboard raw-to-prompt](./dashboard-raw-to-prompt.md)
