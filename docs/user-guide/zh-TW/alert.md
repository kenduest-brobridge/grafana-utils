# Alert 維運人員手冊

這一章整理告警的設計、審查與套用流程。它的重點不是只會建立 rule，而是先把 route、contact point、template 與 plan/apply 的關係說清楚。

## 適用對象

- 負責 Grafana Alerting 設定的人
- 需要先審查告警變更，再套用到 live 環境的人
- 要把 alert 流程接到 Git、review 或 CI 的維運人員

## 主要目標

- 先在本地整理 Desired State
- 再做 plan / review / apply
- 需要回放或遷移時，再走 export / import / diff

## 採用前後對照

- 以前：告警變更常混在 UI 與 YAML 的混合路徑裡，審查脈絡不夠清楚。
- 現在：編寫 desired state、看 plan、真正 apply 分成不同關卡，證據也更明確。

## 成功判準

- 你能分清楚自己是在改 desired state、看 plan，還是要真的套用變更。
- 你能在動 live state 前說清楚這次會影響 alert chain 的哪一段。
- 你看得懂輸出，也能判斷 plan 是否可以繼續往下走。

## 失敗時先檢查

- 如果 plan 輸出少了你預期的 contact point 或 route，先檢查 staged input。
- 如果 apply 會碰到比你預期更多的東西，先把它當成審查失敗，而不是 renderer 問題。
- 如果你還說不出自己在 alert 哪條 lane，先回去看工作流章節，不要直接改 live。

> **維運原則**：透過 **計畫 (Plan) -> 審查 (Review) -> 套用 (Apply)** 週期來謹慎變更告警，防止即時環境發生意外。

## 🔗 指令頁面

如果你現在要查的是指令細節，而不是工作流程章節，可以直接看下面這些指令頁：

- [alert 指令總覽](../../commands/zh-TW/alert.md)
- [alert export](../../commands/zh-TW/alert-export.md)
- [alert import](../../commands/zh-TW/alert-import.md)
- [alert diff](../../commands/zh-TW/alert-diff.md)
- [alert plan](../../commands/zh-TW/alert-plan.md)
- [alert apply](../../commands/zh-TW/alert-apply.md)
- [alert delete](../../commands/zh-TW/alert-delete.md)
- [alert add-rule](../../commands/zh-TW/alert-add-rule.md)
- [alert clone-rule](../../commands/zh-TW/alert-clone-rule.md)
- [alert add-contact-point](../../commands/zh-TW/alert-add-contact-point.md)
- [alert set-route](../../commands/zh-TW/alert-set-route.md)
- [alert preview-route](../../commands/zh-TW/alert-preview-route.md)
- [alert new-rule](../../commands/zh-TW/alert-new-rule.md)
- [alert new-contact-point](../../commands/zh-TW/alert-new-contact-point.md)
- [alert new-template](../../commands/zh-TW/alert-new-template.md)
- [alert list-rules](../../commands/zh-TW/alert-list-rules.md)
- [alert list-contact-points](../../commands/zh-TW/alert-list-contact-points.md)
- [alert list-mute-timings](../../commands/zh-TW/alert-list-mute-timings.md)
- [alert list-templates](../../commands/zh-TW/alert-list-templates.md)
- [指令詳細說明總索引](../../commands/zh-TW/index.md)

---

## 🛠️ 核心工作流用途

告警相關功能主要是為了這幾種場景設計：
- **Desired State**：在不觸碰即時 Grafana 的情況下，於本地建立告警配置。
- **審查差異**：在核准變更前，比對 Desired State 與現有資產。
- **受控套用**：僅執行已通過審查的計畫。
- **遷移與回放**：使用傳統 `raw/` 路徑進行資產快照與環境遷移。

---

## 🚧 工作流程邊界（兩條資料路徑）

告警管理拆成兩條獨立的維運流程。**請不要混用這兩條路徑。**

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
  --desired-dir ./alerts/desired --prune --output-format json
```

**如何解讀計畫輸出：**
- **create**：Desired 資源在即時 Grafana 中缺失。
- **update**：即時 Grafana 與您的 Desired 檔案存在差異。
- **delete**：當啟動 `--prune` 且即時資源不在您的檔案中時觸發。

**驗證套用步驟：**
僅在計畫審查完成並保存後執行。
```bash
# 用途：僅在計畫審查完成並保存後執行。
grafana-util alert apply \
  --plan-file ./alert-plan-reviewed.json \
  --approve --output-format json
```

---

## 🚀 關鍵指令 (完整參數參考)

| 指令 | 帶有參數的完整範例 |
| :--- | :--- |
| **列出規則 (List)** | `grafana-util alert list-rules --all-orgs --table` |
| **匯出 (Export)** | `grafana-util alert export --export-dir ./alerts --overwrite` |
| **計畫 (Plan)** | `grafana-util alert plan --desired-dir ./alerts/desired --prune --output-format json` |
| **套用 (Apply)** | `grafana-util alert apply --plan-file ./plan.json --approve` |
| **設定路由 (Set Route)** | `grafana-util alert set-route --desired-dir ./alerts/desired --receiver pagerduty` |
| **新增規則 (New)** | `grafana-util alert new-rule --name <NAME> --folder <FOLDER> --output <FILE>` |
| **新增聯絡點 (New)** | `grafana-util alert new-contact-point --name <NAME> --type <TYPE> --output <FILE>` |
| **新增範本 (New)** | `grafana-util alert new-template --name <NAME> --template <CONTENT> --output <FILE>` |

---

## 🔬 實作範例

### 1. 告警計畫摘錄
```bash
# 用途：1. 告警計畫摘錄。
grafana-util alert plan --desired-dir ./alerts/desired --prune --output-format json
```
**範例輸出：**
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
# 用途：在套用前於本地驗證路由邏輯。
grafana-util alert preview-route --desired-dir ./alerts/desired --label team=platform --severity critical
```
**範例輸出：**
```json
{
  "input": { "labels": { "team": "platform" }, "severity": "critical" },
  "matches": []
}
```
*註：空白的 match list 代表合約驗證成功，不一定代表存在即時告警實例。*

---
[⬅️ 上一章：Data source 管理](datasource.md) | [🏠 回首頁](index.md) | [➡️ 下一章：Access 管理](access.md)
