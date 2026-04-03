# 📖 維運導引手冊 (Operator Handbook)

歡迎來到 `grafana-util` 的官方維運手冊。這份導覽旨在帶領您從安裝環境到精通 Estate-level 的 Grafana 治理與自動化。

---

## ⚡ 30 秒快速上手 (Quick Start)

僅需三個指令，即可從零開始獲得完整的專案健康報告。

### 1. 安裝 (全域 Binary)
```bash
# 從原始碼儲存庫下載並安裝最新版本到本地 bin 目錄
curl -sSL https://raw.githubusercontent.com/kendlee/grafana-utils/main/scripts/install.sh | bash
```

### 2. 確認安裝版本
```bash
grafana-util --version
```

### 3. 執行第一次全盤稽核
```bash
# 產生整個 Grafana Estate 的高階健康度與資產盤點報告
grafana-util overview live --url http://localhost:3000 --basic-user admin --prompt-password --output interactive
```

**為什麼這很重要？** 在 30 秒內，您已經驗證了連線、盤點了所有組織的 Dashboard/Alert，並識別出任何失效的資料來源設定。

---

## 🧭 章節導覽 (Navigation Map)

### 🚀 第一階段：奠定基礎
*   **[開始使用 (Getting Started)](getting-started.md)**：進階安裝方式、Profile 管理與認證規則。
*   **[新使用者路徑](role-new-user.md)**：從安裝到第一次成功 live read 的最短安全路徑。
*   **[SRE / 維運路徑](role-sre-ops.md)**：適合日常治理、review-first 變更流程與排障的操作路徑。
*   **[自動化 / CI 路徑](role-automation-ci.md)**：適合腳本、自動化與輸出格式掌握的閱讀路徑。
*   **[系統架構與設計原則](architecture.md)**：解構核心設計決策與哲學。

### 🛠️ 第二階段：核心資產管理
*   **[Dashboard 管理](dashboard.md)**：匯出、匯入與即時分析。
*   **[Datasource 管理](datasource.md)**：Masked Recovery 與即時異動。
*   **[告警治理](alert.md)**：告警規則的「計畫 / 套用」生命週期管理。

### 🔐 第三階段：身份與存取
*   **[Access 管理](access.md)**：組織、使用者、團隊與服務帳號。

### 🛡️ 第四階段：治理與整備度
*   **[變更與狀態 (Change & Status)](change-overview-status.md)**：暫存工作流、專案快照與健康門禁。

### 📖 第五階段：深度探索
*   **[維運實戰場景](scenarios.md)**：端到端任務配方 (備份、災難恢復、稽核)。
*   **[實戰錦囊與最佳實踐](recipes.md)**：針對 Grafana 日常痛點的手術級解決方案。
*   **[技術參考手冊](reference.md)**：全量指令地圖與全域旗標字典。
*   **[逐指令說明](../../commands/zh-TW/index.md)**：每個 command 與 subcommand 各有獨立頁面，可直接查目前 CLI 指令面。
*   **[疑難排解與名詞解釋](troubleshooting.md)**：故障排除導引與術語索引。

---

## 👥 依角色選擇閱讀路徑

不同角色通常會需要不同的閱讀順序：

*   **新使用者**
  先看 [新使用者路徑](role-new-user.md)，再看 [開始使用](getting-started.md)，需要精確旗標時再開 [逐指令說明](../../commands/zh-TW/index.md)。
*   **SRE / 維運人員**
  先看 [SRE / 維運路徑](role-sre-ops.md)，再看 [變更與狀態](change-overview-status.md)、[Dashboard 管理](dashboard.md)、[Datasource 管理](datasource.md)、[疑難排解](troubleshooting.md)。
*   **身份 / 權限管理者**
  先看 [Access 管理](access.md)，再看 [技術參考手冊](reference.md)，最後搭配 [逐指令說明](../../commands/zh-TW/index.md)。
*   **自動化 / CI 維護者**
  先看 [自動化 / CI 路徑](role-automation-ci.md)，再看 [技術參考手冊](reference.md)，如需終端機精確查詢可搭配 `docs/man/grafana-util.1`。
*   **維護者 / 架構師**
  先看 [docs/DEVELOPER.md](/Users/kendlee/work/grafana-utils/docs/DEVELOPER.md)，再看 [maintainer-role-map.md](/Users/kendlee/work/grafana-utils/docs/internal/maintainer-role-map.md) 與 [docs/internal/README.md](/Users/kendlee/work/grafana-utils/docs/internal/README.md) 下面的設計與維護文件。

---

## 🎯 如何使用這份導航？
如果您是新使用者，請從「**開始使用**」點入。每一頁的最下方都設有 **「下一章」** 的連結，讓您可以像讀一本書一樣循序漸進。

---
**下一章**：[🚀 開始使用 (Getting Started)](getting-started.md)
