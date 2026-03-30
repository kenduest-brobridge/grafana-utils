# Grafana Utilities

[English Version](README.md) | **繁體中文版**

`grafana-util` 是一套以 Rust 為主的 Grafana 維運 CLI，重點放在盤點、遷移、review-first 的匯出/匯入流程、dashboard 分析、專案層 overview/status 讀取，以及 staged change 工作流。

## 為什麼會有這個工具

Grafana 的 UI 很適合做單一物件的日常管理，但一碰到 estate-level 的工作，例如批次匯出、跨 org 盤點、依賴分析、遷移預演、reviewable 變更證據，就會變得不夠順手。

這個落差在企業環境或多 org 環境特別明顯：

- Grafana UI 的 dashboard export 很難做批次遷移，很多情況還是得一個一個處理
- 要把 dashboard 轉移到別的環境時，常常得選對「可供外部分享 / 可供匯入重映射 datasource」那種 export 形式，目標端匯入時才能重新選 datasource
- 維運人員需要知道目前有哪些 datasources、哪些 dashboards 用到哪些 datasources、每個 dashboard 裡實際用了哪些 query / metric 語法
- user、org、team、service account、permission 這些狀態，在 Grafana UI 裡分散，很難做完整盤點
- alerting 資源的搬移也很麻煩，因為 Grafana 沒有提供像 dashboard 那樣完整直覺的 UI import 路徑
- 真正在維運時，import / replay 這類動作應該要先能 `diff`、`dry-run`、review，再決定是否 apply，而不是直接盲改

`grafana-util` 的定位，就是把 Grafana 狀態轉成可盤點、可匯出、可分析、可 diff、可 dry-run、可 review 的維運材料。它主要解的是 migration、audit、governance、handoff 這類操作流程，不是拿來取代 Grafana 本身的 dashboard 編輯體驗。

## 輸出與操作面相

很多命令都刻意提供不只一種輸出面，讓同一套流程可以同時支援人工巡檢、互動式操作，以及自動化串接。

| 面相 | 適合用途 | 代表命令 |
| --- | --- | --- |
| 互動式 TUI | 導覽、審查、在終端機內逐步操作 | `dashboard browse`、`dashboard inspect-export --interactive`、`dashboard inspect-live --interactive`、`datasource browse`、`overview --output interactive`、`status ... --output interactive` |
| 純文字 | 預設維運摘要、dry-run 預覽、人工閱讀 | `change`、`overview`、`status`、各種 dry-run 摘要 |
| JSON | CI、自動化、結構化審查結果、命令間資料接力 | import dry-run、change 文件、staged/live status contract |
| Table / CSV / report | 清單盤點、分析報表、治理檢查 | list 系列命令、`dashboard inspect-*`、review tables |

## 功能支援層級

如果你想快速判斷「這個專案在每個模組到底支援到多深」，先看這張表。

| 模組 | 支援層級 | 目前可做的事 | 輸出與操作面 | 備註 |
| --- | --- | --- | --- | --- |
| `dashboard` | 最完整、最深入 | list、export、import、diff、delete、live/export inspect、query inventory、datasource dependency review、permission export | text、table/csv/json、report 模式、互動式 TUI、screenshot/PDF | 功能最完整，也是分析能力最重的模組 |
| `datasource` | 深且成熟 | list、export、import、diff、add、modify、delete、live browse、跨 org replay | text、table/csv/json、互動式 browse | 同時支援 live mutation 與檔案回放 |
| `alert` | 成熟的管理與遷移面 | 列出 rules、contact points、mute timings、templates；建立 reviewable alert plan；套用已審查變更；預覽 explicit delete；建立 managed desired-state scaffold；並支援 export、import、diff、dry-run bundle | text/json、table/csv/json | 同時涵蓋 operator-first 管理流程與舊有遷移 / replay 流程 |
| `access` | 成熟的盤點與回放面 | 管理 org、user、team、service account；支援 export、import、diff、dry-run；service-account token add/delete | table/csv/json | 適合 access state inventory 與受控重建 |
| `change` | 進階 staged workflow | 建立 summary、bundle、preflight、plan、review record、apply intent、audit、promotion-preflight 文件 | text/json | 重點是 review-first 的變更流程，不是直接盲目套用 |
| `overview` | 人類優先的專案入口 | 把 staged exports 或 live Grafana 整理成單一專案快照 | text/json/interactive | 適合 handoff、triage、人工巡檢時先進來看 |
| `status` | 正式 status contract | 輸出專案層的 staged/live readiness contract | text/json/interactive | 適合自動化、交接，或需要同一份穩定跨模組 status 時使用 |

## 功能快速矩陣

這是 README 版的精簡支援矩陣；如果要看完整到每個命令層級的差異，請再往 user guide 查。

核心資源工作流：

| 模組 | List | Export | Import | Diff | Inspect / Analyze | Live mutation | TUI |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `dashboard` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| `datasource` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| `alert` | ✓ | ✓ | ✓ | ✓ | - | ✓ | - |
| `access` | ✓ | ✓ | ✓ | ✓ | - | ✓ | - |

專案層工作流：

| 入口 | Staged review | Live read | Interactive view |
| --- | --- | --- | --- |
| `change` | ✓ | ✓ | - |
| `overview` | ✓ | ✓ | ✓ |
| `status` | ✓ | ✓ | ✓ |

## 快速開始

先確認目前維護中的命令面：

```bash
grafana-util -h
grafana-util dashboard -h
grafana-util datasource -h
grafana-util alert -h
grafana-util access -h
grafana-util change -h
grafana-util overview -h
grafana-util status -h
```

本 README 的範例已在本地 Docker Grafana `12.4.1` 驗證，並另外灌入 sample org、dashboard、datasource、alerting resource、user、team、service account。

跨 org 列出 dashboard 並帶出 datasource：

```bash
grafana-util dashboard list \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --all-orgs \
  --with-sources \
  --table
```

分析已匯出的 dashboard，查看 datasource 使用情況、query 結構與治理報表：

```bash
grafana-util dashboard inspect-export \
  --import-dir ./dashboards/raw \
  --output-format report-table
```

在真正匯入前先做 dry-run 預覽：

```bash
grafana-util dashboard import \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --import-dir ./dashboards/raw \
  --replace-existing \
  --dry-run \
  --table
```

匯出 alerting 資源做遷移或審查：

```bash
grafana-util alert export \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --output-dir ./alerts \
  --overwrite
```

從 desired YAML / JSON 建立 reviewable 的 alert management plan：

```bash
grafana-util alert plan \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --desired-dir ./alerts/desired \
  --prune \
  --output json
```

現在 alert 可以分成三層來理解：

1. Authoring layer：`alert add-rule`、`clone-rule`、`add-contact-point`、`set-route`、`preview-route`，以及較低階的 `init` / `new-*` scaffold。這些命令只會寫或預覽 desired-state files，不會直接修改 live Grafana。
2. Review/apply layer：`alert plan` 會把 desired files 和 live Grafana 做比對，`alert apply` 只執行已審查的 create / update / delete row。
3. Migration layer：`alert export`、`import`、`diff` 仍維持在 raw inventory、replay、bundle 型遷移流程。

Authoring 邊界：
- `add-rule` 只適合 simple threshold / classic-condition style authoring。比較複雜的 rule，先用 `clone-rule` 從真實 rule 起手，再手改 desired file。
- `set-route` 管的是同一條 managed route。重跑會覆蓋那條 route，不做 field-by-field merge。
- `preview-route` 只是 desired-state preview helper，不是完整的 Grafana routing simulator。
- `--folder` 目前只會寫進 authored desired metadata，不是 live resolve/create folder workflow。
- authoring commands 都支援 `--dry-run`，可先看輸出的 desired document 再決定是否落檔。

短版 alert 工作流：

1. 先用 authoring layer 在 `./alerts/desired` 產生或修改 desired files。
2. 用 `alert plan` 審查 live Grafana 會 create、update、blocked、delete 哪些資源。
3. 確認 plan 後，再用 `alert apply` 套用已審查的 plan file。
4. 如果要刪除，先移除 desired file，再跑 `alert plan --prune`，最後套用 delete plan。

最小 authoring 到 apply 範例：

```bash
grafana-util alert init --desired-dir ./alerts/desired
grafana-util alert add-contact-point --desired-dir ./alerts/desired --name pagerduty-primary
grafana-util alert add-rule --desired-dir ./alerts/desired --name cpu-high --folder platform-alerts --rule-group cpu --receiver pagerduty-primary --label team=platform --severity critical --expr A --threshold 80 --above --for 5m
grafana-util alert preview-route --desired-dir ./alerts/desired --label team=platform --severity critical
grafana-util alert plan --url http://localhost:3000 --basic-user admin --basic-password admin --desired-dir ./alerts/desired --output json
grafana-util alert apply --url http://localhost:3000 --basic-user admin --basic-password admin --plan-file ./alert-plan-reviewed.json --approve --output json
```

複雜 rule 路徑：

```bash
grafana-util alert clone-rule --desired-dir ./alerts/desired --source cpu-high --name cpu-high-staging --folder staging-alerts --rule-group cpu --receiver slack-platform
# 手改 ./alerts/desired/rules/cpu-high-staging.yaml 或 .json
grafana-util alert plan --url http://localhost:3000 --basic-user admin --basic-password admin --desired-dir ./alerts/desired --output json
grafana-util alert apply --url http://localhost:3000 --basic-user admin --basic-password admin --plan-file ./alert-plan-reviewed.json --approve --output json
```

刪除路徑：

```bash
rm ./alerts/desired/rules/cpu-high.yaml
grafana-util alert plan --url http://localhost:3000 --basic-user admin --basic-password admin --desired-dir ./alerts/desired --prune --output json
grafana-util alert apply --url http://localhost:3000 --basic-user admin --basic-password admin --plan-file ./alert-plan-reviewed.json --approve --output json
```

完整的 alert authoring、review/apply、migration、prune delete 指南在 [docs/user-guide-TW.md](./docs/user-guide-TW.md)。

## 安裝與建置

- [最新版 release](https://github.com/kenduest-brobridge/grafana-utils/releases/latest)
- [所有 releases](https://github.com/kenduest-brobridge/grafana-utils/releases)

若要使用目前 checkout 的版本，請直接本地建置：

```bash
cd rust && cargo build --release
```

## 文件

- [英文使用者指南](docs/user-guide.md)
- [繁體中文使用者指南](docs/user-guide-TW.md)
- [Rust 技術總覽](docs/overview-rust.md)
- [開發者手冊](docs/DEVELOPER.md)

## 相容性

- OS：Linux、macOS
- 執行型態：Rust release binary
- Grafana：已驗證 `12.4.1`，目標支援 `8.x` 到目前 `12.x`
