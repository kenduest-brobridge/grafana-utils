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

- 你知道目前這個任務應該走資產盤點、單一 dashboard authoring、export/import replay、inspect、topology 還是 screenshot。
- 你能在動 live state 前先說清楚自己走的是哪條 lane。
- 你能證明 dashboard 已經適合 replay 或 publish，而不是只是「看起來差不多」。

## 失敗時先檢查

- 如果匯出樹不完整，先修 source path，再談 replay。
- 如果 inspect 顯示查詢或變數缺失，先處理這些問題，再進入匯入流程。
- 如果你說不出 screenshot 或 topology 在證明什麼，可能是開錯工作流了。

## 草稿 authoring 工作流

當這次工作不是從整棵 export tree 開始，而是圍繞一份 dashboard 草稿反覆修改時，請直接走 authoring 這條路。

- 如果 Grafana 裡已經有最接近的來源，先用 `dashboard fetch-live` 或 `dashboard clone-live` 取回草稿。
- 如果你要一邊編修一邊在瀏覽器看本地草稿內容，先用 `dashboard serve` 開一個輕量 preview server。
- 在任何 mutation 前先跑 `dashboard review`，確認 title、UID、tags、folder UID 與阻擋性驗證問題。
- 要重寫本地草稿內容時，用 `dashboard patch-file` 原地修改或輸出成新檔。
- 如果你想從 live dashboard 開始直接編修，但又不想預設就回寫 Grafana，請用 `dashboard edit-live`；它會先給你 review 摘要和阻擋性驗證結果，再決定能不能回寫。
- 草稿準備好後，用 `dashboard publish` 走和正式 import 同一條 replay pipeline。

如果你的團隊是用 Jsonnet、grafanalib 或其他 generator 產 dashboard，不必每次都先落一個中繼暫存檔才能 review 或 publish。

```bash
# 用途：從標準輸入檢視一份生成儀表板。
jsonnet dashboards/cpu.jsonnet | grafana-util dashboard review --input - --output-format json
```

```bash
# 用途：從標準輸入發佈一份生成儀表板。
jsonnet dashboards/cpu.jsonnet | grafana-util dashboard publish --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --input - --replace-existing
```

如果你是在本地反覆編修同一份草稿，請改用檔案路徑搭配 `dashboard publish --watch`，不要用 `--input -`。watch 模式會在每次儲存穩定後重跑 publish 或 dry-run，而且就算其中一次驗證或 API 呼叫失敗，也會繼續監看後續變更。

```bash
# 用途：編修本地草稿時，每次儲存後自動重跑 dry-run publish。
grafana-util dashboard publish --url http://localhost:3000 --basic-user admin --basic-password admin --input ./drafts/cpu-main.json --dry-run --watch
```

```bash
# 用途：在本地瀏覽器裡持續檢視一份 dashboard 草稿。
grafana-util dashboard serve --input ./drafts/cpu-main.json --port 18080 --open-browser
```

```bash
# 用途：從 live dashboard 拉進 editor，但預設仍先落成新的本地草稿。
grafana-util dashboard edit-live --profile prod --dashboard-uid cpu-main --output ./drafts/cpu-main.edited.json
```

`dashboard patch-file --input -` 必須搭配 `--output`，因為標準輸入不能原地覆寫。
如果目標是 Grafana 內建的 General folder，`dashboard publish` 會把它正規化回預設 root publish 路徑，不會硬送出字面上的 `general` folder UID。
`dashboard serve` 的定位是輕量 preview 與文件檢視，不是把完整 Grafana renderer 內嵌到本地 server。

## 歷史與還原工作流

當您要找回一個已知可用的 dashboard 版本時，請直接走 history 這條路，不要手動重建 JSON。

- [dashboard history](../../commands/zh-TW/dashboard-history.md)
- `dashboard history list`：列出單一 dashboard UID 的最近版本歷史。
- `dashboard history restore`：把某個歷史版本複製成新的最新 Grafana revision。
- `dashboard history export`：把版本歷史匯出成可重用的成品，方便審查或 CI。

還原不是破壞性覆蓋。您選到的舊版本會繼續保留在 history 裡，而還原出來的副本會變成新的最新版本。

> **維運優先設計**：本工具把 Dashboard 視為版本控制資產。目標是讓搬移、比對與審查流程更安全，並在變更碰到即時環境前先看清楚會發生什麼事。

## 🔗 指令頁面

如果你現在要查的是指令細節，而不是工作流程章節，可以直接看下面這些指令頁：

- [dashboard 指令總覽](../../commands/zh-TW/dashboard.md)
- [dashboard browse](../../commands/zh-TW/dashboard-browse.md)
- [dashboard fetch-live](../../commands/zh-TW/dashboard-fetch-live.md)
- [dashboard clone-live](../../commands/zh-TW/dashboard-clone-live.md)
- [dashboard list](../../commands/zh-TW/dashboard-list.md)
- [dashboard export](../../commands/zh-TW/dashboard-export.md)
- [dashboard import](../../commands/zh-TW/dashboard-import.md)
- [migrate dashboard raw-to-prompt](../../commands/zh-TW/migrate-dashboard-raw-to-prompt.md)
- [dashboard patch-file](../../commands/zh-TW/dashboard-patch-file.md)
- [dashboard serve](../../commands/zh-TW/dashboard-serve.md)
- [dashboard edit-live](../../commands/zh-TW/dashboard-edit-live.md)
- [dashboard review](../../commands/zh-TW/dashboard-review.md)
- [dashboard publish](../../commands/zh-TW/dashboard-publish.md)
- [dashboard delete](../../commands/zh-TW/dashboard-delete.md)
- [dashboard diff](../../commands/zh-TW/dashboard-diff.md)
- [dashboard analyze（本地）](../../commands/zh-TW/dashboard-analyze-export.md)
- [dashboard analyze（即時）](../../commands/zh-TW/dashboard-analyze-live.md)
- [dashboard list-vars](../../commands/zh-TW/dashboard-list-vars.md)
- [dashboard history](../../commands/zh-TW/dashboard-history.md)
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

- `dashboard analyze（即時）`：直接分析 live dashboard 的結構、查詢與依賴。
- `dashboard analyze（本地）`：離線分析已匯出的 dashboard 樹。
- `dashboard list-vars`：確認變數、data source 選項與 URL 帶入值。
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
| `prompt/` | **UI 匯入** | 與 Grafana UI 內建的 "Upload JSON" 功能相容。若您手上只有一般或 raw 的 dashboard JSON，請先用 `grafana-util migrate dashboard raw-to-prompt` 轉換。 |
| `provisioning/` | **檔案配置** | 供 Grafana 透過其內建配置系統從磁碟讀取 Dashboard。 |

如果您對 `dashboard export` 加上 `--include-history`，匯出樹就會在每個 org 範圍下多出一個 `history/` 子目錄。當您使用 `--all-orgs` 時，每個匯出的 org root 都會各自有一份 history 樹。

當您需要的是單一 dashboard UID 的獨立 JSON 成品時，請直接用 `dashboard history export`。當您想讓版本歷史跟著 export tree 一起輸出時，請用 `dashboard export --include-history`。

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

**黃金守則**：先用 `list` 或 `browse` 發現資產，`export` 到暫存目錄，透過 `analyze` 與 `diff` 驗證，最後在 Dry-run 符合預期後才執行 `import` 或 `delete`。

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
| **匯出 (Export)** | `grafana-util dashboard export --output-dir ./dashboards --overwrite --progress` |
| **匯出 + 歷史** | `grafana-util dashboard export --output-dir ./dashboards --include-history --overwrite --progress` |
| **Raw 轉 Prompt** | `grafana-util migrate dashboard raw-to-prompt --input-dir ./dashboards/raw --output-dir ./dashboards/prompt --overwrite --progress` |
| **匯入 (Import)** | `grafana-util dashboard import --input-dir ./dashboards/raw --replace-existing --dry-run --table` |
| **比對 (Diff)** | `grafana-util dashboard diff --input-dir ./dashboards/raw --input-format raw` |
| **分析 (Analyze)** | `grafana-util dashboard analyze --input-dir ./dashboards/raw --input-format raw --output-format dependency` |
| **刪除 (Delete)** | `grafana-util dashboard delete --uid <UID> --url <URL> --basic-user admin --basic-password admin` |
| **變數檢視 (List Vars)** | `grafana-util dashboard list-vars --uid <UID> --url <URL> --table` |
| **檔案修正 (Patch)** | `grafana-util dashboard patch-file --input <FILE> --name "New Title" --output <FILE>` |
| **發佈 (Publish)** | `grafana-util dashboard publish --input <FILE> --url <URL> --basic-user admin --basic-password admin` |
| **複製 (Clone)** | `grafana-util dashboard clone-live --source-uid <UID> --output <FILE> --url <URL>` |

---

## 🔬 實作範例

### 1. 匯出進度 (Export Progress)
在大規模匯出時使用 `--progress` 以取得簡潔的日誌。
```bash
# 用途：在大規模匯出時使用 --progress 以取得簡潔的日誌。
grafana-util dashboard export --output-dir ./dashboards --overwrite --progress
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
grafana-util dashboard import --input-dir ./dashboards/raw --dry-run --table
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
grafana-util dashboard diff --input-dir ./dashboards/provisioning --input-format provisioning
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
