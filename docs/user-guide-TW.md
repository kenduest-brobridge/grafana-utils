Grafana Utilities 維運指南 (繁體中文)
===================================

本指南以**統一 CLI 介面**為主，使用 `grafana-util ...` 作為範例指令。相同的命令模型適用於安裝後的 CLI，以及專案內不同實作所提供的對應入口：

- **全域參數優先**：通用於所有指令的設定。
- **功能模組獨立**：依資源類型（Dashboard、Alert、Datasource、Access）劃分。
- **情境導向設計**：每個參數 (Flag) 皆標註了用途、差異與適用情境。
- **安全第一**：內建互斥規則與標準作業程序 (SOP) 建議。

1) 全域前置說明
------------

開始之前，您可以透過以下指令確認各模組的說明資訊：

```bash
grafana-util -h
grafana-util dashboard -h
grafana-util alert -h
grafana-util datasource -h
grafana-util access -h
```

安裝後可直接使用：

```text
grafana-util <domain> <command> [options]
```

### 入口點說明：
- **`grafana-util`**: 統一調度器（Unified Dispatcher），支援 `dashboard/alert/datasource/access`。
- 統一 CLI 請使用命名空間形式：`grafana-util <domain> <command>`。
- `dashboard list-data-sources` 仍可使用，但新的資料來源盤點流程應優先改用 `datasource list`。

2) 全域通用參數
----------------

補充預設值：

- `dashboard` / `datasource` 模組預設 `--url` 為 `http://localhost:3000`。
- `alert` / `access` 模組預設 `--url` 為 `http://127.0.0.1:3000`。

| 參數 | 用途 | 適用情境 |
| --- | --- | --- |
| `--url` | Grafana 基礎網址 | 幾乎所有線上（Live）操作 |
| `--token`、`--api-token` | API Token | 適用於自動化腳本、非互動式執行 |
| `--basic-user` | Basic Auth 使用者名稱 | 執行組織管理 (All Orgs) 或 Team 管理時必須 |
| `--basic-password` | Basic Auth 密碼 | 建議搭配 `--prompt-password` 使用以增加安全性 |
| `--prompt-token` | 互動式輸入 Token | CI / 不想在參數記錄中洩漏 Token |
| `--prompt-password` | 互動式輸入密碼 | 跨機器帳號操作時建議使用 |
| `--timeout` | HTTP 請求逾時時間 | 處理大規模資料或網路不穩時可調高（預設 30s） |
| `--verify-ssl` | 啟用 TLS 憑證驗證 | 生產環境建議開啟（預設為關閉） |

### 命令分區（快速導覽）

- Dashboard：`dashboard export`、`dashboard list`、`dashboard import`、`dashboard diff`、`dashboard inspect-export`、`dashboard inspect-live`、`dashboard inspect-vars`、`dashboard screenshot`
- Alert：`alert export`、`alert import`、`alert diff`、`alert list-rules`、`alert list-contact-points`、`alert list-mute-timings`、`alert list-templates`
- Datasource：`datasource list`、`datasource export`、`datasource import`、`datasource diff`
- Access：`access org list`、`access org add`、`access org modify`、`access org delete`、`access org export`、`access org import`、`access user list`、`access user add`、`access user modify`、`access user delete`、`access user export`、`access user import`、`access user diff`、`access team list`、`access team add`、`access team modify`、`access team delete`、`access team export`、`access team import`、`access team diff`、`access service-account list`、`access service-account add`、`access service-account export`、`access service-account import`、`access service-account diff`、`access service-account delete`、`access service-account token add`、`access service-account token delete`

### 指令功能矩陣 (Feature Matrix)

本表可協助您快速確認各類 Grafana 資源的支援程度，以便選擇合適的指令執行資產盤點或狀態同步。

| 資源類型 | List（列表） | Export（匯出） | Import（匯入） | Diff（差異比對） | Inspect（分析） | Add（新增） | Modify（修改） | Delete（刪除） | 備註 |
| --- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | --- |
| **Dashboard** | Yes | Yes | Yes | Yes | Yes | No | No | No | 適合資產盤點、備份與環境遷移 |
| **Datasource** | Yes | Yes | Yes | Yes | No | No | No | No | 支援組態漂移檢查與環境同步 |
| **Alerting** | Yes | Yes | Yes | Yes | No | No | No | No | 涵蓋 Rules, Contact Points, Mute Timings |
| **Organization** | Yes | Yes | Yes | No | No | Yes | Yes | Yes | 支援 org 盤點與 membership replay |
| **User** | Yes | Yes | Yes | Yes | No | Yes | Yes | Yes | 支援全域或組織範圍的使用者盤點 |
| **Team / Group** | Yes | Yes | Yes | Yes | No | Yes | Yes | Yes | 包含成員關係（Membership）同步 |
| **Service Account** | Yes | Yes | Yes | Yes | No | Yes | Yes | Yes | 生命週期管理與 Token 簽發 |
| **SA Token** | Yes | No | No | No | No | Yes | No | Yes | Token 建立與撤銷 |

認證互斥規則（由 CLI parser 強制執行）：

1. `--token` 不可與 `--basic-user` 同時使用。
2. `--token` 不可與 `--prompt-token` 同時使用。
3. `--basic-password` 不可與 `--prompt-password` 同時使用。
4. `--prompt-password` 必須同時提供 `--basic-user`。

3) dashboard 指令模組
-----------------

### 3.1 `dashboard export` (資產匯出)

**用途**：將線上環境的儀表板下載為本地 JSON 檔案。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--export-dir` | 匯出根目錄（預設 `dashboards`） | 輸出將包含 `raw/` 與 `prompt/` |
| `--page-size` | 分頁抓取筆數（預設 `500`） | 處理大規模庫時可降低請求頻率 |
| `--org-id` | 指定特定組織 ID | 與 `--all-orgs` 互斥；通常需搭配 Basic Auth |
| `--all-orgs` | 匯出所有可見組織的資源 | 僅支援 Basic Auth；不支援 API Token |
| `--flat` | 不保留資料夾結構 | 使目錄比對或大量匯入流程更為穩定 |
| `--overwrite` | 覆蓋既有檔案 | 適用於 CI/CD 流程或定期備份 |
| `--dry-run` | 僅模擬執行匯出路徑 | 在實際寫入磁碟前驗證權限與索引 |
| `--progress` | 顯示進度提示 | 適合人工執行時觀察 |
| `-v`, `--verbose` | 詳細日誌輸出 | 會覆蓋進度提示 |

範例指令：
```bash
grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./dashboards --overwrite
```

範例輸出：
```text
Exported raw    cpu-main -> dashboards/raw/Infra/CPU__cpu-main.json
Exported prompt cpu-main -> dashboards/prompt/Infra/CPU__cpu-main.json
Exported raw    mem-main -> dashboards/raw/Infra/MEM__mem-main.json
Exported prompt mem-main -> dashboards/prompt/Infra/MEM__mem-main.json
Dashboard export completed: 2 dashboard(s), 4 file(s) written
```

### 3.2 `dashboard list`

**用途**：列出線上的 dashboards。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--page-size`（預設 `500`） | 每頁筆數 | 針對大型環境調整可減少 API 呼叫次數 |
| `--org-id` | 指定單一 org | 與 `--all-orgs` 互斥 |
| `--all-orgs` | 匯總多 org | 大部分會配合 basic user |
| `--with-sources` | table/csv 時補齊 datasource 名稱 | 較慢；JSON 已內含 uid / name |
| `--table` | 表格輸出（預設） | 人工閱讀 |
| `--csv` | CSV | 外部報表 |
| `--json` | JSON | 自動比對 / 自動化 |
| `--output-format table\|csv\|json` | 單一輸出旗標取代三旗標 | 互斥關係與 parser 一致 |
| `--no-header` | 表格不顯示欄位列 | 只取輸出內容時方便 diff |

範例指令：
```bash
grafana-util dashboard list --url http://localhost:3000 --basic-user admin --basic-password admin --with-sources --table
```

範例輸出：
```text
UID              TITLE            FOLDER   TAGS        DATASOURCES
cpu-main         CPU Overview     Infra    ops,linux   prometheus-main
mem-main         Memory Overview  Infra    ops,linux   prometheus-main
latency-main     API Latency      Apps     api,prod    loki-prod
```

範例指令（JSON）：
```bash
grafana-util dashboard list --url http://localhost:3000 --token <TOKEN> --json
```

```json
[
  {
    "uid": "cpu-main",
    "title": "CPU Overview",
    "folder": "Infra",
    "tags": ["ops", "linux"]
  }
]
```

### 3.3 `dashboard list-data-sources`（相容保留；建議改用 `datasource list`）

**用途**：保留舊的 dashboard datasource 盤點入口，同時把新的腳本與文件導向 `datasource list`。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--table` | 表格輸出 | 人工巡檢 |
| `--csv` | CSV 輸出 | 批次匯出 |
| `--json` | JSON 輸出 | API 串接 |
| `--output-format table/csv/json` | 單一輸出旗標 | 與上述三旗標互斥 |
| `--no-header` | 不列表頭 | 只取值對比 |

範例指令：
```bash
grafana-util datasource list --url http://localhost:3000 --basic-user admin --basic-password admin --table
```

範例輸出：
```text
UID                NAME               TYPE         IS_DEFAULT
prom-main          prometheus-main    prometheus   true
loki-prod          loki-prod          loki         false
tempo-prod         tempo-prod         tempo        false
```

建議路徑：
- 新的自動化腳本、範例與維運文件請優先使用 `5.1 datasource list`。

### 3.4 `dashboard import`（legacy `import-dashboard`）

**用途**：將 `raw/` 導入線上的 dashboards。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--import-dir`（必須） | 指向 `raw/` 目錄或 multi-org export root | 一般匯入用 `raw/`；搭配 `--use-export-org` 時改指向整個匯出根目錄 |
| `--org-id` | 匯入到目標 org | 搭配 basic auth 使用 |
| `--use-export-org` | 依 export 內 org 路由回 Grafana | 匯入 `--all-orgs` 產生的整體匯出根目錄 |
| `--only-org-id` | 限制 `--use-export-org` 只匯入指定 source org | 可重複指定多個 org |
| `--create-missing-orgs` | 路由匯入前自動建立缺少的目標 org | 僅限 `--use-export-org`；搭配 `--dry-run` 時只模擬執行 `would-create-org`，不真的建立 |
| `--import-folder-uid` | 強制匯入到指定 folder uid | 目錄整理統一時使用 |
| `--ensure-folders` | 遇到缺少 folder 自動建立 | 大批匯入前配合 `--dry-run` 驗證 |
| `--replace-existing` | 已存在即覆蓋更新 | 跨環境遷移常用 |
| `--update-existing-only` | 僅更新已存在，不新增 | 僅補齊現場缺失 |
| `--require-matching-folder-path` | folder path 不一致就不更新 | 防止放錯資料夾 |
| `--require-matching-export-org` | 匯入前檢查 export org 與目標 org 一致 | 跨 org 安全機制 |
| `--import-message` | dashboard 版本訊息 | 審計註記 |
| `--dry-run` | 僅模擬執行 import 行為 | 先確認 `create/update/skip` |
| `--table` | dry-run 時顯示表格摘要 | 需要 `--output-columns` 時也用此輸出 |
| `--json` | dry-run 時輸出 JSON 摘要 | 與 `--table` 互斥 |
| `--output-format text/table/json` | dry-run 專用輸出代換旗標 | `text` 為預設摘要行為 |
| `--output-columns` | dry-run table 欄位白名單 | 僅 `--dry-run --table` 有效 |
| `--no-header` | table 不輸出表頭 | 僅 `--dry-run --table` |
| `--progress` | 匯入進度 | 大量匯入時便於追蹤 |
| `-v`, `--verbose` | 每筆詳細訊息，覆蓋 `--progress` | 疑難排解時使用 |

範例指令：
```bash
grafana-util dashboard import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards/raw --replace-existing --dry-run --table
```

範例輸出：
```text
UID          TITLE            ACTION   DESTINATION   FOLDER
cpu-main     CPU Overview     update   existing      Infra
mem-main     Memory Overview  create   missing       Infra

Dry-run checked 2 dashboard(s)
```

### 3.5 `dashboard diff`

**用途**：比較本地 `raw/` 與線上狀態。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--import-dir`（必須） | 指向 raw 匯出目錄 | 僅比對，不改寫 API |
| `--import-folder-uid` | 比對時覆寫 folder UID 對應關係 | 目錄與目標 folder 不一致修正 |
| `--context-lines`（預設 `3`） | diff 上下文行數 | 大文件可提高觀察粒度 |

範例指令：
```bash
grafana-util dashboard diff --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards/raw
```

範例輸出：
```text
Dashboard diff found 1 differing item(s).

--- live/cpu-main
+++ export/cpu-main
@@
-  "title": "CPU Overview"
+  "title": "CPU Overview v2"
```

### 3.6 `dashboard inspect-export`

**用途**：離線分析 raw/export 內容。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--import-dir`（必須） | 指向 raw/ 目錄 | 不連線線上 API |
| `--json` | JSON 輸出 | 與 `--table`/`--report*` 互斥 |
| `--table` | 表格輸出 | 與 `--json` 互斥 |
| `--report` | report mode 快捷；可為空值 | 取預設 report table 或指定 csv/json/tree/governance |
| `--output-format text\|table\|json\|report-table\|report-csv\|report-json\|report-tree\|report-tree-table\|governance\|governance-json` | 單一輸出旗標 | 與 `--json`、`--table`、`--report` 互斥 |
| `--report-columns` | report 輸出欄位白名單 | 僅 report/table/csv/tree-table 類有意義 |
| `--report-filter-datasource` | report/filter 的 datasource 精準匹配 | 問題來源鑑別 |
| `--report-filter-panel-id` | report/filter 的 panel id 精準匹配 | 查單面板差異 |
| `--help-full` | 顯示完整 report 範例與欄位說明 | 首次導入常用 |
| `--no-header` | 表格/可表格化 report 不列表頭 | 便於比對輸出 |

範例指令：
```bash
grafana-util dashboard inspect-export --import-dir ./dashboards/raw --output-format report-table
```

範例輸出：
```text
UID           TITLE             PANEL_COUNT   DATASOURCES
cpu-main      CPU Overview      6             prometheus-main
mem-main      Memory Overview   4             prometheus-main
latency-main  API Latency       8             loki-prod
```

### 3.7 `dashboard inspect-live`

**用途**：live dashboard 即時快照分析（同 inspect-export 的報表邏輯）。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--page-size`（預設 `500`） | 線上資料分頁控制 | 大型環境可先降低每頁筆數以避免逾時 |
| `--org-id` | 指定單一 org | 與 `--all-orgs` 互斥 |
| `--all-orgs` | 跨可見 org 聚合 | 用於跨組織總覽盤點 |
| `--json` / `--table` / `--report` / `--output-format*` | 與 `inspect-export` 完全同義 | 可直接對比離線/線上 |
| `--help-full` | 進一步說明 report 參數 | 導入/診斷複雜情境 |
| `--no-header` | 不列表頭 | 主要供腳本處理 |

範例指令：
```bash
grafana-util dashboard inspect-live --url http://localhost:3000 --basic-user admin --basic-password admin --output-format governance-json
```

範例輸出：
```json
[
  {
    "uid": "cpu-main",
    "title": "CPU Overview",
    "datasource_count": 1,
    "status": "ok"
  }
]
```

### 3.8 `dashboard inspect-vars`

**用途**：在執行 `dashboard screenshot` 前，先檢查 live dashboard 的 templating variables 與目前 query state。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--dashboard-uid` | 指定 dashboard UID | 適合 API 導向檢查 |
| `--dashboard-url` | 直接沿用完整瀏覽器 URL | 自動帶入 UID 與 query state |
| `--vars-query` | 疊加 `${__all_variables}` 形式的 query fragment | 只有 `var-*` 片段時很好用 |
| `--org-id` | 指定單一 org | 送出 `X-Grafana-Org-Id` |
| `--output-format` | 指定 table、csv、json | 腳本通常用 json |
| `--no-header` | 隱藏 table/CSV 表頭 | shell pipeline 較乾淨 |

範例指令：
```bash
grafana-util dashboard inspect-vars --url https://192.168.1.112:3000 --dashboard-uid rYdddlPWk --vars-query 'var-datasource=bMcTJFtVz&var-job=node-exporter&var-node=192.168.1.112:9100&var-diskdevices=%5Ba-z%5D%2B%7Cnvme%5B0-9%5D%2Bn%5B0-9%5D%2B%7Cmmcblk%5B0-9%5D%2B&refresh=1m&showCategory=Panel%20links&timezone=browser' --basic-user admin --basic-password admin --output-format table
```

範例輸出：
```text
NAME         TYPE        LABEL       CURRENT                                DATASOURCE     OPTIONS
datasource   datasource  Datasource  bMcTJFtVz
job          query       Job         node-exporter                          ${datasource}
node         query       Host        192.168.1.112:9100                     ${datasource}
diskdevices  custom                  [a-z]+|nvme[0-9]+n[0-9]+|mmcblk[0-9]+                 [a-z]+|nvme[0-9]+n[0-9]+|mmcblk[0-9]+
```

### 3.9 `dashboard screenshot`

**用途**：以 headless Chromium 開啟 Grafana dashboard，並輸出 PNG、JPEG、PDF；可重播接近瀏覽器當下的 state。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--dashboard-uid` / `--dashboard-url` | 選擇 dashboard 目標 | URL 模式可直接保留瀏覽器 query state |
| `--panel-id` | 擷取單一 panel | 使用 Grafana `d-solo` route |
| `--vars-query` | 重播 `var-*` 與相容 query key | 支援 `refresh`、`showCategory`、`timezone` 與 `${__all_variables}` 片段 |
| `--print-capture-url` | 印出最終解析後的 URL | 除錯時非常實用 |
| `--full-page` | 輸出長頁 dashboard 圖 | 瀏覽器式 full-page 擷取 |
| `--browser-path` | 指定 Chrome/Chromium 路徑 | 工作站有多個瀏覽器時可固定版本 |
| `--header-title`、`--header-url`、`--header-captured-at`、`--header-text` | 在 PNG/JPEG 前面加深色 header 區塊 | header 在最終圖片合成，不會干擾 Grafana layout |

範例指令：
```bash
grafana-util dashboard screenshot --url https://192.168.1.112:3000 --dashboard-uid rYdddlPWk --panel-id 20 --vars-query 'var-datasource=bMcTJFtVz&var-job=node-exporter&var-node=192.168.1.112:9100&var-diskdevices=%5Ba-z%5D%2B%7Cnvme%5B0-9%5D%2Bn%5B0-9%5D%2B%7Cmmcblk%5B0-9%5D%2B&refresh=1m&showCategory=Panel%20links&timezone=browser' --basic-user admin --basic-password admin --output /tmp/node-exporter-full-panel-20-header.png --header-title --header-url --header-captured-at --header-text 'Solo panel debug capture' --print-capture-url --browser-path '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' --wait-ms 20000
```

範例輸出：
```text
Capture URL: https://192.168.1.112:3000/d-solo/rYdddlPWk/node-exporter-full?refresh=1m&showCategory=Panel+links&timezone=browser&panelId=20&viewPanel=20&theme=dark&kiosk=tv&var-datasource=bMcTJFtVz&var-job=node-exporter&var-node=192.168.1.112%3A9100&var-diskdevices=%5Ba-z%5D%2B%7Cnvme%5B0-9%5D%2Bn%5B0-9%5D%2B%7Cmmcblk%5B0-9%5D%2B
```

已驗證輸出檔：
```text
/tmp/node-exporter-full-panel-20-header-v2.png
```

4) alert 命令
-------------

### 4.1 `alert export`

**用途**：匯出 alerting 資源為 raw JSON。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--output-dir`（預設 `alerts`） | 匯出根目錄 | 與 dashboard 區分管理 |
| `--flat` | 不保留子目錄階層 | 大量檔名變更時更好比對 |
| `--overwrite` | 覆蓋 existing 檔案 | 重跑前置步驟 |

範例指令：
```bash
grafana-util alert export --url http://localhost:3000 --basic-user admin --basic-password admin --output-dir ./alerts --overwrite
```

範例輸出：
```text
Exported rule          alerts/raw/rules/cpu_high.json
Exported contact point alerts/raw/contact-points/oncall_webhook.json
Exported template      alerts/raw/templates/default_message.json
Alert export completed: 3 resource(s) written
```

### 4.2 `alert import`（legacy `import-alert`）

**用途**：將 alert raw 匯入 Grafana。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--import-dir`（必須） | 指向 alert `raw/` 目錄 | 不能指向上層目錄 |
| `--replace-existing` | 已存在則更新 | 常見於正式匯入覆寫 |
| `--dry-run` | 僅模擬執行，不真的送 API | 建議先確認變更範圍 |
| `--dashboard-uid-map` | dashboard uid 對照檔 | linked rule 在目標系統 UID 變更時必備 |
| `--panel-id-map` | panel id 對照檔 | 修復 linked alert 內 panel 參考 |

範例指令：
```bash
grafana-util alert import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./alerts/raw --replace-existing --dry-run
```

範例輸出：
```text
kind=contact-point name=oncall-webhook action=would-update
kind=rule-group name=linux-hosts action=would-create
kind=template name=default_message action=no-change
```

### 4.3 `alert diff`（legacy `diff-alert`）

**用途**：比較本地 alert raw 與線上內容。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--diff-dir`（必須） | 指向 raw 目錄 | 比對本地匯出與線上狀態的基準目錄 |
| `--dashboard-uid-map` | dashboard 對映，確保跨環境比對一致 | 跨環境 UID 不一致時使用 |
| `--panel-id-map` | panel 對映，修正 linked path | panel 編號差異時使用 |

範例指令：
```bash
grafana-util alert diff --url http://localhost:3000 --basic-user admin --basic-password admin --diff-dir ./alerts/raw
```

範例輸出：
```text
Diff different

resource=contact-point name=oncall-webhook
- url=http://127.0.0.1/notify
+ url=http://127.0.0.1/updated
```

### 4.4 `alert list-rules`（legacy `list-alert-rules`）
### 4.5 `alert list-contact-points`（legacy `list-alert-contact-points`）
### 4.6 `alert list-mute-timings`（legacy `list-alert-mute-timings`）
### 4.7 `alert list-templates`（legacy `list-alert-templates`）

**用途**：四個 list 命令共用，依名稱回報不同資源。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--table` | 表格輸出（預設） | 人工閱讀 |
| `--csv` | CSV 輸出 | 匯出到外部工具 |
| `--json` | JSON 輸出 | 自動化 |
| `--output-format table\|csv\|json` | 取代 `--table/--csv/--json` 的統一入口 | 建議優先使用的統一寫法 |
| `--no-header` | 不列表頭（table 類） | 結構化比對 |

範例指令：
```bash
grafana-util alert list-rules --url http://localhost:3000 --basic-user admin --basic-password admin --table
```

範例輸出：
```text
UID                 TITLE              FOLDER        CONDITION
cpu-high            CPU High           linux-hosts   A > 80
memory-pressure     Memory Pressure    linux-hosts   B > 90
api-latency         API Latency        apps-prod     C > 500
```

`alert list-contact-points` 範例輸出：
```text
UID               NAME             TYPE      DESTINATION
oncall-webhook    Oncall Webhook   webhook   http://alert.example.com/hook
slack-primary     Slack Primary    slack     #ops-alerts
```

`alert list-mute-timings` 範例輸出：
```text
NAME                 INTERVALS
maintenance-window   mon-fri 01:00-02:00
release-freeze       sat-sun 00:00-23:59
```

`alert list-templates` 範例輸出：
```text
NAME               PREVIEW
default_message    Alert: {{ .CommonLabels.alertname }}
ops_summary        [{{ .Status }}] {{ .CommonLabels.severity }}
```

5) datasource 命令
------------------

### 5.1 `datasource list`

**用途**：列出線上的 datasource。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--table` | 表格輸出 | 人工掃描 |
| `--csv` | CSV 輸出 | 報表 |
| `--json` | JSON 輸出 | 腳本 |
| `--output-format table\|csv\|json` | 取代三旗標 | 建議優先使用的統一寫法 |
| `--no-header` | 不列 header | 比對輸出 |

範例指令：
```bash
grafana-util datasource list --url http://localhost:3000 --token <TOKEN> --table
```

範例輸出：
```text
UID                NAME               TYPE         URL
prom-main          prometheus-main    prometheus   http://prometheus:9090
loki-prod          loki-prod          loki         http://loki:3100
tempo-prod         tempo-prod         tempo        http://tempo:3200
```

### 5.2 `datasource export`

**用途**：匯出 datasource inventory。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--export-dir`（預設 `datasources`） | 匯出目錄 | 含 `datasources.json` + metadata |
| `--org-id` | 匯出指定 org | 僅 Basic Auth 支援明確 org 匯出 |
| `--all-orgs` | 匯出所有可見 org | 每個 org 會寫入 `org_<id>_<name>/` 子目錄 |
| `--overwrite` | 覆蓋既有輸出 | 適合重複匯出流程 |
| `--dry-run` | 僅列出預期輸出，不實際寫入檔案 | 先確認輸出目錄與範圍 |

範例指令：
```bash
grafana-util datasource export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./datasources --overwrite
```

範例輸出：
```text
Exported datasource inventory -> datasources/datasources.json
Exported metadata            -> datasources/export-metadata.json
Datasource export completed: 3 item(s)
```

實跑註記：
- 上面的命令型態已在 `make test-python-datasource-live` 與 `make test-rust-live` 中，對真實 Grafana `12.4.1` Docker 服務驗證過。

### 5.3 `datasource import`

**用途**：匯入 datasource inventory。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--import-dir`（必須） | 指向 export root（含 `datasources.json`）或 combined export root | 搭配 `--use-export-org` 時要指向整個 multi-org 匯出根目錄 |
| `--org-id` | 匯入目標 org | org 變更時必用 |
| `--use-export-org` | 依 export 內 org 路由回 Grafana | 匯入 `--all-orgs` 產生的整體匯出根目錄 |
| `--only-org-id` | 限制 `--use-export-org` 只匯入指定 source org | 可重複指定多個 org |
| `--create-missing-orgs` | 路由匯入前自動建立缺少的目標 org | 僅限 `--use-export-org`；搭配 `--dry-run` 時只模擬執行 `would-create-org`，不真的建立 |
| `--require-matching-export-org` | 匯入前比對 orgId | 避免匯入到錯誤組織 |
| `--replace-existing` | 已存在時更新 | 標準覆寫匯入模式 |
| `--update-existing-only` | 只更新已有，不建立 | 保守同步模式 |
| `--dry-run` | 僅模擬執行 | 建議正式匯入前先執行 |
| `--table` | dry-run 時表格輸出 | 與 `--json` 互斥 |
| `--json` | dry-run 時 JSON 輸出 | 與 `--table` 互斥 |
| `--output-format text\|table\|json` | dry-run 單旗標 | 統一 dry-run 輸出模式 |
| `--output-columns` | dry-run table 欄位白名單 | 僅 `--dry-run --table` |
| `--no-header` | table no header | 僅 `--dry-run --table` |
| `--progress` | 逐筆進度 | 大量匯入穩定觀察 |
| `-v`, `--verbose` | 詳細逐筆日誌 | 覆蓋 `--progress` |

範例指令：
```bash
grafana-util datasource import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./datasources --replace-existing --dry-run --table
```

範例輸出：
```text
UID         NAME               TYPE         ACTION   DESTINATION
prom-main   prometheus-main    prometheus   update   existing
loki-prod   loki-prod          loki         create   missing
```

實跑註記：
- 真實環境 Docker 測試也會驗證路由式資料來源重現 (routed datasource replay)：`--use-export-org`、可重複的 `--only-org-id`、以及 `--create-missing-orgs`。在路由式模擬執行 (routed dry-run) JSON 中，會先看到組織層級的 `exists`、`missing-org`、或 `would-create-org`，再進入每筆資料來源操作。

### 5.4 `datasource diff`

**用途**：比較匯出快照與線上 datasource。

| 參數 | 用途 |
| --- | --- |
| `--diff-dir`（必須） | 指向 datasource 匯出根目錄 |

範例指令：
```bash
grafana-util datasource diff --url http://localhost:3000 --basic-user admin --basic-password admin --diff-dir ./datasources
```

範例輸出：
```text
Datasource diff found 1 differing item(s).

uid=loki-prod
- url=http://loki:3100
+ url=http://loki-prod:3100
```

### 5.5 `datasource add`（僅 Python CLI）

**用途**：直接在 Grafana 建立一筆線上 datasource，不經過本地 export bundle。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--name` | datasource 名稱 | 必填 |
| `--type` | datasource plugin type id | 必填 |
| `--uid` | 穩定 datasource uid | 建議提供 |
| `--access` | datasource access mode | 常見值：`proxy`、`direct` |
| `--datasource-url` | datasource 目標 URL | 常見 HTTP datasource 設定 |
| `--default` | 設成預設 datasource | 選填 |
| `--basic-auth` | 啟用上游 HTTP Basic auth | 常見於受保護的 Prometheus/Loki |
| `--basic-auth-user` | Basic auth 帳號 | 搭配 `--basic-auth-password` |
| `--basic-auth-password` | Basic auth 密碼 | 會放進 `secureJsonData` |
| `--user` | datasource user/login 欄位 | 常見於 Elasticsearch、SQL、InfluxDB |
| `--password` | datasource 密碼欄位 | 會放進 `secureJsonData` |
| `--with-credentials` | 設定 `withCredentials=true` | 支援的類型可帶 browser credentials |
| `--http-header NAME=VALUE` | 加一組自訂 HTTP header | 可重複多次 |
| `--tls-skip-verify` | 設定 `jsonData.tlsSkipVerify=true` | 需要放寬 TLS 驗證時使用 |
| `--server-name` | 設定 `jsonData.serverName` | TLS/SNI override |
| `--json-data` | 內嵌 `jsonData` JSON 物件 | 進階 plugin 專屬設定 |
| `--secure-json-data` | 內嵌 `secureJsonData` JSON 物件 | 進階含 secret 設定 |
| `--dry-run` | 僅模擬執行 | 建議先跑 |
| `--table` / `--json` | dry-run 輸出模式 | 人工或自動化 |

補充：
- 常見 type 包含 `prometheus`、`loki`、`elasticsearch`、`influxdb`、`graphite`、`postgres`、`mysql`、`mssql`、`tempo`、`cloudwatch`。
- 專用 auth/header 旗標會合併進 datasource payload；如果 `--json-data` 或 `--secure-json-data` 已經包含相同 key，命令會直接失敗，不會靜默覆蓋。

範例：Prometheus + basic auth
```bash
grafana-util datasource add \
  --url http://localhost:3000 \
  --token <TOKEN> \
  --uid prom-main \
  --name prometheus-main \
  --type prometheus \
  --access proxy \
  --datasource-url http://prometheus:9090 \
  --basic-auth \
  --basic-auth-user metrics-user \
  --basic-auth-password metrics-pass \
  --dry-run --table
```

範例：Loki + tenant header
```bash
grafana-util datasource add \
  --url http://localhost:3000 \
  --token <TOKEN> \
  --uid loki-main \
  --name loki-main \
  --type loki \
  --access proxy \
  --datasource-url http://loki:3100 \
  --http-header X-Scope-OrgID=tenant-a \
  --dry-run --json
```

範例：InfluxDB + 額外 plugin 設定
```bash
grafana-util datasource add \
  --url http://localhost:3000 \
  --token <TOKEN> \
  --uid influx-main \
  --name influx-main \
  --type influxdb \
  --access proxy \
  --datasource-url http://influxdb:8086 \
  --user influx-user \
  --password influx-pass \
  --json-data '{"version":"Flux","organization":"main-org","defaultBucket":"metrics"}' \
  --dry-run --table
```

6) Access (存取控制) 指令模組
-------------

這是本工具的核心功能，專為大規模環境的**權限治理與狀態同步**設計。

### 6.1 使用者管理 (User Operations)
- `access user list`: 支援 `org` 與 `global` 範圍的權限盤點。
- `access user export`: 建立使用者快照，包含其組織角色與團隊成員關係。
- `access user import`: **宣告式還原**使用者狀態。
- **`--with-teams`**: 匯出/匯入時包含 Team 成員關係同步（還原權限時必備）。

### 6.2 團隊管理 (Team Operations)
- `access team import`: 執行確定性（Deterministic）的成員同步。
- **組態漂移檢查**: 使用 `access team diff` 識別本地快照與線上環境的成員差異。
- **安全警告**: 若匯入操作會移除現有成員，必須加上 `--yes` 以避免非預期的權限丟失。

---

8) 常見維運情境 SOP (Best Practices)
------------------

### 8.1 跨環境 Dashboard 遷移 (Promote to Prod)
1. **備份與提交**: 在來源環境執行 `export` 並將產出的 JSON 提交至 Git 倉庫。
2. **差異模擬執行**: 在目標環境執行 `import --dry-run --table --import-dir <DIR>/raw`。
3. **安全更新**: 確認無誤後，執行 `import --replace-existing` 完成同步。

### 8.2 資產稽核與漂移盤點
1. **線上掃描**: 定期執行 `dashboard inspect-live --output-format governance-json` 識別孤立資源。
2. **組態比對**：利用 `datasource diff` 確保線上資料來源設定與標準庫一致。
3. **權限稽核**: 執行 `access user list --scope global --csv` 產出年度審計報表。

範例指令：
```bash
grafana-util access user list --url http://localhost:3000 --basic-user admin --basic-password admin --scope global --table
```

範例輸出：
```text
ID   LOGIN      EMAIL                NAME             ORG_ROLE   GRAFANA_ADMIN
1    admin      admin@example.com    Grafana Admin    Admin      true
7    svc-ci     ci@example.com       CI Service       Editor     false
9    alice      alice@example.com    Alice Chen       Viewer     false
```

### 6.2 `access user add`

**用途**：建立 user。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--login` | login（必填） | 建立使用者 |
| `--email` | email（必填） | 通訊 |
| `--name` | 顯示名稱（必填） | 人員識別 |
| `--password` | 初始密碼 | 三選一其一 |
| `--password-file` | 從檔案讀取初始密碼 | 較安全的非互動用法 |
| `--prompt-user-password` | 互動式輸入初始密碼 | 較安全的互動用法 |
| `--org-role` | 初始角色 | 建立使用者時一併指定 |
| `--grafana-admin` | `true/false` | 是否授與伺服器管理員權限 |
| `--json` | JSON 回應 | 便於自動化後續處理 |

範例指令：
```bash
grafana-util access user add --url http://localhost:3000 --basic-user admin --basic-password admin --login bob --email bob@example.com --name "Bob Lin" --password '<SECRET>' --org-role Editor --json
```

補充：
- `--password`、`--password-file`、`--prompt-user-password` 只能擇一。
- `--password-file` 會去掉最後一個換行，方便直接讀常見 secret 檔。

使用密碼檔範例：
```bash
grafana-util access user add --url http://localhost:3000 --basic-user admin --basic-password admin --login bob --email bob@example.com --name "Bob Lin" --password-file ./secrets/bob-password.txt --org-role Editor --json
```

範例輸出：
```json
{
  "id": 12,
  "login": "bob",
  "email": "bob@example.com",
  "name": "Bob Lin",
  "orgRole": "Editor",
  "grafanaAdmin": false
}
```

### 6.3 `access user modify`

**用途**：修改使用者。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--user-id` / `--login` / `--email` | 三擇一定位使用者 | 避免歧義 |
| `--set-login` | 更新 login | 變更登入名稱 |
| `--set-email` | 更新 email | 更新通知與識別資訊 |
| `--set-name` | 更新名稱 | 更新顯示名稱 |
| `--set-password` | 重設密碼 | 三選一其一 |
| `--set-password-file` | 從檔案讀取新密碼 | 較安全的非互動輪替 |
| `--prompt-set-password` | 互動式輸入新密碼 | 較安全的互動輪替 |
| `--set-org-role` | 更新角色 | 調整組織權限 |
| `--set-grafana-admin` | 更新管理員身分 | 調整全域管理權限 |
| `--json` | JSON 回應 | 便於自動化後續處理 |

範例指令：
```bash
grafana-util access user modify --url http://localhost:3000 --basic-user admin --basic-password admin --login alice --set-email alice@example.com --set-org-role Editor --json
```

補充：
- `--set-password`、`--set-password-file`、`--prompt-set-password` 最多只能用一個。

互動式改密碼範例：
```bash
grafana-util access user modify --url http://localhost:3000 --basic-user admin --basic-password admin --login alice --prompt-set-password --set-org-role Editor --json
```

範例輸出：
```json
{
  "id": 9,
  "login": "alice",
  "result": "updated",
  "changes": ["set-org-role", "set-email"]
}
```

### 6.4 `access user delete`

**用途**：刪除使用者。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--user-id` / `--login` / `--email` | 三擇一定位 | 任選一種方式指定目標使用者 |
| `--scope org|global`（預設 `global`） | 刪除範圍 |
| `--yes` | 跳過刪除確認（建議自動化必加） | 非互動執行時常用 |
| `--json` | JSON 回應 | 便於自動化後續處理 |

範例指令：
```bash
grafana-util access user delete --url http://localhost:3000 --basic-user admin --basic-password admin --login temp-user --scope global --yes --json
```

範例輸出：
```json
{
  "id": 14,
  "login": "temp-user",
  "scope": "global",
  "result": "deleted"
}
```

### 6.5 `access user export`

**用途**：匯出 users 與 role/team memberships 快照，供移轉重播使用。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--export-dir` | 輸出 `users.json` 與 `export-metadata.json` 的目錄 | 預設 `access-users` |
| `--overwrite` | 覆蓋既有輸出檔 | 避免手動清理 |
| `--dry-run` | 僅模擬執行輸出路徑 | 驗證目錄與權限 |
| `--scope` | `org` / `global` | 切換識別語意 |
| `--with-teams` | 匯出每位使用者的 team 成員關係 | 還原 membership 時必加 |

範例指令：
```bash
grafana-util access user export --url http://localhost:3000 --token <TOKEN> --export-dir ./access-users --scope org --with-teams
```

範例輸出：
```text
Exported users from http://localhost:3000 -> /tmp/access-users/users.json and /tmp/access-users/export-metadata.json
```

### 6.6 `access user import`

**用途**：從快照匯入 users。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--import-dir` | 包含 `users.json` 與 `export-metadata.json` 的目錄 | 必須沿用 export 目錄結構 |
| `--scope` | `org` / `global` | 控制比對與更新規則 |
| `--replace-existing` | 更新已存在帳號而非直接跳過 | 做重播同步時必須 |
| `--dry-run` | 僅模擬執行，不實際改 Grafana | 建議正式匯入或修改前先確認 |
| `--yes` | 跳過 destructive 移除確認 | 當要移除 team 成員會要求 |
| `--table`、`--json`、`--output-format table/json` | dry-run 輸出模式 | 僅 `--dry-run` 可用，且互斥 |

範例指令：
```bash
grafana-util access user import --url http://localhost:3000 --token <TOKEN> --import-dir ./access-users --replace-existing --dry-run --output-format table
```

範例輸出：
```text
INDEX  IDENTITY        ACTION        DETAIL
1      alice@example.com skip          existing and --replace-existing was not set.
2      bob@example.com   create        would create user
3      carol@example.com update-admin  would update grafanaAdmin -> true

Import summary: processed=3 created=1 updated=1 skipped=1 source=./access-users
```

### `access user diff`

**用途**：比較快照 `users.json` 與線上的 users。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--diff-dir` | 包含 `users.json` 與 `export-metadata.json` 的目錄 | 預設 `access-users` |
| `--scope` | `org` / `global` | 與匯出/匯入使用同一識別語意 |

範例指令：
```bash
grafana-util access user diff --url http://localhost:3000 --token <TOKEN> --diff-dir ./access-users --scope org
```

範例輸出：
```text
Diff checked 2 user(s).
alice@example.com  UPDATE  role 從 Viewer 改成 Editor
bob@example.com    DELETE  snapshot 中找不到該使用者
```

### `access team diff`

**用途**：比較快照 `teams.json` 與線上的 teams、team 成員。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--diff-dir` | 包含 `teams.json` 與 `export-metadata.json` 的目錄 | 預設 `access-teams` |

範例指令：
```bash
grafana-util access team diff --url http://localhost:3000 --token <TOKEN> --diff-dir ./access-teams
```

範例輸出：
```text
Diff checked 1 team(s).
Ops               UPDATE   add-member alice@example.com
SRE               DELETE   線上多出的 team，snapshot 中沒有
```

### 6.7 `access team list`

**用途**：列出 teams。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--query` | 模糊搜尋 team | 盤點大量 team 時使用 |
| `--name` | 精準 team name | 已知名稱時快速查詢 |
| `--with-members` | 顯示 members | 需要同步檢查成員時使用 |
| `--page` / `--per-page` | 分頁 | 大量資料時控制輸出 |
| `--table` / `--csv` / `--json` | 輸出 | 傳統輸出切換方式 |
| `--output-format table/csv/json` | 取代上述 | 建議優先使用的統一寫法 |

範例指令：
```bash
grafana-util access team list --url http://localhost:3000 --token <TOKEN> --with-members --table
```

範例輸出：
```text
ID   NAME        EMAIL              MEMBERS   ADMINS
3    sre-team    sre@example.com    5         2
7    app-team    app@example.com    8         1
```

### 6.8 `access team add`

**用途**：新增 team。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--name` | team 名稱 | 建立 team 時必填 |
| `--email` | team 聯絡 email | 選填聯絡資訊 |
| `--member`（可多） | 初始成員 | 可重複指定多位成員 |
| `--admin`（可多） | 初始 admin | 可重複指定多位管理者 |
| `--json` | JSON 回應 | 便於自動化後續處理 |

範例指令：
```bash
grafana-util access team add --url http://localhost:3000 --token <TOKEN> --name platform-team --email platform@example.com --member alice --member bob --admin alice --json
```

範例輸出：
```json
{
  "teamId": 15,
  "name": "platform-team",
  "membersAdded": 2,
  "adminsAdded": 1
}
```

### 6.9 `access team modify`

**用途**：調整 team 成員與 admin。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--team-id` / `--name` | 三擇一定位 | 任選一種方式指定目標 team |
| `--add-member` / `--remove-member` | 成員增刪 | 維護 team 成員關係 |
| `--add-admin` / `--remove-admin` | admin 身分調整 | 維護 team 管理權限 |
| `--json` | JSON 回應 | 便於自動化後續處理 |

範例指令：
```bash
grafana-util access team modify --url http://localhost:3000 --token <TOKEN> --name platform-team --add-member carol --remove-member bob --remove-admin alice --json
```

範例輸出：
```json
{
  "teamId": 15,
  "name": "platform-team",
  "membersAdded": 1,
  "membersRemoved": 1,
  "adminsRemoved": 1
}
```

### 6.10 `access team delete`

**用途**：刪除 team。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--team-id` / `--name` | 三擇一定位 | 任選一種方式指定目標 team |
| `--yes` | 確認強制 | 非互動刪除流程時使用 |
| `--json` | JSON 回應 | 便於自動化後續處理 |

範例指令：
```bash
grafana-util access team delete --url http://localhost:3000 --token <TOKEN> --name platform-team --yes --json
```

範例輸出：
```json
{
  "teamId": 15,
  "name": "platform-team",
  "result": "deleted"
}
```

### 6.11 `access team export`

**用途**：匯出 teams 與成員快照。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--export-dir` | 輸出 `teams.json` 與 `export-metadata.json` 的目錄 | 預設 `access-teams` |
| `--overwrite` | 覆蓋既有輸出檔 | 適合自動化重跑 |
| `--dry-run` | 僅模擬執行輸出路徑 | 驗證目錄與權限 |
| `--with-members` | 匯出 members 與 admins | 還原成員關係必備 |

範例指令：
```bash
grafana-util access team export --url http://localhost:3000 --token <TOKEN> --export-dir ./access-teams --with-members
```

範例輸出：
```text
Exported teams from http://localhost:3000 -> /tmp/access-teams/teams.json and /tmp/access-teams/export-metadata.json
```

### 6.12 `access team import`

**用途**：從快照匯入 teams 並同步 team 成員。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--import-dir` | 包含 `teams.json` 與 `export-metadata.json` 的目錄 | 必須沿用 export 目錄結構 |
| `--replace-existing` | 更新既有 team | 用於跨環境 replay |
| `--dry-run` | 僅模擬執行，不實際變更 | 建議先跑 |
| `--yes` | 跳過 destructive 移除確認 | 當預期移除 team 成員時必須 |
| `--table`、`--json`、`--output-format table/json` | dry-run 輸出模式 | 僅 `--dry-run` 可用，且互斥 |

範例指令：
```bash
grafana-util access team import --url http://localhost:3000 --token <TOKEN> --import-dir ./access-teams --replace-existing --dry-run --output-format table
```

範例輸出：
```text
INDEX  IDENTITY         ACTION       DETAIL
1      platform-team    skip         existing and --replace-existing was not set.
2      sre-team         create       would create team
3      edge-team        add-member   would add team member alice@example.com
4      edge-team        remove-member would remove team member bob@example.com

Import summary: processed=4 created=1 updated=1 skipped=1 source=./access-teams
```

### 6.13 `access service-account list`

**用途**：列出 service account。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--query` | 模糊搜尋名稱 | 盤點大量 service account 時使用 |
| `--page` / `--per-page` | 分頁 | 大量資料時控制輸出 |
| `--table` / `--csv` / `--json` | 輸出 | 傳統輸出切換方式 |
| `--output-format table/csv/json` | 取代三旗標 | 建議優先使用的統一寫法 |

範例指令：
```bash
grafana-util access service-account list --url http://localhost:3000 --token <TOKEN> --table
```

範例輸出：
```text
ID   NAME          ROLE     DISABLED
2    ci-bot        Editor   false
5    backup-bot    Viewer   true
```

### 6.14 `access service-account add`

**用途**：新增服務帳號。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--name` | 名稱 | 建立 service account 時必填 |
| `--role Viewer\|Editor\|Admin\|None`（預設 `Viewer`） | 權限角色 | 指定建立後的組織角色 |
| `--disabled` | `true/false` | Rust 版 `bool` 為文字化輸入 |
| `--json` | JSON 回應 | 便於自動化後續處理 |

範例指令：
```bash
grafana-util access service-account add --url http://localhost:3000 --token <TOKEN> --name deploy-bot --role Editor --json
```

範例輸出：
```json
{
  "id": 21,
  "name": "deploy-bot",
  "role": "Editor",
  "disabled": false
}
```

### 6.15 `access service-account export`

**用途**：匯出 service-account 快照，方便備份、比對與跨環境檢查。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--export-dir` | 輸出 `service-accounts.json` 與 `export-metadata.json` 的目錄 | 預設 `access-service-accounts` |
| `--overwrite` | 覆蓋既有快照檔案 | 定期備份重跑 |
| `--dry-run` | 僅模擬執行輸出路徑，不實際寫檔 | 先確認目錄 |

範例指令：
```bash
grafana-util access service-account export --url http://localhost:3000 --token <TOKEN> --export-dir ./access-service-accounts --overwrite
```

範例輸出：
```text
Exported 3 service-account(s) from http://localhost:3000 -> access-service-accounts/service-accounts.json and access-service-accounts/export-metadata.json
```

實跑註記：
- 這條 snapshot 流程已由 `make test-access-live` 在 Grafana `12.4.1` 上驗證，包含 export、diff、dry-run import、線上 replay、delete，以及 token lifecycle。

### 6.16 `access service-account import`

**用途**：把 service-account 快照回放到 Grafana。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--import-dir` | 包含 `service-accounts.json` 與 `export-metadata.json` 的目錄 | 需沿用 export 結構 |
| `--replace-existing` | 建立缺漏帳號，並更新既有帳號 | 回放時必備 |
| `--dry-run` | 只模擬執行 `create/update/skip` 決策，不實際寫入 | 建議先跑 |
| `--table` / `--json` / `--output-format text\|table\|json` | dry-run 輸出模式 | 人工審查或機器判讀 |

範例指令：
```bash
grafana-util access service-account import --url http://localhost:3000 --token <TOKEN> --import-dir ./access-service-accounts --replace-existing --dry-run --output-format table
```

範例輸出：
```text
INDEX  IDENTITY     ACTION  DETAIL
1      deploy-bot   update  would update fields=role,disabled
2      report-bot   create  would create service account

Import summary: processed=2 created=1 updated=1 skipped=0 source=./access-service-accounts
```

實跑註記：
- live smoke 測試會先改寫匯出的 snapshot，確認 dry-run update preview，再把同一份檔案實際 replay 回 Grafana，驗證線上更新路徑。

### 6.17 `access service-account diff`

**用途**：比較 service-account 快照與線上 Grafana 狀態。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--diff-dir` | 包含 `service-accounts.json` 與 `export-metadata.json` 的目錄 | 預設 `access-service-accounts` |

範例指令：
```bash
grafana-util access service-account diff --url http://localhost:3000 --token <TOKEN> --diff-dir ./access-service-accounts
```

範例輸出：
```text
Diff different service-account deploy-bot fields=role
Diff missing-live service-account report-bot
Diff extra-live service-account old-bot
Diff checked 3 service-account(s); 3 difference(s) found.
```

### 6.18 `access service-account delete`

**用途**：刪除服務帳號。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--service-account-id` / `--name` | 三擇一定位 | 任選一種方式指定目標 service account |
| `--yes` | 需要跳過互動確認 | 非互動刪除流程時使用 |
| `--json` | JSON 回應 | 便於自動化後續處理 |

範例指令：
```bash
grafana-util access service-account delete --url http://localhost:3000 --token <TOKEN> --name deploy-bot --yes --json
```

範例輸出：
```json
{
  "id": 21,
  "name": "deploy-bot",
  "result": "deleted"
}
```

### 6.19 `access service-account token add`

**用途**：建立服務帳號 token。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--service-account-id` / `--name` | 定位 SA | 任選一種方式指定目標 service account |
| `--token-name` | token 名稱 | 建立新 token 時必填 |
| `--seconds-to-live` | TTL（秒） | 控制 token 有效期間 |
| `--json` | JSON 回應 | 便於自動化後續處理 |

範例指令：
```bash
grafana-util access service-account token add --url http://localhost:3000 --token <TOKEN> --name deploy-bot --token-name ci-token --seconds-to-live 86400 --json
```

範例輸出：
```json
{
  "serviceAccountId": 21,
  "tokenId": 34,
  "tokenName": "ci-token",
  "secondsToLive": 86400,
  "key": "glsa_xxxxxxxxx"
}
```

### 6.20 `access service-account token delete`

**用途**：刪除服務帳號 token。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--service-account-id` / `--name` | 定位 SA | 任選一種方式指定目標 service account |
| `--token-id` / `--token-name` | 定位 token（需二擇一） | 任選一種方式指定目標 token |
| `--yes` | 跳過確認 | 非互動刪除流程時使用 |
| `--json` | JSON 回應 | 便於自動化後續處理 |

範例指令：
```bash
grafana-util access service-account token delete --url http://localhost:3000 --token <TOKEN> --name deploy-bot --token-name ci-token --yes --json
```

範例輸出：
```json
{
  "serviceAccountId": 21,
  "tokenName": "ci-token",
  "result": "deleted"
}
```

7) 共通輸出與互斥規則摘要
-------------------------

| 規則 | 說明 |
| --- | --- |
| 輸出格式互斥 | 多數命令以 `Mutually exclusive` 控制 `--table`、`--csv`、`--json`、`--output-format`（不應同時出現）。 |
| legacy 命令 | `dashboard`/`alert` 大多有 legacy 入口，建議新腳本改用正式子命令 |
| dry-run 優先 | 含 `--dry-run` 的流程先跑模擬執行再實際變更 |
| 認證策略 | `org-id`、`all-orgs` 等多數 dashboard/datasource 命令偏向 basic auth；token 更常用於 alert/access 快速操作 |
| 團隊別名 | `access group` 為 `access team` alias |

8) 常見情境快速對照
------------------

### 8.1 跨環境 dashboard 遷移

1. `grafana-util dashboard export --all-orgs --overwrite --export-dir ./dashboards`
2. `grafana-util dashboard import --dry-run --replace-existing --table --import-dir ./dashboards/raw`
3. 確認結果後再跑同一行去掉 `--dry-run`

### 8.2 只做稽核，不改動

1. 用 `dashboard diff` 或 `dashboard inspect-export`/`inspect-live`
2. list 類加 `--json` 並做差異比對
3. `alert`/`datasource import` 一律加 `--dry-run`

### 8.3 使用者權限整理

1. `access user list --scope global --table` 建盤點
2. `access user modify` 調整 role/admin
3. `access team modify` 調整成員與 admin
4. `access service-account` 及 `token` 命令做機器人授權
5. 匯入前先跑 `access user diff` 與 `access team diff` 做快照比對

### 8.4 參數變體選擇原則

1. 需要穩定機器人輸入：優先 `--json`
2. 需要人工讀取：`--table`，並可搭 `--no-header`
3. 需要 import/diff 前檢查：加 `--dry-run`
4. 跨 org 風險高：加 `--org-id`、`--require-matching-export-org`

9) 每命令 SOP（最短可跑版本）
------------------------------

每行可直接貼到腳本，替換參數值即可。

```bash
# dashboard
grafana-util dashboard export --url <URL> --basic-user <USER> --basic-password <PASS> --export-dir <DIR> [--overwrite] [--all-orgs]
grafana-util dashboard export --url <URL> --token <TOKEN> --org-id <ORG_ID> --export-dir <DIR> [--overwrite]
grafana-util dashboard list --url <URL> --basic-user <USER> --basic-password <PASS> [--org-id <ORG_ID>|--all-orgs] [--table|--csv|--json|--output-format table|csv|json] [--with-sources]
grafana-util dashboard list-data-sources --url <URL> --basic-user <USER> --basic-password <PASS> [--table|--csv|--json|--output-format table|csv|json]
grafana-util dashboard import --url <URL> --basic-user <USER> --basic-password <PASS> --import-dir <DIR>/raw --replace-existing [--dry-run] [--table|--json|--output-format text|table|json] [--output-columns uid,destination,action,folder_path,destination_folder_path,file]
grafana-util dashboard diff --url <URL> --basic-user <USER> --basic-password <PASS> --import-dir <DIR>/raw [--import-folder-uid <UID>] [--context-lines 3]
grafana-util dashboard inspect-export --import-dir <DIR>/raw --output-format report-tree
grafana-util dashboard inspect-live --url <URL> --basic-user <USER> --basic-password <PASS> --output-format report-json

# alert
grafana-util alert export --url <URL> --token <TOKEN> --output-dir <DIR> [--flat] [--overwrite]
grafana-util alert import --url <URL> --basic-user <USER> --basic-password <PASS> --import-dir <DIR>/raw --replace-existing [--dry-run] [--dashboard-uid-map <FILE>] [--panel-id-map <FILE>]
grafana-util alert diff --url <URL> --basic-user <USER> --basic-password <PASS> --diff-dir <DIR>/raw [--dashboard-uid-map <FILE>] [--panel-id-map <FILE>]
grafana-util alert list-rules --url <URL> --token <TOKEN> [--table|--csv|--json]

# datasource
grafana-util datasource list --url <URL> --token <TOKEN> [--table|--csv|--json]
grafana-util datasource add --url <URL> --token <TOKEN> --name <NAME> --type <TYPE> [--uid <UID>] [--access proxy|direct] [--datasource-url <URL>] [--basic-auth] [--basic-auth-user <USER>] [--basic-auth-password <PASS>] [--user <USER>] [--password <PASS>] [--with-credentials] [--http-header NAME=VALUE] [--tls-skip-verify] [--server-name <NAME>] [--json-data <JSON>] [--secure-json-data <JSON>] [--dry-run] [--table|--json|--output-format text|table|json]
grafana-util datasource export --url <URL> --basic-user <USER> --basic-password <PASS> --export-dir <DIR> [--overwrite] [--dry-run] [--org-id <ORG_ID>|--all-orgs]
grafana-util datasource import --url <URL> --basic-user <USER> --basic-password <PASS> --import-dir <DIR> --replace-existing [--org-id <ORG_ID>] [--use-export-org [--only-org-id <ORG_ID>]... [--create-missing-orgs]] [--dry-run] [--output-format table|text|json] [--output-columns uid,name,type,destination,action,org_id,file]
grafana-util datasource diff --url <URL> --basic-user <USER> --basic-password <PASS> --diff-dir <DIR>

# access
grafana-util access user list --url <URL> --token <TOKEN> --scope org [--table|--csv|--json]
grafana-util access user add --url <URL> --basic-user <USER> --basic-password <PASS> --login <LOGIN> --email <EMAIL> --name <NAME> --password <PWD> [--org-role Editor] [--grafana-admin true|false]
grafana-util access user modify --url <URL> --basic-user <USER> --basic-password <PASS> --login <LOGIN> --set-email <EMAIL> [--set-name <NAME>] [--set-org-role Viewer|Editor|Admin|None] [--set-grafana-admin true|false]
grafana-util access user delete --url <URL> --basic-user <USER> --basic-password <PASS> --login <LOGIN> --scope global --yes
grafana-util access user export --url <URL> --token <TOKEN> --export-dir ./access-users [--scope org|global] [--with-teams]
grafana-util access user import --url <URL> --token <TOKEN> --import-dir ./access-users --replace-existing [--dry-run] [--table|--json|--output-format text|table|json] [--yes]
grafana-util access user diff --url <URL> --token <TOKEN> --diff-dir ./access-users [--scope org|global]
grafana-util access team list --url <URL> --token <TOKEN> [--query <QUERY>|--name <NAME>] [--with-members] [--table|--csv|--json]
grafana-util access team add --url <URL> --token <TOKEN> --name <NAME> [--email <EMAIL>] [--member <LOGIN_OR_EMAIL>] [--admin <LOGIN_OR_EMAIL>]
grafana-util access team modify --url <URL> --token <TOKEN> --name <NAME> [--add-member <LOGIN_OR_EMAIL>] [--remove-member <LOGIN_OR_EMAIL>] [--add-admin <LOGIN_OR_EMAIL>] [--remove-admin <LOGIN_OR_EMAIL>]
grafana-util access team delete --url <URL> --token <TOKEN> --name <NAME> --yes
grafana-util access team export --url <URL> --token <TOKEN> --export-dir ./access-teams [--with-members]
grafana-util access team diff --url <URL> --token <TOKEN> --diff-dir ./access-teams
grafana-util access team import --url <URL> --token <TOKEN> --import-dir ./access-teams --replace-existing [--dry-run] [--table|--json|--output-format text|table|json] [--yes]
grafana-util access service-account export --url <URL> --token <TOKEN> --export-dir ./access-service-accounts [--overwrite]
grafana-util access service-account import --url <URL> --token <TOKEN> --import-dir ./access-service-accounts --replace-existing [--dry-run] [--table|--json|--output-format text|table|json]
grafana-util access service-account diff --url <URL> --token <TOKEN> --diff-dir ./access-service-accounts
grafana-util access service-account list --url <URL> --token <TOKEN> [--query <QUERY>] [--table|--csv|--json]
grafana-util access service-account add --url <URL> --token <TOKEN> --name <NAME> [--role Viewer|Editor|Admin|None] [--disabled true|false]
grafana-util access service-account delete --url <URL> --token <TOKEN> --name <NAME> --yes
grafana-util access service-account token add --url <URL> --token <TOKEN> --name <SA_NAME> --token-name <TOKEN_NAME> [--seconds-to-live <SECONDS>]
grafana-util access service-account token delete --url <URL> --token <TOKEN> --name <SA_NAME> --token-name <TOKEN_NAME> --yes
```

10) 參數互斥與差異矩陣（Rust）
--------------------------------

`OUTPUT` 類（`--output-format` 與 `--table/--csv/--json` 互斥關係）：

| 命令 | `--output-format` 允許值 | `--table/--csv/--json` 同時可用 | 備註 |
| --- | --- | --- | --- |
| dashboard list | table/csv/json | 不可 | output-format 取代三旗標 |
| dashboard list-data-sources | table/csv/json | 不可 | 同上 |
| dashboard import | text/table/json | 不可（僅 text/table/json） | text 為 dry-run 匯總資訊 |
| alert list-* | table/csv/json | 不可 | list 命令共用 |
| datasource list | table/csv/json | 不可 | 同上 |
| datasource add | text/table/json | 不可（僅 text/table/json） | dry-run 可用，僅 Python CLI |
| datasource import | text/table/json | 不可（僅 text/table/json） | text 為 dry-run 摘要，也支援 routed org-level preview |
| access user list | table/csv/json | 不可 | 同上 |
| access team list | table/csv/json | 不可 | 同上 |
| access user import | text/table/json | 不可（僅 text/table/json） | text 為 dry-run 摘要 |
| access team import | text/table/json | 不可（僅 text/table/json） | text 為 dry-run 摘要 |
| access user diff | text | 否 | 僅摘要 |
| access team diff | text | 否 | 僅摘要 |
| access service-account import | text/table/json | 不可（僅 text/table/json） | text 為 dry-run 摘要 |
| access service-account diff | text | 否 | 僅摘要 |
| access service-account list | table/csv/json | 不可 | 同上 |

`DRY-RUN` 類（模擬執行）：

| 命令 | `--dry-run` 影響 |
| --- | --- |
| dashboard import | 僅模擬執行 `create/update/skip` |
| datasource import | 僅模擬執行 `create/update/skip` |
| alert import | 僅模擬執行 `create/update` |
| access user import | 僅模擬執行 `create/update/skip`，以及 team 變更 preview |
| access team import | 僅模擬執行 `create/update/skip`，以及 membership 變更 preview |

`ORG` 控制：

| 命令 | `--org-id` | `--all-orgs` |
| --- | --- | --- |
| dashboard list | 可用（不可用 token，需 Grafana 帳號密碼） | 可用（不可用 token，需 Grafana 帳號密碼） |
| dashboard export | 可用（不可用 token，需 Grafana 帳號密碼） | 可用（不可用 token，需 Grafana 帳號密碼） |
| dashboard import | 可用（不可用 token，需 Grafana 帳號密碼） | 不可 |
| datasource export | 可用（不可用 token，需 Grafana 帳號密碼） | 可用（不可用 token，需 Grafana 帳號密碼） |
| datasource import | 可用（不可用 token，需 Grafana 帳號密碼） | 不可 |
| alert 全部 | 不支援 `org-id`/`all-orgs` | 不支援 |
| access 全部 | 用 `--scope` 替代 | 不支援 |
