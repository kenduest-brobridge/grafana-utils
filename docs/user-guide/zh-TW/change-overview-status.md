# 專案狀態與變更總覽 (Change & Status)

這一章聚焦在變更前後的最後檢查，幫你確認目前狀態、差異與套用前的準備是否到位。

## 適用對象

- 要先看 live / staged 狀態再決定下一步的人
- 負責變更審查、preflight 或 apply gate 的人
- 需要把 status / overview / change 串成固定流程的人

## 主要目標

- 先區分 live 與 staged
- 再確認變更包、輸入結構與差異摘要
- 最後才進入 apply

## 採用前後對照

- 以前：status、snapshot 與 change review 常常像是三套名稱接近但分工不清的工具。
- 現在：即時檢查、staged 審查與快照式總覽被放進同一條導引路線裡。

## 成功判準

- 你能在進入變更前，先判斷這章是在處理整備度、快照還是審查。
- 你知道流程從 status 進到 mutation 時，應該切到哪一個 command。
- 你能說清楚變更套用前應該先做哪些檢查。

## 失敗時先檢查

- 如果 staged 與 live 看的不是同一個面，先停下來確認哪一條 lane 過期。
- 如果 snapshot 或 summary 跟預期不符，先把它當成流程警訊，不要只當成排版問題。
- 如果你說不出為什麼需要看這章，可能代表你走錯 lane 了。

## 🔗 指令詳細頁面

如果你現在要查的是指令細節，而不是整段工作流程，直接跳到下面這些指令頁就可以：

- [change](../../commands/zh-TW/change.md)
- [change summary](../../commands/zh-TW/change.md#summary)
- [change plan](../../commands/zh-TW/change.md#plan)
- [change review](../../commands/zh-TW/change.md#review)
- [change apply](../../commands/zh-TW/change.md#apply)
- [change audit](../../commands/zh-TW/change.md#audit)
- [change preflight](../../commands/zh-TW/change.md#preflight)
- [change assess-alerts](../../commands/zh-TW/change.md#assess-alerts)
- [change bundle](../../commands/zh-TW/change.md#bundle)
- [change bundle-preflight](../../commands/zh-TW/change.md#bundle-preflight)
- [change promotion-preflight](../../commands/zh-TW/change.md#promotion-preflight)
- [status](../../commands/zh-TW/status.md)
- [status staged](../../commands/zh-TW/status.md#staged)
- [status live](../../commands/zh-TW/status.md#live)
- [overview](../../commands/zh-TW/overview.md)
- [overview live](../../commands/zh-TW/overview.md#live)
- [snapshot](../../commands/zh-TW/snapshot.md)
- [snapshot export](../../commands/zh-TW/snapshot.md#export)
- [snapshot review](../../commands/zh-TW/snapshot.md#review)
- [profile](../../commands/zh-TW/profile.md)
- [profile list](../../commands/zh-TW/profile.md#list)
- [profile show](../../commands/zh-TW/profile.md#show)
- [profile add](../../commands/zh-TW/profile.md#add)
- [profile example](../../commands/zh-TW/profile.md#example)
- [profile init](../../commands/zh-TW/profile.md#init)
- [指令詳細總索引](../../commands/zh-TW/index.md)

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
grafana-util status live --profile prod --sync-summary-file ./sync-summary.json --bundle-preflight-file ./bundle-preflight.json --output-format json
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

## 📋 變更生命週期 (Change Lifecycle)

管理從 Git 到正式 Grafana 環境的過渡。

### 1. 變更摘要 (Change Summary)
獲取目前變更包的高階摘要。
```bash
# 用途：獲取目前變更包的高階摘要。
grafana-util change summary --desired-file ./desired.json
```

```bash
# 用途：獲取目前變更包的高階摘要。
grafana-util change summary --desired-file ./desired.json --output-format json
```
**預期輸出：**
```text
CHANGE PACKAGE SUMMARY:
- dashboards: 5 modified, 2 added
- alerts: 3 modified
- access: 1 added
- total impact: 11 operations
```
先用 summary 看整個變更包的規模，再往下看 plan。若總數異常偏大，應先停下來檢查 staged 輸入。

### 2. 預檢驗證 (Preflight Validation)
驗證匯出 / 匯入目錄結構的完整性。
```bash
# 用途：驗證匯出 / 匯入目錄結構的完整性。
grafana-util change preflight --desired-file ./desired.json --availability-file ./availability.json
```

```bash
# 用途：驗證匯出 / 匯入目錄結構的完整性。
grafana-util change preflight --desired-file ./desired.json --fetch-live --output-format json
```
**預期輸出：**
```text
PREFLIGHT CHECK:
- dashboards: valid (7 files)
- datasources: valid (1 inventory found)
- result: 0 errors, 0 blockers
```
preflight 適合放在規劃或套用前，做結構層的檢查。通過只代表輸入形狀合理，不代表 live 狀態已經完全吻合。

---

## 🖥️ 互動模式 (TUI) 語意

`overview live --output-format interactive` 會透過共用的 live status 路徑顯示 live project overview。

```bash
# 用途：overview live --output-format interactive 會透過共用的 live status 路徑顯示 live project overview。
grafana-util overview live --url http://localhost:3000 --basic-user admin --basic-password admin --output-format interactive
```

TUI 使用以下視覺語言：
- **🟢 綠色**：組件健康且完全可達。
- **🟡 黃色**：組件可用，但有警告，例如缺少中繼資料。
- **🔴 紅色**：組件受阻，在進行任何部署前都需要處理。

如果要看 staged 產物的人工審查畫面，用不帶 `live` 的 `overview`；如果要拿結構化輸出做 live 檢查，改用 `status live`。

---
[⬅️ 上一章：Access 管理](access.md) | [🏠 回首頁](index.md) | [➡️ 下一章：維運情境手冊](scenarios.md)
