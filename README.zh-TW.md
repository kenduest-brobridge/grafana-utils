# 📊 Grafana Utilities (維運治理工具集)

[English Version](README.md) | **繁體中文版**

`grafana-utils` 是一套專為 Grafana 管理者與 SRE 打造的維運治理工具集。

## 專案狀態

本專案目前仍處於持續開發階段。

- CLI 介面、作業流程與文件內容仍會持續調整與補強。
- 歡迎回報 bug、邊界案例與實際維運情境中的使用回饋。
- 建議透過 GitHub issues 或 pull requests 進行回報與討論。
- 維護者：`Kenduest`

### 💡 設計初衷：為什麼需要這個工具？

**「官方工具是給使用者用的，Grafana Utilities 是給管理員用的。」**

官方 UI 與 CLI 適合處理單一資源的日常操作。然而，當環境規模擴張到數十個資料來源（Datasource）、上百個儀表板（Dashboard），甚至橫跨多個 Grafana 叢集時，維運人員將面臨以下挑戰：

- **資產盤點盲區 (Inventory Blind Spots)**：難以快速回答「目前有哪些資產？」、「哪些資料來源已失效或未被使用？」或「本次變更與上次快照的差異為何？」
- **搬遷與同步摩擦 (Migration Friction)**：手動匯入匯出難以保留資料夾結構與 UID 一致性，且缺乏可重播（Repeatable）的自動化流程。
- **高風險的線上變更 (Risky Live Mutations)**：直接在生產環境修改資料來源或權限極其危險。缺乏預覽（Dry-run）機制容易導致告警失效或儀表板損壞。
- **破碎的治理流程 (Fragmented Governance)**：儀表板、資料來源、告警規則與使用者權限分散在不同人的操作習慣中，難以實施標準化作業流程。

`grafana-utils` 的核心價值在於將這些維運痛點轉化為**標準化的 CLI 操作**，支援穩定輸出、差異比對（Diff）、預覽機制，以及跨環境的狀態同步。

---

## 🚀 核心功能與優勢

### 1. 環境深度盤點 (Environment Inventory)
- 支援 Dashboard、Datasource、Alerting、Organization、User、Team 與 Service Account 的全面盤點。
- 提供 Table、CSV、JSON 多種輸出模式，方便人工審查或串接自動化腳本。

### 2. 安全的變更管理 (Safe Change Management)
- **差異比對 (Diff)**：在執行匯入或清理前，先比對本地快照與線上環境的差異。
- **預覽機制 (Dry-run)**：在實際寫入前，完整呈現預期行為（Create/Update/Skip），確保操作符合預期。

### 3. 智慧搬遷與備份 (Smart Backup & Migration)
- **資料夾識別 (Folder-aware)**：自動重建資料夾結構，支援路徑自動對應，解決跨環境遷移時的路徑差異問題。
- **狀態重播 (State Replay)**：將 Grafana 狀態轉化為可版本控管（Git-ops friendly）的 JSON 格式，實現環境間的快速還原或對等重製。


### 4. 治理導向的分析 (Governance Inspection)
- 深入分析 Dashboard 結構、資料來源使用情況與查詢語句盤點，識別多餘資源。
- 專為大規模環境設計的分頁擷取與效能最佳化（由 Rust 核心補強）。

### 支援矩陣 (Support Matrix)

| 模組 | 盤點 / 檢視 | 新增 / 修改 / 刪除 | 匯出 / 匯入 / 差異比對 | 備註 |
| --- | --- | --- | --- | --- |
| Dashboard | Yes | No | Yes | 以匯入驅動變更，支援資料夾識別遷移、dry-run，以及路由式多組織匯出/匯入與組織缺失時自動建立 |
| Alerting | Yes | No | Yes | 以匯入驅動規則 / 聯絡點（Contact Point）作業流程 |
| Datasource | Yes | Yes | Yes | 支援 dry-run、diff、全組織匯出，以及路由式多組織匯入與組織缺失時自動建立 |
| Access User | Yes | Yes | Yes | 支援 `--password-file` / `--prompt-user-password` 與 `--set-password-file` / `--prompt-set-password` |
| Access Org | Yes | Yes | Yes | 匯入時可重播組織成員關係 |
| Access Team | Yes | Yes | Yes | 成員關係可匯出 / 匯入 / diff |
| Access Service Account | Yes | Yes | Yes | 支援快照匯出/匯入/diff，以及 token 新增/刪除作業流程 |

---

## 🏗️ 技術架構

本專案結合了雙重語言優勢：
- **Python (流程邏輯)**：負責 CLI 介面定義、複雜的業務邏輯與高度靈活的整合流程。
- **Rust (效能引擎)**：負責高效能的資料解析、查詢驗證以及跨平台單一執行檔的建置。

---

## 🛠️ 快速上手

### 安裝方式

**Python 套件：**
```bash
python3 -m pip install .
```

**Rust 二進位執行檔：**
```bash
cd rust && cargo build --release
```

### 常用情境範例

**批次匯出儀表板 (保留結構)：**
```bash
grafana-util dashboard export \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --export-dir ./dashboards \
  --overwrite
```

**匯入前先執行預覽與比對：**
```bash
grafana-util dashboard import \
  --url http://localhost:3000 \
  --import-dir ./dashboards/raw \
  --replace-existing \
  --dry-run --table
```

---

## 📄 文件導航

- **[繁體中文使用者指南](docs/user-guide-TW.md)**：包含全域參數、認證規則與各模組指令詳解。
- **[English User Guide](docs/user-guide.md)**: Standard operator instructions.
- **[技術細節 (Python)](docs/overview-python.md)** | **[技術細節 (Rust)](docs/overview-rust.md)**
- **[開發者手冊](docs/DEVELOPER.md)**：維護與貢獻說明。

---

## 📈 相容性與目標
- 支援 RHEL 8 / macOS 等作業系統。
- Python 執行環境：3.9+。
- Grafana 版本：支援 8.x, 9.x, 10.x+。
