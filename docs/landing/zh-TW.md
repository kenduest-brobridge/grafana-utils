# 從這裡開始

先從手冊看完整脈絡，或直接進指令參考查精確語法。

- [從手冊開始](../user-guide/zh-TW/index.md)
- [直接查指令](../commands/zh-TW/index.md)

## 第一次執行

如果你是在新機器或新的 Grafana 環境開始，請照這個順序：

### 先確認 binary 可用

先跑 `grafana-util --version`，確認 CLI 已安裝完成，而且 shell 找得到它。

- [開始使用](../user-guide/zh-TW/getting-started.md)
- [Version 指令參考](../commands/zh-TW/version.md)

### 先做第一個唯讀檢查

把 `grafana-util status live` 當成第一次對 Grafana 的即時唯讀讀取，不要一開始就跳進更大的流程。

- [開始使用](../user-guide/zh-TW/getting-started.md)
- [Status 指令參考](../commands/zh-TW/status.md)

### 建立可重用的 connection profile

第一次成功連線後，再把 host 與 credentials 收進 `grafana-util config profile add ...`。

- [新手快速入門](../user-guide/zh-TW/role-new-user.md)
- [Profile 指令參考](../commands/zh-TW/profile.md)

## 依角色開始

依照今天操作者的角色，先走最合適的閱讀路線。

### 新使用者

如果你是第一次用 CLI 或第一次連這個 Grafana 環境，先走這條最安全的路線。

- [新手快速入門](../user-guide/zh-TW/role-new-user.md)
- [開始使用](../user-guide/zh-TW/getting-started.md)

### SRE / 維運人員

這條路線從日常維運、變更前檢查與 workspace 審查開始，不把你直接丟進指令表。

- [SRE / 維運角色導讀](../user-guide/zh-TW/role-sre-ops.md)
- [Workspace 審查與狀態](../user-guide/zh-TW/status-workspace.md)

### 自動化 / CI

如果 CLI 會跑在 pipeline、release automation 或重複驗證流程裡，從這裡開始。

- [自動化 / CI 角色導讀](../user-guide/zh-TW/role-automation-ci.md)
- [技術參考手冊](../user-guide/zh-TW/reference.md)

### 維護者 / 架構師

這條路線是給 repository 結構、設計規則與實作契約使用的，不是給一般 operator 的入門頁。

- [系統架構與設計原則](../user-guide/zh-TW/architecture.md)
- [開發者指南](../DEVELOPER.md)

## 依任務開始

如果你是帶著工作目標進來，就從這裡挑對應章節，不必先猜命令名字。

### 先理解工具定位

如果你還不確定這個 CLI 要保護什麼、要解決什麼問題，先補這個心智模型。

- [這個工具是做什麼的](../user-guide/zh-TW/what-is-grafana-util.md)

### 看 live 或 staged 狀態

用在唯讀檢查、staged 審查或任何變更前的狀態確認。

- [Workspace 審查與狀態](../user-guide/zh-TW/status-workspace.md)
- [Status 指令參考](../commands/zh-TW/status.md)

### 處理 dashboard

這條路線包含 browse、export、summary、review、patch、publish 與 screenshot 等 dashboard 工作流。

- [Dashboard 管理](../user-guide/zh-TW/dashboard.md)
- [Dashboard 指令參考](../commands/zh-TW/dashboard.md)

### 處理 data source 或 alert

如果你要改的是 Grafana 整合、告警規則、contact point 或治理檢查，從這裡進去。

- [Data source 管理](../user-guide/zh-TW/datasource.md)
- [告警治理](../user-guide/zh-TW/alert.md)

### 管理 access 與 credentials

用在 org、team、service account、token 與權限導向的操作變更。

- [Access 管理](../user-guide/zh-TW/access.md)
- [Access 指令參考](../commands/zh-TW/access.md)

## 依指令群組瀏覽

如果你已經知道要找哪個 command family，這裡是最短的章節與語法入口。

### `status` 與 `workspace`

這兩個 root 對應檢查、審查與 workspace 驗證流程。

- [Status / Workspace 章節](../user-guide/zh-TW/status-workspace.md)
- [Workspace 指令參考](../commands/zh-TW/workspace.md)

### `config profile`

這條路線專門處理重複使用的連線預設與 secret storage。

- [開始使用](../user-guide/zh-TW/getting-started.md)
- [Profile 指令參考](../commands/zh-TW/profile.md)

### `dashboard`

這個 root 處理 browse、summary、variables、export、diff、patch、publish 與 screenshot。

- [Dashboard 章節](../user-guide/zh-TW/dashboard.md)
- [Dashboard 指令參考](../commands/zh-TW/dashboard.md)

### `datasource`、`alert`、`access`

這三個 root 對應整合、告警與 Grafana 身分面向的變更工作流。

- [Datasource 指令參考](../commands/zh-TW/datasource.md)
- [Alert 指令參考](../commands/zh-TW/alert.md)
- [Access 指令參考](../commands/zh-TW/access.md)

## 完整參考

如果你要的是完整覆蓋，而不是精簡入口，請從這些完整面向進去。

### 讀完整手冊

如果你要的是章節脈絡、操作順序與建議閱讀路線，就走手冊。

- [維運導引手冊](../user-guide/zh-TW/index.md)

### 讀完整指令參考

如果你要的是逐指令頁面、subcommand 導流與穩定語法查找，就走指令參考。

- [指令參考](../commands/zh-TW/index.md)
- [grafana-util(1)](../html/man/grafana-util.html)

### 看 source 與 release

如果你要的是變更歷史、release notes 或 issue 狀態，改走版本庫面向。

- [GitHub repository](https://github.com/kenduest-brobridge/grafana-util)
- [GitHub releases](https://github.com/kenduest-brobridge/grafana-util/releases)
- [Issue tracker](https://github.com/kenduest-brobridge/grafana-util/issues)

## 維護者

維護者文件會留在 repo 內部文件，不跟公開手冊混在一起。

- [開發者指南](../DEVELOPER.md)
