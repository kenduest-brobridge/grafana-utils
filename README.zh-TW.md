# grafana-util 🚀
### 專為專業維運打造的 Grafana 全域管理工具

[![CI](https://img.shields.io/github/actions/workflow/status/kendlee/grafana-utils/ci.yml?branch=main)](https://github.com/kendlee/grafana-utils/actions)
[![License](https://img.shields.io/github/license/kendlee/grafana-utils)](LICENSE)
[![Release](https://img.shields.io/github/v/release/kendlee/grafana-utils)](https://github.com/kendlee/grafana-utils/releases)

**別再手動點擊 UI。開始大規模治理您的 Grafana。**

`grafana-util` 是一款基於 Rust 的高性能 CLI，專為管理多組織、多實例等複雜 Grafana 環境的 SRE 與平台工程師設計。它填補了原始 API 呼叫與企業級治理之間的巨大鴻溝。

---

## 🌟 為什麼選擇 `grafana-util`？

| 功能特性 | 一般 CLI / curl | **grafana-util** |
| :--- | :---: | :--- |
| **多組織掃描** | 需手動切換組織 | ✅ 一個指令自動掃描所有組織 |
| **依賴性審查** | 極其困難 | ✅ 匯入前自動檢測失效的資料來源 |
| **告警生命週期** | 直接強制覆蓋 | ✅ **計畫 / 套用 (Plan/Apply)** 審查機制 |
| **秘密資訊安全** | 容易外洩密碼 | ✅ **遮蔽式恢復 (Masked Recovery)** 機制 |
| **視覺化審查** | 只有原始 JSON | ✅ 互動式 **TUI** 與精美的表格報告 |

---

## ⚡ 30 秒快速上手 (Quick Start)

```bash
# 1. 一鍵安裝 (全域 Binary)
curl -sSL https://raw.githubusercontent.com/kendlee/grafana-utils/main/scripts/install.sh | bash

# 2. 確認安裝版本
grafana-util --version

# 3. 立即檢視全域資產狀態
grafana-util overview live --url http://my-grafana:3000 --basic-user admin --prompt-password --output interactive
```

---

## 🚀 核心工作流 (精華指令集)

### 📊 Dashboard：全域資產管理
```bash
# 1. 跨組織匯出：一鍵備份所有組織的所有儀表板 (含進度條)
grafana-util dashboard export --all-orgs --export-dir ./backup --progress

# 2. 將一般/raw dashboard JSON 轉成 Grafana UI prompt JSON
grafana-util dashboard raw-to-prompt --input-dir ./backup/raw --output-dir ./backup/prompt --overwrite --progress

# 3. 匯入預覽：在正式提交前，先在終端機查看變更表格 (Dry-Run)
grafana-util dashboard import --import-dir ./backup/raw --replace-existing --dry-run --table

# 4. 依賴盤點：在匯入前，自動偵測匯出目錄中是否有失效的資料來源
grafana-util dashboard inspect-export --import-dir ./backup/raw --output-format report-table

# 5. 互動瀏覽：在終端機直接搜尋、瀏覽與發現現有的儀表板
grafana-util dashboard browse
```

### 🚨 Alerting：計畫 / 套用生命週期
```bash
# 1. 建立變更計畫：比對本地檔案與伺服器之間的差異
grafana-util alert plan --desired-dir ./alerts/desired --prune --output json

# 2. 安全優先：根據標籤模擬告警路由，確認其會送往哪個 Receiver
grafana-util alert preview-route --desired-dir ./alerts/desired --label team=sre --severity critical
```

### 🔐 Datasources：遮蔽式恢復 (Masked Recovery)
```bash
# 匯出資料來源時自動遮蔽密鑰 (可安全提交至 Git！)
grafana-util datasource export --export-dir ./datasources --overwrite

# 匯入時自動啟動秘密注入協議 (重新補齊密碼)
grafana-util datasource import --import-dir ./datasources --replace-existing --prompt-password
```

### 🛡️ 專案健康度：統一操作介面
```bash
# 互動式 TUI：在終端機開啟整個 Grafana 莊園的即時健康度儀表板
grafana-util overview live --output interactive
```

---

## 🛠️ 核心功能

*   **Dashboards**：全真匯出 / 匯入、變數分析、批次手術修補 (Patching)。
*   **Alerting**：Grafana 告警的宣告式管理。支援預覽路由 (Preview Route) 與安全清理過期規則。
*   **Datasources**：遮蔽式匯出 / 匯入。在恢復時安全地重新注入密鑰。
*   **Access**：稽核與回放組織、使用者、團隊與服務帳號的身份狀態。
*   **Status & Readiness**：為 CI/CD 提供機器可讀的技術合約，為維運人員提供 TUI 互動報告。

---

## 📖 維運導引手冊 (Operator Handbook)

不只是執行指令，而是精通工作流。我們為您準備了完整的中文手冊：

如果直接讀 Markdown 不方便，請先產生本機 HTML 文件站，再開啟入口頁：

```bash
make html
open ./docs/html/index.html
```

在 Linux 上請把 `open` 換成 `xdg-open`。這批 checked-in HTML 檔案主要是給 repo 本機閱讀；GitHub 本身不會把它當成完整靜態文件站來瀏覽。

如果要直接用瀏覽器看公開版，請使用這個 repo 的 GitHub Pages 站點：

*   **公開 HTML 文件站**：<https://kendlee.github.io/grafana-utils/>
*   站點內容由 `docs/commands/*/*.md` 與 `docs/user-guide/*/*.md` 生成，並由 `.github/workflows/docs-pages.yml` 從 `main` 分支部署。

*   **[開始使用](./docs/user-guide/zh-TW/getting-started.md)**：Profile 設定與安裝。
*   **[系統架構與設計原則](./docs/user-guide/zh-TW/architecture.md)**：解構設計哲學。
*   **[實戰錦囊](./docs/user-guide/zh-TW/recipes.md)**：解決常見的 Grafana 維運痛點。
*   **[逐指令說明](./docs/commands/zh-TW/index.md)**：每個 command 與 subcommand 各有獨立頁面，可直接查目前 CLI 指令面。
*   **[HTML 文件入口](./docs/html/index.html)**：執行 `make html` 之後可本機直接瀏覽的 handbook + command reference 入口。
*   **[Man Page](./docs/man/grafana-util.1)**：頂層 `man` 格式參考；macOS 可用 `man ./docs/man/grafana-util.1`，GNU/Linux 可用 `man -l docs/man/grafana-util.1`。
*   **[疑難排解](./docs/user-guide/zh-TW/troubleshooting.md)**：診斷指南與名詞索引。

**[完整手冊目錄入口 →](./docs/user-guide/zh-TW/index.md)**

---

## 🧭 文件導覽地圖

如果您不確定該先看哪一份文件，請直接從這裡判斷：

*   **維運手冊**：[docs/user-guide/zh-TW/](./docs/user-guide/zh-TW/index.md) 適合看完整工作流、觀念與閱讀順序。
*   **逐指令參考**：[docs/commands/zh-TW/](./docs/commands/zh-TW/index.md) 適合逐頁查 command 與 subcommand。
*   **可瀏覽 HTML 文件站**：本機可看 [docs/html/index.html](./docs/html/index.html)，或直接使用公開站點 <https://kendlee.github.io/grafana-utils/>。
*   **終端機 manpage**：[docs/man/grafana-util.1](./docs/man/grafana-util.1) 適合 `man` 風格查詢。
*   **維護者入口**：[docs/DEVELOPER.md](./docs/DEVELOPER.md) 適合看程式架構、文件分層、build/test 路線與 maintainer 引導。
*   **maintainer quickstart**：[docs/internal/maintainer-quickstart.md](./docs/internal/maintainer-quickstart.md) 提供第一次進 repo 的最短閱讀路徑、source of truth 地圖、generated 檔邊界與安全驗證命令。
*   **generated docs 設計說明**：[docs/internal/generated-docs-architecture.md](./docs/internal/generated-docs-architecture.md) 說明 Markdown 轉 HTML/manpage 的整體設計。
*   **generated docs 操作手冊**：[docs/internal/generated-docs-playbook.md](./docs/internal/generated-docs-playbook.md) 提供常見維護工作的步驟。
*   **Secret storage 架構說明**：[docs/internal/profile-secret-storage-architecture.md](./docs/internal/profile-secret-storage-architecture.md) 說明 profile secret 模式、macOS/Linux 支援、限制與維護規則。
*   **內部文件總索引**：[docs/internal/README.md](./docs/internal/README.md) 彙整目前有效的內部 spec、架構與 trace 文件。

---

## 👥 依角色選擇閱讀路徑

如果您覺得用檔案類型找文件不直覺，可以直接依角色進入：

*   **新使用者**：先看專用的 [新使用者路徑](./docs/user-guide/zh-TW/role-new-user.md)，再看 [開始使用](./docs/user-guide/zh-TW/getting-started.md) 與 [技術參考手冊](./docs/user-guide/zh-TW/reference.md)。
*   **SRE / 維運人員**：先看專用的 [SRE / 維運路徑](./docs/user-guide/zh-TW/role-sre-ops.md)，再看 [變更與狀態](./docs/user-guide/zh-TW/change-overview-status.md)、[Dashboard 管理](./docs/user-guide/zh-TW/dashboard.md)、[Datasource 管理](./docs/user-guide/zh-TW/datasource.md)、[疑難排解](./docs/user-guide/zh-TW/troubleshooting.md)。
*   **自動化 / CI 維護者**：先看專用的 [自動化 / CI 路徑](./docs/user-guide/zh-TW/role-automation-ci.md)，再看 [技術參考手冊](./docs/user-guide/zh-TW/reference.md)、[逐指令說明](./docs/commands/zh-TW/index.md)，再搭配頂層 [manpage](./docs/man/grafana-util.1)。
*   **平台架構師 / maintainer**：先看 [maintainer quickstart](./docs/internal/maintainer-quickstart.md)，再看 [docs/DEVELOPER.md](./docs/DEVELOPER.md)、[Maintainer Role Map](./docs/internal/maintainer-role-map.md)、[generated docs 設計說明](./docs/internal/generated-docs-architecture.md)、[generated docs 操作手冊](./docs/internal/generated-docs-playbook.md)、[secret storage 架構說明](./docs/internal/profile-secret-storage-architecture.md)、[docs/internal/README.md](./docs/internal/README.md)。

---

## 🏗️ 技術基礎
*   **Rust 引擎**：單一靜態 Binary，無依賴，極速響應。
*   **驗證環境**：在 Docker 環境下針對 **Grafana 12.4.1** 完成完整驗證。
*   **CI/CD 友善**：具備可預測的結束代碼 (Exit Codes) 與 JSON 優先的輸出設計。

---

## 🤝 參與貢獻
我們歡迎任何形式的貢獻！請參閱 [開發者指南](./docs/DEVELOPER.md) 了解設定步驟。

---
*專案維護：[kendlee](https://github.com/kendlee)*
