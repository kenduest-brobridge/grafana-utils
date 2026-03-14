# 📊 Grafana Utilities (維運治理工具集)

[English Version](README.md) | **繁體中文版**

`grafana-utils` 是一套專為 Grafana 管理者與 SRE 打造的維運治理工具集。

### 💡 設計初衷：為什麼需要這個工具？

**「官方工具是給使用者用的，Grafana Utilities 是給管理員用的。」**

官方 UI 與 CLI 適合處理單一資源的日常操作。然而，當環境規模擴張到數十個資料源（Datasource）、上百個儀表板（Dashboard），甚至橫跨多個 Grafana 叢集時，維運人員將面臨以下挑戰：

- **資產盤點盲區 (Inventory Blind Spots)**：難以快速回答「目前有哪些資產？」、「哪些資料源已失效或未被使用？」或「本次變更與上次快照的差異為何？」
- **搬遷與同步摩擦 (Migration Friction)**：手動匯入匯出難以保留資料夾結構與 UID 一致性，且缺乏可重播（Repeatable）的自動化流程。
- **高風險的線上變更 (Risky Live Mutations)**：直接在生產環境修改資料源或權限極其危險。缺乏預覽（Dry-run）機制容易導致告警失效或儀表板損壞。
- **破碎的治理流程 (Fragmented Governance)**：儀表板、資料源、告警規則與使用者權限分散在不同人的操作習慣中，難以實施標準化工作流。

`grafana-utils` 的核心價值在於將這些維運痛點轉化為**標準化的 CLI 操作**，支援穩定輸出、差異比對（Diff）、預覽機制，以及跨環境的狀態同步。

---

## 🚀 核心功能與優勢

### 1. 環境深度盤點 (Environment Inventory)
- 支援 Dashboard、Datasource、Alerting、User、Team 與 Service Account 的全面掃描。
- 提供 Table、CSV、JSON 多種輸出模式，方便人工審查或串接自動化腳本。

### 2. 安全的變更管理 (Safe Change Management)
- **差異比對 (Diff)**：在執行匯入或清理前，先比對本地快照與線上環境的差異。
- **預覽機制 (Dry-run)**：在實際寫入前，完整呈現預期行為（Create/Update/Skip），確保操作符合預期。

### 3. 智慧搬遷與備份 (Smart Backup & Migration)
- **資料夾感知 (Folder-aware)**：自動重建資料夾結構，支援路徑匹配，解決跨環境遷移的對應問題。
- **狀態重播 (State Replay)**：將 Grafana 狀態轉化為可版本控管（Git-ops friendly）的 JSON 格式，實現環境間的快速還原或對等重製。

### 4. 治理導向的分析 (Governance Inspection)
- 深入分析 Dashboard 結構、資料源使用情況與查詢語句盤點，識別冗餘資源。
- 專為大規模環境設計的分頁抓取與效能優化（由 Rust 核心補強）。

---

## 🏗️ 技術架構

本專案結合了雙重語言優勢：
- **Python (工作流邏輯)**：負責 CLI 介面定義、複雜的業務邏輯與高度靈活的整合流程。
- **Rust (效能引擎)**：負責高效能的資料解析、查詢驗證以及跨平台單一執行檔的建置。

---

## 🛠️ 快速上手

### 安裝方式

**Python 套件：**
```bash
python3 -m pip install .
```

**Rust 二進制檔：**
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

- **[繁體中文使用者指南](docs/user-guide-TW.md)**：包含全域參數、認證規則與各 Domain 指令詳解。
- **[English User Guide](docs/user-guide.md)**: Standard operator instructions.
- **[技術細節 (Python)](docs/overview-python.md)** | **[技術細節 (Rust)](docs/overview-rust.md)**
- **[開發者手冊](docs/DEVELOPER.md)**：維護與貢獻說明。

---

## 📈 相容性與目標
- 支援 RHEL 8 / macOS 等作業系統。
- Python 執行環境：3.9+。
- Grafana 版本：支援 8.x, 9.x, 10.x+。
