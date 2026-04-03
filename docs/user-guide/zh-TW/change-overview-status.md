# Change, Overview, and Status 維運人員手冊

本指南涵蓋專案層級的介面指令：`change`、`overview` 與 `status`。

> **目標**：將分散的資源 (Dashboard, Alert, Access) 統合成一個統一的專案視角，用於生命週期管理與整備度報告。

---

## 🛠️ 這些介面的用途

不同的任務需要不同的分析介面。

| 介面 | 最佳用途 | 合約類型 |
| :--- | :--- | :--- |
| **`change`** | 暫存工作流與執行意圖。 | 暫存審查週期。 |
| **`overview`** | 人類專案審查。 | 人類觀點快照。 |
| **`status`** | 整備度門禁與自動化。 | 官方整備度合約。 |

---

## 🚧 工作流路徑邊界 (審查週期)

當暫存資產需要受控且審查優先的路徑時，請依序使用 `change` 指令。

1. **`plan`**：在本地檔案與即時狀態之間產生可審查的差異。
2. **`review`**：紀錄維運人員的決策 (批准 / 拒絕)。
3. **`apply`**：在核准後發出最終的執行意圖 (Apply Intent)。

---

## 📋 閱讀專案狀態 (Project Status)

使用 `status live` 驗證即時 Grafana 資產的健康度與整備度。

```bash
grafana-util status live --url http://localhost:3000 --basic-user admin --basic-password admin --table
```

**驗證輸出摘錄：**
```text
Project status
Overall: status=partial scope=live domains=6 present=6 blocked=0 blockers=0 warnings=4
Domains:
- dashboard status=ready mode=live-read primary=3 blockers=0 warnings=0
- datasource status=ready mode=live-inventory primary=1 blockers=0 warnings=1
- alert status=ready mode=live-alert-surfaces primary=2 blockers=0 warnings=0
```

**如何解讀：**
- **Overall status**：`ready` (良好), `partial` (存在警告), 或 `blocked` (發現錯誤)。
- **Domains**：各資源家族 (Dashboard, Datasource, Alert 等) 的整備度報告。
- **Blockers**：在專案被視為「準備就緒」前，必須解決的具體問題。

---

## 🚀 關鍵指令 (完整參數參考)

| 指令 | 帶有參數的完整範例 |
| :--- | :--- |
| **即時狀態** | `grafana-util status live --url <URL> --basic-user admin --table` |
| **暫存狀態** | `grafana-util status staged --dashboard-export-dir ./dashboards --output json` |
| **總覽** | `grafana-util overview --dashboard-export-dir ./dashboards --output interactive` |
| **變更計畫** | `grafana-util change plan --desired-file <FILE> --live-file <FILE> --output json` |
| **審查** | `grafana-util change review --plan-file <FILE> --reviewed-by admin --approve` |

---

## 🔬 Docker 驗證範例

### 1. 變更計畫摘錄 (Change Plan)
預覽暫存變更包的意圖。
```bash
grafana-util change plan --desired-file ./desired.json --live-file ./live.json --output json
```
**輸出摘錄：**
```json
{
  "summary": { "would_create": 3, "would_update": 0, "would_delete": 0, "noop": 0 },
  "reviewRequired": true
}
```

### 2. 暫存狀態合約 (Staged Status)
在 CI 中使用此功能，根據本地檔案的整備度來控制部署門禁。
```bash
grafana-util status staged --desired-file ./desired.json --output json
```
**輸出摘錄：**
```json
{
  "overall": { "status": "blocked", "domainCount": 6, "blockedCount": 1, "blockerCount": 3 }
}
```
*註：'blocked' 狀態代表本地檔案尚未達到專案的整備標準。*

---

## ⚠️ 專案介面維運規則

1.  **區分介面用途**：人類審查使用 `overview`，機器合約或 CI 門禁使用 `status`。
2.  **暫存 vs 即時**：`status staged` 讀取本地檔案；`status live` 讀取 Grafana。切勿假設兩者代表相同的狀態。
3.  **審查鏈**：`change review` 對於追蹤「誰在變更包到達生產環境前核准了它」至關重要。
4.  **TUI 導覽**：對於複雜的資產審查，使用 `overview --output interactive` 來深入了解具體的 Blockers 或警告。

---

## ⏭️ 下一步
- 參考 [**Dashboard**](./dashboard.md) 或 [**Alert**](./alert.md) 手冊了解領域細節。
- 參閱 [**情境手冊**](./scenarios.md) 了解端到端範例。
