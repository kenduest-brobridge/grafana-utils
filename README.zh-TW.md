# grafana-state-kit
### 面向 SRE 與維運流程的 review-first Grafana 盤點、workspace 預覽與安全套用工具

[![CI](https://img.shields.io/github/actions/workflow/status/kenduest-brobridge/grafana-state-kit/ci.yml?branch=main)](https://github.com/kenduest-brobridge/grafana-state-kit/actions)
[![License](https://img.shields.io/github/license/kenduest-brobridge/grafana-state-kit)](LICENSE)
[![Version](https://img.shields.io/github/v/tag/kenduest-brobridge/grafana-state-kit)](https://github.com/kenduest-brobridge/grafana-state-kit/tags)

[English](./README.md) | 繁體中文

**先看 live state，再進 workspace 預覽、審查差異，最後才 workspace 套用。**

`grafana-state-kit` 提供 `grafana-util` 這個 Rust CLI，給 Grafana 維運、SRE 與 Dashboard 開發人員使用。它可以協助檢查 live Grafana 資源、封裝本地 workspace、在變更落地前先 workspace 預覽，並在審查後再 workspace 套用到不同環境。它不是單純把 API 包一層，而是把日常 Grafana 工作整理成以 `workspace scan`、`workspace preview` 與 `workspace apply` 為中心的 review-first 流程。

這個工具不是單純把 API 包一層，而是把日常常用的狀態查看、匯出匯入、差異比對、工作區預覽、安全套用、不同環境的連線設定檔與認證密鑰處理，整理成一套可以重複執行的流程。要查狀態、備份、比對或套用變更時，不用在不同腳本和手動操作之間切來切去。

開發者前言：

這個工具來自我自己在 Grafana 專案裡反覆遇到的幾種情境：

1. Dashboard 常常先在 Lab 或本機 Grafana 開發，完成後還需要匯出到其他環境重複使用。
2. Dashboard 的修改方式不一定只在 Grafana Web UI 裡完成，有時也會透過 AI Agent 直接修改本地 Grafana JSON。
3. 客戶環境裡有許多 Dashboard 需要匯出、調整後再匯入；比較安全的做法，是先匯入到本機開發環境確認。
4. 維運時常需要盤點 Dashboard 使用了哪些 Data Source，也需要了解目前有哪些 user、team、帳號群組與權限設定。

Grafana 本身對 Dashboard Developer、SRE 或內部使用者來說，還沒有把這些流程整理成一個方便操作的工具。`grafana-state-kit` 就是為了透過 `grafana-util` CLI 補上這段日常維運與開發流程而做的。

## 採用前後對照

| 採用前 | 採用後 |
| :--- | :--- |
| live 檢查、本地 JSON 修改、dashboard 匯出與套用步驟分散在不同腳本或 UI 操作裡。 | 先跑 `grafana-util status live`，檢查 workspace，再跑 `grafana-util workspace preview`，審查後才套用。 |
| Dashboard 相依性審查需要手動打開 panel 和 data source 設定。 | 用 `grafana-util dashboard analyze --input-dir ./dashboards/raw --input-format raw --output-format dependency` 產生可審查的相依性報告。 |

常見用途：

| 你想做什麼 | 先從這裡開始 |
| :--- | :--- |
| 確認 Grafana 是否可連線 | `grafana-util status live` |
| 保存可重複使用的連線設定 | `grafana-util config profile add ...` |
| 匯出或審查 dashboards | `grafana-util export dashboard` 或 `grafana-util dashboard summary` |
| 套用前先審查本地變更 | `grafana-util workspace scan` 再跑 `workspace preview` |
| 處理 alerts 或 route 預覽 | `grafana-util alert plan` 或 `alert preview-route` |
| 管理 user、team、org 與 service accounts | `grafana-util access ...` |

CLI 主要圍繞這幾個指令家族：`status`、`workspace`、`dashboard`、`datasource`、`alert`、`access`、`config profile`。工作流程脈絡請看 handbook，精確語法請看 command reference。

支援的 Grafana 面向：

| 面向 | 目前涵蓋 | 建議先跑 |
| :--- | :--- | :--- |
| Dashboards | 瀏覽、列表、匯出/匯入、比對、審查、修補、發布、歷史版本、相依性分析、政策檢查、截圖、面板轉換成不同環境再次匯入支援。 | `grafana-util dashboard browse` |
| Datasources | 盤點、匯出/匯入、比對、建立/修改/刪除、密鑰感知復原、類型探索。 | `grafana-util datasource list` |
| Alerting | 規則、contact points、mute timings、templates、notification routes、審查計畫、套用流程、route 預覽。 | `grafana-util alert plan` |
| Access | org、user、team、service accounts、service-account tokens、匯出/匯入、比對、刪除前審查。 | `grafana-util access user list` |
| Status 與 workspace | live readiness、資源盤點、本地 workspace scan/test/preview/package/apply、適合 CI 的檢查。 | `grafana-util status live` |
| Profiles 與 secrets | repo-local 連線 profiles、直接旗標、環境變數驗證、互動輸入、支援的認證密鑰不同方式儲存。 | `grafana-util config profile add` |

---

## 安裝

安裝最新版本：

```bash
curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-state-kit/main/scripts/install.sh | sh
```

安裝最新版本，並替目前 shell 寫入 completion：

```bash
curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-state-kit/main/scripts/install.sh | INSTALL_COMPLETION=auto sh
```

互動安裝，依提示選擇安裝目錄與是否啟用 shell completion：

```bash
curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-state-kit/main/scripts/install.sh | sh -s -- --interactive
```

指定安裝版本：

```bash
curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-state-kit/main/scripts/install.sh | VERSION=0.10.2 sh
```

安裝到自訂目錄：

```bash
curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-state-kit/main/scripts/install.sh | BIN_DIR="$HOME/.local/bin" sh
```

查看本地 installer 說明：

```bash
sh ./scripts/install.sh --help
```

用同一套 installer 流程安裝並驗證目前 local 環境 checkout 進行 build：

```bash
make install-local-interactive
```

- **Releases**：[GitHub releases](https://github.com/kenduest-brobridge/grafana-state-kit/releases)
- **執行檔**：標準版提供 `linux-amd64` 與 `macos-arm64`；需要截圖功能請選 `*-browser-*`
- **預設路徑**：優先 `/usr/local/bin`，否則改用 `$HOME/.local/bin`
- **Completion**：設定 `INSTALL_COMPLETION=auto`、`INSTALL_COMPLETION=bash` 或 `INSTALL_COMPLETION=zsh`，即可用下載後的 binary 產生並安裝 completion
- **互動安裝**：pipe 後使用 `sh -s -- --interactive`，即可依提示選擇安裝目錄與 completion 設定；Zsh 安裝也可以協助更新 `~/.zshrc`，讓 `~/.zfunc` 在 `compinit` 前載入
- **本地安裝測試**：使用 `make install-local` 或 `make install-local-interactive`，可用 `scripts/install.sh` 安裝目前 checkout build

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

Zsh 請確認 `~/.zfunc` 已經在 `compinit` 之前放進 `fpath`。互動安裝可以替你把這段設定加到 `~/.zshrc`，並清掉過期的 `.zcompdump*` completion cache。

---

## 第一次執行

三步完成第一次工作階段：

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

- 看完整流程：[新手快速入門](https://kenduest-brobridge.github.io/grafana-state-kit/handbook/zh-TW/role-new-user.html)
- 查精確語法：[指令參考](https://kenduest-brobridge.github.io/grafana-state-kit/commands/zh-TW/index.html)

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

以指定 profile 的連線組態列出 dashboards：

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

手冊用來看流程脈絡；指令參考用來查精確語法。

- [官方文件站](https://kenduest-brobridge.github.io/grafana-state-kit/)
- 第一次設定：[開始使用](https://kenduest-brobridge.github.io/grafana-state-kit/handbook/zh-TW/getting-started.html) 與 [新手快速入門](https://kenduest-brobridge.github.io/grafana-state-kit/handbook/zh-TW/role-new-user.html)
- 日常維運流程：[維運導引手冊](https://kenduest-brobridge.github.io/grafana-state-kit/handbook/zh-TW/index.html) 與 [SRE / 維運角色導讀](https://kenduest-brobridge.github.io/grafana-state-kit/handbook/zh-TW/role-sre-ops.html)
- 查精確指令語法：[指令參考](https://kenduest-brobridge.github.io/grafana-state-kit/commands/zh-TW/index.html) 與 `grafana-util --help`
- [疑難排解](https://kenduest-brobridge.github.io/grafana-state-kit/handbook/zh-TW/troubleshooting.html)

版本庫內維護文件：

- **本地 HTML 文件入口**：[docs/html/index.html](./docs/html/index.html)
- **維護者文件**：[docs/DEVELOPER.md](./docs/DEVELOPER.md)
- **Manpage source**：[docs/man/grafana-util.1](./docs/man/grafana-util.1)

依角色開始：

- [新使用者](https://kenduest-brobridge.github.io/grafana-state-kit/handbook/zh-TW/role-new-user.html)
- [SRE / 維運人員](https://kenduest-brobridge.github.io/grafana-state-kit/handbook/zh-TW/role-sre-ops.html)
- [自動化 / CI 維護者](https://kenduest-brobridge.github.io/grafana-state-kit/handbook/zh-TW/role-automation-ci.html)
- **維護者 / 開發者**：[docs/DEVELOPER.md](./docs/DEVELOPER.md)

---

## 專案狀態

此專案仍在積極開發中。CLI 路徑、help 輸出、範例與文件結構可能會有異動。指令介面請以 command reference 和 `--help` 輸出為準。

---

## 貢獻

若要看開發環境設定與 maintainer 指南，請直接使用 [docs/DEVELOPER.md](./docs/DEVELOPER.md)。
