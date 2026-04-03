# Alert 維運人員手冊

本指南涵蓋 `grafana-util alert` 維運工作流，包含告警 Desired-State 編寫、審查優先變更與遷移式回放流。

> **維運原則**：透過 **計畫 (Plan) -> 審查 (Review) -> 套用 (Apply)** 週期來謹慎變更告警，防止即時環境發生意外。

## 🔗 逐指令頁面

如果您現在想看的是逐指令說明，而不是工作流章節，請直接使用逐指令頁面：

- [alert 指令總覽](../../commands/zh-TW/alert.md)
- [alert plan](../../commands/zh-TW/alert-plan.md)
- [alert apply](../../commands/zh-TW/alert-apply.md)
- [alert export](../../commands/zh-TW/alert-export.md)
- [alert preview-route](../../commands/zh-TW/alert-preview-route.md)
- [逐指令總索引](../../commands/zh-TW/index.md)

---

## 🛠️ 核心工作流用途

告警領域專為下列場景設計：
- **Desired State**：在不觸碰即時 Grafana 的情況下，於本地建立告警配置。
- **審查差異**：在核准變更前，比對 Desired State 與現有資產。
- **受控套用**：僅執行已通過審查的計畫。
- **遷移與回放**：使用傳統 `raw/` 路徑進行資產快照與環境遷移。

---

## 🚧 工作流路徑邊界 (兩條路徑)

告警管理拆分為兩條獨立的維運路徑。**請勿混用這些路徑。**

| 路徑 (Lane) | 用途 | 常用指令 |
| :--- | :--- | :--- |
| **編寫路徑 (Authoring)** | 供審查 / 套用的 Desired-State 檔案。 | `init`, `add-rule`, `add-contact-point`, `plan`, `apply` |
| **遷移路徑 (Migration)** | 資產快照與原始回放。 | `export`, `import`, `diff`, `list-rules` |

---

## 📋 編寫 Desired State

從建置 Desired-State 樹狀結構開始。這會建立代表您「變更意圖」的本地檔案。

```bash
# 初始化 Desired-State 目錄
grafana-util alert init --desired-dir ./alerts/desired

# 新增規則到本地檔案 (尚未觸及 Grafana)
grafana-util alert add-rule \
  --desired-dir ./alerts/desired \
  --name cpu-high --folder platform-alerts \
  --receiver pagerduty-primary --threshold 80 --above --for 5m
```

---

## 🔬 審查與套用 (審查週期)

使用 `plan` 來建立本地檔案與即時 Grafana 之間的差異預覽。

```bash
# 產生供審查的計畫
grafana-util alert plan \
  --url http://localhost:3000 \
  --basic-user admin --basic-password admin \
  --desired-dir ./alerts/desired --prune --output json
```

**如何解讀計畫輸出：**
- **create**：Desired 資源在即時 Grafana 中缺失。
- **update**：即時 Grafana 與您的 Desired 檔案存在差異。
- **delete**：當啟動 `--prune` 且即時資源不在您的檔案中時觸發。

**驗證套用步驟：**
僅在計畫審查完成並保存後執行。
```bash
grafana-util alert apply \
  --plan-file ./alert-plan-reviewed.json \
  --approve --output json
```

---

## 🚀 關鍵指令 (完整參數參考)

| 指令 | 帶有參數的完整範例 |
| :--- | :--- |
| **列出規則 (List)** | `grafana-util alert list-rules --all-orgs --table` |
| **匯出 (Export)** | `grafana-util alert export --export-dir ./alerts --overwrite` |
| **計畫 (Plan)** | `grafana-util alert plan --desired-dir ./alerts/desired --prune --output json` |
| **套用 (Apply)** | `grafana-util alert apply --plan-file ./plan.json --approve` |
| **設定路由 (Set Route)** | `grafana-util alert set-route --desired-dir ./alerts/desired --receiver pagerduty` |
| **新增規則 (New)** | `grafana-util alert new-rule --name <NAME> --folder <FOLDER> --output <FILE>` |
| **新增聯絡點 (New)** | `grafana-util alert new-contact-point --name <NAME> --type <TYPE> --output <FILE>` |
| **新增範本 (New)** | `grafana-util alert new-template --name <NAME> --template <CONTENT> --output <FILE>` |

---

## 🔬 Docker 驗證範例

### 1. 告警計畫摘錄
```bash
grafana-util alert plan --desired-dir ./alerts/desired --prune --output json
```
**輸出摘錄：**
```json
{
  "summary": {
    "create": 1,
    "update": 2,
    "delete": 1,
    "noop": 0,
    "blocked": 0
  }
}
```

### 2. 路由預覽
在套用前於本地驗證路由邏輯。
```bash
grafana-util alert preview-route --desired-dir ./alerts/desired --label team=platform --severity critical
```
**輸出摘錄：**
```json
{
  "input": { "labels": { "team": "platform" }, "severity": "critical" },
  "matches": []
}
```
*註：空白的 match list 代表合約驗證成功，不一定代表存在即時告警實例。*

---
[⬅️ 上一章：Datasource 管理](datasource.md) | [🏠 回首頁](index.md) | [➡️ 下一章：Access 管理](access.md)
