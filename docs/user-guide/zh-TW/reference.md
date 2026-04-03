# 參考手冊 (Reference Manual)

本章提供 `grafana-util` 的完整技術參考。專為需要指令語法、驗證協議與輸出合約精確細節的維運人員設計。

---

## 🏗️ 指令家族 (Command Families)

`grafana-util` 依功能領域組織。每個領域支援一組特定的 Grafana 資產管理操作。

| 領域 | 用途 | 關鍵指令 (含參數範例) |
| :--- | :--- | :--- |
| **Dashboard** | 盤點與分析 | `list --all-orgs`, `export --export-dir <dir>`, `import --import-dir <dir> --dry-run`, `inspect-vars --uid <uid>`, `patch-file --input <file>`, `clone-live --uid <uid>`, `browse` |
| **Datasource** | 生命週期與恢復 | `list --table`, `export --output-dir <dir>`, `import --replace-existing`, `diff`, `add --type <type>`, `modify --id <id>`, `delete --id <id>`, `browse` |
| **Alert** | 審查優先管理 | `plan --desired-dir <dir>`, `apply --plan-file <file>`, `add-rule --name <name>`, `set-route`, `preview-route`, `new-rule`, `export --overwrite` |
| **Access** | 身份與組織 | `org list`, `user add --login <user>`, `team list --org-id <id>`, `service-account token add --id <id>`, `export`, `import`, `diff` |
| **Change** | 暫存工作流 | `summary`, `bundle --output-file <file>`, `preflight --staged-dir <dir>`, `assess-alerts`, `plan`, `review`, `apply` |
| **Status** | 整備度報告 | `status live --output-format json`, `status staged --output-format interactive`, `overview`, `overview live` |

---

## 🔐 全域驗證與連線

這些旗標常見於幾乎所有與 Grafana 直連的指令。

### 連線旗標
| 旗標 | 說明 | 預設值 |
| :--- | :--- | :--- |
| `--url` | Grafana 實例的基礎 URL。 | `http://localhost:3000` |
| `--timeout` | HTTP 請求超時時間 (秒)。 | `30` |
| `--verify-ssl` | 啟用 / 禁用 TLS 憑證驗證。 | `true` |
| `--profile` | 從 `grafana-profiles.yaml` 載入特定 Profile 的設定。 | (無) |

### 驗證模式
| 模式 | 旗標 |
| :--- | :--- |
| **Token** | `--token`, `--api-token`, `--prompt-token` |
| **Basic Auth** | `--basic-user`, `--basic-password`, `--prompt-password` |

> **安全提示**：在互動式 Session 中建議優先使用 `--prompt-token` 或 `--prompt-password`，避免密鑰流落在 Shell 歷史紀錄中。

---

## 📊 輸出介面 (Output Surfaces)

CLI 提供多種資料呈現方式，取決於接收者是人類還是機器。

| 模式 | 旗標 | 典型使用場景 |
| :--- | :--- | :--- |
| **純文字** | (預設) | 快速摘要、dry-run 預覽與日誌。 |
| **JSON** | `--output-format json` | 自動化、搭配 `jq` 處理或存成 Artifact。 |
| **表格** | `--table` 或 `--output-format table` | 稽核與人類可讀的資產清單。 |
| **互動式** | `--output-format interactive` | 引導式瀏覽與複雜狀態審查 (TUI)。 |

---

## 🛠️ 設定檔 (`grafana-profiles.yaml`)

Profile 消除重複輸入連線旗標的需要。典型配置如下：

```yaml
default_profile: dev
profiles:
  dev:
    url: http://localhost:3000
    token_env: GRAFANA_DEV_TOKEN
    verify_ssl: false
  prod:
    url: https://grafana.example.com
    username: admin
    password_env: GRAFANA_PROD_PASSWORD
```

### Profile 選擇邏輯
1.  **明確指定**：`--profile prod` 始終優先。
2.  **隱含預設**：若未提供旗標，則使用 `default_profile`。
3.  **自動選擇**：若僅存在一個 Profile，則自動選用。
4.  **覆蓋規則**：任何 CLI 旗標 (如 `--url`) 都會覆蓋 Profile 內的值。

---

## ⚖️ 資源功能矩陣 (Resource Capability Matrix)

| 資源類型 | List | Export | Import | Diff | 變更 (Add/Mod/Del) |
| :--- | :---: | :---: | :---: | :---: | :---: |
| Dashboards | ✅ | ✅ | ✅ | ✅ | ✅ |
| Datasources | ✅ | ✅ | ✅ | ✅ | ✅ |
| Alerts | ✅ | ✅ | ✅ | ✅ | ✅ (透過 Plan/Apply) |
| 組織 / 使用者 | ✅ | ✅ | ✅ | ✅ | ✅ |
| 服務帳號 | ✅ | ✅ | ✅ | ✅ | ✅ |

---

## ⏭️ 下一步
了解實務的任務導向指南，請前往 [**情境手冊**](./scenarios.md) 章節。
