# grafana-util
### 專為 Grafana 維運與管理設計的 Rust CLI

[![CI](https://img.shields.io/github/actions/workflow/status/kenduest-brobridge/grafana-util/ci.yml?branch=main)](https://github.com/kenduest-brobridge/grafana-util/actions)
[![License](https://img.shields.io/github/license/kenduest-brobridge/grafana-util)](LICENSE)
[![Version](https://img.shields.io/github/v/tag/kenduest-brobridge/grafana-util)](https://github.com/kenduest-brobridge/grafana-util/tags)

[English](./README.md) | 繁體中文

**提供 dashboard、alert、datasource、access control 與維運審查所需的可重複操作流程。**

`grafana-util` 是我長期維護的一個 Rust 個人工具，出發點是處理自己在 Grafana 維運上反覆遇到的痛點。它把 dashboard、alert、datasource、access control 與整體狀態檢查這些日常工作，整理成比較可審查、帶治理意識、也更容易重複執行的流程。它主要面向 SRE、平台工程師、sysadmin 與維護者，適合那些不想只靠零散 API 呼叫、純 UI 點選或一次性腳本的人。

它不是要變成完整的 Grafana 平台，也不是要取代所有其他 CLI。這個工具比較明確的設計重心是維運流程本身：先 inspect，再 review，變更前先看清楚，secret 要有意識地處理，盡量把操作收斂成可重複的路徑。

如果你也知道 `grafanactl` 或 `grizzly`，這裡比較適合把差異理解成「設計取向不同」：

- `grafanactl` 比較接近通用的 Grafana 資源/API 操作 CLI。
- `grizzly` 比較接近宣告式的 Grafana-as-code 管理流程。
- `grafana-util` 目前更偏向可審查操作、inspection/governance 流程，以及較安全的搬移或回放路徑。

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

## 採用前後對照

| 原本常見做法 | 改用 `grafana-util` 後 |
| :--- | :--- |
| 想看 Grafana 全貌時，只能一直切 UI 或自己拼 API。 | 先跑 `overview live` 或 `status live`，快速知道下一步該看哪裡。 |
| 匯出/匯入像一次性動作，缺少中間檢查點。 | 先匯出、再盤點依賴、再 dry-run，最後才決定要不要回放。 |
| Grafana 變更很難在套用前說清楚會改到什麼。 | 先跑 `change inspect`、`change check`、`change preview`，再決定要不要做 live mutation。 |
| 認證資訊容易散落在 shell history 或平面檔案裡。 | 改用 prompt、環境變數或 profile secret 模式整理起來。 |

重點不是多幾個 command，而是把維運順序收斂成比較安全、可審查的流程。

---

## 快速上手

### 安裝

```bash
# 1. 一鍵安裝 (全域 Binary)
curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-util/main/scripts/install.sh | sh
```

```bash
# 2. 確認安裝版本
grafana-util --version
```

```bash
# 3. 檢視目前 Grafana 狀態
grafana-util overview live --url http://my-grafana:3000 --basic-user admin --prompt-password --output-format interactive
```

### 安裝選項

固定版本安裝：

```bash
# 用途：安裝固定版本。
VERSION=0.8.0 \
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

- Release 下載頁：<https://github.com/kenduest-brobridge/grafana-util/releases>
- 已發布 binary：目前提供 `linux-amd64` 與 `macos-arm64` 的標準版。若需要支援瀏覽器截圖的版本，請到同一個 release 下載 `*-browser-*` 壓縮檔。
- 預設安裝位置：若有設定 `BIN_DIR` 就優先使用；否則會先嘗試可寫入的 `/usr/local/bin`，再退回 `$HOME/.local/bin`。
- PATH 設定提醒：如果安裝目錄還沒在 `PATH` 內，安裝腳本會印出對應 `zsh` / `bash` 可直接使用的設定方式。

---

## 實用範例

這裡放的是大多數人第一次真的會用到的例子：先看 live 狀況、再匯出成可審查的目錄、匯入前先檢查、告警先看計畫、最後再處理 datasource 的 secret 恢復。

以下範例重點放在工作流程本身，所以後面不會每次都重複把連線參數寫滿。實際操作時，你可以用 `--url`、`--basic-user`、`--basic-password`、`--prompt-password`、`--token` 或 `--profile` 提供 Grafana 連線資訊；部分命令也支援 `GRAFANA_USERNAME`、`GRAFANA_PASSWORD`、`GRAFANA_API_TOKEN` 等環境變數。如果你要先把連線與認證方式弄清楚，請先看 [開始使用](./docs/user-guide/zh-TW/getting-started.md)。

### 1. 變更前先看 live 環境全貌

```bash
# 在終端機中打開目前 Grafana 環境的互動式總覽。
grafana-util overview live \
  --url http://my-grafana:3000 \
  --basic-user admin \
  --prompt-password \
  --output-format interactive
```

當你只是想先知道「現在這套 Grafana 到底長什麼樣」時，這通常是最好的起點。

預期你會先看到類似：

```text
Live status: ready
Dashboards: ...
Alerts: ...
Datasources: ...
```

### 2. 把 dashboards 匯出成可審查的目錄樹

```bash
# 跨組織匯出 dashboard，建立本地備份與審查基礎。
grafana-util dashboard export --all-orgs --export-dir ./backup --progress
```

這是做備份、搬移、審查和 CI 檢查的起點。

### 3. 匯入前先檢查 export tree 是否安全

```bash
# 盤點匯出目錄中的 datasource 相依性與結構問題。
grafana-util dashboard analyze \
  --import-dir ./backup/raw \
  --input-format raw \
  --output-format dependency
```

如果你想先抓出失效的 datasource 參照或可疑結構，這一步很有用。

預期你會先看到像：

```text
Dependency report:
  prometheus-main
  loki-prod
```

### 4. 正式匯入前先預覽會發生什麼事

```bash
# 先 dry-run dashboard 匯入，表格化顯示預計變更。
grafana-util dashboard import \
  --import-dir ./backup/raw \
  --replace-existing \
  --dry-run \
  --table
```

適合在真正碰 live Grafana 之前，先看會新增、覆蓋或變動哪些項目。

### 5. 單一 dashboard 草稿快速反覆編修

```bash
# 直接從標準輸入 review 一份 generator 產生的 dashboard。
jsonnet dashboards/cpu.jsonnet | \
  grafana-util dashboard review --input - --output-format json
```

```bash
# 不落中繼暫存檔，直接把 generator 產生的 dashboard 發佈到 Grafana。
jsonnet dashboards/cpu.jsonnet | \
  grafana-util dashboard publish \
    --url http://localhost:3000 \
    --token "$GRAFANA_API_TOKEN" \
    --input - \
    --replace-existing
```

```bash
# 本地編修同一份草稿時，每次儲存後自動重跑 dry-run publish。
grafana-util dashboard publish \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --input ./drafts/cpu-main.json \
  --dry-run \
  --watch
```

### 6. 告警變更先審查，再套用

```bash
# 依 desired state 與 live server 建立可審查的 alert 計畫。
grafana-util alert plan \
  --desired-dir ./alerts/desired \
  --prune \
  --output-format json
```

```bash
# 在 apply 前先預覽某組 critical 告警實際會怎麼路由。
grafana-util alert preview-route \
  --desired-dir ./alerts/desired \
  --label team=sre \
  --severity critical
```

這兩步適合用在你不想直接改 live 告警，而是想先有 review surface 的情境。

### 7. 匯出 datasource，之後再恢復 secret 匯回

```bash
# 匯出 data source，secret 會遮蔽，方便審查或納入版本控制。
grafana-util datasource export --export-dir ./datasources --overwrite
```

```bash
# 匯回時再互動式補回必要 secret。
grafana-util datasource import \
  --import-dir ./datasources \
  --replace-existing \
  --prompt-password
```

這是把 datasource 設定在環境間搬移，又不想把原始憑證直接寫進檔案時最實用的流程。

---

## 第一條實用工作流

如果你現在只想知道這工具到底怎麼幫忙，先照這條順序走：

1. 用 `overview live` 確認目標 Grafana 真的連得到
2. 用 `dashboard export` 匯出成可審查的目錄樹
3. 用 `grafana-util dashboard analyze --import-dir ./dashboards/raw --input-format raw --output-format dependency` 先抓出缺少的 datasource 依賴
4. 用 `dashboard import --dry-run` 預覽回放結果，再決定要不要動 live

這是最短、也最能感受到工具價值的一條公開工作流。

---

## 快速掌握

*   **先看清楚，再決定要不要動**：`overview`、`status`、匯出檢查與 governance 檢查，讓你先知道環境現況與風險。
*   **把 Grafana 資產安全搬移與回放**：針對 dashboard、alert、data source、access 資源提供可審查的匯出/匯入流程。
*   **讓維運流程可重複、可自動化**：提供表格/JSON 導向輸出、非互動操作路徑與較安全的 secret 處理方式。

---

## 文件入口

手冊負責說明工作流程與維運脈絡，指令頁則負責提供目前 CLI 的精確語法。這裡只做快速導引，不再把 README 寫成第二份完整手冊。

如果直接讀 Markdown 不方便，請先產生本機 HTML 文件站，再開啟入口頁：

```bash
# 用途：產生本機 HTML 文件站並開啟主入口頁。
make html
open ./docs/html/index.html
```

在 Linux 上請把 `open` 換成 `xdg-open`。如果要直接用瀏覽器看公開版，請使用 <https://kenduest-brobridge.github.io/grafana-utils/>。

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

## 專案說明
*   **Rust 為主體**：主要實作位於 `rust/src/`。
*   **驗證環境**：在 Docker 環境下針對 **Grafana 12.4.1** 進行驗證。
*   **自動化友善**：提供可預測的 exit code 與結構化輸出，便於 CI/CD 與批次流程整合。

---

## 參與貢獻
我們歡迎任何形式的貢獻！請參閱 [開發者指南](./docs/DEVELOPER.md) 了解設定步驟。

---
