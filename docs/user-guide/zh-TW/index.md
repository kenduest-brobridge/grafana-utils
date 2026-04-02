# Grafana Utilities 維運人員手冊

歡迎閱讀 `grafana-util` 官方維運手冊。本手冊專為需要大規模管理 Grafana 的工程師設計，提供資產盤點、遷移與治理的結構化工作流。

---

## 📖 如何使用這份手冊

本指南採主題式章節編排，旨在減少上下文切換。請根據您目前的任務選擇起點：

| 如果您是... | 建議起點 | 原因 |
| :--- | :--- | :--- |
| **初次使用工具** | [開始使用](./getting-started.md) | 驗證安裝並建立第一次安全連線。 |
| **執行特定任務** | [情境手冊](./scenarios.md) | 遵循常見維運任務的端到端工作流。 |
| **查詢指令旗標** | [參考手冊](./reference.md) | 詳細的指令語法、驗證規則與輸出合約。 |
| **管理特定資源** | 資源章節 | 深入了解 Dashboard、Datasource 或 Alert。 |

---

## 🗺️ 章節地圖

### 1. 導覽與核心概念
- [**開始使用**](./getting-started.md)：安裝、Profile 設定與初步連線驗證。
- [**參考手冊**](./reference.md)：全域旗標、驗證規則與輸出格式 (JSON, Table, TUI)。
- [**情境手冊**](./scenarios.md) : 串聯多個指令家族的任務導向指南。

### 2. 資源維運手冊
- [**Dashboard 手冊**](./dashboard.md)：盤點、差異審查與多路徑匯出/匯入。
- [**Datasource 手冊**](./datasource.md)：Masked Recovery、即時變更與 Provisioning 投影。
- [**Alert 手冊**](./alert.md)：Plan/Apply 工作流、狀態編寫與遷移 Bundle。
- [**Access 手冊**](./access.md)：組織、使用者、團隊與服務帳號管理。

### 3. 進階操作
- [**Change, Overview & Status**](./change-overview-status.md)：跨域暫存變更與全專案整備度檢查。

---

## ⚙️ 指令架構

所有指令均遵循一致且可預測的模式：

```bash
grafana-util <domain> <command> [options]
```

### 支援的輸出模式
不同的任務需要不同的資料呈現方式。`grafana-util` 支援：
- 📝 **純文字 (Plain Text)**：預設的人類可讀摘要與 dry-run 預覽。
- 🔢 **JSON**：針對 CI/CD 流水線與穩定機器讀取優化。
- 📊 **表格 (Table/CSV)**：適合稽核、資產清單與側重審查。
- 🖥️ **互動式 TUI**：提供引導式瀏覽 (例如 `dashboard browse`)。

---

## 🎯 目標環境
本文件範例均在 **Grafana 12.4.1** 環境下驗證。雖然工具支援多種版本，但在進行大規模變更前，請務必先在測試環境驗證指令行為。
