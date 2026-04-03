# grafana-util
### 專為 Grafana 維運與管理設計的 Rust CLI

[![CI](https://img.shields.io/github/actions/workflow/status/kenduest-brobridge/grafana-utils/ci.yml?branch=main)](https://github.com/kenduest-brobridge/grafana-utils/actions)
[![License](https://img.shields.io/github/license/kenduest-brobridge/grafana-utils)](LICENSE)
[![Version](https://img.shields.io/github/v/tag/kenduest-brobridge/grafana-utils)](https://github.com/kenduest-brobridge/grafana-utils/tags)

[English](./README.md) | 繁體中文

**提供 dashboard、alert、datasource、access control 與維運審查所需的可重複操作流程。**

`grafana-util` 是一款以 Rust 為核心的 Grafana 維運 CLI，面向需要穩定、可審查、可自動化操作方式的 SRE、平台工程師、sysadmin 與維護者。它提供 dashboard、alert、datasource、access control 與整體狀態檢查等操作流程，用來取代零散 API 呼叫與臨時腳本。

---

## 為什麼選擇 `grafana-util`？

| 能力面向 | 一般 CLI / curl | **grafana-util** |
| :--- | :---: | :--- |
| **多組織掃描** | 需手動切換組織 | ✅ 一個指令自動掃描所有組織 |
| **依賴性審查** | 能力有限 | ✅ 匯入前檢查失效的資料來源相依性 |
| **告警變更流程** | 直接修改 | ✅ 可審查的 **計畫 / 套用 (Plan/Apply)** 流程 |
| **機密資料管理** | 容易處理失當 | ✅ **遮蔽式恢復 (Masked Recovery)** 與 profile secret 模式 |
| **審查介面** | 只有原始 JSON | ✅ 互動式 TUI 與結構化表格/報表輸出 |

---

## 快速上手

```bash
# 1. 一鍵安裝 (全域 Binary)
curl -sSL https://raw.githubusercontent.com/kendlee/grafana-utils/main/scripts/install.sh | bash

# 2. 確認安裝版本
grafana-util --version

# 3. 檢視目前 Grafana 狀態
grafana-util overview live --url http://my-grafana:3000 --basic-user admin --prompt-password --output interactive
```

---

## 核心操作流程

### Dashboard：匯出、審查與遷移
```bash
# 1. 跨組織匯出 dashboard
grafana-util dashboard export --all-orgs --export-dir ./backup --progress

# 2. 將一般/raw dashboard JSON 轉成 Grafana UI prompt JSON
grafana-util dashboard raw-to-prompt --input-dir ./backup/raw --output-dir ./backup/prompt --overwrite --progress

# 3. 在正式匯入前先預覽 dashboard 變更
grafana-util dashboard import --import-dir ./backup/raw --replace-existing --dry-run --table

# 4. 在匯入前盤點匯出目錄中的 datasource 相依性
grafana-util dashboard inspect-export --import-dir ./backup/raw --output-format report-table

# 5. 在終端機內搜尋與瀏覽 live dashboards
grafana-util dashboard browse
```

### Alerting：先審查，再套用
```bash
# 1. 比對 desired state 與 live server 建立變更計畫
grafana-util alert plan --desired-dir ./alerts/desired --prune --output json

# 2. 在 apply 前先預覽告警路由
grafana-util alert preview-route --desired-dir ./alerts/desired --label team=sre --severity critical
```

### Datasources：匯出與機密資訊恢復
```bash
# 匯出資料來源時自動遮蔽密鑰，方便審查或納入版本控制
grafana-util datasource export --export-dir ./datasources --overwrite

# 匯入時重新補齊必要的秘密資訊
grafana-util datasource import --import-dir ./datasources --replace-existing --prompt-password
```

### 專案健康度：統一檢視介面
```bash
# 互動式 TUI：在終端機內檢視整體 Grafana 狀態
grafana-util overview live --output interactive
```

---

## 核心功能

*   **Dashboards**：匯出、匯入、檢查、修補、審查與 raw-to-prompt 轉換流程。
*   **Alerting**：desired-state 管理、路由預覽、plan/apply 審查與審慎清理。
*   **Datasources**：匯出/匯入、遮蔽式恢復、provisioning 投影與 inspection 支援。
*   **Access**：稽核與重建 organizations、users、teams、service accounts。
*   **Status & Readiness**：提供 CI/CD 可讀的結構化輸出，以及互動式與表格式維運檢視。

---

## 維運導引手冊 (Operator Handbook)

手冊與指令詳細手冊各自扮演不同角色：手冊負責說明操作流程與維運脈絡，指令說明頁面則緊貼目前 CLI 介面。

如果直接讀 Markdown 不方便，請先產生本機 HTML 文件站，再開啟入口頁：

```bash
make html
open ./docs/html/index.html
```

在 Linux 上請把 `open` 換成 `xdg-open`。這批已簽入的 HTML 檔案主要是給 repo 本機閱讀；GitHub 本身不會把它當成完整靜態文件站來瀏覽。

如果要直接用瀏覽器看公開版，請使用這個 repo 的 GitHub Pages 站點：

*   **公開 HTML 文件站**：<https://kenduest-brobridge.github.io/grafana-utils/>
*   站點內容由 `docs/commands/*/*.md` 與 `docs/user-guide/*/*.md` 生成，並由 `.github/workflows/docs-pages.yml` 從 `main` 分支部署。

*   **[開始使用](./docs/user-guide/zh-TW/getting-started.md)**：安裝、profile 設定與第一批常用命令。
*   **[系統架構與設計原則](./docs/user-guide/zh-TW/architecture.md)**：維運模型、lane 設計與邊界。
*   **[實戰範例](./docs/user-guide/zh-TW/recipes.md)**：常見維運任務與操作流程範例。
*   **[指令詳細說明](./docs/commands/zh-TW/index.md)**：每個 command 與 subcommand 各有獨立頁面，可直接查目前 CLI 指令面，並提供完整 command map 入口，像是 `dashboard screenshot`、`access service-account token`、`change bundle-preflight` 這類較深的子命令也能直接找到。
*   **[HTML 文件入口](./docs/html/index.html)**：執行 `make html` 後可本機瀏覽的 handbook 與 command reference 入口。
*   **[Man Page](./docs/man/grafana-util.1)**：頂層 `man` 格式參考；macOS 可用 `man ./docs/man/grafana-util.1`，GNU/Linux 可用 `man -l docs/man/grafana-util.1`。
*   **[疑難排解](./docs/user-guide/zh-TW/troubleshooting.md)**：診斷、限制與恢復建議。

**[完整手冊目錄入口 →](./docs/user-guide/zh-TW/index.md)**

---

## 文件導覽地圖

如果您不確定該先看哪一份文件，請直接從這裡判斷：

*   **維運手冊**：[docs/user-guide/zh-TW/](./docs/user-guide/zh-TW/index.md) 適合看完整操作流程、觀念與閱讀順序。
*   **指令詳細參考**：[docs/commands/zh-TW/](./docs/commands/zh-TW/index.md) 適合逐頁查 command 與 subcommand。
*   **可瀏覽 HTML 文件站**：本機可看 [docs/html/index.html](./docs/html/index.html)，或直接使用公開站點 <https://kenduest-brobridge.github.io/grafana-utils/>。
*   **終端機 manpage**：[docs/man/grafana-util.1](./docs/man/grafana-util.1) 適合 `man` 風格查詢。
*   **維護者入口**：[docs/DEVELOPER.md](./docs/DEVELOPER.md) 適合看程式架構、文件分層、build/test 路線與 maintainer 引導。
*   **維護者快速上手**：[docs/internal/maintainer-quickstart.md](./docs/internal/maintainer-quickstart.md) 提供第一次進 repo 的最短閱讀路徑、事實來源地圖、產出的檔案邊界與安全驗證命令。
*   **generated docs 設計說明**：[docs/internal/generated-docs-architecture.md](./docs/internal/generated-docs-architecture.md) 說明 Markdown 轉 HTML/manpage 的整體設計。
*   **generated docs 操作手冊**：[docs/internal/generated-docs-playbook.md](./docs/internal/generated-docs-playbook.md) 提供常見維護工作的步驟。
*   **Secret storage 架構說明**：[docs/internal/profile-secret-storage-architecture.md](./docs/internal/profile-secret-storage-architecture.md) 說明 profile secret 模式、macOS/Linux 支援、限制與維護規則。
*   **內部文件總索引**：[docs/internal/README.md](./docs/internal/README.md) 彙整目前有效的內部規格、架構與 trace 文件。

---

## 依角色選擇閱讀路徑

如果您覺得用檔案類型找文件不直覺，可以直接依角色進入：

*   **新使用者**：先看專用的 [新使用者路徑](./docs/user-guide/zh-TW/role-new-user.md)，再看 [開始使用](./docs/user-guide/zh-TW/getting-started.md) 與 [技術參考手冊](./docs/user-guide/zh-TW/reference.md)。
*   **SRE / 維運人員**：先看專用的 [SRE / 維運路徑](./docs/user-guide/zh-TW/role-sre-ops.md)，再看 [變更與狀態](./docs/user-guide/zh-TW/change-overview-status.md)、[Dashboard 管理](./docs/user-guide/zh-TW/dashboard.md)、[Datasource 管理](./docs/user-guide/zh-TW/datasource.md)、[疑難排解](./docs/user-guide/zh-TW/troubleshooting.md)。
*   **自動化 / CI 維護者**：先看專用的 [自動化 / CI 路徑](./docs/user-guide/zh-TW/role-automation-ci.md)，再看 [技術參考手冊](./docs/user-guide/zh-TW/reference.md)、[逐指令說明](./docs/commands/zh-TW/index.md)，再搭配頂層 [manpage](./docs/man/grafana-util.1)。
*   **平台架構師 / maintainer**：先看 [維護者快速上手](./docs/internal/maintainer-quickstart.md)，再看 [docs/DEVELOPER.md](./docs/DEVELOPER.md)、[Maintainer Role Map](./docs/internal/maintainer-role-map.md)、[generated docs 設計說明](./docs/internal/generated-docs-architecture.md)、[generated docs 操作手冊](./docs/internal/generated-docs-playbook.md)、[secret storage 架構說明](./docs/internal/profile-secret-storage-architecture.md)、[docs/internal/README.md](./docs/internal/README.md)。

---

## 技術基礎
*   **Rust 引擎**：以 Rust 為主體的單一 CLI binary。
*   **驗證環境**：在 Docker 環境下針對 **Grafana 12.4.1** 進行驗證。
*   **自動化友善**：提供可預測的 exit code 與結構化輸出，便於 CI/CD 與批次流程整合。

---

## 參與貢獻
我們歡迎任何形式的貢獻！請參閱 [開發者指南](./docs/DEVELOPER.md) 了解設定步驟。

---
*專案維護：[kendlee](https://github.com/kendlee)*
