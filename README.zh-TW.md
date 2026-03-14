# Grafana Utilities 中文說明

英文版： [README.md](README.md)

這個 repository 用來將 Grafana dashboards、datasource inventory、alerting 資源與 access-management 操作統一到同一個 CLI 下，方便備份、搬移、再匯入，以及納入版本控制。

這裡提供一個主要 CLI，而且同時有兩種實作：

- `grafana-util`：統一的 dashboard、datasource、alerting、access-management CLI
- packaged Python 實作位於 [`grafana_utils/`](grafana_utils/)
- Rust 實作位於 [`rust/`](rust/)

這個 repo 適合以下情境：

- 從 Grafana 備份 dashboards 或 alerting 資源
- 在不同環境之間搬移 Grafana 內容
- 將 Grafana JSON 納入版本控制
- 準備可供 API 匯入的 dashboard JSON，或可供 Grafana Web UI 匯入並重新 mapping datasource 的 dashboard JSON

相容性：

- 支援 RHEL 8 與更新版本
- Python 入口腳本現在以 Python 3.9+ 作為語法與執行環境基線

## 目錄

- [總覽](#總覽)
- [選擇 Python 或 Rust](#選擇-python-或-rust)
- [快速開始](#快速開始)
- [Dashboard 工具](#dashboard-工具)
- [Datasource 工具](#datasource-工具)
- [Alerting 工具](#alerting-工具)
- [Access 工具](#access-工具)
- [Build 與安裝](#build-與安裝)
- [認證與 TLS](#認證與-tls)
- [輸出目錄結構](#輸出目錄結構)
- [驗證](#驗證)
- [文件說明](#文件說明)

## 總覽

這個 repo 現在使用一個主要命令名稱，底下再分 dashboard、datasource、alert、access 等區域：

- `grafana-util dashboard export ...`
- `grafana-util dashboard list ...`
- `grafana-util datasource list ...`
- `grafana-util dashboard inspect-export ...`
- `grafana-util dashboard inspect-live ...`
- `grafana-util dashboard import ...`
- `grafana-util dashboard diff ...`
- `grafana-util datasource export ...`
- `grafana-util datasource import ...`
- `grafana-util datasource diff ...`
- `grafana-util alert export ...`
- `grafana-util alert import ...`
- `grafana-util alert diff ...`
- `grafana-util alert list-rules ...`
- `grafana-util alert list-contact-points ...`
- `grafana-util alert list-mute-timings ...`
- `grafana-util alert list-templates ...`
- `grafana-util access user list ...`
- `grafana-util access user add ...`
- `grafana-util access user modify ...`
- `grafana-util access user delete ...`
- `grafana-util access team list ...`
- `grafana-util access team add ...`
- `grafana-util access team modify ...`
- `grafana-util access service-account ...`

相容性說明：

- 舊的 dashboard direct form，例如 `grafana-util export-dashboard ...`、`grafana-util list-dashboard ...` 仍可使用
- 舊的 alert direct form，例如 `grafana-util export-alert ...`、`grafana-util list-alert-rules ...` 仍可使用
- Rust 仍保留 `grafana-access-utils ...` 作為相容 binary，但 Python 主要只使用 `grafana-util access ...`

這個 repo 最重要的差別是 dashboard 匯出格式分成兩種：

- `dashboards/raw/`：給 Grafana API 再匯入
- `dashboards/prompt/`：給 Grafana Web UI 匯入時做 datasource mapping

## 選擇 Python 或 Rust

請依你的使用方式選擇：

| 選項 | 適合情境 | 指令 |
| --- | --- | --- |
| 已安裝的 Python package | 一般使用的預設路徑 | `grafana-util dashboard ...`、`grafana-util datasource ...`、`grafana-util alert ...`、`grafana-util access ...` |
| 直接從 git checkout 執行 Python | 開發或直接在 repo 內測試 | `python3 python/grafana-util.py dashboard ...`、`python3 python/grafana-util.py datasource ...`、`python3 python/grafana-util.py alert ...`、`python3 python/grafana-util.py access ...` |
| 直接從 git checkout 執行 Rust | 驗證或開發 Rust 實作 | `cargo run --bin grafana-util -- dashboard ...`、`cargo run --bin grafana-util -- alert ...`、`cargo run --bin grafana-util -- access ...` |

說明：

- Python package 是這個 repo 目前最直接的安裝與使用方式
- Rust binaries 是從 [`rust/`](rust/) 建立，不會被 `python3 -m pip install .` 安裝
- 兩種實作共用相同的命令名稱與操作概念

## 快速開始

Dashboard 匯出，同時產生 `raw/` 與 `prompt/`：

```bash
python3 python/grafana-util.py dashboard export \
  --url http://127.0.0.1:3000 \
  --basic-user admin \
  --basic-password admin \
  --export-dir ./dashboards \
  --overwrite
```

列出目前 Grafana 上的 dashboard，不寫出匯出檔：

```bash
python3 python/grafana-util.py dashboard list \
  --url http://127.0.0.1:3000 \
  --basic-user admin \
  --basic-password admin
```

用 table 顯示 dashboard，並帶出 folder tree path：

```bash
python3 python/grafana-util.py dashboard list \
  --table \
  --url http://127.0.0.1:3000 \
  --basic-user admin \
  --basic-password admin
```

用 JSON 做 raw export summary inspection：

```bash
python3 python/grafana-util.py dashboard inspect-export \
  --import-dir ./dashboards/raw \
  --json
```

補充：

- 目前 list 類與部分 dry-run 類命令已開始支援統一的 `--output-format`。
- 這一版先不改既有預設，只是新增一致的單旗標選法。
- 例如可用 `--output-format table|csv|json` 取代部分 command 的 `--table`、`--csv`、`--json`，以及用 `--output-format text|table|json` 控制部分 dry-run 輸出。

從 raw 匯出結果做 dashboard API 匯入：

```bash
python3 python/grafana-util.py dashboard import \
  --url http://127.0.0.1:3000 \
  --basic-user admin \
  --basic-password admin \
  --import-dir ./dashboards/raw \
  --replace-existing
```

將 dashboard 匯出檔與目前 Grafana 狀態做比對：

```bash
python3 python/grafana-util.py dashboard diff \
  --url http://127.0.0.1:3000 \
  --basic-user admin \
  --basic-password admin \
  --import-dir ./dashboards/raw
```

Datasource inventory 匯出：

```bash
python3 python/grafana-util.py datasource export \
  --url http://127.0.0.1:3000 \
  --basic-user admin \
  --basic-password admin \
  --export-dir ./datasources \
  --overwrite
```

Alerting 匯出：

```bash
python3 python/grafana-util.py alert export \
  --url http://127.0.0.1:3000 \
  --basic-user admin \
  --basic-password admin \
  --output-dir ./alerts \
  --overwrite
```

Alerting 匯入：

```bash
python3 python/grafana-util.py alert import \
  --url http://127.0.0.1:3000 \
  --basic-user admin \
  --basic-password admin \
  --import-dir ./alerts/raw \
  --replace-existing
```

Access user list：

```bash
python3 python/grafana-util.py access user list \
  --url http://127.0.0.1:3000 \
  --basic-user admin \
  --basic-password admin
```

## Dashboard 工具

`grafana-util dashboard` 使用明確的子命令：

- `list`
- `export`
- `inspect-export`
- `inspect-live`
- `import`
- `diff`

### 匯出格式

一次 dashboard 匯出預設會產生兩種格式：

- `dashboards/raw/`
- `dashboards/prompt/`

如果你只想保留其中一種，使用：

- `--without-dashboard-raw`
- `--without-dashboard-prompt`

`raw/` 適合：

- 保留相同 dashboard `uid`
- 盡量少改動內容
- 用 API 再匯入

`prompt/` 適合：

- 用 Grafana Web UI 匯入
- 在匯入時做 datasource mapping
- 需要 Grafana 風格的 `__inputs`

### 常用匯出參數

| 參數 | 用途 |
| --- | --- |
| `--url` | Grafana 基礎 URL，預設 `http://127.0.0.1:3000` |
| `--export-dir` | 匯出根目錄，預設 `dashboards/` |
| `--page-size` | Dashboard search page size，預設 `500` |
| `--flat` | 不建立依 folder 分類的子目錄 |
| `--overwrite` | 覆寫既有匯出檔案 |
| `--without-dashboard-raw` | 跳過 `raw/` 匯出 |
| `--without-dashboard-prompt` | 跳過 `prompt/` 匯出 |
| `--dry-run` | 預覽將會輸出的檔案，不真的寫入磁碟 |
| `--verify-ssl` | 啟用 TLS 憑證驗證 |

### Raw 匯出

Raw 匯出會盡量保留 Grafana dashboard identity：

- 保留 dashboard `uid`
- 保留 dashboard `title`
- 將數字型 dashboard `id` 設為 `null`
- 保留 datasource references，不改寫

如果你只要 prompt 格式：

```bash
python3 python/grafana-util.py dashboard export \
  --export-dir ./dashboards \
  --without-dashboard-raw
```

### Prompt 匯出

Prompt 匯出會把 dashboard 改寫成 Grafana Web import 比較能直接理解的格式：

- 產生非空的 `__inputs`
- 保留 `__elements`
- 把 datasource references 改寫成 import placeholders
- 若整個 dashboard 只使用一種 datasource type，可能會把 panel datasource refs 正規化成 `{"uid":"$datasource"}`

重要說明：

- 混合多種 datasource 的 dashboards 會保留明確的 `DS_...` placeholders
- 無法安全轉換的 untyped datasource variables 會保留原樣
- prompt JSON 是給 Grafana Web UI 匯入，不是給 API 匯入

如果你只要 raw 格式：

```bash
python3 python/grafana-util.py dashboard export \
  --export-dir ./dashboards \
  --without-dashboard-prompt
```

### Dashboard 匯入

Dashboard 匯入會透過 Grafana API 讀取一般 dashboard JSON。

範例：

```bash
python3 python/grafana-util.py dashboard import \
  --url http://127.0.0.1:3000 \
  --import-dir ./dashboards/raw \
  --replace-existing
```

只看 folder mismatch 相關欄位的 dry-run table：

```bash
python3 python/grafana-util.py dashboard import \
  --url http://127.0.0.1:3000 \
  --import-dir ./dashboards/raw \
  --dry-run \
  --output-format table \
  --output-columns uid,source_folder_path,destination_folder_path,reason,file
```

重要規則：

- `--import-dir` 要指向 `dashboards/raw/`，不是整個 `dashboards/`
- 不要把 `prompt/` 檔案拿去走 API 匯入
- 含有 `__inputs` 的檔案應該改走 Grafana Web UI 匯入
- `--import-folder-uid` 可覆寫所有匯入 dashboard 的目標 folder
- `--import-message` 可設定 dashboard version-history message
- `--dry-run` 只顯示每個 dashboard 會 create、update 或 fail，不真的呼叫匯入 API
- `--dry-run --table --output-columns uid,source_folder_path,destination_folder_path,reason,file` 可只顯示指定欄位，特別適合看 folder path mismatch
- `diff` 會把本地 raw 檔與目前 Grafana dashboard payload 做比較；若有差異，exit code 會是 `1`

Dashboard 匯出時也會在根目錄與各 variant 目錄額外寫入 `export-metadata.json`。它描述匯出 schema version，讓 `import` 與 `diff` 可以驗證目錄是否真的是預期的 `raw/` 匯出格式。

## Datasource 工具

`grafana-util datasource` 目前提供：

- `list`
- `export`
- `import`
- `diff`

常用形式：

- `python3 python/grafana-util.py datasource list ...`
- `python3 python/grafana-util.py datasource export ...`
- `python3 python/grafana-util.py datasource import ...`
- `python3 python/grafana-util.py datasource diff ...`

常用 dry-run table 範例：

```bash
python3 python/grafana-util.py datasource import \
  --url http://127.0.0.1:3000 \
  --basic-user admin \
  --basic-password admin \
  --import-dir ./datasources \
  --dry-run \
  --output-format table \
  --output-columns uid,action,org_id,file
```

## Alerting 工具

`grafana-util alert` 專門處理 Grafana alerting 資源。

支援的資源：

- alert rules
- contact points
- mute timings
- notification policies
- notification message templates

### Alerting 匯出

範例：

```bash
python3 python/grafana-util.py alert export \
  --url http://127.0.0.1:3000 \
  --basic-user admin \
  --basic-password admin \
  --output-dir ./alerts \
  --overwrite
```

若你想使用較平面的目錄結構：

```bash
python3 python/grafana-util.py alert export --output-dir ./alerts --flat
```

### Alerting 匯入

範例：

```bash
python3 python/grafana-util.py alert import \
  --url http://127.0.0.1:3000 \
  --basic-user admin \
  --basic-password admin \
  --import-dir ./alerts/raw \
  --replace-existing
```

若 linked dashboard 或 panel 需要重對應：

```bash
python3 python/grafana-util.py alert import \
  --url https://grafana.example.com \
  --basic-user admin \
  --basic-password admin \
  --import-dir ./alerts/raw \
  --replace-existing \
  --dashboard-uid-map ./dashboard-map.json \
  --panel-id-map ./panel-map.json
```

## Access 工具

`grafana-util access` 目前支援：

- `user list`
- `user add`
- `user modify`
- `user delete`
- `team list`
- `team add`
- `team modify`
- `service-account list`
- `service-account add`
- `service-account token add`

常用形式：

- `python3 python/grafana-util.py access user list ...`
- `python3 python/grafana-util.py access team list ...`
- `python3 python/grafana-util.py access service-account list ...`

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

### Alerting 匯入規則

- `--replace-existing` 會依 `uid` 更新既有 rules
- `--replace-existing` 會依 `uid` 更新既有 contact points
- `--replace-existing` 會依 `name` 更新既有 mute timings
- notification policies 一律用 `PUT`
- notification templates 使用 `PUT`；若有 `--replace-existing`，會先讀取目前版本
- 未使用 `--replace-existing` 時，rule/contact-point/mute-timing 匯入走 create，若衝突則 Grafana 會拒絕
- 未使用 `--replace-existing` 時，若 template 名稱已存在，template 匯入會失敗
- 匯入只接受由本工具匯出的檔案
- 不要把 `--import-dir` 指到整個 `alerts/` 根目錄
- `--dry-run` 會先預測每個檔案是會 create、update，還是因衝突而 fail，但不真的改 Grafana
- `--diff-dir` 會把本地匯出檔與 Grafana 目前 alerting 資源做比較；若有差異，exit code 會是 `1`

重要限制：

- Grafana 官方 alert provisioning `/export` 匯出的檔案，不是本工具支援的匯入格式
- 本工具只保證由 `grafana-util alert export` 匯出的檔案可以再匯入

原因說明：

- Grafana 的 alert provisioning `export` payload 比較偏向 provisioning 表示格式，不是設計成給 HTTP API 直接 round-trip 匯入
- Grafana 的 create/update API 預期的 request shape，和 `/export` 回傳的 response shape 並不相同
- 因為這兩套格式不對稱，所以本工具另外定義了自己的匯出格式來做備份與還原

對 linked alert rules：

- 當 dashboard 或 panel identity 已改變時，請使用 `--dashboard-uid-map` 與 `--panel-id-map`
- 關於 fallback matching 與修復行為的 maintainer 細節，請看 [`DEVELOPER.md`](DEVELOPER.md)

## Build 與安裝

### Python Package

安裝到目前的 Python 環境：

```bash
python3 -m pip install .
```

安裝到使用者自己的環境：

```bash
python3 -m pip install --user .
```

Python 3.9+ 若要安裝可選的 HTTP/2 依賴：

```bash
python3 -m pip install '.[http2]'
```

### Makefile 快捷指令

Repo 根目錄提供 [`Makefile`](Makefile)：

- `make help`
- `make build-python`
- `make build-rust`
- `make build`
- `make test-python`
- `make test-rust`
- `make test-rust-live`
- `make test`

輸出位置：

- `make build-python` 會把 wheel 放到 `dist/`
- `make build-rust` 會把 release binaries 放到 `rust/target/release/`

### Rust Build 與執行

建立 Rust release binaries：

```bash
make build-rust
```

直接從 repo 執行 Rust dashboard CLI：

```bash
cd rust
cargo run --bin grafana-util -- dashboard -h
```

直接從 repo 執行 Rust alerting CLI：

```bash
cd rust
cargo run --bin grafana-util -- alert -h
```

執行 Docker-backed 的 Rust live smoke test：

```bash
make test-rust-live
```

說明：

- 需要 Docker，且本機必須能存取 Docker daemon
- 預設使用 `grafana/grafana:12.4.1`，可用 `GRAFANA_IMAGE=...` 覆寫
- 預設使用隨機 localhost port；若要固定 port，可指定 `GRAFANA_PORT=43000`
- 會啟動暫時的 Grafana container，seed 一個 dashboard、一個 datasource、以及一個 contact point
- 會驗證 Rust dashboard 的 export/import/diff/dry-run 與 Rust alerting 的 export/import/diff/dry-run

## 認證與 TLS

支援的認證方式：

- `--token` 或環境變數 `GRAFANA_API_TOKEN`
- `--prompt-token`
- `--basic-user` / `--basic-password`
- `--basic-user` / `--prompt-password`

API token 範例：

```bash
export GRAFANA_API_TOKEN='your-token'
python3 python/grafana-util.py dashboard export --export-dir ./dashboards
```

互動式輸入 API token 範例：

```bash
python3 python/grafana-util.py dashboard export \
  --prompt-token \
  --export-dir ./dashboards
```

使用者名稱與密碼範例：

```bash
export GRAFANA_USERNAME='your-user'
export GRAFANA_PASSWORD='your-pass'
python3 python/grafana-util.py dashboard export --export-dir ./dashboards
```

補充：

- `--prompt-token` 會隱藏 token 輸入，不把值直接放進 shell history 或 process args
- `--prompt-password` 也是同樣概念，但用於 Basic auth 密碼
- token auth 與 Basic auth 不能混用

TLS 說明：

- 預設關閉 SSL verification
- 若你要嚴格驗證憑證，請加上 `--verify-ssl`

範例：

```bash
python3 python/grafana-util.py dashboard export --verify-ssl
```

## 輸出目錄結構

Dashboard 匯出目錄：

```text
dashboards/
  index.json
  export-metadata.json
  raw/
    export-metadata.json
    index.json
    ...
  prompt/
    export-metadata.json
    index.json
    ...
```

Alerting 匯出目錄：

```text
alerts/
  index.json
  raw/
    rules/
    contact-points/
    mute-timings/
    policies/
    templates/
```

## 驗證

常用驗證指令：

```bash
make test
python3 -m unittest -v
cd rust && cargo test
make test-rust-live
```

## 文件說明

- 英文 README：[`README.md`](README.md)
- 繁體中文 README：[`README.zh-TW.md`](README.zh-TW.md)
- 最近變更紀錄：[`CHANGELOG.md`](CHANGELOG.md)
- maintainer 與實作細節：[`DEVELOPER.md`](DEVELOPER.md)
