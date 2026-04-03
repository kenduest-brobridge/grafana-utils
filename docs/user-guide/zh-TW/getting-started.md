# 開始使用

本章提供逐步指引，幫助您驗證 `grafana-util` 安裝，並建立與 Grafana 實例的連線。

> **目標**：確保 binary 正確安裝，驗證 Grafana 連線，並安全執行第一次唯讀指令。

---

## 🛠️ 步驟 1：驗證安裝

確認已安裝的版本並探索可用的命令面。這能確保您執行的是正確的 binary。

```bash
# 驗證 binary 版本
grafana-util --version

# 探索全域與特定領域的說明
grafana-util -h
grafana-util dashboard -h
grafana-util alert -h
grafana-util datasource -h
grafana-util access -h
grafana-util profile -h
grafana-util change -h
grafana-util overview -h
grafana-util status -h
```

---

## 🔐 步驟 2：連線模型

`grafana-util` 支援兩種主要方式與 Grafana 互動：

### 1. 直接帶旗標 (Direct CLI Flags)
適合一次性任務或測試。您直接在每個指令中傳遞憑證。

```bash
grafana-util dashboard list \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin
```

### 2. 本地設定檔 (Repo-Local Profiles)
最適合重複性的維運工作。Profile 將 URL、驗證資訊與 timeout 設定儲存在 `grafana-profiles.yaml`。

**初始化 Profile：**
```bash
grafana-util profile init
```

**檢視與管理 Profile：**
```bash
# 列出所有 Profile
grafana-util profile list

# 顯示特定 Profile 的詳細內容
grafana-util profile show --profile prod --output-format yaml
```

---

## 📋 步驟 3：第一步安全指令

在執行任何變更 (Import/Apply) 前，請先透過這些安全、非破壞性的指令驗證存取權限。

| 任務 | 完整指令範例 | 驗證目的 |
| :--- | :--- | :--- |
| **資產盤點** | `grafana-util dashboard list --all-orgs --with-sources --table` | 驗證 API 連線與目錄可見度。 |
| **資料來源** | `grafana-util datasource list --table` | 驗證讀取 Datasource 外掛與類型的能力。 |
| **整備度** | `grafana-util status live --output-format table` | 全專案整備度與健康度報告。 |
| **總覽** | `grafana-util overview live` | 人類可讀的整體資產摘要。 |

---

## 🖥️ 互動模式 (TUI)

部分指令支援 **終端機使用者介面 (TUI)**，提供引導式的人類審查。

| 命令 | 用法 | 最佳用途 |
| :--- | :--- | :--- |
| `dashboard browse` | `grafana-util dashboard browse` | 引導式發現現有的 Dashboard。 |
| `inspect` | `grafana-util dashboard inspect-export --interactive` | 離線審查匯出的 Dashboard 目錄。 |
| `overview` | `grafana-util overview --output interactive` | 在終端機內以互動方式檢視專案狀態。 |

---

## ⏭️ 下一步

- 參考 [**參考手冊**](./reference.md) 了解詳細的旗標與驗證規則。
- 遵循 [**情境手冊**](./scenarios.md) 了解端到端的工作流 (例如遷移、備份)。
