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

如果要維護 `grafana-util overview` 的 staged overview / workbench 路徑，請先讀
`docs/internal/overview-architecture.md`。那份文件是 `overview` 子系統的專用維護地圖，
聚焦在資料流、模組責任與擴充規則；本文件則維持 crate 級架構導覽。

如果要規劃「整個專案」的概觀看進度、跨 domain status contract、或 project-home /
domain-drill-down / action-handoff 類 TUI 路徑，先讀
`docs/internal/project-status-architecture.md`。那份文件刻意比 `overview` 更上位，
避免把整個專案的 status 架構錯誤地收斂成單一 command 的內部設計。
如果要直接看各 domain 還缺哪些 status producer，再接著看
`docs/internal/project-status-producer-gap-list.md`。
如果要看目前真正建議執行的下一階段順序，再接著看
`docs/internal/next-phase-execution-plan-2026-03-29.md`。
如果要先看目前哪些 domain 已經夠穩可以停、哪些還值得再做一輪，再接著看
`docs/internal/domain-producer-maturity-review-2026-03-29.md`。
如果要直接看目前的專案層 stop / continue 結論與立即執行順序，再接著看
`docs/internal/current-execution-review-2026-03-29.md`。
如果要先看整個專案目前已成型能力、主要 command 用途、以及哪些區域目前不建議再擴，
再接著看 `docs/internal/current-capability-inventory-2026-03-30.md`。

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

- `rust/src/dashboard/mod.rs`
  - 導出 dashboard 的 parser 型別、client helper、runner、以及 `rust/src/dashboard/` 子模組共用型別。
  - `run_dashboard_cli` 是核心 runtime 執行入口：normalize、建 client、分派到 export/import/diff/inspect/list/topology/impact/governance/screenshot/validate/vars 的子流程。
  - inspect 的 query extraction 與 query analysis helper 主要落在 `inspect_query.rs`，live inspect 的 command 路由與 TUI 分工則分別落在 `inspect_live.rs` 與 `inspect_live_tui.rs`。
  - `run_dashboard_cli_with_client` 提供已有 client 的測試/整合替代路徑。
  - `dashboard export` 的 raw metadata contract 會備份 `permissions.json`，但 `dashboard import` 目前仍忽略權限 bundle，只還原 dashboard/folder 與既有 raw inventory。

- `rust/src/alert.rs`
  - 處理 alert 命令入口、legacy/namespaced normalization、`GrafanaAlertClient` 組裝與 routing。
  - `run_alert_cli` 依 `list` / `import` / `diff` / default-export 決定執行路徑。

- `rust/src/access/mod.rs`
  - 處理 access 命令入口與巢狀 dispatch（user/org/team/service-account）。
  - `run_access_cli_with_request` 可注入 request 函式，這是測試時 decouple transport 的主要入口。
  - `run_access_cli` 主要負責 normalize 與 client 注入。

- `rust/src/sync/mod.rs`
  - 處理 sync 命令入口、staged document builder、review/audit/bundle workflow，並把 JSON、bundle inputs、staged document、以及 live fetch/apply helpers 分別收在 `sync/json.rs`、`sync/bundle_inputs.rs`、`sync/staged_documents.rs`、`sync/cli.rs`、`sync/live.rs`。
  - `run_sync_cli` 主要負責 normalize、document routing 與可選 live wiring 的分派。
  - Sync data contract is intentionally narrow:
    1. Input contract: the CLI accepts raw JSON arrays or objects from `--desired-file`, `--live-file`, `--lock-file`, `--availability-file`, `--source-bundle`, and `--target-inventory`; live-backed commands only consult Grafana when `--fetch-live` is set and they reuse `CommonCliArgs` plus optional `--org-id` / `--page-size`.
    2. Normalized internal model: `normalize_resource_spec` collapses staged resources into `SyncResourceSpec { kind, identity, title, body, managed_fields, source_path }`; `kind` is lower-cased, `identity` is derived from `uid/name/title/path`, and alert-like resources must keep `managedFields` so ownership stays explicit.
    3. Output contract: the staged builders emit stable JSON envelopes such as `grafana-utils-sync-summary`, `grafana-utils-sync-plan`, `grafana-utils-sync-apply-intent`, `grafana-utils-sync-source-bundle`, `grafana-utils-sync-preflight`, `grafana-utils-sync-lock`, and `grafana-utils-sync-audit`, each with fixed summary and lineage metadata that later stages validate.
  - Shortest modification path:
    - Change local resource shape or compare/apply semantics: start in `rust/src/sync/workbench.rs`.
    - Change Grafana request mapping or live endpoint handling: start in `rust/src/sync/live_fetch.rs` or `rust/src/sync/live_apply.rs`.
    - Change command routing or live-vs-local selection: start in `rust/src/sync/cli.rs`.
    - Change staged lineage/review metadata: start in `rust/src/sync/staged_documents.rs`.
    - Keep offline contract tests in `rust/src/sync/rust_tests.rs` and request-backed live tests in `rust/src/sync/live_rust_tests.rs`.

- `rust/src/datasource.rs`
  - 管理 list/export/import/diff 四類流程與輸出模式（table/csv/json）。
  - `run_datasource_cli` 先 normalize 再 build client，接著進入對應 handler。

### 2.3.1 Ownership map: facade vs contract vs renderer/state-machine

- Facade / orchestrator：
  - `rust/src/cli.rs`
  - `rust/src/dashboard/mod.rs`
  - `rust/src/access/mod.rs`
  - `rust/src/datasource.rs`
  - `rust/src/sync/mod.rs`
  - 這些檔案應維持在「command topology、normalize、client/request wiring、top-level dispatch」層級，不承接細部資料契約或 renderer 細節。

- Typed contract / shared model：
  - `rust/src/dashboard/inspect_summary.rs`, `rust/src/dashboard/inspect_report.rs`
  - `rust/src/dashboard_reference_models.rs`
  - `rust/src/dashboard_inspection_dependency_contract.rs`
  - `rust/src/sync/workbench.rs`, `rust/src/sync/staged_documents.rs`, `rust/src/sync/bundle_alert_contracts.rs`
  - 這些模組定義跨流程要穩定的中介文件、summary/report 形狀、或 staged contract，改動時要先想 downstream render/tests 是否跟著變。

- Renderer / presentation：
  - `rust/src/dashboard/inspect_render.rs`, `rust/src/dashboard/inspect_dependency_render.rs`, `rust/src/dashboard/inspect_governance_render.rs`
  - `rust/src/dashboard/browse_render.rs`
  - `rust/src/sync/review_tui.rs`, `rust/src/sync/audit_tui.rs`
  - 這些模組主要擁有 text/table/TUI 的可見輸出，不應反向擴張成新的 workflow owner。

- State machine / interactive workbench：
  - `rust/src/dashboard/import_interactive_state.rs`
  - `rust/src/dashboard/inspect_workbench_state.rs`
  - `rust/src/dashboard/import_interactive*.rs`
  - `rust/src/dashboard/inspect_workbench*.rs`
  - 這些模組承接互動式模式的 state、事件處理、view-model 與畫面分工；如果變更只影響 TUI 互動，不應先從 facade 改起。

### 2.3.2 Cross-module relationships maintainers usually need first

- `datasource` 沒有自建 auth/client stack；它直接重用 `dashboard` 匯出的 `build_auth_context`、`build_http_client`、`build_http_client_for_org` 與 `CommonCliArgs`。
- `sync` 是跨 resource 的 staged contract layer，不直接重做各 domain 的 parser；它依賴 `alert_sync`、`datasource_provider`、`datasource_secret` 這些 crate-private assessment helper。
- `dashboard` 是目前最大也是最容易失焦的 domain：`mod.rs` 只應保留 facade/re-export/dispatch，`inspect_*`、`import_interactive_*`、`governance_*`、`browse_*` 則各自維持子系統邊界。
- `access` 的 auth scope 有刻意分流：org 管理走 `build_http_client_no_org_id`，user/team/service-account 流程則維持一般 org-aware client 路徑。

### 2.4 Domain 子模組（實作重點）

- `rust/src/dashboard/`：`cli_defs`, `export`, `files`, `governance_gate`, `governance_gate_tui`, `help`, `impact_tui`, `import`, `inspect`, `inspect_analyzer_*`, `inspect_governance`, `inspect_live`, `inspect_live_tui`, `inspect_query`, `inspect_render`, `inspect_report`, `inspect_summary`, `list`, `live`, `models`, `prompt`, `screenshot`, `topology`, `topology_tui`, `validate`, `vars`；typed inspect summary/report contract surfaces are split across `inspect_summary.rs` and `inspect_report.rs`.
- `rust/src/alert_*`：`alert_cli_defs`, `alert_client`, `alert_list`。
- `rust/src/access/`：`cli_defs`, `org`, `pending_delete`, `render`, `service_account`, `team`, `user`。
- `rust/src/sync/`：`audit`, `audit_tui`, `bundle_alert_contracts`, `bundle_inputs`, `bundle_preflight`, `cli`, `json`, `live`, `preflight`, `review_tui`, `staged_documents`, `workbench`；typed apply/live contract boundaries are split across `live.rs`, `staged_documents.rs`, and `workbench.rs`.
- `rust/src/datasource_diff.rs`：diff 合併/欄位對齊與結果摘要模型。
- `rust/src/http.rs`：HTTP transport 實作、query/url 建構、錯誤對映。
- `rust/src/common.rs`：錯誤型別、訊息、解析工具與共用 helper。

補充維護心法：

- 如果模組名稱帶 `render`、`report`、`summary`，先假設它是 presentation 或 typed-output boundary，不是 command owner。
- 如果模組名稱帶 `state`、`workbench`、`interactive`、`tui`，先假設它承接互動流程或畫面狀態，不是核心 API contract。
- 如果要找「真正的入口」，先回到各 domain 的 `mod.rs` 或單檔 facade（如 `alert.rs`, `datasource.rs`）。

### 2.5 Contract 與 test split

- `dashboard inspect` 的主要 contract 落在 `rust/src/dashboard/inspect.rs`, `rust/src/dashboard/inspect_query.rs`, `rust/src/dashboard/inspect_live.rs`, `rust/src/dashboard/inspect_live_tui.rs`，typed summary/report boundary 則分別在 `rust/src/dashboard/inspect_summary.rs` 與 `rust/src/dashboard/inspect_report.rs`。
- `dashboard inspect` 的回歸測試主要落在 `rust/src/dashboard/rust_tests.rs`，parser/help 類變更則仍跟著對應的 `*_cli_defs.rs`。
- `sync` 的主要 contract 落在 `rust/src/sync/mod.rs`, `rust/src/sync/cli.rs`, `rust/src/sync/live.rs`, `rust/src/sync/json.rs`, `rust/src/sync/bundle_inputs.rs`, `rust/src/sync/staged_documents.rs`, `rust/src/sync/workbench.rs`。
- `sync` 的回歸測試主要落在 `rust/src/sync/cli_rust_tests.rs` 與 `rust/src/sync/rust_tests.rs`。

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
- 改 dashboard inspect 相關 contract：
  - 先看 `dashboard/mod.rs`，再往 `dashboard/inspect.rs`、`dashboard/inspect_query.rs`、`dashboard/inspect_live.rs`、`dashboard/inspect_live_tui.rs` 分流。
- 改 sync contract 或 live plumbing：
  - 先看 `sync/mod.rs`，再按責任拆到 `sync/cli.rs`、`sync/live.rs`、`sync/json.rs`、`sync/bundle_inputs.rs`、`sync/staged_documents.rs`。
- 改 dashboard import interactive workbench：
  - 先看 `dashboard/mod.rs` 確認入口；
  - 狀態/鍵盤事件優先看 `import_interactive_state.rs`；
  - live review/diff 組裝優先看 `import_interactive_review.rs`；
  - local artifact/context loading 優先看 `import_interactive_loader.rs`；
  - TUI layout/context pane 優先看 `import_interactive_render.rs` 與 `import_interactive_context.rs`。
- 改 access auth scope 或 request shape：
  - 先看 `access/mod.rs` 的 `run_*_access_cli` 與 request helper；
  - parser/flag 再回 `access/cli_defs.rs`；
  - resource-specific workflow 再進 `user.rs`、`team.rs`、`service_account.rs`、`org.rs`。
- 改 datasource import/export/diff contract：
  - 先看 `datasource.rs` 的 normalize/dispatch；
  - import/export record shape 進 `datasource_import_export.rs`；
  - live/export compare semantics 進 `datasource_diff.rs`；
  - add/modify/delete payload 與 dry-run 呈現則進 `datasource_mutation_support.rs`。
- 改單一命令流程：
  - 只改對應 domain orchestrator（如 `dashboard/mod.rs` 或 `alert.rs`）中的 dispatch + runner。
- 改 API 呼叫/傳輸：
  - 先看 `http.rs` 是否有可複用封裝；
  - 有限域特例改在 domain 子模組 handler。
- 改輸出格式：
  - dashboard/list/import/diff 常在 `rust/src/dashboard/list.rs`, `rust/src/dashboard/export.rs`, `rust/src/dashboard/import.rs`, `rust/src/dashboard/prompt.rs`, 以及 `datasource` 對應輸出路徑修改。
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

- 新增/調整命令：先看 `cli.rs` 的統一 topology，再更新 `dashboard/mod.rs|alert.rs|access/mod.rs|sync/mod.rs|datasource.rs` 的 runner。
- 新增/調整 dashboard live inspect：優先看 `dashboard/mod.rs` 再往 `dashboard/inspect_live.rs` 與 `dashboard/inspect_live_tui.rs` 分流。
- 新增/調整 dashboard query analysis：優先看 `dashboard/mod.rs` 再往 `dashboard/inspect.rs` 與 `dashboard/inspect_query.rs` 分流。
- 新增/調整 dashboard inspect 資料契約時，先對齊這條最短修改路徑：
  - input contract：`dashboard/inspect_live.rs` 先把 live fetch 轉成與 offline inspect 相同的 raw export tree；`dashboard/inspect.rs` 假設輸入已包含 `export-metadata.json`、`index.json`、folder inventory、datasource inventory 與 dashboard JSON。
  - normalized internal model：`ExportInspectionSummary`、`ExportInspectionQueryReport`、`QueryAnalysis`，以及 governance document rows 是 downstream renderer 與跨路徑回歸測試的共同中介形狀。
  - output contract：`inspect_summary.rs` 與 `inspect_report.rs` 負責 typed summary/report document boundary；`inspect_governance.rs` 與 `inspect_render.rs` 分別擁有 governance 與 text-table-json 的輸出形狀。
  - shortest modification path：改 query extraction 就看 `inspect.rs` + `inspect_query.rs`；改 live staging 就看 `inspect_live.rs`；改輸出形狀就看 summary/report/governance/render 模組；改回歸測試先看 `inspect_live_rust_tests.rs`，再看 `dashboard_rust_tests.rs` 中跨路徑 contract case。
- 新增/調整 sync live/apply boundary：優先看 `sync/mod.rs`，再分別調整 `sync/cli.rs`、`sync/live.rs`、`sync/json.rs`、`sync/bundle_inputs.rs`、`sync/staged_documents.rs`、`sync/workbench.rs`。
- 改 parser：先改對應 `*_cli_defs.rs` 再補 test。
- 改輸出：優先對應子模組的 render/report 檔案。
- 需改 transport：優先 `http.rs` 與 `common.rs`。
