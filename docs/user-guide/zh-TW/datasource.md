# Datasource 維運人員手冊

本指南涵蓋 `grafana-util datasource` 維運工作流，包含資產盤點、災難恢復、回放與受控的即時變更。

> **維運目標**：確保資料來源配置可以被安全地備份、比對與回放，並透過 **Masked Recovery** 合約保護敏感憑證。

---

## 🛠️ 核心工作流用途

資料來源領域專為下列場景設計：
- **資產盤點**：稽核現有的資料來源、其類型以及後端 URL。
- **恢復與回放**：維護可供災難恢復的資料來源匯出紀錄。
- **Provisioning 投影**：產生 Grafana 檔案式配置系統所需的 YAML 檔案。
- **差異審查 (Drift Review)**：在套用變更前，比對本地暫存檔案與 live Grafana。
- **受控變更**：在 Dry-run 保護下新增、修改或刪除 live 資料來源。

---

## 🚧 工作流路徑邊界

資料來源匯出會產生兩個主要的 Artifact，各自負責不同的工作：

| 檔案 (Artifact) | 用途 | 最佳使用場景 |
| :--- | :--- | :--- |
| `datasources.json` | **Masked Recovery** | 標準回放合約。用於還原、回放與差異比對。 |
| `provisioning/datasources.yaml` | **Provisioning 投影** | 模擬 Grafana 檔案配置系統所需的磁碟結構。 |

**重要提示**：請始終將 `datasources.json` 視為「權威恢復來源」。Provisioning YAML 僅是從恢復包中衍生的次要投影。

---

## 📋 閱讀即時資產盤點

使用 `datasource list` 驗證目前 Grafana 的外掛與目標狀態。

```bash
grafana-util datasource list \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --table
```

**驗證輸出摘錄：**
```text
UID             NAME        TYPE        URL                     IS_DEFAULT  ORG  ORG_ID
--------------  ----------  ----------  ----------------------  ----------  ---  ------
dehk4kxat5la8b  Prometheus  prometheus  http://prometheus:9090  true             1
```

**如何解讀：**
- **UID**：用於自動化的穩定身份識別。
- **TYPE**：識別外掛實作 (例如 prometheus, loki)。
- **IS_DEFAULT**：標示這是否為該組織的預設資料來源。
- **URL**：該紀錄關聯的後端目標位址。

---

## 🚀 關鍵指令 (完整參數參考)

| 指令 | 帶有參數的完整範例 |
| :--- | :--- |
| **盤點 (List)** | `grafana-util datasource list --all-orgs --table` |
| **匯出 (Export)** | `grafana-util datasource export --export-dir ./datasources --overwrite` |
| **匯入 (Import)** | `grafana-util datasource import --import-dir ./datasources --replace-existing --dry-run --table` |
| **比對 (Diff)** | `grafana-util datasource diff --import-dir ./datasources` |
| **新增 (Add)** | `grafana-util datasource add --uid <UID> --name <NAME> --type prometheus --datasource-url <URL> --dry-run --table` |

---

## 🔬 Docker 驗證範例

### 1. 匯出盤點資產
```bash
grafana-util datasource export --export-dir ./datasources --overwrite
```
**輸出摘錄：**
```text
Exported datasource inventory -> datasources/datasources.json
Exported metadata            -> datasources/export-metadata.json
Datasource export completed: 3 item(s)
```

### 2. Dry-Run 匯入預覽
```bash
grafana-util datasource import --import-dir ./datasources --replace-existing --dry-run --table
```
**輸出摘錄：**
```text
UID         NAME               TYPE         ACTION   DESTINATION
prom-main   prometheus-main    prometheus   update   existing
loki-prod   loki-prod          loki         create   missing
```
- **ACTION=create**：將建立新的資料來源紀錄。
- **ACTION=update**：將取代現有的紀錄。

### 3. 直接即時新增 (Dry-Run)
```bash
grafana-util datasource add \
  --uid prom-main --name prom-new --type prometheus \
  --datasource-url http://prometheus:9090 --dry-run --table
```
**輸出摘錄：**
```text
INDEX  NAME       TYPE         ACTION  DETAIL
1      prom-new   prometheus   create  would create datasource uid=prom-main
```

---

## ⏭️ 下一步
- 了解 [**Dashboard 資產管理**](./dashboard.md)。
- 探索 [**告警工作流**](./alert.md)。
