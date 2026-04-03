Grafana Utilities 維運指南 (繁體中文)
===================================

本指南以目前維護中的 Rust `grafana-util` 統一 CLI 介面為主，使用 `grafana-util ...` 作為範例指令：

- **全域參數優先**：通用於所有指令的設定。
- **功能模組獨立**：依資源類型（Dashboard、Alert、Datasource、Access）劃分。
- **情境導向設計**：每個參數 (Flag) 皆標註了用途、差異與適用情境。
- **安全第一**：內建互斥規則與標準作業程序 (SOP) 建議。

安裝
----

使用 repo 內建安裝腳本，一行完成安裝：

```bash
curl -fsSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-utils/main/scripts/install.sh | sh
```

需要指定安裝位置或固定版本時：

```bash
curl -fsSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-utils/main/scripts/install.sh | BIN_DIR=/usr/local/bin VERSION=v0.6.2 sh
```

如果您已經有本地 checkout，也可以直接在 repo 根目錄執行：

```bash
sh ./scripts/install.sh
```

目錄索引
--------

- [1) 全域前置說明](#tw-before-you-start)
- [2) 全域通用參數](#tw-global-options)
- [3) dashboard 指令模組](#tw-dashboard-commands)
- [4) alert 指令模組](#tw-alert-commands)
- [5) datasource 指令模組](#tw-datasource-commands)
- [6) Access 指令模組](#tw-access-commands)
- [7) 共通輸出規則、`change`、`overview`、`status`](#tw-shared-output-rules)
- [8) 常見情境快速對照](#tw-common-scenarios)
- [9) 每命令 SOP（最短可跑版本）](#tw-minimal-sop)
- [10) 參數互斥與差異矩陣（Rust）](#tw-matrix)

快速跳轉：

- [dashboard](#tw-dashboard-commands): `browse/export/list/import/delete/diff/inspect-export/inspect-live/screenshot`
- [alert](#tw-alert-commands): `plan/apply/delete/init/new-rule/new-contact-point/new-template/export/import/diff/list-rules/list-contact-points/list-mute-timings/list-templates`
- [datasource](#tw-datasource-commands): `browse/list/export/import/diff/add`
- [access](#tw-access-commands): `org/user/team/service-account`
- [change](#tw-shared-output-rules): `summary/bundle/bundle-preflight/plan/review/apply/assess-alerts`
- [overview](#tw-shared-output-rules)
- [status](#tw-shared-output-rules)

<a id="tw-before-you-start"></a>
1) 全域前置說明
------------

開始之前，您可以透過以下指令確認各模組的說明資訊：

```bash
grafana-util --version
grafana-util version
grafana-util -h
grafana-util dashboard -h
grafana-util alert -h
grafana-util datasource -h
grafana-util access -h
grafana-util change -h
grafana-util overview -h
grafana-util status -h
```

安裝後可直接使用：

```text
grafana-util <domain> <command> [options]
```

### 入口點說明：
- **`grafana-util`**：統一入口，支援 `dashboard/alert/datasource/access/change/overview/status`。
- `grafana-util --version` 和 `grafana-util version` 都可以顯示目前 CLI 版本。
- 統一 CLI 請使用命名空間形式：`grafana-util <domain> <command>`。
- `dashboard list-data-sources` 仍可使用，但新的資料來源盤點流程應優先改用 `datasource list`。
- `overview` 是人類優先的專案入口，`status` 是正式 status contract，`change` 則承接 staged change workflow。

### 1.1 這份手冊怎麼用

- 如果您是從 GitHub 首頁進來，先看 `README.md` 或 `README.zh-TW.md` 了解專案定位、支援範圍與快速範例。
- 如果您要實際操作，這份手冊才是完整入口，重點在命令行為、參數差異、認證規則、實際輸出樣式與 dry-run 判讀。
- 第一次接觸這個工具時，建議閱讀順序是：命令分區 -> 支援層級 -> 輸出面相 -> 對應命令章節。
- 如果您已經有固定流程，可以直接跳到各模組章節，並把第 9 節與第 10 節當成最短重複操作參考。

### 1.2 輸出與操作面相

同一類 Grafana 工作流，常常同時提供人工巡檢與自動化兩種用法，所以這裡先把操作面整理清楚。

| 面相 | 最適合的用途 | 代表命令 | 補充說明 |
| --- | --- | --- | --- |
| 互動式 TUI | 導覽、審查、在終端機內逐步操作 | `dashboard browse`、`dashboard inspect-export --interactive`、`dashboard inspect-live --interactive`、`datasource browse`、`overview --output interactive`、`status ... --output interactive` | 只有部分工作流支援；`interactive` 需要 TUI-capable build |
| 純文字 | 預設維運摘要、dry-run 預覽、人工閱讀 | `change`、`overview`、`status`、多數 dry-run 摘要 | 最適合終端機人工巡檢 |
| JSON | CI、自動化、機器可讀交接資料 | import dry-run、change 文件、staged/live status contract | 有其他工具要接結果時優先選這個 |
| Table / CSV / report | 清單盤點、差異審查、dashboard 分析報表 | list 系列命令、`dashboard inspect-*`、review tables | 最適合審計、治理與報表整理 |

### 1.3 支援層級總覽

如果您想先判斷每個模組到底成熟到什麼程度，先看這張表，再進入各命令細節。

| 模組 | 支援深度 | 主要工作流 | 主要輸出面 | 備註 |
| --- | --- | --- | --- | --- |
| `dashboard` | 最深、最完整 | browse、list、export、import、diff、delete、inspect、依賴分析、permission export、screenshot/PDF | text、table/csv/json、report 模式、互動式 TUI | 功能最完整，也是分析與遷移能力最重的模組 |
| `datasource` | 深且成熟 | browse、list、export、import、diff、add、modify、delete、跨 org replay | text、table/csv/json、互動式 browse | 同時涵蓋 live mutation 與檔案回放 |
| `alert` | 成熟的管理與遷移面 | list、export、import、diff、dry-run，涵蓋 rule bundle 與 alerting 資源 | table/csv/json | 同時涵蓋 operator-first 管理流程與 migration / replay |
| `access` | 成熟的盤點與回放面 | org/user/team/service-account 的 list、add、modify、delete、export、import、diff | table/csv/json | 適合 access state inventory、重建與審查 |
| `change` | 進階 staged workflow | summary、bundle、preflight、plan、review、apply intent、audit、promotion-preflight | text/json | 重點是 review-first 的變更流程，不是直接盲目套用 |
| `overview` | 人類優先的專案入口 | staged/live 專案快照、跨模組摘要、handoff 視圖 | text/json/interactive | 當您需要先看整體專案畫面時，先從這裡進來 |
| `status` | 正式 status contract | staged/live readiness、跨模組摘要、機器可讀交接 | text/json/interactive | 當您需要穩定的跨模組 readiness contract 時使用 |

<a id="tw-global-options"></a>
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

### 2.1 如何閱讀範例輸出

- `範例指令` 代表實際可用的呼叫方式。
- `範例輸出` 代表預期格式，不保證您的 UID、名稱、筆數、folder 一定完全相同。
- 若段落帶有 `實跑註記`，代表命令形態與輸出片段已用本地 Docker Grafana `12.4.1` 服務驗證過。
- 這次修訂另外用 `scripts/seed-grafana-sample-data.sh` 灌入多 org、巢狀 folder，並補了 alerting resource 與 service account/token，讓 live 範例不只覆蓋最小 happy path。
- 表格輸出適合人工操作。
- JSON 輸出適合腳本、自動化與 CI。
- 常見 `ACTION` 值：
  - `create`：目標尚不存在。
  - `update`：目標已存在，dry-run 或實際執行會修改它。
  - `no-change`：匯出內容與 live 狀態已一致。
  - `would-*`：純 dry-run 預測，不會真的改動。
- 常見 dry-run `DESTINATION` / 狀態提示：
  - `missing`：目前還找不到 live 目標。
  - `exists` / `existing`：live 目標已存在。
  - `exists-uid`：依 UID 找到對應的 live 目標。
  - `exists-name`：依名稱找到對應的 live 目標。
  - `missing-org`：路由後的目標 org 不存在。
  - `would-create-org`：在 `--dry-run --create-missing-orgs` 下，代表真正執行時會先建立目標 org。

### 命令分區（快速導覽）

如果您知道要處理的事情，但還不確定要進哪個命令入口，先看這張路由表。

| 目標 | 先從哪個入口開始 | 常用命令 |
| --- | --- | --- |
| Dashboard 盤點與分析 | `dashboard` | `browse`、`list`、`export`、`import`、`diff`、`delete`、`inspect-export`、`inspect-live`、`inspect-vars`、`screenshot` |
| Alerting 管理、盤點與遷移 | `alert` | `plan`、`apply`、`delete`、`init`、`new-rule`、`new-contact-point`、`new-template`、`list-rules`、`list-contact-points`、`list-mute-timings`、`list-templates`、`export`、`import`、`diff` |
| Datasource 盤點與回放 | `datasource` | `browse`、`list`、`export`、`import`、`diff`、`add`、`modify`、`delete` |
| Org 類 access 管理 | `access org` | `list`、`add`、`modify`、`delete`、`export`、`import` |
| User 類 access 管理 | `access user` | `list`、`add`、`modify`、`delete`、`export`、`import`、`diff` |
| Team 類 access 管理 | `access team` | `list`、`add`、`modify`、`delete`、`export`、`import`、`diff` |
| Service account 類 access 管理 | `access service-account` | `list`、`add`、`delete`、`export`、`import`、`diff`、`token add`、`token delete` |
| Staged change 與 promotion 工作流 | `change` | `summary`、`bundle`、`bundle-preflight`、`preflight`、`assess-alerts`、`plan`、`review`、`apply`、`audit`、`promotion-preflight` |
| 專案層 staged/live 狀態檢視 | `overview`、`status` | `overview`、`overview live`、`status staged`、`status live` |

### 指令功能總覽

本表可協助您快速確認各類 Grafana 資源的支援程度，以便選擇合適的指令執行資產盤點或狀態同步。

| 資源類型 | List（列表） | Export（匯出） | Import（匯入） | Diff（差異比對） | Inspect（分析） | Add（新增） | Modify（修改） | Delete（刪除） | 備註 |
| --- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | --- |
| **Dashboard** | Yes | Yes | Yes | Yes | Yes | No | No | No | 適合資產盤點、備份與環境遷移 |
| **Datasource** | Yes | Yes | Yes | Yes | No | No | No | No | 支援組態漂移檢查與環境同步 |
| **Alerting** | Yes | Yes | Yes | Yes | No | No | No | No | 管理 lane：`plan/apply/delete/init/new-*`；遷移 lane：`export/import/diff` |
| **Organization** | Yes | Yes | Yes | No | No | Yes | Yes | Yes | 支援 org 盤點與成員關係重建 |
| **User** | Yes | Yes | Yes | Yes | No | Yes | Yes | Yes | 支援全域或組織範圍的使用者盤點 |
| **Team** | Yes | Yes | Yes | Yes | No | Yes | Yes | Yes | 包含成員關係（Membership）同步 |
| **Service Account** | Yes | Yes | Yes | Yes | No | Yes | Yes | Yes | 生命週期管理與 Token 簽發 |
| **SA Token** | Yes | No | No | No | No | Yes | No | Yes | Token 建立與撤銷 |

專案層入口：

| 入口 | 輸入 | 會讀 live Grafana 嗎 | 輸出模式 | 主要用途 |
| --- | --- | --- | --- | --- |
| `change` | desired JSON、bundle、lock、availability/mapping 檔 | 視子命令而定 | text/json | staged review、preflight、plan、review、apply intent |
| `overview` | staged exports 與選用的 change/promotion 輸入 | 只有 `overview live` 會 | text/json/interactive | 專案層級 staged/live 快照 |
| `status` | staged exports 或 live Grafana | 會 | text/json/interactive | 正式的專案層級 staged/live readiness surface |

認證互斥規則（由 CLI parser 強制執行）：

1. `--token` 不可與 `--basic-user` 同時使用。
2. `--token` 不可與 `--prompt-token` 同時使用。
3. `--basic-password` 不可與 `--prompt-password` 同時使用。
4. `--prompt-password` 必須同時提供 `--basic-user`。

<a id="tw-dashboard-commands"></a>
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
grafana-util dashboard export --url http://localhost:3000 --basic-user admin --basic-password admin --export-dir ./dashboards --overwrite --progress
```

範例輸出：
```text
Exporting dashboard 1/7: mixed-query-smoke
Exporting dashboard 2/7: smoke-prom-only
Exporting dashboard 3/7: query-smoke
Exporting dashboard 4/7: smoke-main
Exporting dashboard 5/7: subfolder-chain-smoke
Exporting dashboard 6/7: subfolder-main
Exporting dashboard 7/7: two-prom-query-smoke
```

補充：
- `--progress` 只顯示簡潔進度；若要看到每個輸出檔路徑，改用 `--verbose`。
- 使用 `--all-orgs` 時，匯出根目錄的 `export-metadata.json` 會包含 `orgCount` 與每個已匯出 org 的 `orgs[]` 摘要。
- 若這份 export 後面還要直接餵 `dashboard import` / `dashboard diff` 做 dry-run，這次實跑最穩定的形態是加上 `--flat`。

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
grafana-util dashboard import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./dashboards-flat/raw --replace-existing --dry-run --table
```

範例輸出：
```text
Import mode: create-or-update
UID                    DESTINATION  ACTION  FOLDER_PATH                    FILE
---------------------  -----------  ------  -----------------------------  ------------------------------------------------------------
mixed-query-smoke      exists       update  General                        ./dashboards-flat/raw/Mixed_Query_Dashboard__mixed-query-smoke.json
smoke-main             exists       update  General                        ./dashboards-flat/raw/Smoke_Dashboard__smoke-main.json
subfolder-chain-smoke  exists       update  Platform / Team / Apps / Prod  ./dashboards-flat/raw/Subfolder_Chain_Dashboard__subfolder-chain-smoke.json

Dry-run checked 7 dashboard(s) from ./dashboards-flat/raw
```

實跑註記：
- 上面的 dry-run 表格已在本地 Grafana `12.4.1` 上，用 `dashboard export --flat` 先匯出後再回放驗證。

怎麼看：
- `ACTION=update` 代表 live dashboard 已存在，實際執行會更新它。
- `ACTION=create` 代表 live 尚未存在，實際執行會新增它。
- `DESTINATION` 描述的是 live 目標狀態，不是本地檔案目錄。
- `DESTINATION=missing` 代表 dry-run 尚未找到對應的 live dashboard。
- 多 org 路由 dry-run 還可能先出現 `missing-org` 或 `would-create-org`。

### 3.4a `dashboard delete`

**用途**：依 dashboard UID 或 folder path 子樹刪除線上 dashboards。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--uid` | 依穩定 UID 刪除單一 dashboard | 自動化最安全 |
| `--path` | 刪除某個 folder path 子樹下的 dashboards | 依階層整理清理 |
| `--delete-folders` | 連同 matching folders 一起刪除 | 僅能搭配 `--path`；先刪 dashboard 再刪 folder |
| `--interactive` | 互動式選擇、預覽、確認 | 手動維護最方便 |
| `--yes` | 確認 live delete | 非互動式 live delete 必填 |
| `--dry-run` | 只預覽不刪除 | 建議先跑 |
| `--table` | dry-run 表格輸出 | 人工審查 |
| `--json` | dry-run JSON 輸出 | 自動化串接 |
| `--output-format text/table/json` | dry-run 單一輸出旗標 | 統一 selector |
| `--org-id` | 指定單一 org | 比 cross-org 更安全 |

範例指令：
```bash
grafana-util dashboard delete --url http://localhost:3000 --basic-user admin --basic-password admin --path "Platform / Infra" --dry-run --table
```

目前說明：
- `--path` 會比對 Grafana 解析後的 folder path，並遞迴刪除該子樹下的 dashboards。
- 不加 `--delete-folders` 時，只會刪 dashboard，folder 資源會保留。

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
| `--import-dir`（必須） | 指向單一 org 的 raw/ 目錄，或 `--all-orgs` 產生的整體匯出根目錄 | 不連線線上 API |
| `--json` | JSON 輸出 | 與 `--table`/`--report*` 互斥 |
| `--table` | 表格輸出 | 與 `--json` 互斥 |
| `--report` | report mode 快捷；可為空值 | 空值預設是 flat table；也可指定 `csv`、`json`、`tree`、`tree-table`、`dependency`、`dependency-json`、`governance`、`governance-json` |
| `--output-format text\|table\|json\|report-table\|report-csv\|report-json\|report-tree\|report-tree-table\|dependency\|dependency-json\|report-dependency\|report-dependency-json\|governance\|governance-json` | 單一輸出旗標 | 與 `--json`、`--table`、`--report` 互斥 |
| `--report-columns` | report 輸出欄位白名單 | 只適用 `report-table`、`report-csv`、`report-tree-table` 與等價 `--report` 模式 |
| `--report-filter-datasource` | report/filter 的 datasource 精準匹配 | 會精準比對 datasource label、uid、type、normalized family |
| `--report-filter-panel-id` | report/filter 的 panel id 精準匹配 | 只適用 report 類輸出，適合查單面板差異 |
| `--help-full` | 顯示完整 report 範例與欄位說明 | 首次導入常用 |
| `--no-header` | 表格/可表格化 report 不列表頭 | 便於比對輸出 |

範例指令：
```bash
grafana-util dashboard inspect-export --import-dir ./dashboards/raw --output-format report-table
```

多 org 整體匯出根目錄：
```bash
grafana-util dashboard inspect-export --import-dir ./dashboards --output-format report-tree-table
```

檢查 datasource 自己的 org、database、bucket、index pattern 欄位：
```bash
grafana-util dashboard inspect-export --import-dir ./dashboards/raw --report csv \
  --report-columns datasource_name,datasource_org,datasource_org_id,datasource_database,datasource_bucket,datasource_index_pattern,query
```

檢查 metrics、functions、bucket 抽取：
```bash
grafana-util dashboard inspect-export --import-dir ./dashboards/raw --report csv \
  --report-columns panel_id,ref_id,datasource_name,metrics,functions,buckets,query
```

檢查 folder 身分和來源檔案路徑：
```bash
grafana-util dashboard inspect-export --import-dir ./dashboards/raw --report csv \
  --report-columns dashboard_uid,folder_path,folder_uid,parent_folder_uid,file
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
| `--json` / `--table` / `--report` / `--output-format*` | 與 `inspect-export` 完全同義 | 包含 `dependency` / `dependency-json` 與 governance 模式 |
| `--help-full` | 進一步說明 report 參數 | 導入/診斷複雜情境 |
| `--no-header` | 不列表頭 | 主要供腳本處理 |

範例指令：
```bash
grafana-util dashboard inspect-live --url http://localhost:3000 --basic-user admin --basic-password admin --output-format governance-json
```

範例輸出：
```json
{
  "kind": "grafana-utils-dashboard-governance",
  "summary": {
    "dashboardCount": 1,
    "mixedDashboardCount": 0
  },
  "dashboardDependencies": [
    {
      "dashboardUid": "cpu-main",
      "dashboardTitle": "CPU Overview",
      "datasources": ["prom-main"],
      "datasourceFamilies": ["prometheus"],
      "pluginIds": ["timeseries"]
    }
  ]
}
```

補充說明：
- `--report-columns` 只適用 flat 或 grouped table 類 report；summary JSON、dependency contract、governance 輸出都會拒絕。
- `--report-filter-datasource` 會精準匹配 datasource label、uid、type、normalized family。
- `--report-filter-panel-id` 只適用 report 類輸出。
- `dependency` / `dependency-json` 會輸出機器可讀的契約文件：含 `queryCount`、`datasourceCount`、`dashboardCount` 等彙總欄位，以及 `queries`、`datasourceUsage` 內容區。

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
| `--width`、`--height` | 控制瀏覽器 viewport 尺寸 | 適合寬 dashboard 或 panel crop |
| `--device-scale-factor` | 在不改變 CSS viewport 的情況下提高 raster 密度 | `2` 適合需要較銳利 PNG/JPEG 的情境 |
| `--vars-query` | 重播 `var-*` 與相容 query key | 支援 `refresh`、`showCategory`、`timezone` 與 `${__all_variables}` 片段 |
| `--print-capture-url` | 印出最終解析後的 URL | 除錯時非常實用 |
| `--full-page` | 輸出長頁 dashboard 圖 | 瀏覽器式 full-page 擷取 |
| `--full-page-output` | 保留單一大圖或改成分段檔案輸出 | `tiles` 會輸出 `part-0001.*` 等檔案；`manifest` 會再附帶 title/dashboard/panel 資訊的 `manifest.json` |
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

<a id="tw-alert-commands"></a>
4) alert 命令
-------------

目前 `alert` 建議拆成三層來理解：
- Authoring layer：`init`、`add-rule`、`clone-rule`、`add-contact-point`、`set-route`、`preview-route`，以及較低階的 `new-*` scaffold。
- Review/apply layer：`plan`、`apply`，以及 explicit delete preview surface。
- Migration layer：`export`、`import`、`diff`，以及 live inventory 用的 `list-*`。

這三層要分清楚：
- authoring commands 只會寫或預覽 desired-state files，不會直接打 live Grafana mutation API。
- `plan` / `apply` 才是 authoring lane 對 live Grafana 的正式 mutation path。
- `export/import/diff` 仍留在舊的 `raw/` replay lane，不要和 desired-state authoring lane 混用。

實務上要記住的 authoring 邊界：
- `add-rule` 刻意只做 simple threshold / classic-condition style authoring。
- 比較複雜的 rule，先用 `clone-rule` 從既有 desired rule 起手，再手改檔案。
- `set-route` 管的是同一條 tool-owned managed route。重跑會覆蓋那條 route，不做 field-by-field merge。
- `preview-route` 只是 desired-state preview，不是完整的 Grafana routing simulator。
- `--folder` 只會記錄 desired folder identity，不是 live resolve/create folder workflow。
- authoring commands 的 `--dry-run` 會輸出 desired document，但不落檔。

### 4.1 Authoring Layer

**用途**：先在同一棵 managed desired tree 下建立或修改 alert desired files，再進到 live review/apply。

先初始化 desired tree：

```bash
grafana-util alert init --desired-dir ./alerts/desired
```

這棵 tree 會和 migration 用的 `alerts/raw` export tree 分開。

Authoring command 對照表：

| 指令 | 什麼情況用 | 重要邊界 |
| --- | --- | --- |
| `alert add-contact-point` | 快速建立簡單 contact-point desired document | 只寫 desired files |
| `alert add-rule` | 建 simple threshold / classic-condition rule | 不適合複雜 multi-query authoring |
| `alert clone-rule` | 從既有 desired rule clone 出新 rule 再手改 | 比較適合複雜 rule body |
| `alert set-route` | 寫 tool-owned managed route document | 重跑會覆蓋同一條 managed route |
| `alert preview-route` | 預覽 labels 對 managed route contract 的輸入形狀 | 只是 preview，不是 Grafana routing simulation |
| `alert new-rule`、`new-contact-point`、`new-template` | 當高階 authoring surface 不夠時，用低階 scaffold 起手 | 需要手動補更多欄位 |

常見 authoring 指令：

```bash
grafana-util alert add-contact-point --desired-dir ./alerts/desired --name pagerduty-primary
grafana-util alert add-rule --desired-dir ./alerts/desired --name cpu-high --folder platform-alerts --rule-group cpu --receiver pagerduty-primary --label team=platform --severity critical --expr A --threshold 80 --above --for 5m
grafana-util alert preview-route --desired-dir ./alerts/desired --label team=platform --severity critical
```

`preview-route` 的驗證輸出摘錄：

```json
{
  "input": {
    "labels": {
      "team": "platform"
    },
    "severity": "critical"
  },
  "matches": []
}
```

這裡 `matches: []` 是預期行為。`preview-route` 看的是 desired-state preview contract，不是 live alert 實例在 Grafana 裡的完整路由模擬。

Managed route 覆蓋示例：

```bash
grafana-util alert set-route --desired-dir ./alerts/desired --receiver pagerduty-primary --label team=platform --severity critical
grafana-util alert set-route --desired-dir ./alerts/desired --receiver pagerduty-primary --label team=infra --severity critical
```

第二次執行後，managed route 會從 `team=platform` 直接改成 `team=infra`，不會保留舊 matcher 再 merge 新 matcher。

### 4.2 Review And Apply Layer

#### `alert plan`

**用途**：把 desired alert YAML / JSON 與 live Grafana 做比對，產生 reviewable plan。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--desired-dir` | desired alert 目錄 | 讀的是 managed desired files，不是 `raw/` export |
| `--prune` | 把 live-only resource 轉成 delete row | 沒開時不會產生 delete |
| `--dashboard-uid-map` | dashboard UID 對照 | linked alert-rule 跨環境修復 |
| `--panel-id-map` | panel id 對照 | linked panel 跨環境修復 |
| `--output text\|json` | 輸出模式 | `json` 適合 CI 與 review handoff |

範例指令：
```bash
grafana-util alert plan --url http://localhost:3000 --basic-user admin --basic-password admin --desired-dir ./alerts/desired --prune --output json
```

如何判讀：
- `create`：desired 有，但 live 還沒有。
- `update`：live 已存在，但內容不同。
- `noop`：desired 和 live 已一致。
- `delete`：只有開 `--prune` 才會出現。
- `blocked`：有需要處理的差異，但這次 plan 不把它當作 live-safe 動作。

#### `alert apply`

**用途**：把已審查的 alert plan 套回 Grafana。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--plan-file` | 已審查的 alert plan 文件 | `apply` 不直接讀 desired dir |
| `--approve` | 明確批准旗標 | 沒有這個就不執行 |
| `--output text\|json` | 輸出模式 | `json` 較適合 audit |

範例指令：
```bash
grafana-util alert apply --url http://localhost:3000 --basic-user admin --basic-password admin --plan-file ./alert-plan-reviewed.json --approve --output json
```

如何判讀：
- `appliedCount` 表示實際執行了幾筆。
- `results[]` 會列出每個真的送出的 create/update/delete。
- `noop` 和 `blocked` row 不會被執行。

#### `alert delete`

**用途**：預覽單一 alert 資源的 explicit delete 請求。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--kind` | 資源類型 | `rule`、`contact-point`、`mute-timing`、`template`、`policy-tree` |
| `--identity` | 指定 identity | 視資源類型而定，可能是 UID 或名稱 |
| `--allow-policy-reset` | 允許 notification policy reset | `policy-tree` delete 需要明確開啟 |
| `--output text\|json` | 預覽輸出模式 | JSON 較方便交接或自動判讀 |

範例指令：
```bash
grafana-util alert delete --kind policy-tree --identity default --allow-policy-reset --output json
```

如何判讀：
- 這是 preview，不是直接盲刪。
- `policy-tree` 是 reset 路徑，不是一般 delete。
- 沒有 `--allow-policy-reset` 時，這筆會被標成 `blocked`。

### 4.3 Operator Workflows

#### Simple add path（`add-contact-point -> add-rule -> preview-route -> plan -> apply`）

這組命令在 2026-03-30 以 Docker Grafana `12.4.1`、`http://127.0.0.1:43111` 做過本機驗證。

1. 建 desired tree。

```bash
grafana-util alert init --desired-dir ./alerts/desired
grafana-util alert add-contact-point --desired-dir ./alerts/desired --name pagerduty-primary
grafana-util alert add-rule --desired-dir ./alerts/desired --name cpu-high --folder platform-alerts --rule-group cpu --receiver pagerduty-primary --label team=platform --severity critical --expr A --threshold 80 --above --for 5m
grafana-util alert preview-route --desired-dir ./alerts/desired --label team=platform --severity critical
```

2. 先看 live plan。

已驗證指令：

```bash
grafana-util alert plan --url http://127.0.0.1:43111 --basic-user admin --basic-password admin --desired-dir ./alerts/desired --output json
```

已驗證 summary 摘錄：

```json
{
  "summary": {
    "blocked": 1,
    "create": 2,
    "delete": 0,
    "noop": 0,
    "processed": 4,
    "update": 1
  }
}
```

這次驗證裡：
- `create` 是新的 contact point 與 alert rule。
- `update` 是 managed notification policy document，因為 Grafana 起始狀態是 default empty policy tree。
- `blocked` 是沒開 `--prune` 時保留下來的 live default policy tree row。

3. 套用已審查的 plan。

已驗證指令：

```bash
grafana-util alert apply --url http://127.0.0.1:43111 --basic-user admin --basic-password admin --plan-file ./alert-plan-reviewed.json --approve --output json
```

已驗證結果摘錄：

```json
{
  "appliedCount": 3,
  "results": [
    {
      "action": "create",
      "identity": "pagerduty-primary",
      "kind": "grafana-contact-point"
    },
    {
      "action": "update",
      "identity": "pagerduty-primary",
      "kind": "grafana-notification-policies"
    },
    {
      "action": "create",
      "identity": "cpu-high",
      "kind": "grafana-alert-rule"
    }
  ]
}
```

4. 同一份驗證也看到一個實務限制：接著再跑 `plan --prune`，結果不會回到全 `noop`。Grafana 會正規化部分 live payload 欄位，所以目前 authoring 文件不保證 byte-for-byte round-trip。

#### Complex path（`clone-rule -> edit desired file -> plan -> apply`）

當 `add-rule` 太簡化、不足以描述你要的 rule 時，用這條路。

```bash
grafana-util alert clone-rule --desired-dir ./alerts/desired --source cpu-high --name cpu-high-staging --folder staging-alerts --rule-group cpu --receiver slack-platform
# 手改 ./alerts/desired/rules/cpu-high-staging.yaml 或 .json
grafana-util alert plan --url http://localhost:3000 --basic-user admin --basic-password admin --desired-dir ./alerts/desired --output json
grafana-util alert apply --url http://localhost:3000 --basic-user admin --basic-password admin --plan-file ./alert-plan-reviewed.json --approve --output json
```

建議操作模式：
- 先從已知正常的 desired rule 或 export 出來的 rule clone。
- 手改 clone 後的 rule body，處理較複雜的 query、expression、annotation、recording semantics 或 linked dashboard metadata。
- 改完後先 `plan`，確認 live diff 再 `apply`。

#### Delete path（`移除 desired file -> plan --prune -> apply`）

這條 prune delete 流程也在 2026-03-30 用同一個 Docker Grafana `12.4.1` fixture 做過本機驗證。

1. 先把 rule file 從 desired state 移掉。

```bash
rm ./alerts/desired/rules/cpu-high.yaml
```

2. 開 `--prune` 重建 plan。

已驗證指令：

```bash
grafana-util alert plan --url http://127.0.0.1:43111 --basic-user admin --basic-password admin --desired-dir ./alerts/desired --prune --output json
```

已驗證 summary 摘錄：

```json
{
  "summary": {
    "blocked": 0,
    "create": 0,
    "delete": 1,
    "noop": 0,
    "processed": 3,
    "update": 2
  }
}
```

已驗證 delete row 摘錄：

```json
{
  "action": "delete",
  "identity": "cpu-high",
  "kind": "grafana-alert-rule",
  "reason": "missing-from-desired-state"
}
```

這次驗證仍然有兩筆 `update` row，原因和上面同樣是 live normalization 差異；真正的 prune 訊號是 `missing-from-desired-state` 對應出的 `delete` row。

3. 套用 prune plan。

已驗證結果摘錄：

```json
{
  "appliedCount": 3,
  "results": [
    {
      "action": "update",
      "identity": "pagerduty-primary",
      "kind": "grafana-contact-point"
    },
    {
      "action": "update",
      "identity": "pagerduty-primary",
      "kind": "grafana-notification-policies"
    },
    {
      "action": "delete",
      "identity": "cpu-high",
      "kind": "grafana-alert-rule"
    }
  ]
}
```

### 4.4 Migration Layer

#### `alert export`

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
Exported alert rule cpu-main -> /tmp/alert-export-after-apply/raw/rules/general/example-rules/CPU_Main__cpu-main.json
Exported contact point contact-point-uid -> /tmp/alert-export-after-apply/raw/contact-points/example-contact-point/example-contact-point__contact-point-uid.json
Exported notification policies empty -> /tmp/alert-export-after-apply/raw/policies/notification-policies.json
Exported template example-template -> /tmp/alert-export-after-apply/raw/templates/example-template/example-template.json
Exported 1 alert rules, 1 contact points, 0 mute timings, 1 notification policy documents, 1 templates. Root index: /tmp/alert-export-after-apply/index.json
```

#### `alert import`（legacy `import-alert`）

**用途**：將 alert raw 匯入 Grafana。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--import-dir`（必須） | 指向 alert `raw/` 目錄 | 不能指向上層目錄 |
| `--replace-existing` | 已存在則更新 | 常見於正式匯入覆寫 |
| `--dry-run` | 僅模擬執行，不真的送 API | 建議先確認變更範圍 |
| `--json` | 結構化 dry-run 預覽 | 適合自動化 |
| `--dashboard-uid-map` | dashboard uid 對照檔 | linked rule 在目標系統 UID 變更時必備 |
| `--panel-id-map` | panel id 對照檔 | 修復 linked alert 內 panel 參考 |

範例指令：
```bash
grafana-util alert import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./alerts/raw --replace-existing --dry-run --json
```

範例輸出：
```json
{
  "summary": {
    "processed": 4,
    "wouldCreate": 0,
    "wouldUpdate": 4,
    "wouldFailExisting": 0
  },
  "rows": [
    {
      "path": "/tmp/alert-export-after-apply/raw/contact-points/example-contact-point/example-contact-point__contact-point-uid.json",
      "kind": "grafana-contact-point",
      "identity": "contact-point-uid",
      "action": "would-update"
    },
    {
      "path": "/tmp/alert-export-after-apply/raw/policies/notification-policies.json",
      "kind": "grafana-notification-policies",
      "identity": "empty",
      "action": "would-update"
    },
    {
      "path": "/tmp/alert-export-after-apply/raw/rules/general/example-rules/CPU_Main__cpu-main.json",
      "kind": "grafana-alert-rule",
      "identity": "cpu-main",
      "action": "would-update"
    },
    {
      "path": "/tmp/alert-export-after-apply/raw/templates/example-template/example-template.json",
      "kind": "grafana-notification-template",
      "identity": "example-template",
      "action": "would-update"
    }
  ]
}
```

如何判讀：
- `summary` 是回放前最快的安全檢查。
- `would-*` 是 dry-run 預測結果。
- `kind` 可快速看出哪一類 alert 資源會變動。

Migration 範例：

```bash
grafana-util alert export --url http://localhost:3000 --basic-user admin --basic-password admin --output-dir ./alerts --overwrite
grafana-util alert import --url http://localhost:3000 --basic-user admin --basic-password admin --import-dir ./alerts/raw --replace-existing --dry-run --json
```

建議心智模型固定成這樣：
- `add-rule/clone-rule/add-contact-point/set-route/preview-route/init/new-*` 是 desired-state authoring lane。
- `plan/apply` 是 review-first live mutation lane。
- `export/import/diff` 是 migration / replay lane。

### 4.5 `alert diff`（legacy `diff-alert`）

**用途**：比較本地 alert raw 與線上內容。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--diff-dir`（必須） | 指向 raw 目錄 | 比對本地匯出與線上狀態的基準目錄 |
| `--json` | 結構化 diff 輸出 | 適合自動化 |
| `--dashboard-uid-map` | dashboard 對映，確保跨環境比對一致 | 跨環境 UID 不一致時使用 |
| `--panel-id-map` | panel 對映，修正 linked path | panel 編號差異時使用 |

範例指令：
```bash
grafana-util alert diff --url http://localhost:3000 --basic-user admin --basic-password admin --diff-dir ./alerts/raw --json
```

範例輸出：
```json
{
  "summary": {
    "checked": 2,
    "same": 1,
    "different": 1,
    "missingRemote": 0
  },
  "rows": [
    {
      "path": "alerts/raw/contact-points/Smoke_Webhook/Smoke_Webhook__smoke-webhook.json",
      "kind": "grafana-contact-point",
      "identity": "smoke-webhook",
      "action": "different"
    },
    {
      "path": "alerts/raw/policies/notification-policies.json",
      "kind": "grafana-notification-policies",
      "identity": "grafana-default-email",
      "action": "same"
    }
  ]
}
```

### 4.9 `alert list-rules`（legacy `list-alert-rules`）
### 4.10 `alert list-contact-points`（legacy `list-alert-contact-points`）
### 4.11 `alert list-mute-timings`（legacy `list-alert-mute-timings`）
### 4.12 `alert list-templates`（legacy `list-alert-templates`）

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

跨 org 說明：
- `--org-id` 與 `--all-orgs` 在 alert list 命令中只支援 Basic Auth，因為 Grafana org 切換需要管理員式 org scope 變更。

<a id="tw-datasource-commands"></a>
5) datasource 命令
------------------

### 5.1 `datasource list`

**用途**：列出線上的 datasource。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--org-id` | 限定單一 org | 需搭配 Basic Auth |
| `--all-orgs` | 彙整所有可見 org | 跨 org 盤點，需搭配 Basic Auth |
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

跨 org 說明：
- `--org-id` 與 `--all-orgs` 只支援 Basic Auth，因為 datasource list 需要透過 Grafana 管理員式 org 切換來盤點資料。

### 5.2 `datasource export`

**用途**：匯出 datasource masked recovery bundle，並另外輸出 Grafana provisioning YAML lane。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--export-dir`（預設 `datasources`） | 匯出目錄 | 預設包含 `datasources.json`、metadata 與 `provisioning/` |
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
- 上面的命令型態已在 Rust Docker 實測流程中，對真實 Grafana `12.4.1` 服務驗證過。
- 預設匯出根目錄也會包含 `provisioning/datasources.yaml`；但 canonical restore/replay contract 仍是 `datasources.json`。

### 5.3 `datasource import`

**用途**：匯入 datasource inventory，或匯入由 provisioning lane 正規化後的 datasource 定義。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--import-dir`（必須） | 指向 export root（含 `datasources.json`）或 combined export root | 搭配 `--use-export-org` 時要指向整個 multi-org 匯出根目錄 |
| `--input-format inventory\|provisioning` | 選擇磁碟上的匯入 contract | `provisioning` 可接受 export root、`provisioning/` 目錄，或具體 YAML 檔 |
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
- 真實環境 Docker 測試也會驗證依匯出來源 org 回放資料來源：`--use-export-org`、可重複的 `--only-org-id`、以及 `--create-missing-orgs`。在這種模擬執行 JSON 中，會先看到組織層級的 `exists`、`missing-org`、或 `would-create-org`，再進入每筆資料來源操作。
- provisioning 匯入會先把 Grafana datasource YAML 正規化進同一條匯入 pipeline；預設與 canonical restore contract 仍是 `datasources.json` 的 inventory lane。

怎麼看：
- `UID` 與 `NAME` 都重要，但自動化比對應優先以 `UID` 為準。
- `DESTINATION=missing` 代表 live datasource 尚不存在，dry-run 會走建立流程。
- `DESTINATION=exists`、`exists-uid`、`exists-name` 則代表 importer 已找到 live 對象，只是比對後可能決定 `would-update` 或 `no-change`。

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

### 5.5 `datasource add`

**用途**：直接在 Grafana 建立一筆線上 datasource，不經過本地 export bundle。

說明：
- 目前 `datasource add`、`datasource modify`、`datasource delete` 已納入維護中的 `grafana-util` 指令面。

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

範例輸出：
```text
INDEX  NAME          TYPE       ACTION  DETAIL
1      influx-main   influxdb   create  would create datasource uid=influx-main
```

實跑註記：
- datasource mutation 這組命令已在 Docker Grafana `12.4.1` 實測流程中驗證，包含 dry-run 預覽，以及線上 add/modify 後 secret 欄位的保留行為。

<a id="tw-access-commands"></a>
6) Access (存取控制) 指令模組
-------------

`group` 是 `team` 的別名。

### 6.1 `access user list`

**用途**：列出 org 或 global 範圍的使用者。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--scope` | `org` 或 `global` | 指定列舉範圍 |
| `--query` | 模糊搜尋 login/email/name | 大範圍搜尋 |
| `--login` | 精準比對 login | 精準定位 |
| `--email` | 精準比對 email | 精準定位 |
| `--org-role` | 依 org role 篩選 | 權限盤點 |
| `--grafana-admin` | 依 server admin 身分篩選 | 管理員盤點 |
| `--with-teams` | 顯示 team 成員資訊 | 檢查團隊歸屬 |
| `--page`、`--per-page` | 分頁 | 大量使用者 |
| `--table`、`--csv`、`--json` | 輸出模式 | 人工與自動化 |
| `--output-format table\|csv\|json` | 單一輸出旗標 | 取代舊三旗標 |

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

補充：
- `ORG_ROLE` 是 org 內角色，不等於全域管理員權限。
- `GRAFANA_ADMIN=true` 通常只應出現在少數維運帳號。

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
| `--replace-existing` | 更新既有 team | 用於跨環境回放 |
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
- 這條 snapshot 流程已由 `make test-access-live` 在 Grafana `12.4.1` 上驗證，包含 export、diff、dry-run import、線上回放、delete，以及 token 建立與刪除流程。

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
- Docker 實測會先改寫匯出的 snapshot，確認 dry-run 更新預覽，再把同一份檔案實際回放到 Grafana，驗證線上更新路徑。

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

<a id="tw-shared-output-rules"></a>
7) 共通輸出與互斥規則摘要
-------------------------

| 規則 | 說明 |
| --- | --- |
| 輸出格式互斥 | 多數命令以 `Mutually exclusive` 控制 `--table`、`--csv`、`--json`、`--output-format`（不應同時出現）。 |
| 頂層名稱各有角色 | `overview` 是人讀總覽，`status` 是正式 readiness contract，`change` 是 staged change 工作流 |
| dry-run 優先 | 含 `--dry-run` 的流程先跑模擬執行再實際變更 |
| 認證策略 | `org-id`、`all-orgs` 等多數 dashboard/datasource 命令偏向 basic auth；token 更常用於 alert/access 快速操作 |
| 團隊命令 | 本指南統一使用 `access team` |

### 7.1 `change`、`overview`、`status` 這三條線怎麼分

- `change`：偏 staged change 工作流，負責 summary、bundle、preflight、plan/review/apply intent、alert-change assessment。
- `overview`：人類優先的專案入口，用來輸出 staged exports 或 live Grafana 的專案快照。
- `status`：正式的專案層 readiness contract；`staged` 看匯出物，`live` 看目前 Grafana。

### 7.2 `change summary`

**用途**：把 desired resource JSON 正規化成穩定的 staged summary。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--desired-file` | desired resource JSON | 必填 |
| `--output text\|json` | 輸出模式 | JSON 適合後續 plan/review |

範例指令：
```bash
grafana-util change summary --desired-file ./desired.json --output json
```

範例輸出：
```json
{
  "kind": "grafana-utils-sync-summary",
  "summary": {
    "resourceCount": 4,
    "dashboardCount": 1,
    "datasourceCount": 1,
    "folderCount": 1,
    "alertCount": 1
  }
}
```

### 7.3 `change bundle` 與 `change bundle-preflight`

**用途**：先把 dashboard / alert / datasource 匯出物包成單一 source bundle，再檢查哪些項目可以走 staged review、哪些仍是 plan-only 或 blocked。

| 命令 | 關鍵旗標 | 主要用途 |
| --- | --- | --- |
| `change bundle` | `--dashboard-export-dir`、`--alert-export-dir`、`--datasource-export-file`、`--output-file` | 把 staged exports 打包成可攜 bundle |
| `change bundle-preflight` | `--source-bundle`、`--target-inventory`、`--availability-file` | 做 plugin / datasource / alert artifact / secret-provider 的預檢 |

範例指令：
```bash
grafana-util change bundle \
  --dashboard-export-dir ./dashboards/raw \
  --alert-export-dir ./alerts/raw \
  --datasource-export-file ./datasources/datasources.json \
  --output-file ./change-source-bundle.json \
  --output json
```

範例輸出摘錄：
```json
{
  "kind": "grafana-utils-sync-source-bundle",
  "summary": {
    "dashboardCount": 7,
    "datasourceCount": 3,
    "folderCount": 5,
    "alertRuleCount": 1,
    "contactPointCount": 1,
    "muteTimingCount": 1,
    "policyCount": 1,
    "templateCount": 1
  }
}
```

範例指令：
```bash
grafana-util change bundle-preflight \
  --source-bundle ./change-source-bundle.json \
  --target-inventory ./target-inventory.json \
  --output json
```

範例輸出摘錄：
```json
{
  "kind": "grafana-utils-sync-bundle-preflight",
  "summary": {
    "resourceCount": 20,
    "syncBlockingCount": 8,
    "alertArtifactCount": 4,
    "alertArtifactPlanOnlyCount": 1,
    "alertArtifactBlockedCount": 3
  }
}
```

### 7.4 `change plan`、`change review`、`change apply`、`change assess-alerts`

**用途**：把 staged desired 轉成 reviewable plan，標記已審查，再輸出 gated apply intent；若只想看 alert change 評估，就用 `assess-alerts`。

| 命令 | 關鍵旗標 | 主要用途 |
| --- | --- | --- |
| `change plan` | `--desired-file`、`--live-file` 或 `--fetch-live`、`--allow-prune`、`--output json` | 建立 staged plan 文件 |
| `change review` | `--plan-file`、`--review-note`、`--reviewed-by` | 對 plan 蓋章，不直接套用 |
| `change apply` | `--plan-file`、`--approve`、`--execute-live` | 先輸出 apply intent；只有明確指定才做 live execute |
| `change assess-alerts` | `--alerts-file`、`--output json` | 單看 alert candidate / plan-only / blocked 分類 |

範例指令：
```bash
grafana-util change plan --desired-file ./desired-plan.json --live-file ./live.json --output json
```

範例輸出摘錄：
```json
{
  "kind": "grafana-utils-sync-plan",
  "summary": {
    "would_create": 3,
    "would_update": 0,
    "would_delete": 0,
    "noop": 0
  },
  "reviewRequired": true
}
```

範例指令：
```bash
grafana-util change review --plan-file ./change-plan.json --review-note "docs-reviewed" --reviewed-by docs-user --output json
grafana-util change apply --plan-file ./change-plan-reviewed.json --approve --output json
```

範例輸出摘錄：
```json
{
  "kind": "grafana-utils-sync-apply-intent",
  "approved": true,
  "reviewed": true,
  "summary": {
    "would_create": 3,
    "would_update": 0,
    "would_delete": 0,
    "noop": 0
  }
}
```

範例指令：
```bash
grafana-util change assess-alerts --alerts-file ./alerts-only.json --output json
```

範例輸出摘錄：
```json
{
  "kind": "grafana-utils-alert-sync-plan",
  "summary": {
    "alertCount": 1,
    "candidateCount": 0,
    "planOnlyCount": 1,
    "blockedCount": 0
  }
}
```

### 7.5 `overview`

**用途**：把 staged exports 與 staged change 輸入整合成一個專案層級、偏操作員使用的快照。

| 參數 | 用途 | 差異 / 情境 |
| --- | --- | --- |
| `--dashboard-export-dir` | staged dashboard 匯出根目錄 | 通常是一個 `raw/` 目錄 |
| `--datasource-export-dir` | staged datasource 匯出目錄 | 通常是含 `datasources.json` 的 org 匯出目錄 |
| `--alert-export-dir` | staged alert 匯出目錄 | 指到 alert export root，不是只指 `raw/` |
| `--access-*-export-dir` | staged access bundles | 只帶你要納入總覽的 bundle |
| `--desired-file` | 選填 change summary 輸入 | 會加上 staged change 狀態 |
| `--source-bundle`、`--target-inventory`、`--mapping-file` | 選填 bundle/promotion 輸入 | 專案層級視角會更完整 |
| `--output text\|json\|interactive` | 輸出模式 | `interactive` 需要 TUI build |

範例指令：
```bash
grafana-util overview \
  --dashboard-export-dir ./dashboards/raw \
  --datasource-export-dir ./datasources \
  --alert-export-dir ./alerts \
  --access-user-export-dir ./access-users \
  --access-team-export-dir ./access-teams \
  --access-org-export-dir ./access-orgs \
  --access-service-account-export-dir ./access-service-accounts \
  --desired-file ./desired.json \
  --output text
```

範例輸出：
```text
Project overview
Status: blocked domains=6 present=5 blocked=1 blockers=3 warnings=0 freshness=current oldestAge=222s
Artifacts: 8 total, 1 dashboard export, 1 datasource export, 1 alert export, 1 access user export, 1 access team export, 1 access org export, 1 access service-account export, 1 change summary, 0 bundle preflight, 0 promotion preflight
Domain status:
- dashboard status=blocked reason=blocked-by-blockers primary=10 blockers=3 warnings=0 freshness=current next=resolve orphaned datasources, then mixed dashboards
- datasource status=ready reason=ready primary=3 blockers=0 warnings=0 freshness=current
- alert status=ready reason=ready primary=1 blockers=0 warnings=0 freshness=current next=re-run alert export after alerting changes
- access status=ready reason=ready primary=13 blockers=0 warnings=0 freshness=current next=re-run access export after membership changes
- change status=ready reason=ready primary=4 blockers=0 warnings=0 freshness=current next=re-run change summary after staged changes
```

### 7.6 `status staged` 與 `status live`

**用途**：從 staged exports 或目前 Grafana 狀態輸出正式的專案層 readiness contract，作為機器可讀的穩定狀態面。

| 命令 | 關鍵旗標 | 主要用途 |
| --- | --- | --- |
| `status staged` | staged export dirs + 選填 desired/bundle 輸入 | 機器可讀的 staged readiness |
| `status live` | `--url`、認證、選填 staged context 檔案 | 機器可讀的 live readiness |
| `overview live` | 與 live auth 相同 | 人類優先的 live 專案入口，實作上走共享的 `status live` 路徑 |

範例指令：
```bash
grafana-util status staged \
  --dashboard-export-dir ./dashboards/raw \
  --datasource-export-dir ./datasources \
  --alert-export-dir ./alerts \
  --access-user-export-dir ./access-users \
  --access-team-export-dir ./access-teams \
  --access-org-export-dir ./access-orgs \
  --access-service-account-export-dir ./access-service-accounts \
  --desired-file ./desired.json \
  --output json
```

範例輸出摘錄：
```json
{
  "scope": "staged-only",
  "overall": {
    "status": "blocked",
    "domainCount": 6,
    "blockedCount": 1,
    "blockerCount": 3
  }
}
```

範例指令：
```bash
grafana-util status live --url http://localhost:3000 --basic-user admin --basic-password admin --output json
```

範例輸出摘錄：
```json
{
  "scope": "live",
  "overall": {
    "status": "blocked",
    "domainCount": 6,
    "blockedCount": 1,
    "blockerCount": 1,
    "warningCount": 21
  }
}
```

<a id="tw-common-scenarios"></a>
8) 常見情境快速對照
------------------

### 8.1 跨環境 dashboard 遷移

1. `grafana-util dashboard export --all-orgs --overwrite --flat --export-dir ./dashboards`
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

### 8.5 在 CI 中做 dashboard 治理門禁

1. 先準備 `raw/` dashboard 匯出目錄。
2. 產出治理與 flat query JSON：

```bash
grafana-util dashboard inspect-export --import-dir ./dashboards/raw --report governance-json > governance.json
grafana-util dashboard inspect-export --import-dir ./dashboards/raw --report json > queries.json
```

3. 用 policy 檔做治理檢查：

```bash
./scripts/check_dashboard_governance.py \
  --policy examples/dashboard-governance-policy.json \
  --governance governance.json \
  --queries queries.json \
  --json-output governance-check.json
```

4. 只有在你餵的是較舊的 governance artifact、還沒有把 dashboard dependency facts 帶進 `governance.json` 時，才需要額外加 `--import-dir ./dashboards/raw` 當 fallback。

5. governance-json-first checker 目前可阻擋：
   - 不在 allowlist 的 datasource family / uid
   - 無法識別的 datasource
   - mixed-datasource dashboard
   - 不在 allowlist 的 panel plugin
   - 不在 allowlist 的 library panel
   - 不允許的 dashboard folder prefix / routing 邊界
   - dashboard panel / query 引用但未定義的 datasource 變數
   - dashboard / panel query 數超標
   - query / dashboard complexity 分數超標
   - SQL `select *`
   - 缺少 Grafana SQL time filter
   - Loki 過寬 selector / regex

<a id="tw-minimal-sop"></a>
9) 每命令 SOP（最短可跑版本）
------------------------------

每行可直接貼到腳本，替換參數值即可。

```bash
# dashboard
grafana-util dashboard export --url <URL> --basic-user <USER> --basic-password <PASS> --export-dir <DIR> [--overwrite] [--all-orgs] [--flat]
grafana-util dashboard export --url <URL> --token <TOKEN> --org-id <ORG_ID> --export-dir <DIR> [--overwrite]
grafana-util dashboard list --url <URL> --basic-user <USER> --basic-password <PASS> [--org-id <ORG_ID>|--all-orgs] [--output-format table|csv|json] [--with-sources]
grafana-util dashboard list-data-sources --url <URL> --basic-user <USER> --basic-password <PASS> [--output-format table|csv|json]
grafana-util dashboard import --url <URL> --basic-user <USER> --basic-password <PASS> --import-dir <DIR>/raw --replace-existing [--dry-run] [--output-format text|table|json] [--output-columns uid,destination,action,folder_path,destination_folder_path,file]
grafana-util dashboard delete --url <URL> --basic-user <USER> --basic-password <PASS> [--org-id <ORG_ID>] (--uid <UID>|--path <FOLDER_PATH>) [--delete-folders] [--dry-run|--yes]
grafana-util dashboard diff --url <URL> --basic-user <USER> --basic-password <PASS> --import-dir <DIR>/raw [--import-folder-uid <UID>] [--context-lines 3]
grafana-util dashboard inspect-export --import-dir <DIR>/raw --output-format report-tree
grafana-util dashboard inspect-live --url <URL> --basic-user <USER> --basic-password <PASS> --output-format report-json

# alert
grafana-util alert export --url <URL> --token <TOKEN> --output-dir <DIR> [--flat] [--overwrite]
grafana-util alert import --url <URL> --basic-user <USER> --basic-password <PASS> --import-dir <DIR>/raw --replace-existing [--dry-run] [--json] [--dashboard-uid-map <FILE>] [--panel-id-map <FILE>]
grafana-util alert diff --url <URL> --basic-user <USER> --basic-password <PASS> --diff-dir <DIR>/raw [--json] [--dashboard-uid-map <FILE>] [--panel-id-map <FILE>]
grafana-util alert list-rules --url <URL> --token <TOKEN> [--table|--csv|--json]
grafana-util alert list-rules --url <URL> --basic-user <USER> --basic-password <PASS> [--org-id <ORG_ID>|--all-orgs] [--table|--csv|--json]

# datasource
grafana-util datasource list --url <URL> --token <TOKEN> [--table|--csv|--json]
grafana-util datasource list --url <URL> --basic-user <USER> --basic-password <PASS> [--org-id <ORG_ID>|--all-orgs] [--table|--csv|--json]
grafana-util datasource add --url <URL> --token <TOKEN> --name <NAME> --type <TYPE> [--uid <UID>] [--access proxy|direct] [--datasource-url <URL>] [--basic-auth] [--basic-auth-user <USER>] [--basic-auth-password <PASS>] [--user <USER>] [--password <PASS>] [--with-credentials] [--http-header NAME=VALUE] [--tls-skip-verify] [--server-name <NAME>] [--json-data <JSON>] [--secure-json-data <JSON>] [--dry-run] [--output-format text|table|json]
grafana-util datasource export --url <URL> --basic-user <USER> --basic-password <PASS> --export-dir <DIR> [--overwrite] [--dry-run] [--org-id <ORG_ID>|--all-orgs]
grafana-util datasource import --url <URL> --basic-user <USER> --basic-password <PASS> --import-dir <DIR> --replace-existing [--org-id <ORG_ID>] [--use-export-org [--only-org-id <ORG_ID>]... [--create-missing-orgs]] [--dry-run] [--output-format table|text|json] [--output-columns uid,name,type,destination,action,org_id,file]
grafana-util datasource diff --url <URL> --basic-user <USER> --basic-password <PASS> --diff-dir <DIR>

# access
grafana-util access user list --url <URL> --token <TOKEN> --scope org [--table|--csv|--json]
grafana-util access user add --url <URL> --basic-user <USER> --basic-password <PASS> --login <LOGIN> --email <EMAIL> --name <NAME> --password <PWD> [--org-role Editor] [--grafana-admin true|false]
grafana-util access user modify --url <URL> --basic-user <USER> --basic-password <PASS> --login <LOGIN> --set-email <EMAIL> [--set-name <NAME>] [--set-org-role Viewer|Editor|Admin|None] [--set-grafana-admin true|false]
grafana-util access user delete --url <URL> --basic-user <USER> --basic-password <PASS> --login <LOGIN> --scope global --yes
grafana-util access user export --url <URL> --token <TOKEN> --export-dir ./access-users [--scope org|global] [--with-teams]
grafana-util access user import --url <URL> --token <TOKEN> --import-dir ./access-users --replace-existing [--dry-run] [--output-format text|table|json] [--yes]
grafana-util access user diff --url <URL> --token <TOKEN> --diff-dir ./access-users [--scope org|global]
grafana-util access team list --url <URL> --token <TOKEN> [--query <QUERY>|--name <NAME>] [--with-members] [--table|--csv|--json]
grafana-util access team add --url <URL> --token <TOKEN> --name <NAME> [--email <EMAIL>] [--member <LOGIN_OR_EMAIL>] [--admin <LOGIN_OR_EMAIL>]
grafana-util access team modify --url <URL> --token <TOKEN> --name <NAME> [--add-member <LOGIN_OR_EMAIL>] [--remove-member <LOGIN_OR_EMAIL>] [--add-admin <LOGIN_OR_EMAIL>] [--remove-admin <LOGIN_OR_EMAIL>]
grafana-util access team delete --url <URL> --token <TOKEN> --name <NAME> --yes
grafana-util access team export --url <URL> --token <TOKEN> --export-dir ./access-teams [--with-members]
grafana-util access team diff --url <URL> --token <TOKEN> --diff-dir ./access-teams
grafana-util access team import --url <URL> --token <TOKEN> --import-dir ./access-teams --replace-existing [--dry-run] [--output-format text|table|json] [--yes]
grafana-util access service-account export --url <URL> --token <TOKEN> --export-dir ./access-service-accounts [--overwrite]
grafana-util access service-account import --url <URL> --token <TOKEN> --import-dir ./access-service-accounts --replace-existing [--dry-run] [--output-format text|table|json]
grafana-util access service-account diff --url <URL> --token <TOKEN> --diff-dir ./access-service-accounts
grafana-util access service-account list --url <URL> --token <TOKEN> [--query <QUERY>] [--table|--csv|--json]
grafana-util access service-account add --url <URL> --token <TOKEN> --name <NAME> [--role Viewer|Editor|Admin|None] [--disabled true|false]
grafana-util access service-account delete --url <URL> --token <TOKEN> --name <NAME> --yes
grafana-util access service-account token add --url <URL> --token <TOKEN> --name <SA_NAME> --token-name <TOKEN_NAME> [--seconds-to-live <SECONDS>]
grafana-util access service-account token delete --url <URL> --token <TOKEN> --name <SA_NAME> --token-name <TOKEN_NAME> --yes

# change / project
grafana-util change summary --desired-file ./desired.json --output json
grafana-util change bundle --dashboard-export-dir ./dashboards/raw --alert-export-dir ./alerts/raw --datasource-export-file ./datasources/datasources.json --output-file ./change-source-bundle.json --output json
grafana-util change bundle-preflight --source-bundle ./change-source-bundle.json --target-inventory ./target-inventory.json --output json
grafana-util change plan --desired-file ./desired.json --live-file ./live.json --output json
grafana-util change review --plan-file ./change-plan.json --review-note "peer-reviewed" --reviewed-by ops-user --output json
grafana-util change apply --plan-file ./change-plan-reviewed.json --approve --output json
grafana-util change assess-alerts --alerts-file ./alerts-only.json --output json
grafana-util overview --dashboard-export-dir ./dashboards/raw --datasource-export-dir ./datasources --alert-export-dir ./alerts --output text
grafana-util overview live --url <URL> --basic-user <USER> --basic-password <PASS> --output json
grafana-util status staged --dashboard-export-dir ./dashboards/raw --datasource-export-dir ./datasources --alert-export-dir ./alerts --output json
grafana-util status live --url <URL> --basic-user <USER> --basic-password <PASS> --output json
```

<a id="tw-matrix"></a>
10) 參數互斥與差異矩陣（Rust）
--------------------------------

`OUTPUT` 類（`--output-format` 與 `--table/--csv/--json` 互斥關係）：

| 命令 | `--output-format` 允許值 | `--table/--csv/--json` 同時可用 | 備註 |
| --- | --- | --- | --- |
| dashboard list | table/csv/json | 不可 | output-format 取代三旗標 |
| dashboard list-data-sources | table/csv/json | 不可 | 相容命令；新流程優先 `datasource list` |
| dashboard import | text/table/json | 不可（僅 text/table/json） | text 為 dry-run 匯總資訊 |
| dashboard delete | text/table/json | 不可（僅 text/table/json） | text 為 dry-run 匯總資訊 |
| alert list-* | table/csv/json | 不可 | list 命令共用 |
| datasource list | table/csv/json | 不可 | 同上 |
| datasource add | text/table/json | 不可（僅 text/table/json） | dry-run 可用 |
| datasource import | text/table/json | 不可（僅 text/table/json） | text 為 dry-run 摘要，也支援依來源 org 顯示預覽結果 |
| access user list | table/csv/json | 不可 | 同上 |
| access team list | table/csv/json | 不可 | 同上 |
| access user import | text/table/json | 不可（僅 text/table/json） | text 為 dry-run 摘要 |
| access team import | text/table/json | 不可（僅 text/table/json） | text 為 dry-run 摘要 |
| access user diff | text | 否 | 僅摘要 |
| access team diff | text | 否 | 僅摘要 |
| access service-account import | text/table/json | 不可（僅 text/table/json） | text 為 dry-run 摘要 |
| access service-account diff | text | 否 | 僅摘要 |
| access service-account list | table/csv/json | 不可 | 同上 |
| change summary | text/json | 不可（僅 text/json） | desired-resource 摘要 |
| change bundle | text/json | 不可（僅 text/json） | source bundle 文件 |
| change bundle-preflight | text/json | 不可（僅 text/json） | bundle review 文件 |
| change plan | text/json | 不可（僅 text/json） | 可審查的 change plan |
| change review | text/json | 不可（僅 text/json） | reviewed stamp |
| change apply | text/json | 不可（僅 text/json） | apply intent 或 live execute 摘要 |
| change assess-alerts | text/json | 不可（僅 text/json） | alert change 分類 |
| overview | text/json/interactive | 不可（僅 text/json/interactive） | 專案層 staged/live 總覽 |
| status staged/live | text/json/interactive | 不可（僅 text/json/interactive） | 正式的專案層 readiness |

`DRY-RUN` 類（模擬執行）：

| 命令 | `--dry-run` 影響 |
| --- | --- |
| dashboard import | 僅模擬執行 `create/update/skip` |
| datasource import | 僅模擬執行 `create/update/skip` |
| alert import | 僅模擬執行 `create/update` |
| access user import | 僅模擬執行 `create/update/skip`，以及 team 變更預覽 |
| access team import | 僅模擬執行 `create/update/skip`，以及成員變更預覽 |

常見 dry-run 狀態讀法：
- `missing`：live 目標不存在，通常會搭配 `create` / `would-create`
- `exists` / `existing`：live 目標存在，接著看 action 判斷是否 update 或 no-change
- `exists-uid` / `exists-name`：代表 importer 是用哪種方式找到 live 對象
- `missing-org` / `would-create-org`：多 org 路由匯入時，組織層先決條件還沒滿足

`ORG` 控制：

| 命令 | `--org-id` | `--all-orgs` |
| --- | --- | --- |
| dashboard list | 可用（不可用 token，需 Grafana 帳號密碼） | 可用（不可用 token，需 Grafana 帳號密碼） |
| dashboard export | 可用（不可用 token，需 Grafana 帳號密碼） | 可用（不可用 token，需 Grafana 帳號密碼） |
| dashboard import | 可用（不可用 token，需 Grafana 帳號密碼） | 不可 |
| dashboard delete | 可用（不可用 token，需 Grafana 帳號密碼） | 不可 |
| datasource list | 可用（不可用 token，需 Grafana 帳號密碼） | 可用（不可用 token，需 Grafana 帳號密碼） |
| datasource export | 可用（不可用 token，需 Grafana 帳號密碼） | 可用（不可用 token，需 Grafana 帳號密碼） |
| datasource import | 可用（不可用 token，需 Grafana 帳號密碼） | 不可 |
| alert list-* | 可用（不可用 token，需 Grafana 帳號密碼） | 可用（不可用 token，需 Grafana 帳號密碼） |
| alert export/import/diff | 不支援 `org-id`/`all-orgs` | 不支援 |
| alert plan/apply/delete | 不支援 `org-id`/`all-orgs` | 不支援 |
| access 全部 | 用 `--scope` 替代 | 不支援 |
