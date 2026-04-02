# 維運情境手冊 (Operator Scenarios)

本章將離散的指令家族轉換為端對端的工作流。當您遇到具體問題、且需要一份導覽圖而非單一指令說明時，請參閱本章。

---

## 📖 情境地圖 (Scenario Map)

| 情境 | 任務目標 | 關鍵指令介面 |
| :--- | :--- | :--- |
| **[1. 變更前環境驗證](#1-變更前環境驗證)** | 在正式變更前，證明連線正確且環境健康。 | `status`, `profile` |
| **[2. 全資產稽核](#2-全資產稽核)** | 產生資產清單、整備度與健康度報告。 | `dashboard`, `datasource`, `overview` |
| **[3. 可靠的備份](#3-可靠的備份)** | 匯出資產以供版本控制或災難恢復。 | `dashboard export` |
| **[4. 受控的恢復](#4-受控的恢復)** | 安全地將備份回放到即時 Grafana 實例。 | `dashboard import` |
| **[5. 告警治理流程](#5-告警治理流程)** | 透過「計畫 / 套用」週期管理告警變更。 | `alert` |
| **[6. 身份快照回放](#6-身份快照回放)** | 管理組織、使用者與團隊的身份狀態。 | `access` |
| **[7. 暫存發佈流程](#7-暫存發佈流程)** | 處理跨域的大型變更包。 | `change`, `status` |

---

## 1. 變更前環境驗證

在執行任何變更 (Mutation) 前，請先證明 CLI 指向正確的目標。

**工作流：**
1. 驗證 binary 版本。
2. 載入 Profile 或手動指定連線。
3. 執行一個安全的唯讀健康檢查。

```bash
grafana-util --version
grafana-util profile list
grafana-util status live --profile prod --output-format table
```

**如何解讀狀態：**
- **Overall: status=ready**：您的連線與專案健康度處於最佳狀態。
- **Overall: status=blocked**：存在關鍵錯誤 (Blockers)，這會阻擋安全的維運作業。

---

## 2. 全資產稽核

最適合用於上線前稽核、安全性檢查或變更前快照。

**工作流：**
1. 列出所有 Dashboard 與其資料來源依賴。
2. 總結整個專案的整備度。

```bash
# 盤點跨組織的所有 Dashboard 與資料來源
grafana-util dashboard list --profile prod --all-orgs --with-sources --table

# 全專案的高階健康度快照
grafana-util overview live --profile prod
```

---

## 3. 可靠的備份 (Dashboard 匯出)

將 live Dashboard 匯出為可持久化的結構化目錄，以便進入版本控制。

```bash
grafana-util dashboard export \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --export-dir ./backups \
  --overwrite --progress
```

**檢查重點：**
- `raw/`：您的主要備份來源，供日後還原使用。
- `export-metadata.json`：總結包含哪些組織與 Dashboard。

---

## 4. 受控的恢復 (Dashboard 匯入)

將備份重新回放到 live Grafana。**請務必先執行 Dry-Run。**

```bash
grafana-util dashboard import \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --import-dir ./backups/raw \
  --replace-existing \
  --dry-run \
  --table
```

**如何解讀匯入預覽：**
- **ACTION=create**：找不到該 UID 的現有 Dashboard；將會新增。
- **ACTION=update**：該 UID 的 Dashboard 已存在；將會被覆蓋。
- **DESTINATION=exists**：確認目標 UID 確實已存在於 Grafana 中。

---

## 5. 告警治理流程 (計畫 / 套用)

告警變更應遵循受審查的生命週期。

**工作流：**
1. 編寫：在 Desired 目錄中編輯檔案或使用 `alert add-rule`。
2. 審查：執行 `alert plan` 產生差異報告。
3. 執行：執行 `alert apply` 提交已審查的變更。

```bash
# 產生變更計畫
grafana-util alert plan --profile prod --desired-dir ./alerts/desired --prune --output json

# 審查後才執行套用
grafana-util alert apply --profile prod --plan-file ./reviewed-plan.json --approve
```

---

## 6. 身份快照回放 (Access 管理)

透過快照管理組織、帳號、團隊或服務帳號。

```bash
# 稽核服務帳號及其權杖
grafana-util access service-account list --profile prod --table

# 回放使用者角色與組織成員資格
grafana-util access user import --import-dir ./access-users --replace-existing --dry-run
```

---

## 7. 暫存發佈流程 (Change 管理)

處理包含 Dashboard、Datasource 與 Alert 的整套跨域變更包。

**工作流：**
1. 準備暫存資產。
2. 執行 `change summary` 進行初步檢查。
3. 執行 `status staged` 作為最終整備度門禁。

```bash
grafana-util change summary
grafana-util status staged --desired-file ./desired.json --output-format interactive
```

**為什麼這很重要：**
這能確保整個變更包在任何部分觸及正式環境前，都是一致且準備就緒的。

---

## 🔬 驗證說明
本手冊中所有情境均在 **Docker Grafana 12.4.1** 環境下通過驗證。遵循這些模式可確保在正式環境中獲得一致且可預測的結果。
