# Dashboard 維運人員手冊

本指南涵蓋 `grafana-util dashboard` 維運工作流，包含資產盤點、匯出 / 匯入、Drift 審查與 Dashboard 分析。

> **維運優先設計**：本工具將 Dashboard 視為版本控制資產。目標是安全地搬移與治理 Dashboard 狀態，在變更觸及即時環境前提供清晰的可視化預覽。

---

## 🛠️ 核心工作流用途

Dashboard 領域專為大規模治理而設計：
- **資產盤點**：了解跨一個或多個組織的 Dashboard 現況。
- **結構化匯出**：使用專屬「路徑 (Lanes)」在環境間遷移 Dashboard。
- **深度檢視**：離線分析查詢 (Queries) 與資料來源 (Datasource) 依賴。
- **差異審查 (Drift Review)**：在套用變更前，比對本地暫存檔案與 live Grafana。
- **受控變更**：透過強制性的 Dry-run 執行匯入或刪除。

---

## 🚧 工作流路徑邊界 (三條路徑)

Dashboard 匯出刻意產生三種不同的路徑，因為每一條路徑都對應不同的維運工作流。**這些路徑之間不可互換。**

| 路徑 (Lane) | 用途 | 最佳使用場景 |
| :--- | :--- | :--- |
| `raw/` | **標準回放 (Replay)** | `grafana-util import` 的主要來源。可還原且 API 友善。 |
| `prompt/` | **UI 匯入** | 與 Grafana UI 內建的 "Upload JSON" 功能相容。 |
| `provisioning/` | **檔案配置** | 供 Grafana 透過其內建配置系統從磁碟讀取 Dashboard。 |

---

## ⚖️ 暫存 vs 即時：維運邏輯

- **暫存工作 (Staged)**：本地匯出樹、驗證、離線檢視與 Dry-run 審查。
- **即時工作 (Live)**：直接對接 Grafana 的盤點、即時 Diff、匯入與刪除。

**黃金守則**：先用 `list` 或 `browse` 發現資產，`export` 到暫存目錄，透過 `inspect` 與 `diff` 驗證，最後在 Dry-run 符合預期後才執行 `import` 或 `delete`。

---

## 📋 閱讀即時資產盤點

使用 `dashboard list` 快速取得資產全貌。

```bash
grafana-util dashboard list \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --table
```

**驗證輸出摘錄：**
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
| **匯入 (Import)** | `grafana-util dashboard import --import-dir ./dashboards/raw --replace-existing --dry-run --table` |
| **比對 (Diff)** | `grafana-util dashboard diff --import-dir ./dashboards/raw --input-format raw` |
| **分析 (Inspect)** | `grafana-util dashboard inspect-export --import-dir ./dashboards/raw --output-format report-table` |
| **刪除 (Delete)** | `grafana-util dashboard delete --uid <UID> --url <URL> --basic-user admin --basic-password admin` |
| **變數檢視 (Vars)** | `grafana-util dashboard inspect-vars --uid <UID> --url <URL> --table` |
| **檔案修正 (Patch)** | `grafana-util dashboard patch-file --input <FILE> --title "New Title" --output <FILE>` |
| **發佈 (Publish)** | `grafana-util dashboard publish --input <FILE> --url <URL> --basic-user admin --basic-password admin` |
| **複製 (Clone)** | `grafana-util dashboard clone-live --uid <UID> --output <FILE> --url <URL>` |

---

## 🔬 Docker 驗證範例

### 1. 匯出進度 (Export Progress)
在大規模匯出時使用 `--progress` 以取得簡潔的日誌。
```bash
grafana-util dashboard export --export-dir ./dashboards --overwrite --progress
```
**輸出摘錄：**
```text
Exporting dashboard 1/7: mixed-query-smoke
Exporting dashboard 2/7: smoke-prom-only
...
Exporting dashboard 7/7: two-prom-query-smoke
```

### 2. Dry-Run 匯入預覽
在變更前務必確認目標動作。
```bash
grafana-util dashboard import --import-dir ./dashboards/raw --dry-run --table
```
**輸出摘錄：**
```text
UID                    DESTINATION  ACTION  FOLDER_PATH                    FILE
---------------------  -----------  ------  -----------------------------  --------------------------------------
mixed-query-smoke      exists       update  General                        ./dashboards/raw/Mixed_Query_Dashboard.json
subfolder-chain-smoke  missing      create  Platform / Team / Apps / Prod  ./dashboards/raw/Subfolder_Chain.json
```
- **ACTION=create**：將新增 Dashboard。
- **ACTION=update**：將取代現有的 live Dashboard。

### 3. Provisioning 比對
比對本地配置檔案與實例現況。
```bash
grafana-util dashboard diff --import-dir ./dashboards/provisioning --input-format provisioning
```
**輸出摘錄：**
```text
--- live/cpu-main
+++ export/cpu-main
-  "title": "CPU Overview"
+  "title": "CPU Overview v2"
```

---

## ⏭️ 下一步
- 了解 [**資料來源管理**](./datasource.md)。
- 探索 [**告警工作流**](./alert.md)。
