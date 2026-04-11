# grafana-util

從這裡直接進手冊、指令頁或常用工作流程。

## 快速跳轉

輸入頁面或指令名稱，直接跳到最接近的文件頁。

## 快速開始

先從大多數人第一天就會用到的入口開始。

### 這個工具是做什麼的

如果你想先搞清楚工具定位，再決定要看哪條工作流或哪組指令，先從這裡開始。

- [瀏覽章節](../user-guide/zh-TW/what-is-grafana-util.md)

### 第一次連線與檢查

先確認 binary、Grafana 連線與第一個唯讀檢查都正常，再決定後面要不要整理成 profile。

- [瀏覽章節](../user-guide/zh-TW/getting-started.md)

### 新手安全路線

如果你是第一次接觸這個工具，先走這條路線最快進入狀況。

- [瀏覽章節](../user-guide/zh-TW/role-new-user.md)

## 常用任務

從常見工作流進入，比直接翻完整索引更快。

### 變更前日常檢查

想先看目前 Grafana 環境的狀態、變更前檢查與日常維運節奏，從這裡進去。

- [瀏覽章節](../user-guide/zh-TW/role-sre-ops.md)

### Dashboard 備份、遷移與回放

如果你要處理 dashboard 的匯出、匯入、搬遷或依賴檢查，直接從這裡進去。

- [瀏覽章節](../user-guide/zh-TW/dashboard.md)

### Dashboard 截圖與檢視

如果你要檢查變數、看查詢依賴、整理事件處理附圖，或產出可重現的 dashboard 截圖，直接從這裡進去。

- [瀏覽章節](../user-guide/zh-TW/dashboard.md)
- [查看 dashboard screenshot](../commands/zh-TW/dashboard-screenshot.md)

### Data source 與告警治理

要處理 data source 驗證、告警規則與通知路由時，從這組章節開始比較完整。

- [瀏覽 Data source 章節](../user-guide/zh-TW/datasource.md)
- [瀏覽告警章節](../user-guide/zh-TW/alert.md)

### Access 與自動化憑證

要處理 org、team、service account、token 輪替或權限管理時，直接從 Access 章節進去。

- [瀏覽章節](../user-guide/zh-TW/access.md)

### 維運情境與疑難排解

如果你不是要查單一指令，而是要解一個真實問題，先看情境手冊與疑難排解通常比較快。

- [瀏覽維運情境](../user-guide/zh-TW/scenarios.md)
- [瀏覽疑難排解](../user-guide/zh-TW/troubleshooting.md)

## 功能總覽

先用這裡快速判斷自己遇到的是哪一類工作，再往對應章節或指令說明走。

### Status

想先看 live 或 staged 狀態、快速盤點環境，先從 `status` 開始。

- [瀏覽技術參考](../user-guide/zh-TW/reference.md)
- [瀏覽 status 指令](../commands/zh-TW/status.md)

### 資產操作

要處理 dashboard、data source、alert 這些 Grafana 資產的匯出、匯入、檢查或治理時，從這組開始。

- [瀏覽 Dashboard 章節](../user-guide/zh-TW/dashboard.md)
- [瀏覽 Data source 章節](../user-guide/zh-TW/datasource.md)
- [瀏覽告警章節](../user-guide/zh-TW/alert.md)

### 身分與憑證

要整理 org、team、service account、token，或把連線與 secret 來源收進 profile，從這裡進去。

- [瀏覽 Access 章節](../user-guide/zh-TW/access.md)
- [瀏覽 config profile 指令](../commands/zh-TW/profile.md)

### Workspace 審查

如果你不想直接套用 workspace，而是想先做 scan、test、preview 與 package，請從這裡開始。

- [瀏覽 workspace 與狀態](../user-guide/zh-TW/status-workspace.md)
- [瀏覽 workspace 指令](../commands/zh-TW/workspace.md)

## 完整參考

如果你已經知道要查哪一塊，直接從這裡進去。

### 完整手冊

角色導覽、架構說明、操作脈絡、實戰情境與疑難排解都整理在這裡。

- [瀏覽手冊](../user-guide/zh-TW/index.md)

### 指令參考

依指令群組整理的完整指令說明，含子指令與旗標。

- [瀏覽指令](../commands/zh-TW/index.md)
- [瀏覽 manpages](../man/index.html)

### 原始碼與版本庫

想看原始碼、版本發布或回報問題時，直接從這裡進去。

- [瀏覽 GitHub repository](https://github.com/kenduest-brobridge/grafana-util)
- [查看 Releases](https://github.com/kenduest-brobridge/grafana-util/releases)
- [回報問題](https://github.com/kenduest-brobridge/grafana-util/issues)

## 維護者

維護者文件另外整理，不跟公開手冊混在一起。

- [開發者指南](../DEVELOPER.md)
