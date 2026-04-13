# grafana-util
### 專為 Grafana 維運與管理設計的 Rust CLI

[![CI](https://img.shields.io/github/actions/workflow/status/kenduest-brobridge/grafana-util/ci.yml?branch=main)](https://github.com/kenduest-brobridge/grafana-util/actions)
[![License](https://img.shields.io/github/license/kenduest-brobridge/grafana-util)](LICENSE)
[![Version](https://img.shields.io/github/v/tag/kenduest-brobridge/grafana-util)](https://github.com/kenduest-brobridge/grafana-util/tags)

[English](./README.md) | 繁體中文

**用先審查再變更的方式處理 Grafana 的 dashboard、alert、datasource、access control 與 workspace 變更。**

`grafana-util` 是一個給日常 Grafana 維運使用的 Rust CLI。它把唯讀檢查、匯出/匯入、比對、workspace 審查、連線 profile 與密鑰處理收斂到同一個指令介面，讓維運者可以先看清楚，再決定要不要變更。

常見用途：

| 你想做什麼 | 先從這裡開始 |
| :--- | :--- |
| 確認 Grafana 是否可連線 | `grafana-util status live` |
| 保存可重複使用的連線設定 | `grafana-util config profile add ...` |
| 匯出或審查 dashboards | `grafana-util export dashboard` 或 `grafana-util dashboard summary` |
| 套用前先審查本地變更 | `grafana-util workspace scan` 再跑 `workspace preview` |
| 處理 alerts 或 route 預覽 | `grafana-util alert plan` 或 `alert preview-route` |
| 管理 user、team、org 與 service accounts | `grafana-util access ...` |

CLI 主要圍繞幾個穩定 root：`status`、`workspace`、`dashboard`、`datasource`、`alert`、`access`、`config profile`。工作流程脈絡請看 handbook，精確語法請看 command reference。

支援的 Grafana 面向：

| 面向 | 目前涵蓋 | 建議先跑 |
| :--- | :--- | :--- |
| Dashboards | 瀏覽、列表、匯出/匯入、比對、審查、修補、發布、歷史版本、相依性分析、政策檢查、截圖、raw-to-prompt 轉換。 | `grafana-util dashboard browse` |
| Datasources | 盤點、匯出/匯入、比對、建立/修改/刪除、密鑰感知復原、類型探索。 | `grafana-util datasource list` |
| Alerting | 規則、contact points、mute timings、templates、notification routes、審查計畫、套用流程、route 預覽。 | `grafana-util alert plan` |
| Access | org、user、team、service accounts、service-account tokens、匯出/匯入、比對、刪除前審查。 | `grafana-util access user list` |
| Status 與 workspace | live readiness、資源盤點、本地 workspace scan/test/preview/package/apply、適合 CI 的檢查。 | `grafana-util status live` |
| Profiles 與 secrets | repo-local 連線 profiles、直接旗標、環境變數驗證、互動輸入、支援的密鑰儲存。 | `grafana-util config profile add` |

---

## 安裝

安裝最新版本：

```bash
curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-util/main/scripts/install.sh | sh
```

安裝最新版本，並替目前 shell 寫入 completion：

```bash
curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-util/main/scripts/install.sh | INSTALL_COMPLETION=auto sh
```

互動安裝，依提示選擇安裝目錄與是否啟用 shell completion：

```bash
curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-util/main/scripts/install.sh | sh -s -- --interactive
```

指定安裝版本：

```bash
curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-util/main/scripts/install.sh | VERSION=0.10.0 sh
```

安裝到自訂目錄：

```bash
curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-util/main/scripts/install.sh | BIN_DIR="$HOME/.local/bin" sh
```

查看本地 installer 說明：

```bash
sh ./scripts/install.sh --help
```

- **Releases**：[GitHub releases](https://github.com/kenduest-brobridge/grafana-util/releases)
- **執行檔**：標準版提供 `linux-amd64` 與 `macos-arm64`；需要截圖功能請選 `*-browser-*`
- **預設路徑**：優先 `/usr/local/bin`，否則改用 `$HOME/.local/bin`
- **Completion**：設定 `INSTALL_COMPLETION=auto`、`INSTALL_COMPLETION=bash` 或 `INSTALL_COMPLETION=zsh`，即可用下載後的 binary 產生並安裝 completion
- **互動安裝**：pipe 後使用 `sh -s -- --interactive`，即可依提示選擇安裝目錄與 completion 設定

Shell completion：

```bash
# Bash
mkdir -p ~/.local/share/bash-completion/completions
grafana-util completion bash > ~/.local/share/bash-completion/completions/grafana-util
```

```zsh
# Zsh
mkdir -p ~/.zfunc
grafana-util completion zsh > ~/.zfunc/_grafana-util
```

Zsh 請確認 `~/.zfunc` 已經在 `compinit` 之前放進 `fpath`。

---

## 第一次執行

用這組路線完成第一次成功執行：

```bash
# 1. 先確認 CLI 已安裝。
grafana-util --version
```

```bash
# 2. 先跑一個唯讀 live 檢查。
grafana-util status live \
  --url http://grafana.example:3000 \
  --basic-user admin \
  --prompt-password \
  --output-format yaml
```

```bash
# 3. 把同一組連線存成可重複使用的 profile。
grafana-util config profile add dev \
  --url http://grafana.example:3000 \
  --basic-user admin \
  --prompt-password
```

接下來：

- 看完整流程：[新手快速入門](https://kenduest-brobridge.github.io/grafana-util/handbook/zh-TW/role-new-user.html)
- 查精確語法：[指令參考](https://kenduest-brobridge.github.io/grafana-util/commands/zh-TW/index.html)

---

## 範例指令

確認 Grafana 是否可連線：

```bash
grafana-util status live --profile prod --output-format interactive
```

保存可重複使用的連線 profile：

```bash
grafana-util config profile add prod \
  --url http://grafana.example:3000 \
  --basic-user admin \
  --prompt-password
```

匯出 dashboards：

```bash
grafana-util export dashboard --profile prod --output-dir ./backup --overwrite
```

列出 dashboards，不先產生匯出檔：

```bash
grafana-util dashboard list --profile prod
```

列出 datasources：

```bash
grafana-util datasource list --profile prod
```

查某個 command family 的精確語法：

```bash
grafana-util dashboard --help
grafana-util config profile --help
```

---

## 文件

handbook 用來看 workflow 脈絡。command reference 用來查精確 CLI 語法。

- [官方文件站](https://kenduest-brobridge.github.io/grafana-util/)
- 第一次設定：[開始使用](https://kenduest-brobridge.github.io/grafana-util/handbook/zh-TW/getting-started.html) 與 [新手快速入門](https://kenduest-brobridge.github.io/grafana-util/handbook/zh-TW/role-new-user.html)
- 日常維運流程：[維運導引手冊](https://kenduest-brobridge.github.io/grafana-util/handbook/zh-TW/index.html) 與 [SRE / 維運角色導讀](https://kenduest-brobridge.github.io/grafana-util/handbook/zh-TW/role-sre-ops.html)
- 查精確指令語法：[指令參考](https://kenduest-brobridge.github.io/grafana-util/commands/zh-TW/index.html) 與 `grafana-util --help`
- [疑難排解](https://kenduest-brobridge.github.io/grafana-util/handbook/zh-TW/troubleshooting.html)

若你是在版本庫裡直接維護文件，可使用本地 HTML mirror 與原始 source：

- **本地 HTML 文件入口**：[docs/html/index.html](./docs/html/index.html)
- **維護者文件**：[docs/DEVELOPER.md](./docs/DEVELOPER.md)
- **Manpage source**：[docs/man/grafana-util.1](./docs/man/grafana-util.1)

依角色開始：

- [新使用者](https://kenduest-brobridge.github.io/grafana-util/handbook/zh-TW/role-new-user.html)
- [SRE / 維運人員](https://kenduest-brobridge.github.io/grafana-util/handbook/zh-TW/role-sre-ops.html)
- [自動化 / CI 維護者](https://kenduest-brobridge.github.io/grafana-util/handbook/zh-TW/role-automation-ci.html)
- **維護者 / 開發者**：[docs/DEVELOPER.md](./docs/DEVELOPER.md)

---

## 專案狀態

這個專案目前仍在積極開發中。CLI 路徑、help 輸出、範例寫法與文件結構，都可能在不同版本之間出現明顯調整。

若要確認目前版本的指令介面，請優先以指令參考與 `--help` 輸出為準，不要直接依賴舊 issue、舊片段或先前版本的範例。

---

## 貢獻

若要看開發環境設定與 maintainer 指南，請直接使用 [docs/DEVELOPER.md](./docs/DEVELOPER.md)。
