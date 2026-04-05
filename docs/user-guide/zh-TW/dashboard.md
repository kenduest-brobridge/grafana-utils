# Dashboard 維運人員手冊

這一章是給要管 dashboard 生命週期的人。重點不是只會匯出或匯入，而是先看懂現況、知道變更會影響什麼，再決定要不要回放或套用。

## 適用對象

- 需要盤點、搬移、審查或截圖 dashboard 的 SRE / 平台工程師
- 想把 dashboard 放進 Git、review 或 CI 流程的人
- 需要先看 live 現況，再決定要不要匯出、匯入或刪除的人

## 主要目標

- 先看懂 live dashboard 長什麼樣
- 再確認匯出樹、依賴與變數是不是合理
- 最後才進入匯入、發佈或刪除

## 採用前後對照

- 以前：dashboard 工作常常是從零散 UI 點選、脆弱 JSON 處理、或看不清楚的依賴開始。
- 現在：先盤點、再 inspect、再 diff，最後才 replay 或發佈，順序更清楚。

## 成功判準

- 你知道目前這個任務應該走 list、export、inspect、topology、screenshot 還是 import。
- 你能在動 live state 前先說清楚自己走的是哪條 lane。
- 你能證明 dashboard 已經適合 replay 或 publish，而不是只是「看起來差不多」。

## 失敗時先檢查

- 如果匯出樹不完整，先修 source path，再談 replay。
- 如果 inspect 顯示查詢或變數缺失，先處理這些問題，再進入匯入流程。
- 如果你說不出 screenshot 或 topology 在證明什麼，可能是開錯工作流了。

> **維運優先設計**：本工具把 Dashboard 視為版本控制資產。目標是讓搬移、比對與審查流程更安全，並在變更碰到即時環境前先看清楚會發生什麼事。

## 🔗 指令頁面

如果你現在要查的是指令細節，而不是工作流程章節，可以直接看下面這些指令頁：

- [dashboard 指令總覽](../../commands/zh-TW/dashboard.md)
- [dashboard browse](../../commands/zh-TW/dashboard-browse.md)
- [dashboard get](../../commands/zh-TW/dashboard-get.md)
- [dashboard clone-live](../../commands/zh-TW/dashboard-clone-live.md)
- [dashboard list](../../commands/zh-TW/dashboard-list.md)
- [dashboard export](../../commands/zh-TW/dashboard-export.md)
- [dashboard import](../../commands/zh-TW/dashboard-import.md)
- [dashboard raw-to-prompt](../../commands/zh-TW/dashboard-raw-to-prompt.md)
- [dashboard patch-file](../../commands/zh-TW/dashboard-patch-file.md)
- [dashboard review](../../commands/zh-TW/dashboard-review.md)
- [dashboard publish](../../commands/zh-TW/dashboard-publish.md)
- [dashboard delete](../../commands/zh-TW/dashboard-delete.md)
- [dashboard diff](../../commands/zh-TW/dashboard-diff.md)
- [dashboard inspect-export](../../commands/zh-TW/dashboard-inspect-export.md)
- [dashboard inspect-live](../../commands/zh-TW/dashboard-inspect-live.md)
- [dashboard inspect-vars](../../commands/zh-TW/dashboard-inspect-vars.md)
- [dashboard governance-gate](../../commands/zh-TW/dashboard-governance-gate.md)
- [dashboard topology](../../commands/zh-TW/dashboard-topology.md)
- [dashboard screenshot](../../commands/zh-TW/dashboard-screenshot.md)
- [指令詳細說明總索引](../../commands/zh-TW/index.md)

---

## 🛠️ 核心工作流用途

Dashboard 相關功能主要是為了處理大規模維運而設計：
- **資產盤點**：了解跨一個或多個組織的 Dashboard 現況。
- **結構化匯出**：用分開的資料路徑在環境間遷移 Dashboard。
- **深度檢視**：離線分析查詢 (Queries) 與 data source 依賴。
- **截圖與視覺檢查**：產出可重現的 dashboard 或 panel 截圖，用於文件、事件處理紀錄與除錯。
- **差異審查 (Drift Review)**：在套用變更前，比對本地暫存檔案與 live Grafana。
- **受控變更**：透過強制性的 Dry-run 執行匯入或刪除。

---

## 🔎 檢視與截圖工作流

如果你的目標不是匯入或匯出，而是先看清楚 dashboard 目前長什麼樣、依賴哪些 data source、變數怎麼帶入，這一組指令應該先看。

- `dashboard inspect-live`：直接看 live dashboard 的結構、查詢與依賴。
- `dashboard inspect-export`：離線檢查已匯出的 dashboard 檔案。
- `dashboard inspect-vars`：確認變數、data source 選項與 URL 帶入值。
- `dashboard screenshot`：用 headless browser 產生可重現的 dashboard 或 panel 截圖。
- `dashboard topology`：快速掌握 dashboard 與上游依賴之間的關係。

常見情境：

- 事件處理後需要補一張當下畫面的截圖
- 想先確認某個 panel 是不是吃到正確的變數與 data source
- 要整理文件或 review 附圖，但不想手動截圖
- 想在變更前先看 dashboard 依賴與查詢結構

---

## 🚧 工作流程邊界（三條資料路徑）

Dashboard 匯出會刻意產生三種不同的資料路徑，因為每一條都對應不同的維運流程。**這些路徑不能互換使用。**

| 路徑 (Lane) | 用途 | 最佳使用場景 |
| :--- | :--- | :--- |
| `raw/` | **標準回放 (Replay)** | `grafana-util dashboard import` 的主要來源。可還原且 API 友善。 |
| `prompt/` | **UI 匯入** | 與 Grafana UI 內建的 "Upload JSON" 功能相容。若您手上只有一般或 raw 的 dashboard JSON，請先用 `grafana-util dashboard raw-to-prompt` 轉換。 |
| `provisioning/` | **檔案配置** | 供 Grafana 透過其內建配置系統從磁碟讀取 Dashboard。 |

---

## 🔤 Prompt 與變數說明

- `$datasource` 是 dashboard variable 參照。
- `${DS_*}` 是由 `__inputs` 產生的 external import placeholder。
- 一份 prompt dashboard 同時出現這兩種寫法是合理的。
- 這通常代表 dashboard 一邊保留 Grafana datasource variable 的工作流，一邊也需要 external import input。
- 不要把 `$datasource` 直接解讀成 mixed datasource family；很多情況只是 panel 仍透過同一個 datasource variable 做選擇。

---

## ⚖️ 暫存 vs 即時：維運邏輯

- **暫存工作 (Staged)**：本地匯出樹、驗證、離線檢視與 Dry-run 審查。
- **即時工作 (Live)**：直接對接 Grafana 的盤點、即時 Diff、匯入與刪除。

**黃金守則**：先用 `list` 或 `browse` 發現資產，`export` 到暫存目錄，透過 `inspect` 與 `diff` 驗證，最後在 Dry-run 符合預期後才執行 `import` 或 `delete`。

---

## 📋 閱讀即時資產盤點

使用 `dashboard list` 快速取得資產全貌。

```bash
# 用途：使用 dashboard list 快速取得資產全貌。
grafana-util dashboard list \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --table
```

**範例輸出：**
```text
UID                      NAME                                      FOLDER  FOLDER_UID      FOLDER_PATH  ORG        ORG_ID
-----------------------  ----------------------------------------  ------  --------------  -----------  ---------  ------
rYdddlPWl                Node Exporter Full for Host               Demo    ffhrmit0usjk0b  Demo         Main Org.  1
spring-jmx-node-unified  Spring JMX + Node Unified Dashboard (VM)  Demo    ffhrmit0usjk0b  Demo         Main Org.  1
```

**如何解讀：**
- **UID**：用於自動化與刪除的穩定身份識別。
- **FOLDER_PATH**：Dashboard 所屬的目錄路徑。
- **ORG/ORG_ID**：確認該物件隸屬於哪個組織。

---

## 🚀 關鍵指令 (完整參數參考)

| 指令 | 帶有參數的完整範例 |
| :--- | :--- |
| **盤點 (List)** | `grafana-util dashboard list --all-orgs --with-sources --table` |
| **匯出 (Export)** | `grafana-util dashboard export --export-dir ./dashboards --overwrite --progress` |
| **Raw 轉 Prompt** | `grafana-util dashboard raw-to-prompt --input-dir ./dashboards/raw --output-dir ./dashboards/prompt --overwrite --progress` |
| **匯入 (Import)** | `grafana-util dashboard import --import-dir ./dashboards/raw --replace-existing --dry-run --table` |
| **比對 (Diff)** | `grafana-util dashboard diff --import-dir ./dashboards/raw --input-format raw` |
| **分析 (Inspect)** | `grafana-util dashboard inspect-export --import-dir ./dashboards/raw --output-format report-table` |
| **刪除 (Delete)** | `grafana-util dashboard delete --uid <UID> --url <URL> --basic-user admin --basic-password admin` |
| **變數檢視 (Vars)** | `grafana-util dashboard inspect-vars --uid <UID> --url <URL> --table` |
| **檔案修正 (Patch)** | `grafana-util dashboard patch-file --input <FILE> --title "New Title" --output <FILE>` |
| **發佈 (Publish)** | `grafana-util dashboard publish --input <FILE> --url <URL> --basic-user admin --basic-password admin` |
| **複製 (Clone)** | `grafana-util dashboard clone-live --uid <UID> --output <FILE> --url <URL>` |

---

## 🔬 實作範例

### 1. 匯出進度 (Export Progress)
在大規模匯出時使用 `--progress` 以取得簡潔的日誌。
```bash
# 用途：在大規模匯出時使用 --progress 以取得簡潔的日誌。
grafana-util dashboard export --export-dir ./dashboards --overwrite --progress
```
**範例輸出：**
```text
Exporting dashboard 1/7: mixed-query-smoke
Exporting dashboard 2/7: smoke-prom-only
...
Exporting dashboard 7/7: two-prom-query-smoke
```

### 2. Dry-Run 匯入預覽
在變更前務必確認目標動作。
```bash
# 用途：在變更前務必確認目標動作。
grafana-util dashboard import --import-dir ./dashboards/raw --dry-run --table
```
**範例輸出：**
```text
UID                    DESTINATION  ACTION  FOLDER_PATH                    FILE
---------------------  -----------  ------  -----------------------------  --------------------------------------
mixed-query-smoke      exists       update  General                        ./dashboards/raw/Mixed_Query_Dashboard.json
subfolder-chain-smoke  missing      create  Platform / Team / Apps / Prod  ./dashboards/raw/Subfolder_Chain.json
```
- **ACTION=create**：將新增 Dashboard。
- **ACTION=update**：將取代現有的 live Dashboard。
- **DESTINATION=missing**：Grafana 目前沒有這個 UID，因此匯入時會建立新紀錄。
- **DESTINATION=exists**：Grafana 目前已經有這個 UID，因此匯入時會對應到既有 Dashboard。

### 3. Provisioning 比對
比對本地配置檔案與實例現況。
```bash
# 用途：比對本地配置檔案與實例現況。
grafana-util dashboard diff --import-dir ./dashboards/provisioning --input-format provisioning
```
**範例輸出：**
```text
--- live/cpu-main
+++ export/cpu-main
-  "title": "CPU Overview"
+  "title": "CPU Overview v2"
```

---
[⬅️ 上一章：系統架構與設計原則](architecture.md) | [🏠 回首頁](index.md) | [➡️ 下一章：Data source 管理](datasource.md)
