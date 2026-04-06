# grafana-util
### 專為 Grafana 維運與管理設計的 Rust CLI

[![CI](https://img.shields.io/github/actions/workflow/status/kenduest-brobridge/grafana-util/ci.yml?branch=main)](https://github.com/kenduest-brobridge/grafana-util/actions)
[![License](https://img.shields.io/github/license/kenduest-brobridge/grafana-util)](LICENSE)
[![Version](https://img.shields.io/github/v/tag/kenduest-brobridge/grafana-util)](https://github.com/kenduest-brobridge/grafana-util/tags)

[English](./README.md) | 繁體中文

**標準化 Grafana 維運流程：包含儀表板、告警、資料源、存取控制與操作審查。**

`grafana-util` 是一款專為 Grafana 日常維運設計的 Rust CLI 工具。它把盤點、匯出/匯入、比對、回放、profile 管理與 secret 處理整理在一起，讓 SRE 與平台工程師可以先看清楚，再決定要不要變更。

它的重點是先審查再動手、把儀表板匯入/匯出的不同路徑分清楚，以及用可重複的 profile 讓日常操作保持簡短、穩定。

---

## 支援的工作流

- **儀表板 (Dashboards)**：瀏覽、列表、匯出/匯入、比對 (diff)、審查、發佈與分析。支援 `raw` (API 直接匯入)、`prompt` (UI 匯入) 與 `provisioning` (檔案配置用) 三種路徑。
- **資料來源 (Datasources)**：匯出時可先遮蔽敏感資訊，匯入時再補回認證，也能對應檔案配置用的輸出。
- **告警 (Alerts)**：匯出/匯入、比對 (diff)、計畫與套用 (`plan`/`apply`) 以及路由預覽。
- **存取控制 (Access)**：使用者、團隊、組織、服務帳號 (Service Account) 與 Token 管理。
- **變更管理 (Change)**：先審查再變更的流程 (`inspect`、`check`、`preview`)，讓正式套用前的狀態更清楚。
- **狀態與總覽 (Status / Overview)**：針對即時環境與暫存資源的就緒檢查。
- **設定檔 (Profiles)**：集中管理連線資訊，支援 `file`、`os` (Keyring) 與 `encrypted-file` 等秘密資訊儲存模式。
- **快照 (Snapshot)**：資源套件的匯出與審查。
- **資源 (Resource)**：針對 Grafana 資源的唯讀式 `inspect`/`get`/`list`/`describe` 操作。

---

## 維運模式轉變

| 功能項 | 傳統作法 | 使用 `grafana-util` |
| :--- | :--- | :--- |
| **環境盤點** | 需手動切換 UI 或自行組合 API 呼叫以暸解現況。 | 使用 `overview live` 或 `status live` 快速取得環境統一視圖。 |
| **儀表板路徑** | 難以區分 API 直接匯入與 UI 匯入所需的格式。 | 提供明確的 `raw`、`prompt` 與 `provisioning` 路徑，並具備 `raw-to-prompt` 轉換工具。 |
| **資料來源** | 匯出後的憑證資訊不容易安全保存，也不容易直接對應檔案配置。 | 匯出時先遮蔽敏感資訊，匯入時再補回認證，並保留和檔案配置對應的內容。 |
| **審查機制** | 直接套用變更，缺乏中間審查層。 | 使用 `change inspect`、`check` 與 `preview`，在變動正式伺服器前先完成審查。 |
| **安全性** | 認證資訊容易散落在 Shell 歷史紀錄或明文檔案中。 | 透過 `profile` 搭配作業系統 Keyring 或加密儲存空間管理憑證。 |

---

## 快速上手

### 安裝

```bash
# 使用一鍵安裝腳本
curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-util/main/scripts/install.sh | sh
```

```bash
# 確認安裝版本
grafana-util --version
```

```bash
# 檢視目前 Grafana 狀態
grafana-util overview live --url http://my-grafana:3000 --basic-user admin --prompt-password --output-format interactive
```

### 安裝選項

固定版本安裝：

```bash
# 用途：安裝固定版本。
VERSION=0.9.1 \
  curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-util/main/scripts/install.sh | sh
```

指定安裝目錄：

```bash
# 用途：安裝到指定的 binary 目錄。
BIN_DIR="$HOME/.local/bin" \
  curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-util/main/scripts/install.sh | sh
```

先看安裝腳本說明：

```bash
sh ./scripts/install.sh --help
```

- **發佈頁面**：<https://github.com/kenduest-brobridge/grafana-util/releases>
- **執行檔**：提供 `linux-amd64` 與 `macos-arm64` 標準版。若需截圖功能，請下載 `*-browser-*` 版本。
- **預設路徑**：優先使用 `/usr/local/bin`；若無權限則退回至 `$HOME/.local/bin`。
- **PATH 設定提醒**：如果安裝目錄還沒在 `PATH` 內，安裝腳本會印出對應 `zsh` / `bash` 可直接使用的設定方式。

---

## 實用範例

以下範例展示核心維運流程。連線方式可以直接帶 `--basic-password`，也可以用 `--prompt-password` 互動輸入，或改成 token、用 `bash` / `zsh` 的 `export` 設定環境變數，再搭配 `profile` 設定檔管理。若需完整的連線設定指南，請參考 [開始使用](./docs/user-guide/zh-TW/getting-started.md)。

```bash
# bash / zsh
export GRAFANA_USERNAME=admin
export GRAFANA_PASSWORD=admin
```

如果你想把這些設定放進 profile，也可以直接用 `profile add` 分開存：

```bash
grafana-util profile add prod \
  --url http://my-grafana:3000 \
  --basic-user admin \
  --prompt-password \
  --store-secret os

grafana-util profile add ci \
  --url http://my-grafana:3000 \
  --token-env GRAFANA_CI_TOKEN \
  --store-secret encrypted-file
```

從第二個例子開始，我們先省略連線資訊，讓畫面更簡潔。你還是可以直接帶 `--url`、`--basic-user`、`--basic-password` 或 `--token`，也可以先用 `export` 設好環境變數，或者放進 `profile`，分別管理 username、password、token。

### 1. 檢視環境維運總覽
```bash
grafana-util overview live \
  --url http://my-grafana:3000 \
  --basic-user admin \
  --basic-password admin \
  --output-format interactive
```

### 2. 先列出儀表板
```bash
# 先看看現有內容，再決定要不要匯出或修改。
grafana-util dashboard list --all-orgs --table
```

### 3. 匯出儀表板以供審查
```bash
# 跨組織匯出所有儀表板，建立本地審查目錄樹。
grafana-util dashboard export --all-orgs --output-dir ./backup --progress
```

### 4. 分析儀表板相依性
```bash
# 在匯入前檢查資料源參照是否失效或結構是否異常。
grafana-util dashboard analyze \
  --input-dir ./backup/raw \
  --input-format raw \
  --output-format tree-table
```

### 5. 開啟儀表板互動式工作台
```bash
# 開啟互動式的儀表板分析工作台。
grafana-util dashboard analyze \
  --input-dir ./backup/raw \
  --input-format raw \
  --interactive
```

### 6. 預覽儀表板匯入變更
```bash
grafana-util dashboard import \
  --input-dir ./backup/raw \
  --replace-existing \
  --dry-run \
  --table
```

### 7. 儀表板快速反覆編修
```bash
# 直接把本機產生的 dashboard JSON 送進 review，Grafana 不會被改到。
cat cpu.json | grafana-util dashboard review --input - --output-format json
```

### 8. 先看告警會怎麼變
```bash
# 先看看這次改動會影響哪些告警。
grafana-util alert plan --desired-dir ./alerts/desired --prune

# 先預覽告警最後會送到哪裡。
grafana-util alert preview-route \
  --desired-dir ./alerts/desired \
  --label team=sre --severity critical
```

### 9. 資料源匯出與還原
```bash
# 匯出時先遮蔽敏感資訊，匯入時再把連線資訊補回去。
grafana-util datasource export --output-dir ./datasources
grafana-util datasource import --input-dir ./datasources --prompt-password
```

---

## 文件入口

請參考手冊瞭解維運情境，或參考指令頁面取得精確語法說明。

如果您偏好瀏覽器介面，請開啟本地 HTML 文件 [docs/html/index.html](./docs/html/index.html)，或造訪官方文件站：<https://kenduest-brobridge.github.io/grafana-util/>。

依需求進入：

*   **開始使用**：[docs/user-guide/zh-TW/getting-started.md](./docs/user-guide/zh-TW/getting-started.md)
*   **完整手冊**：[docs/user-guide/zh-TW/index.md](./docs/user-guide/zh-TW/index.md)
*   **指令詳細說明**：[docs/commands/zh-TW/index.md](./docs/commands/zh-TW/index.md)
*   **疑難排解**：[docs/user-guide/zh-TW/troubleshooting.md](./docs/user-guide/zh-TW/troubleshooting.md)
*   **Man Page**：[docs/man/grafana-util.1](./docs/man/grafana-util.1)

依角色進入：

*   **新使用者**：[新手快速入門](./docs/user-guide/zh-TW/role-new-user.md)
*   **SRE / 維運人員**：[SRE / 維運角色導讀](./docs/user-guide/zh-TW/role-sre-ops.md)
*   **自動化 / CI 維護者**：[自動化 / CI 角色導讀](./docs/user-guide/zh-TW/role-automation-ci.md)
*   **維護者 / 開發者**：[docs/DEVELOPER.md](./docs/DEVELOPER.md) 與 [docs/internal/maintainer-quickstart.md](./docs/internal/maintainer-quickstart.md)

---

## 持續開發中
本專案目前由社群持續維護。指令介面與文件仍會持續演進，精確語法請以指令說明頁面為準。

## 參與貢獻
我們歡迎任何形式的貢獻！請參閱 [開發者指南](./docs/DEVELOPER.md) 瞭解設定步驟。
