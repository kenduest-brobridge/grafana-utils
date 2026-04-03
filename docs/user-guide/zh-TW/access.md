# Access 維運人員手冊

本指南涵蓋 Grafana 身份識別與成員管理：組織 (Orgs)、使用者 (Users)、團隊 (Teams)、服務帳號 (Service Accounts) 及其權杖 (Tokens)。

> **目標**：管理誰可以存取您的 Grafana 實例、其組織角色，以及具備盤點與回放能力的自動化身份。

---

## 🛠️ Access 領域用途

使用 `grafana-util access ...` 指令集來執行：
- **盤點身份**：稽核整個資產中的使用者、團隊與服務帳號。
- **直接即時變更**：在特定的組織中建立、修改或刪除身份。
- **快照與回放**：將身份狀態匯出為快照，用於審查或跨環境回放。
- **權杖管理**：服務帳號權杖 (Tokens) 的生命週期控制。

---

## 🚧 工作流路徑邊界

| 家族 | 用途 | 常用操作 |
| :--- | :--- | :--- |
| **組織 (Org)** | 組織生命週期管理。 | `list`, `add`, `modify`, `export`, `import` |
| **使用者 (User)** | 人類帳號管理。 | `list`, `add`, `modify`, `export`, `import`, `diff` |
| **團隊 (Team)** | 成員群組管理。 | `list`, `add`, `modify`, `export`, `import`, `diff` |
| **服務帳號 (SA)** | 自動化身份管理。 | `list`, `add`, `token add`, `token delete`, `export`, `import` |

---

## 📋 閱讀即時身份盤點

使用 `access user list` 驗證人類帳號及其組織角色。

```bash
grafana-util access user list --scope global --table
```

**驗證輸出摘錄：**
```text
ID   LOGIN      EMAIL                NAME             ORG_ROLE   GRAFANA_ADMIN
1    admin      admin@example.com    Grafana Admin    Admin      true
7    svc-ci     ci@example.com       CI Service       Editor     false
```

**如何解讀：**
- **LOGIN**：用於登入的唯一使用者名稱。
- **ORG_ROLE**：在目前組織中的角色 (Admin, Editor, Viewer)。
- **GRAFANA_ADMIN**：標示該使用者是否具備伺服器層級的管理員權限。

---

## 🚀 關鍵指令 (完整參數參考)

| 指令 | 帶有參數的完整範例 |
| :--- | :--- |
| **列出使用者** | `grafana-util access user list --scope global --table` |
| **新增使用者** | `grafana-util access user add --login dev-user --email dev@example.com --password <PASS>` |
| **匯出團隊** | `grafana-util access team export --output-dir ./access/teams --overwrite` |
| **新增權杖** | `grafana-util access service-account token add --id <SA_ID> --name ci-token` |
| **列出組織** | `grafana-util access org list --all-orgs` |

---

## 🔬 Docker 驗證範例

### 1. 團隊匯入 (Dry-Run Replay)
預覽本地團隊檔案將如何影響即時 Grafana 實例。
```bash
grafana-util access team import --import-dir ./access/teams --replace-existing --dry-run --table
```
**輸出摘錄：**
```text
INDEX  IDENTITY         ACTION       DETAIL
1      platform-team    skip         existing and --replace-existing was not set.
2      sre-team         create       would create team
3      edge-team        add-member   would add team member alice@example.com
```

### 2. 服務帳號快照
匯出服務帳號以便進行備份或遷移。
```bash
grafana-util access service-account export --output-dir ./access/service-accounts --overwrite
```
**輸出摘錄：**
```text
Exported 3 service-account(s) -> access/service-accounts/service-accounts.json
```

---

## ⚠️ Access 維運規則

1.  **Scope 控制**：使用 `--scope global` 進行伺服器層級的使用者稽核，或預設為目前的組織上下文。
2.  **破壞性操作**：諸如 `delete` 或會移除項目的匯入，皆需要明確加上 `--yes` 確認旗標。
3.  **權杖安全**：權杖僅在建立時顯示一次。`grafana-util` 在權杖產生後不會儲存或管理其明文。
4.  **管理員權限**：使用 `access org` 或 `access user list --all-orgs` 時需謹慎，因為這些操作需要 Basic Auth 或伺服器層級的管理員權杖。

---

## ⏭️ 下一步
- 了解 [**專案狀態與總覽**](./change-overview-status.md)。
- 遵循 [**情境手冊**](./scenarios.md) 了解端到端的工作流。
