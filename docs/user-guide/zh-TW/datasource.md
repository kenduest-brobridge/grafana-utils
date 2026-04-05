# Data source 維運手冊

這一章整理 data source 的盤點、備份、回放與受控變更。重點是讓你知道哪些資料適合進 Git，哪些資料只適合在恢復時補回。

## 適用對象

- 負責 data source 盤點、同步與恢復的人
- 需要把 data source 資產接進 Git 或 provisioning 流程的人
- 需要先做 dry-run，再決定要不要套用的人

## 主要目標

- 先看懂 live data source 長什麼樣
- 再建立可回放的輸出樹
- 最後才進行匯入、修改或刪除

## 採用前後對照

- 以前：data source 變更常被當成單次設定修改，回復方式與還原層級不夠清楚。
- 現在：先盤點、再做遮蔽式還原、再投影到 provisioning，最後才做 dry-run 或實際變更。

## 成功判準

- 你知道哪些欄位要放進恢復包，哪些欄位一定要遮蔽。
- 你能先確認 live inventory，再決定要不要動它。
- 你能分清楚自己是在處理 recovery、provisioning，還是直接 live mutation。

## 失敗時先檢查

- 如果回放包裡出現明文 secret，先停下來修匯出路徑，不要先存檔。
- 如果匯入預覽跟你以為的 live data source 對不上，先檢查 UID 與 type mapping。
- 如果 provisioning 投影跟恢復包不一致，先確認你到底要的是哪一條 lane。

> **維運目標**：確保 data source 設定可以安全地備份、比對與回放，並透過 **Masked Recovery（遮蔽式還原）** 保護敏感憑證。

## 🔗 指令頁面

如果你現在要查的是指令細節，而不是工作流程章節，可以直接看下面這些指令頁：

- [datasource 指令總覽](../../commands/zh-TW/datasource.md)
- [datasource types](../../commands/zh-TW/datasource-types.md)
- [datasource browse](../../commands/zh-TW/datasource-browse.md)
- [datasource inspect-export](../../commands/zh-TW/datasource-inspect-export.md)
- [datasource export](../../commands/zh-TW/datasource-export.md)
- [datasource import](../../commands/zh-TW/datasource-import.md)
- [datasource diff](../../commands/zh-TW/datasource-diff.md)
- [datasource list](../../commands/zh-TW/datasource-list.md)
- [datasource add](../../commands/zh-TW/datasource-add.md)
- [datasource modify](../../commands/zh-TW/datasource-modify.md)
- [datasource delete](../../commands/zh-TW/datasource-delete.md)
- [指令詳細說明總索引](../../commands/zh-TW/index.md)

---

## 🛠️ 核心工作流用途

data source 這組功能主要是為了這幾種場景設計：
- **資產盤點**：稽核現有的 data source、其類型以及後端 URL。
- **恢復與回放**：維護可供災難恢復的 data source 匯出紀錄。
- **Provisioning 投影**：產生 Grafana 檔案式配置系統所需的 YAML 檔案。
- **差異審查 (Drift Review)**：在套用變更前，比對本地暫存檔案與 live Grafana。
- **受控變更**：在 Dry-run 保護下新增、修改或刪除 live 的 data source。

---

## 🚧 工作流程邊界

data source 匯出會產生兩個主要輸出物，各自負責不同的用途：

| 檔案 | 用途 | 最佳使用場景 |
| :--- | :--- | :--- |
| `datasources.json` | **Masked Recovery（遮蔽式還原）** | 標準回放合約。用於還原、回放與差異比對。 |
| `provisioning/datasources.yaml` | **Provisioning 投影** | 模擬 Grafana 檔案配置系統所需的磁碟結構。 |

**重要提示**：請始終把 `datasources.json` 視為真正的恢復來源。Provisioning YAML 只是從恢復包衍生出來的次要投影。

---

## 📋 閱讀即時資產盤點

使用 `datasource list` 驗證目前 Grafana 的外掛與目標狀態。

```bash
# 用途：使用 datasource list 驗證目前 Grafana 的外掛與目標狀態。
grafana-util datasource list \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --table
```

**範例輸出：**
```text
UID             NAME        TYPE        URL                     IS_DEFAULT  ORG  ORG_ID
--------------  ----------  ----------  ----------------------  ----------  ---  ------
dehk4kxat5la8b  Prometheus  prometheus  http://prometheus:9090  true             1
```

**如何解讀：**
- **UID**：用於自動化的穩定身份識別。
- **TYPE**：識別外掛實作 (例如 prometheus, loki)。
- **IS_DEFAULT**：標示這是否為該 org 的預設 data source。
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

## 🔬 實作範例

### 1. 匯出盤點資產
```bash
# 用途：1. 匯出盤點資產。
grafana-util datasource export --export-dir ./datasources --overwrite
```
**範例輸出：**
```text
Exported datasource inventory -> datasources/datasources.json
Exported metadata            -> datasources/export-metadata.json
Datasource export completed: 3 item(s)
```

### 2. Dry-Run 匯入預覽
```bash
# 用途：2. Dry-Run 匯入預覽。
grafana-util datasource import --import-dir ./datasources --replace-existing --dry-run --table
```
**範例輸出：**
```text
UID         NAME               TYPE         ACTION   DESTINATION
prom-main   prometheus-main    prometheus   update   existing
loki-prod   loki-prod          loki         create   missing
```
- **ACTION=create**：將建立新的 data source 紀錄。
- **ACTION=update**：將取代現有的紀錄。
- **DESTINATION=missing**：Grafana 目前沒有這個 UID，因此匯入時會建立新紀錄。
- **DESTINATION=existing**：Grafana 目前已經有這個 UID，因此匯入時會覆蓋既有 data source 紀錄。

### 3. 直接即時新增 (Dry-Run)
```bash
# 用途：3. 直接即時新增 (Dry-Run)。
grafana-util datasource add \
  --uid prom-main --name prom-new --type prometheus \
  --datasource-url http://prometheus:9090 --dry-run --table
```
**範例輸出：**
```text
INDEX  NAME       TYPE         ACTION  DETAIL
1      prom-new   prometheus   create  would create datasource uid=prom-main
```

---
[⬅️ 上一章：Dashboard 管理](dashboard.md) | [🏠 回首頁](index.md) | [➡️ 下一章：告警治理](alert.md)
