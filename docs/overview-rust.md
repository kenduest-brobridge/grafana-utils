# Grafana Utils Rust Architecture for Maintainers

這份文件是 `rust/` crate 的工程維護導覽。目標是：新加入人員不需要先看每一行程式，就能理解資料流、模組邊界與最小修改路徑。

## 1) 這個 crate 在做什麼

Rust crate 提供四個 CLI domain 的核心執行能力：

- `dashboard`
- `alert`
- `access`
- `datasource`

共同資源由 `common`、`http` 兩層承接，並以 `cli` 做統一入口分流。

這個 crate 的對外定位是：  
- CLI 參數解析與路由是入口層；  
- Domain 模組是命令執行器；  
- `common/http` 是輸出、驗證、傳輸基礎層；  
- 不在這裡直接實作跨 domain 的業務策略。

## 2) 檔案導覽與責任邊界

### 2.1 Entrypoint

- `rust/src/bin/grafana-util.rs`
  - 入口行為：
    1. 先做 `--help-full` 特殊分支（dashboard inspect 用）；
    2. 否則交由 `cli::parse_cli_from` 與 `cli::run_cli`。
  - 只處理 process exit 邏輯，不處理 domain 行為。

### 2.2 Unified Dispatcher 層

- `rust/src/cli.rs`
  - 擁有 `UnifiedCommand`、`DashboardGroupCommand` 等 command enum。
  - 透過 `parse_cli_from` 完成 CLI 解析（純解析，無 side effect）。
  - 透過 `run_cli` 與 `dispatch_with_handlers` 實作 alias、legacy 及 namespaced 轉換，最後呼叫 domain runner。
  - 任何「domain 邏輯」都不應放在這裡；這一層只做「命令路徑決定」。

### 2.3 Domain orchestrator 層

- `rust/src/dashboard.rs`
  - 導出 dashboard 的 parser 型別、client helper、runner、以及 submodule 共用型別。
  - `run_dashboard_cli` 是核心 runtime 執行入口：normalize、建 client、分派到 export/import/diff/inspect/list 的子流程。
  - `run_dashboard_cli_with_client` 提供已有 client 的測試/整合替代路徑。

- `rust/src/alert.rs`
  - 處理 alert 命令入口、legacy/namespaced normalization、`GrafanaAlertClient` 組裝與 routing。
  - `run_alert_cli` 依 `list` / `import` / `diff` / default-export 決定執行路徑。

- `rust/src/access.rs`
  - 處理 access 命令入口與巢狀 dispatch（user/team/service-account）。
  - `run_access_cli_with_request` 可注入 request 函式，這是測試時 decouple transport 的主要入口。
  - `run_access_cli` 主要負責 normalize 與 client 注入。

- `rust/src/datasource.rs`
  - 管理 list/export/import/diff 四類流程與輸出模式（table/csv/json）。
  - `run_datasource_cli` 先 normalize 再 build client，接著進入對應 handler。

### 2.4 Domain 子模組（實作重點）

- `rust/src/dashboard_*`：`dashboard_*_defs`, `export`, `import`, `list`, `live`, `inspect`, `inspect_report`, `inspect_render`, `models`, `files`, `prompt`, `help`。
- `rust/src/alert_*`：`alert_cli_defs`, `alert_client`, `alert_list`。
- `rust/src/access_*`：`access_cli_defs`, `access_render`, `access_user`, `access_team`, `access_service_account`, `access_pending_delete`。
- `rust/src/datasource_diff.rs`：diff 合併/欄位對齊與結果摘要模型。
- `rust/src/http.rs`：HTTP transport 實作、query/url 建構、錯誤對映。
- `rust/src/common.rs`：錯誤型別、訊息、解析工具與共用 helper。

## 3) 執行資料流（可複製做 debug）

### Dashboard 命令流（以 `grafana-util dashboard export` 為例）

1. CLI 二進位收到 argv。  
2. `cli::parse_cli_from` -> `CliArgs`（無 side effect）。  
3. `cli::run_cli` -> `DashboardGroupCommand::Export` 轉為 `DashboardCliArgs::command = DashboardCommand::Export(...)`。  
4. `dashboard::run_dashboard_cli` -> `normalize_dashboard_cli_args`。  
5. 進入 `DashboardCommand::Export`：
   - 檢查 `without-dashboard-raw` 與 `without-dashboard-prompt` 的互斥。
   - 呼叫 `dashboard_export::export_dashboards_with_org_clients`。  
6. export 子模組呼叫 `common/http` 取得 JSON，轉換輸出與寫檔。

### Alert 命令流（以 `grafana-util alert import` 為例）

1. `cli` 命令轉為 `AlertCliArgs`。  
2. `alert::run_alert_cli` 判斷輸入欄位，進入 import 路徑。  
3. `alert` module 組裝 auth context，建立 `GrafanaAlertClient`。  
4. client 與 import handler 透過 `http` 取得 API 回應並進行檔案格式化。

### Access 命令流（以 `grafana-util access user list` 為例）

1. `cli` 命令轉為 `AccessCliArgs`。  
2. `run_access_cli` `normalize_access_cli_args`。  
3. 依 `AccessCommand::User/Team/ServiceAccount` dispatch 到 `run_access_cli_with_request`。  
4. 進入 `access_user|team|service_account` 子模組做實際 API 列表或 CRUD。

### Datasource 命令流

1. `cli` 命令 -> `DatasourceGroupCommand`。  
2. `run_datasource_cli` 呼叫 `normalize_datasource_group_command`，處理輸出格式 alias。  
3. 分流到 list/export/import/diff 分支。  
4. list 直接輸出；export 產生 `datasources.json` 與 index/manifest；import/diff 依錄入 metadata 與 live records 驗證。

## 4) 關鍵設計意圖（不只是概念，是真實維運規範）

- 命令可讀性優先於「壓縮重構」：`cli`/domain 分層清楚時，help text、deprecation alias、parser 改動最不容易破壞執行邏輯。
- 測試友善性：  
  - Domain runner 提供「注入 client 或 request function」版本，能用測試替代網路行為。  
  - 重要 parser 行為有 `*_cli_defs` + `*_rust_tests.rs` 覆蓋。
- 向後相容優先：legacy alias 與 namespaced command 在 `cli.rs` 集中管理，降低散落修改風險。
- 模組邊界不混：transport 由 `http`/`common` 做；parser 規格在 `*_cli_defs.rs`；執行路由在 domain runner；IO/輸出集中在子模組。

## 5) 你要改某條命令時，建議改哪一層

- 新增/調整 CLI 旗標：
  - 先改 `*_cli_defs.rs`（例如 `dashboard_cli_defs.rs`）  
  - 再看 `cli.rs` 是否需要 alias/command 樹更新  
  - 最後補 parser/help/錯誤訊息對齊測試
- 改單一命令流程：
  - 只改對應 domain orchestrator（如 `dashboard.rs` 或 `alert.rs`）中的 dispatch + runner。
- 改 API 呼叫/傳輸：
  - 先看 `http.rs` 是否有可複用封裝；
  - 有限域特例改在 domain 子模組 handler。
- 改輸出格式：
  - dashboard/list/import/diff 常在 `dashboard_list.rs`, `dashboard_export.rs`, `dashboard_prompt.rs`, `datasource` 對應輸出路徑修改。
- 加新 domain：
  - 先定義 CLI 入口（`*_cli_defs.rs`）  
  - 再加 runner 分派（`cli.rs`）  
  - 最後加 domain orchestrator 與子模組。

## 6) 常見維運風險與紅旗（先看這裡）

- 避免在 `cli.rs` 新增 domain 判斷邏輯（破壞 dispatch 可測試性）。  
- 避免直接把 API 欄位轉換放進 `run_cli` 或 `run_*_cli`（應放在 handler 專屬模組）。  
- legacy alias 不能隨意刪除；需保留 fallback 覆蓋路徑並更新 `help text`。  
- 資料輸出格式旗標衝突（`--table`, `--csv`, `--json`）要保持單一路徑規則。  
- 跨 domain 共用常數要放在各 domain module 的 `pub const`，不要散在 handler 實作內聯。

## 7) 快速驗證指令（維護 SOP）

### 單純語法/邏輯

- `cargo test --quiet`  
- `cargo test -- --ignored`（若套件有 ignored case）

### rustdoc 可讀性

- `cargo doc --no-deps --document-private-items`
- `rg -n "run_.*_cli|dispatch_with_handlers|normalize_.*command" rust/src`

### 行為變更時

- 新命令/旗標新增前：先跑 `cargo test --quiet`  
- 加入輸出變更後：補對應測試，特別是 `*_rust_tests.rs` 中的 parser 或 formatter 行為

## 8) 維護節點參考（Rust）

- 新增/調整命令：先看 `cli.rs` 的統一 topology，再更新 `dashboard.rs|alert.rs|access.rs|datasource.rs` 的 runner。
- 改 parser：先改對應 `*_cli_defs.rs` 再補 test。
- 改輸出：優先對應子模組的 render/report 檔案。
- 需改 transport：優先 `http.rs` 與 `common.rs`。
