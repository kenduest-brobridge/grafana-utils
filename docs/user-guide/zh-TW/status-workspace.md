# Workspace 審查與狀態

這一章聚焦在 workspace apply 前後的最後檢查，幫你確認目前狀態、差異與套用前的準備是否到位。

## 適用對象

- 要先看 live / staged 狀態再決定下一步的人
- 負責 workspace 審查、test 或 apply gate 的人
- 需要把 status / workspace / config profile 串成固定流程的人

## 主要目標

- 先區分 live 與 staged
- 再確認 workspace package、輸入結構與差異摘要
- 最後才進入 apply

## 採用前後對照

- 以前：status、snapshot 與 workspace review 常常像是三套名稱接近但分工不清的工具。
- 現在：即時檢查、staged 審查與快照式總覽被放進同一條導引路線裡。

## 成功判準

- 你能在 workspace apply 前，先判斷這章是在處理整備度、快照還是審查。
- 你知道流程從 status 進到 mutation 時，應該切到哪一個 command。
- 你能說清楚 workspace apply 前應該先做哪些檢查。

## 失敗時先檢查

- 如果 staged 與 live 看的不是同一個面，先停下來確認哪一條 lane 過期。
- 如果 snapshot 或 summary 跟預期不符，先把它當成流程警訊，不要只當成排版問題。
- 如果你說不出為什麼需要看這章，可能代表你走錯 lane 了。

## 🔗 指令詳細頁面

如果你現在要查的是指令細節，而不是整段工作流程，先看這兩組：

Primary lane：

- [workspace](../../commands/zh-TW/workspace.md)
- [workspace scan](../../commands/zh-TW/workspace-scan.md)
- [workspace test](../../commands/zh-TW/workspace-test.md)
- [workspace preview](../../commands/zh-TW/workspace-preview.md)
- [workspace apply](../../commands/zh-TW/workspace-apply.md)
- [status staged](../../commands/zh-TW/status.md#staged)
- [status live](../../commands/zh-TW/status.md#live)
- [status overview live](../../commands/zh-TW/status.md#overview)

Advanced workflows：

- 如果你需要較低階 staged contract，或要看 bundle / promotion handoff 文件，從 [workspace ci](../../commands/zh-TW/workspace.md#ci) 或 [指令詳細總索引](../../commands/zh-TW/index.md) 開始。
- [snapshot](../../commands/zh-TW/snapshot.md)
- [snapshot export](../../commands/zh-TW/snapshot.md#export)
- [snapshot review](../../commands/zh-TW/snapshot.md#review)
- [config profile](../../commands/zh-TW/config.md)
- [config profile list](../../commands/zh-TW/config.md#list)
- [config profile show](../../commands/zh-TW/config.md#show)
- [config profile add](../../commands/zh-TW/config.md#add)
- [config profile example](../../commands/zh-TW/config.md#example)
- [config profile init](../../commands/zh-TW/config.md#init)

---

## 🚦 狀態操作面

這裡會區分 **Live**（目前 Grafana 上真的在跑的內容）和 **Staged**（你準備要部署的內容）。

### 1. 即時整備度檢查 (Live Check)
```bash
# 用途：1. 即時整備度檢查 (Live Check)。
grafana-util status live --output-format table
```

```bash
# 用途：1. 即時整備度檢查 (Live Check)。
grafana-util status live --profile prod --sync-summary-file ./sync-summary.json --package-test-file ./workspace-package-test.json --output-format json
```
**預期輸出：**
```text
OVERALL: status=ready

COMPONENT    HEALTH   REASON
Dashboards   ok       32/32 可存取
Datasources  ok       秘密資訊恢復驗證通過
Alerts       ok       無孤立規則
```
`status live` 走的是共用的 live 狀態檢查流程。若同時帶入 staged sync 檔案，就能在不改變指令用法的前提下，讓 live 視圖多出更多對照資訊。

### 2. 暫存整備度檢查 (Staged Check)
在執行 `apply` 之前，這一步很適合拿來當 CI/CD 的強制檢查。
```bash
# 用途：在執行 apply 之前，這一步很適合拿來當 CI/CD 的強制檢查。
grafana-util status staged --desired-file ./desired.json --output-format json
```

```bash
# 用途：在執行 apply 之前，這一步很適合拿來當 CI/CD 的強制檢查。
grafana-util status staged --dashboard-export-dir ./dashboards/raw --alert-export-dir ./alerts --desired-file ./desired.json --output-format table
```
**預期輸出：**
```json
{
  "status": "ready",
  "blockers": [],
  "warnings": ["1 個儀表板缺少唯一的目錄分配"]
}
```
`status staged` 比較偏向給腳本或 CI 判讀的驗證結果。`blockers` 代表一定得先處理，`warnings` 則表示需要人工再多看一眼。

---

## 📋 Workspace 審查生命週期

管理從 Git 到正式 Grafana 環境的過渡。

### 第一次使用，先走這條最短路徑

如果你還不確定要從哪裡開始，先照這個順序走：

1. `workspace scan .`
2. `workspace test .`
3. `workspace preview . --fetch-live --profile <profile>`
4. `workspace apply --preview-file ./workspace-preview.json --approve --execute-live --profile <profile>`

workspace 路徑是最短路徑，因為 `workspace` 會先嘗試在目前 repo 或工作目錄裡找常見 staged inputs，包含同一個 mixed repo root 裡的 Git Sync dashboards、`alerts/raw`、`datasources/provisioning`。若這不符合你的目錄布局，再改用 `--desired-file`、`--dashboard-export-dir`、`--alert-export-dir`、`workspace package`、`--target-inventory` 這些明確旗標。

混合 workspace tree 範例：

```text
./grafana-oac-repo/
  dashboards/git-sync/raw/
  dashboards/git-sync/provisioning/
  alerts/raw/
  datasources/provisioning/datasources.yaml
```

### 1. Workspace 掃描
先看目前 workspace package 的高階摘要與輸入形狀。
```bash
# 用途：先從同一個 mixed repo root 自動發現常見 staged inputs。
grafana-util workspace scan ./grafana-oac-repo
```

同一個 workspace root 可以同時包含 `dashboards/git-sync/raw`、`dashboards/git-sync/provisioning`、`alerts/raw` 與 `datasources/provisioning/datasources.yaml`。

```bash
# 用途：用明確 staged 匯出目錄建立 inspection 輸出。
grafana-util workspace scan --dashboard-export-dir ./dashboards/raw --alert-export-dir ./alerts/raw --output-format json
```
**預期輸出：**
```text
WORKSPACE PACKAGE SUMMARY:
- dashboards: 5 modified, 2 added
- alerts: 3 modified
- access: 1 added
- total impact: 11 operations
```
先用 scan 看整個 workspace package 的規模與輸入形狀，再往下看 preview。若總數異常偏大，應先停下來檢查 staged 輸入。

### 2. Workspace 測試
驗證匯出 / 匯入目錄結構與 staged readiness。
```bash
# 用途：先檢查目前 mixed workspace 自動發現到的 staged package。
grafana-util workspace test ./grafana-oac-repo --availability-file ./availability.json
```

```bash
# 用途：把 live availability hints 併進 staged 檢查。
grafana-util workspace test ./grafana-oac-repo --fetch-live --output-format json
```
**預期輸出：**
```text
PREFLIGHT CHECK:
- dashboards: valid (7 files)
- datasources: valid (1 inventory found)
- result: 0 errors, 0 blockers
```
test 適合放在 preview 或 apply 前，做 staged readiness 與結構層檢查。通過只代表輸入形狀合理，不代表 live 狀態已經完全吻合。

### 3. Workspace 預覽
先建立可操作的 preview，確認這次真的會改到哪些東西。
```bash
# 用途：預覽目前 mixed workspace 對 live Grafana 的影響。
grafana-util workspace preview ./grafana-oac-repo --fetch-live --profile prod
```

```bash
# 用途：用明確 desired/live 輸入產出 JSON preview。
grafana-util workspace preview --desired-file ./desired.json --live-file ./live.json --output-format json
```

preview 對應底層 plan contract。對使用者來說，先想「這次會改到什麼」比先想「我要 build 哪種 plan 文件」更自然。
這份 preview contract 也是排序契約的公開面：`ordering.mode`、每筆 operation 的 `orderIndex` / `orderGroup` / `kindOrder`，以及 `summary.blocked_reasons` 會讓審查者看出 plan 的執行順序與尚未解除的受阻工作。

如果同一個 mixed workspace root 最後要交接成 bundle，直接跑 `workspace package ./grafana-oac-repo --output-file ./workspace-package.json`，保留產生的 `workspace-package.json` 作為可攜式的 review artifact。

---

## 🖥️ 互動模式 (TUI) 語意

`status overview live --output-format interactive` 會透過共用的 status live 路徑顯示 live project overview。

```bash
# 用途：status overview live --output-format interactive 會透過共用的 status live 路徑顯示 live project overview。
grafana-util status overview live --url http://localhost:3000 --basic-user admin --basic-password admin --output-format interactive
```

TUI 使用以下視覺語言：
- **🟢 綠色**：組件健康且完全可達。
- **🟡 黃色**：組件可用，但有警告，例如缺少中繼資料。
- **🔴 紅色**：組件受阻，在進行任何部署前都需要處理。

如果要看 staged 產物的人工審查畫面，用不帶 `live` 的 `status overview`；如果要拿結構化輸出做 live 檢查，改用 `status live`。

---
[⬅️ 上一章：Access 管理](access.md) | [🏠 回首頁](index.md) | [➡️ 下一章：維運情境手冊](scenarios.md)
