# Grafana Utils Python Architecture for Maintainers

這份文件是 `grafana_utils/` 的維運導覽。目標是：新加入人員不用先閱讀每一行原始碼，就能理解資料流、模組邊界與最小修改路徑。

## 1) Python CLI 整體職責

Python CLI 的定位是保留一個統一進入點、穩定的域名封裝與可測試的 workflow：

- `python/grafana-util.py`：source-tree wrapper，確保 `python3 python/grafana-util.py` 可直接呼叫未安裝環境中的套件入口。
- `grafana_utils/unified_cli.py`：統一路由層，不實作 domain 邏輯，只做 legacy / namespaced 命令正規化。
- `grafana_utils/dashboard_cli.py` / `alert_cli.py` / `access_cli.py` / `datasource_cli.py`：各 domain 的 facade，負責 parser + auth + client 生成 + dispatch。
- `grafana_utils/clients/*` + `grafana_utils/http_transport.py`：共享 HTTP transport 與 request/response 例外轉譯。

## 2) 檔案邊界與責任

### 2.1 統一進入點

- `grafana_utils/unified_cli.py`
  - 支援 legacy 類型：`export-dashboard`、`import-dashboard`、`list-dashboard`、`list-alert-*`、`export-alert` 等。
  - 支援 namespaced 類型：`grafana-util dashboard <cmd>`、`grafana-util alert <cmd>`、`grafana-util access <cmd>`、`grafana-util datasource <cmd>`。
  - 只做兩件事：正規化進入點、呼叫對應 facade 的 parser 並轉給 `main`。
  - 保留舊版 `-h` 行為，故對應 command help 會有較高維持成本；新增命令要同步兩條路徑時務必加到映射表。

### 2.2 Domain Facade（薄封裝）

- `grafana_utils/dashboard_cli.py`
  - 較胖但仍以「參數進/流程出」為主軸。
  - 常見職責：公共參數組裝、輸出格式正規化、例外邏輯關卡、workflow 依賴組裝與分流。
- `grafana_utils/alert_cli.py`
  - 同時處理 legacy 與 namespaced alert 命令名稱。
  - 分流後呼叫 `GrafanaAlertClient` + `alerts/provisioning.py` 的資源函式。
- `grafana_utils/access_cli.py`
  - 解析 `access parser` 產生參數，呼叫 `resolve_cli_auth_from_namespace` 後建立 `GrafanaAccessClient`，再交給 `access.workflows`。
- `grafana_utils/datasource_cli.py`
  - parser 常數與輸出模式在 `datasource/parser.py`。
  - 執行前做 `output-format` + dry-run 欄位正規化，最後 handoff 給 `datasource.workflows`。
  - 有 `__all__` 與 façade re-export，維持既有外部測試與 script 匯入介面。

### 2.3 Domain workflow / parser / model 分層

- Dashboard
  - `grafana_utils/dashboard_cli.py` 是 facade。
  - 低階輸出/格式：`dashboards/output_support.py`、`dashboards/progress.py`、`dashboards/common.py`。
  - 列舉/目錄/資料來源查詢：`dashboards/listing.py`、`dashboards/folder_support.py`。
  - 匯出/匯入/差異/檢查：`dashboards/export_workflow.py`、`dashboards/import_workflow.py`、`dashboards/diff_workflow.py`、`dashboards/inspection_workflow.py`。
  - 主要資料處理/轉換：`dashboards/transformer.py`、`dashboards/export_inventory.py`、`dashboards/import_support.py`。
  - 檢視輸出：`dashboards/inspection_summary.py`、`dashboards/inspection_report.py`、`dashboards/inspection_dispatch.py`、`dashboards/inspection_render.py`、`dashboards/inspection_governance.py`。
  - folder 路徑規則：`dashboards/folder_path_match.py`、`dashboards/folder_support.py`。
  - 報告分析器插件：`dashboards/inspection_analyzers/*`（`flux`、`sql`、`loki`、`prometheus`、`generic`、`dispatcher`）。
- Alert
  - `grafana_utils/alerts/provisioning.py`：export/import/diff 核心函式與文件轉換邏輯。
  - `grafana_utils/alerts/common.py`：錯誤型別、目錄/kind 常數、共用常量。
  - `grafana_utils/clients/alert_client.py`：純 HTTP 呼叫。
- Access
  - `grafana_utils/access/parser.py`：argparse wiring + 命令樹。
  - `grafana_utils/access/workflows.py`：validation + 解析 lookup + command dispatch。
  - `grafana_utils/access/models.py`：輸出格式化與列舉資料標準化。
  - `grafana_utils/access/common.py`：shared helper/錯誤類型。
- Datasource
  - `grafana_utils/datasource/parser.py`：parser/aliases/output 格式化定義。
  - `grafana_utils/datasource/workflows.py`：export/import/diff 核心流程。
  - `grafana_utils/datasource_contract.py`：匯入/差異的 strict schema 檢查。

## 3) 主要執行資料流

### Dashboard export (`grafana-util dashboard export`)

1. `python/grafana-util.py` → `unified_cli.parse_args`。
2. 透過 legacy/namespaced 映射到 `dashboard_cli` 的 `forwarded_argv`。
3. `dashboard_cli.parse_args` 解析 → `run`。
4. `run` 組裝 `GrafanaClient`（透過 `auth_staging.resolve_cli_auth_from_namespace`）。
5. 進入 `dashboards.export_workflow.run_export_dashboards(args, deps)`。
6. workflow 會：
   - 依 `all-orgs` / `org-id` 建立 scoped client
   - 拉 dashboard summary、folder、datasource inventory
   - 分別寫 `raw/`、`prompt/` variant
   - 寫 `index.json`、`export-metadata.json`、`folders.json`、`datasources.json`
7. 完成後回傳流程結果（stderr / summary lines）。

### Dashboard import (`grafana-util dashboard import`)

1. facade/dispatcher 同樣先完成 parser + client + 正規化。
2. 進入 `dashboards/import_workflow.run_import_dashboards(args, deps)`。
3. workflow 讀取 `RAW` manifest，驗證 contract、folder inventory、org 安全條件（可選）。
4. dry-run 模式：先產生 action record，不打 API。
5. live 模式：逐 dashboard 建構 import payload、可選 folder 確保、呼叫 `client.import_dashboard`。

### Dashboard inspect (`dashboard inspect-export` / `inspect-live`)

- `inspect-export`：直接讀取本地 `raw/` 目錄，重組 index + metadata -> summary/report/gov output。
- `inspect-live`：先由 workflow 物化暫存 raw-like 目錄，再走同一個 `run_inspect_export` pipeline，確保兩者輸出行為一致。

### Access (`grafana-util access user list`)

1. unified parser 進入 access facade。
2. `access_cli.run` 解析 auth，建立 `GrafanaAccessClient`。
3. 進入 `access.workflows.dispatch_access_command`，依 resource/command 做 validation、lookup、client API 呼叫。
4. 渲染與 summary 在 `models.py`，並輸出文字/JSON/csv/table。

### Datasource (`grafana-util datasource diff`)

1. parser `datasource.parser` 解析 flags + normalize。
2. `datasource_cli` 轉送到 `datasource.workflows.diff_datasources`。
3. workflows 讀 `diff-dir` 的 index/metadata + live 目錄，透過 `datasource_diff.py` 產生差異文檔。
4. 輸出以 `--table`、`--json`、`--csv` 規則輸出。

### Alert (`grafana-util alert list-alert-rules`)

1. unified facade 選到 alert domain。
2. `alert_cli.parse_args` 處理 legacy / namespaced alias。
3. `alert_cli.main` 建 auth client 後透過 `alerts/provisioning.py` 執行 list/export/import/diff 操作。
4. 與輸出相關的 render 與比較邏輯集中在 provisioning/common 模組，CLI 只做參數路由。

## 4) 維護建議：要改哪一層

- 新增/修改輸入參數
  - 先改 domain facade（`dashboard_cli.py`、`alert_cli.py`、`access_cli.py`、`datasource_cli.py` 或對應 parser 模組）。
  - 再補對應 `build_parser` / help / `__all__`（若有對外依賴）。
- 新增命令流程
  - 只在 facade + workflow 新增 dispatch，避免混雜到 unified_cli。
  - 對 workflow 模組新增專用測試輸出，確保 parser 與流程各自可獨立驗證。
- 修改 Grafana API 呼叫
  - 先改 `clients/*` 或 `http_transport.py`；workflows 只接收已封裝 client 行為。
- 重構輸出格式
  - `dashboards/listing.py`、`dashboards/diff` / `inspection_*`、`access/models.py`、`datasource` render helper 是第一優先修改點。

## 5) 常見維運風險（請先看）

- 避免在 facade 做業務邏輯：維護上最容易讓 parser、測試、domain 混在一起。
- 未更新 `unified_cli` legacy 映射會導致舊 CLI 參數「靜默無法啟動」。
- 變更 `resolve_auth` 前要先確認 `--token` / `--basic-user` / `--basic-password` 互斥規則，尤其是 `org-id` / `all-orgs` 的 Basic-only 路徑。
- Datasource contract 不是建議欄位，不一致 record 會在 `datasource_contract.py` 止於入口。
- `inspect-live` 會寫暫存 raw-like 目錄；若測試修改了 manifest 鍵值，務必同步 `inspection_workflow` 讀取路徑。

## 6) 維護者驗證指令

- `python3 -m unittest -v`（所有 Python 測試）
- `python3 -m unittest -v tests/test_python_dashboard_cli.py`
- `python3 -m unittest -v tests/test_python_dashboard_inspection_cli.py`
- `python3 -m unittest -v tests/test_python_access_cli.py`
- `python3 -m unittest -v tests/test_python_alert_cli.py`
- `python3 -m unittest -v tests/test_python_datasource_cli.py`
- `python3 -m unittest -v tests/test_python_packaging.py`

## 7) 設計意圖（摘要）

- 保持 `unified_cli` 為路由器，讓四個 domain 都能獨立演進。
- 保持 `dashboard_cli`、`access_cli`、`alert_cli`、`datasource_cli` 為穩定 API/CLI facade。
- 把大量可測試邏輯放進 workflow/helper modules，讓可測試性和 mock 替換更直接。
- 透過 `export` / `import` / `inspect` 的 manifest contract，讓不同命令可以共用同一套輸出/解析器，不改 API 的情況下可持續擴充。
