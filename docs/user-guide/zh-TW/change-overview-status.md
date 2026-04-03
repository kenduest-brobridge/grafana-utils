# 專案狀態與變更總覽 (Change & Status)

本章聚焦在治理關卡，也就是在執行變更前後的最後一層驗證。

## 🔗 逐指令頁面

如果您現在想看的是逐指令說明，而不是工作流章節，請直接使用逐指令頁面：

- [change](../../commands/zh-TW/change.md)
- [status](../../commands/zh-TW/status.md)
- [overview](../../commands/zh-TW/overview.md)
- [snapshot](../../commands/zh-TW/snapshot.md)
- [逐指令總索引](../../commands/zh-TW/index.md)

---

## 🚦 狀態介面 (Status Surfaces)

我們區分 **Live** (實際運行中) 與 **Staged** (您打算部署的內容)。

### 1. 即時整備度檢查 (Live Check)
```bash
grafana-util status live --output table
grafana-util status live --profile prod --sync-summary-file ./sync-summary.json --bundle-preflight-file ./bundle-preflight.json --output json
```
**預期輸出：**
```text
OVERALL: status=ready

COMPONENT    HEALTH   REASON
Dashboards   ok       32/32 可存取
Datasources  ok       秘密資訊恢復驗證通過
Alerts       ok       無孤立規則
```
`status live` 走的是共用的 live 狀態路徑。搭配 staged sync 檔案時，可以在不改變命令形狀的前提下，讓 live 視圖帶入更多脈絡。

### 2. 暫存整備度檢查 (Staged Check)
在執行 `apply` 之前，將此作為 CI/CD 的強制性門禁。
```bash
grafana-util status staged --desired-file ./desired.json --output json
grafana-util status staged --dashboard-export-dir ./dashboards/raw --alert-export-dir ./alerts --desired-file ./desired.json --output table
```
**預期輸出：**
```json
{
  "status": "ready",
  "blockers": [],
  "warnings": ["1 個儀表板缺少唯一的目錄分配"]
}
```
`status staged` 是給機器讀的驗證關卡。`blockers` 代表必須停下來處理，`warnings` 則是需要人工再確認的風險。

---

## 📋 變更生命週期 (Change Lifecycle)

管理從 Git 到正式 Grafana 環境的過渡。

### 1. 變更摘要 (Change Summary)
獲取目前變更包的高階摘要。
```bash
grafana-util change summary --desired-file ./desired.json
grafana-util change summary --desired-file ./desired.json --output json
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
grafana-util change preflight --desired-file ./desired.json --availability-file ./availability.json
grafana-util change preflight --desired-file ./desired.json --fetch-live --output json
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

`overview live --output interactive` 會透過共用的 live status 路徑開啟 live project overview。

```bash
grafana-util overview live --url http://localhost:3000 --basic-user admin --basic-password admin --output interactive
```

TUI 使用以下視覺語言：
- **🟢 綠色**：組件健康且完全可達。
- **🟡 黃色**：組件可用，但有警告，例如缺少元數據。
- **🔴 紅色**：組件受阻，在進行任何部署前都需要處理。

如果要看 staged 產物的人工審查畫面，用不帶 `live` 的 `overview`；如果要機器可讀的 live 驗證關卡，改用 `status live`。

---
[⬅️ 上一章：Access 管理](access.md) | [🏠 回首頁](index.md) | [➡️ 下一章：維運情境手冊](scenarios.md)
