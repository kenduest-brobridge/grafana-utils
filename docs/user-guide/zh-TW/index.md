# 📖 維運導引手冊 (Operator Handbook)

## 語言切換

- 繁體中文手冊：[目前頁面](./index.md)
- English handbook: [英文手冊](../en/index.md)
- 繁體中文指令詳細說明：[指令詳細說明](../../commands/zh-TW/index.md)
- English command reference: [Command Docs](../../commands/en/index.md)

---

歡迎來到 `grafana-util` 的維運手冊。這份手冊會帶你從安裝、連線、profile 這些基本設定開始，一路看到日常維運、自動化流程，以及整體 Grafana 環境怎麼管。

如果你想先知道這個工具到底在解什麼問題、適合哪些工作、什麼情況不一定要先用它，建議先看：

- [這個工具是做什麼的](what-is-grafana-util.md)

## 採用前後對照

- 以前：要先猜應該開哪一章，才能知道從哪裡開始讀。
- 現在：先看用途頁，再依照角色選路線，就不容易迷路。

## 成功判準

- 你知道目前這個任務該看哪一章。
- 你能把手冊章節和精確指令說明分開。
- 你能從第一次唯讀檢查，順利走到對應的工作流。

## 失敗時先檢查

- 如果還不知道自己該看 dashboard、alert、access 還是 change，先回到用途頁和角色路線圖。
- 如果第一條即時唯讀命令失敗，先修連線或驗證，不要先往變更章節走。
- 如果你只是在找精確旗標，請直接切到指令詳細說明，不要硬從手冊猜。

---

## 適用對象

- 想先看完章節地圖，再決定從哪裡開始的人
- 想先知道這份手冊涵蓋哪些工作流的人
- 想快速切到角色導讀、完整參考或指令索引的人

## 主要目標

- 先讓你看懂這份手冊在解什麼問題
- 幫你快速找到適合自己角色的路線
- 讓你知道何時該看手冊、何時該直接查指令頁

---

## ⚡ 30 秒快速上手 (Quick Start)

只要三個指令，就能從零開始確認安裝、連線，並快速掌握目前環境狀態。

### 1. 安裝 (全域 Binary)
```bash
# 從原始碼儲存庫下載並安裝最新版本到本地 bin 目錄
curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-utils/main/scripts/install.sh | sh
```

### 2. 確認安裝版本
```bash
# 用途：2. 確認安裝版本。
grafana-util --version
```

### 3. 執行第一次完整巡檢
```bash
# 產生整個 Grafana Estate 的高階健康度與資產盤點報告
grafana-util overview live --url http://localhost:3000 --basic-user admin --prompt-password --output-format interactive
```

**為什麼這很重要？** 30 秒內，你就能先確認連線正常，快速看 Dashboard、Alert 和 data source 的狀態，也能先看出哪些設定可能已經失效。

---

## 🧭 章節導覽 (Navigation Map)

### 🚀 第一階段：奠定基礎
*   **[這個工具是做什麼的](what-is-grafana-util.md)**：先看它在解哪些痛點、適合哪些維運工作。
*   **[開始使用 (Getting Started)](getting-started.md)**：先看安裝、連線、profile 與認證怎麼配合。
*   **[新手快速入門](role-new-user.md)**：從第一次連線到第一次成功讀到 live 狀態的最短安全路線。
*   **[SRE / 維運角色導讀](role-sre-ops.md)**：適合日常維運、先審查再變更的流程與排障。
*   **[自動化 / CI 角色導讀](role-automation-ci.md)**：適合腳本、自動化與輸出格式相關工作。
*   **[系統架構與設計原則](architecture.md)**：說明核心設計決策和背後取捨。

### 🛠️ 第二階段：核心資產管理
*   **[Dashboard 管理](dashboard.md)**：看儀表板匯出、匯入與即時分析。
*   **[Data source 管理](datasource.md)**：看 data source 的匯出、匯入與即時異動。
*   **[告警治理](alert.md)**：看告警規則的 plan / apply 管理流程。

### 🔐 第三階段：身份與存取
*   **[Access 管理](access.md)**：看 org、使用者、team 與 service account 管理作業。

### 🛡️ 第四階段：治理與整備度
*   **[變更與狀態 (Change & Status)](change-overview-status.md)**：看變更前後的檢查、狀態確認與快照流程。

### 📖 第五階段：深度探索
*   **[維運情境手冊](scenarios.md)**：看備份、災難復原、稽核等端到端任務範例。
*   **[實戰錦囊與最佳實踐](recipes.md)**：整理 Grafana 日常常見問題與建議做法。
*   **[技術參考手冊](reference.md)**：集中說明常用旗標、輸出格式與操作原則。
*   **[指令詳細說明](../../commands/zh-TW/index.md)**：每個 command 和 subcommand 都有獨立頁面，適合直接查語法與旗標。
*   **[疑難排解與名詞解釋](troubleshooting.md)**：故障排除導引與術語索引。

---

## 👥 依角色選擇閱讀路徑

不同角色適合的閱讀順序不太一樣：

*   **新使用者**
  先看 [這個工具是做什麼的](what-is-grafana-util.md)，再看 [新手快速入門](role-new-user.md) 與 [開始使用](getting-started.md)，需要查精確旗標時再查看 [指令詳細說明](../../commands/zh-TW/index.md)。
*   **SRE / 維運人員**
  先看 [SRE / 維運角色導讀](role-sre-ops.md)，再看 [變更與狀態](change-overview-status.md)、[Dashboard 管理](dashboard.md)、[Data source 管理](datasource.md)、[疑難排解](troubleshooting.md)。
*   **身份 / 權限管理者**
  先看 [Access 管理](access.md)，再看 [技術參考手冊](reference.md)，最後搭配 [指令詳細說明](../../commands/zh-TW/index.md)。
*   **自動化 / CI 維護者**
  先看 [自動化 / CI 角色導讀](role-automation-ci.md)，再看 [技術參考手冊](reference.md)，需要終端機版摘要時可搭配 `docs/man/grafana-util.1`。
*   **維護者 / 架構師**
  先看 [DEVELOPER.md](../DEVELOPER.md)，再看 [maintainer-role-map.md](../internal/maintainer-role-map.md) 與 [internal README](../internal/README.md) 裡的設計與維護文件。

---

## 🎯 如何使用這份導航？
如果你是第一次使用，建議先從「**開始使用**」進去。每頁最下方都有 **「下一章」** 連結，可以照順序一路讀下去。

---
**下一章**：[🧭 這個工具是做什麼的](what-is-grafana-util.md)
