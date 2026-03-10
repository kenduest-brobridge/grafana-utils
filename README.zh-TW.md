# Grafana Utilities 中文說明

英文版： [README.md](README.md)

這個 repository 用來將 Grafana 設定匯出成 JSON，方便備份、搬移、再匯入，以及納入版本控制。

它提供兩個命令列工具：

- `cmd/grafana-utils.py`：匯出與匯入 dashboards
- `cmd/grafana-alert-utils.py`：匯出與匯入 alerting 資源，例如 alert rules、contact points、mute timings、notification policies、templates

當你有以下需求時，可以使用這個 repo：

- 從某個 Grafana instance 備份 dashboards 或 alerting 資源
- 將 dashboards 或 alerting 資源從一套 Grafana 搬移到另一套 Grafana
- 將 Grafana JSON 納入版本控制
- 準備可供 API 再匯入的 dashboard JSON，或準備可供 Grafana Web UI 匯入並重新對應 datasource 的 dashboard JSON

Dashboard 流程由 `cmd/grafana-utils.py` 處理。請使用明確的子命令，避免把匯出與匯入流程混在一起：

- `python3 cmd/grafana-utils.py export ...`
- `python3 cmd/grafana-utils.py import ...`

Alerting 流程由 `cmd/grafana-alert-utils.py` 獨立處理，因為 Grafana alerting 使用的 API 與檔案格式和 dashboard 不同。

相容性：

- 支援 RHEL 8 與更新版本
- 兩個 Python 入口腳本都維持 Python 3.6 語法相容，確保在 RHEL 8 環境中可被解析

預設匯出根目錄是 `dashboards/`。一次匯出會自動產生兩種格式：

- `dashboards/raw/`
- `dashboards/prompt/`

如果你想明確關掉其中一種輸出：

- `--without-raw`
- `--without-prompt`

## 模式

### `export` 參數

| 參數 | 用途 |
| --- | --- |
| `--url` | Grafana 基礎 URL。預設為 `http://127.0.0.1:3000`。 |
| `--api-token` | 使用 Grafana API token。若未提供，會回退到 `GRAFANA_API_TOKEN`。 |
| `--username` | Grafana 使用者名稱。若未提供，會回退到 `GRAFANA_USERNAME`。 |
| `--password` | Grafana 密碼。若未提供，會回退到 `GRAFANA_PASSWORD`。 |
| `--timeout` | HTTP timeout 秒數。預設為 `30`。 |
| `--verify-ssl` | 啟用 TLS 憑證驗證。預設關閉。 |
| `--export-dir` | Dashboard 匯出的根目錄。預設為 `dashboards/`。 |
| `--page-size` | Grafana dashboard search page size。預設為 `500`。 |
| `--flat` | 直接把檔案寫到匯出根目錄，不使用依 folder 分類的子目錄。 |
| `--overwrite` | 若匯出檔已存在則覆寫。 |
| `--without-raw` | 跳過 `dashboards/raw/` 匯出格式。 |
| `--without-prompt` | 跳過 `dashboards/prompt/` 匯出格式。 |

### `raw/` 匯出

- 保留 dashboard `uid`
- 保留 dashboard `title`
- 將 dashboard `id` 設為 `null`
- 保留原本的 datasource references，不做改寫

範例：

```bash
python3 cmd/grafana-utils.py export \
  --url http://127.0.0.1:3000 \
  --export-dir ./dashboards \
  --overwrite
```

如果你希望盡量少改動，並且想用相同 identity 將 dashboard 再匯入，請使用 `dashboards/raw/`。

如果你只想要 prompt 格式：

```bash
python3 cmd/grafana-utils.py export --export-dir ./dashboards --without-raw
```

### `prompt/` 匯出

`dashboards/prompt/` 會在同一次匯出流程中一起產生。這個格式用於 Grafana Web import，適合在匯入時讓 Grafana 詢問要把 datasource 對應到哪一個目標 datasource。

範例：

```bash
python3 cmd/grafana-utils.py export \
  --url http://127.0.0.1:3000 \
  --export-dir ./dashboards \
  --overwrite
```

這個 prompt 格式會遵循 Grafana Web import 預期的資料形狀：

- 產生非空的 `__inputs`
- 保留 `__elements`
- 在適用時新增或正規化 dashboard datasource 變數
- 將相依的 template-query datasource references 改寫成 `${DS_...}`
- 對於單一 datasource type 的 dashboards，將 panel datasource references 正規化為 `{"uid":"$datasource"}`

注意：

- 混合多種 datasource 的 dashboards 會保留明確的 `DS_...` placeholders，因為單一 `$datasource` 變數無法安全表示多組 datasource 對應。
- 沒有 datasource `type` 的 datasource 變數，例如 `{"uid":"$datasource"}`，無法安全轉成 Grafana import prompt，因此會原樣保留。

如果你只想要 raw 格式：

```bash
python3 cmd/grafana-utils.py export --export-dir ./dashboards --without-prompt
```

### API 匯入

`import` 會透過 Grafana API 匯入 dashboard JSON 檔案。

範例：

```bash
python3 cmd/grafana-utils.py import \
  --url http://127.0.0.1:3000 \
  --import-dir ./dashboards/raw \
  --replace-existing
```

這條路徑只適用於一般 dashboard JSON，不適用於 prompt JSON。包含 `__inputs` 的檔案應該改用 Grafana Web UI 匯入。

請明確把 `--import-dir` 指向 `dashboards/raw/`，不要指向合併後的 `dashboards/` 根目錄。

## 認證

你可以使用 API token，或是 username/password。

API token：

```bash
export GRAFANA_API_TOKEN='your-token'
python3 cmd/grafana-utils.py export --export-dir ./dashboards
```

使用者名稱與密碼：

```bash
export GRAFANA_USERNAME='your-user'
export GRAFANA_PASSWORD='your-pass'
python3 cmd/grafana-utils.py export --export-dir ./dashboards
```

## SSL

預設會停用 SSL 驗證。

如果你需要嚴格驗證：

```bash
python3 cmd/grafana-utils.py export --verify-ssl
```

## 匯入行為摘要

- `dashboards/raw/`：最適合在保留相同 dashboard `uid` 的情況下，以最少改動重新匯入。
- `dashboards/prompt/`：最適合用 Grafana Web import，並在匯入時重新做 datasource mapping。
- `python3 cmd/grafana-utils.py import --import-dir ./dashboards/raw`：最適合用 API 匯入一般 dashboard JSON。

## Alerting 工具

`cmd/grafana-alert-utils.py` 是處理 Grafana alerting 資源的獨立 CLI。它存在的目的，是把 alerting 邏輯與 `cmd/grafana-utils.py` 分開。

目前支援的範圍：

- alert rules
- contact points
- mute timings
- notification policies
- notification message templates
- 匯出為工具自有的 JSON 格式，存放在 `alerts/raw/`
- 將同一種工具自有格式再透過 Grafana alerting provisioning HTTP API 匯入

不支援的範圍：

- 直接重用 Grafana provisioning `/export` 產生的檔案來做 API 匯入

### Alerting 匯出

範例：

```bash
python3 cmd/grafana-alert-utils.py \
  --url http://127.0.0.1:3000 \
  --output-dir ./alerts \
  --overwrite
```

會寫出：

- `alerts/raw/rules/`
- `alerts/raw/contact-points/`
- `alerts/raw/mute-timings/`
- `alerts/raw/policies/`
- `alerts/raw/templates/`
- `alerts/index.json`

如果你想使用平面目錄結構：

```bash
python3 cmd/grafana-alert-utils.py --output-dir ./alerts --flat
```

常見使用範例：

API token：

```bash
export GRAFANA_API_TOKEN='your-token'

python3 cmd/grafana-alert-utils.py \
  --url https://grafana.example.com \
  --output-dir ./alerts \
  --overwrite
```

使用者名稱與密碼：

```bash
export GRAFANA_USERNAME='admin'
export GRAFANA_PASSWORD='secret'

python3 cmd/grafana-alert-utils.py \
  --url https://grafana.example.com \
  --output-dir ./alerts \
  --overwrite
```

### Alerting 匯入

範例：

```bash
python3 cmd/grafana-alert-utils.py \
  --url http://127.0.0.1:3000 \
  --import-dir ./alerts/raw \
  --replace-existing
```

匯入時若需要重對應 linked dashboard 或 panel：

```bash
python3 cmd/grafana-alert-utils.py \
  --url https://grafana.example.com \
  --import-dir ./alerts/raw \
  --replace-existing \
  --dashboard-uid-map ./dashboard-map.json \
  --panel-id-map ./panel-map.json
```

`dashboard-map.json` 範例：

```json
{
  "old-dashboard-uid": "new-dashboard-uid"
}
```

`panel-map.json` 範例：

```json
{
  "old-dashboard-uid": {
    "7": "19"
  }
}
```

行為說明：

- `--replace-existing` 會依 `uid` 更新既有 rules、依 `uid` 更新 contact points、依 `name` 更新 mute timings
- notification policies 一律使用 `PUT` 套用，因為 Grafana 將它們視為單一 policy tree
- notification templates 以 `PUT` 套用；若有 `--replace-existing`，工具會先讀取目前 template 版本，再在更新時一起送回
- 未使用 `--replace-existing` 時，rule/contact-point/mute-timing 匯入會走 create；若 identity 衝突，Grafana 會拒絕
- 未使用 `--replace-existing` 時，如果 template 名稱已存在，template 匯入會失敗
- 匯入只接受由 `cmd/grafana-alert-utils.py` 匯出的檔案
- 不要把 `--import-dir` 指到合併後的 `alerts/` 根目錄
- 當 linked alert rules 在匯入時需要重對應，請使用 `--dashboard-uid-map` 與 `--panel-id-map`
- 內部 matching 與 mapping 細節記錄在 `DEVELOPER.md`

重要限制：

- Grafana alert provisioning `/export` 的輸出不能直接拿來走這條匯入路徑
- Grafana 官方文件也說明 provisioning export format 是給 file/Terraform provisioning 使用，不是給 HTTP API 直接 round-trip update 使用

驗證方式：

- 單元測試：`python3 -m unittest -v`
- 開發期間會做 container-based end-to-end validation
- 已驗證 rules、contact points、mute timings、notification policies、notification templates，以及 dashboard-linked alert rules 的 export/import

## 驗證

執行測試：

```bash
python3 -m unittest -v
```
