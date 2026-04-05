# 🚀 開始使用 (Getting Started)

本章說明 `grafana-util` 目前的首次使用流程。

## 適用對象

- 第一次接觸這個工具的人
- 還在確認連線、profile 與驗證方式的人
- 想先跑安全唯讀檢查，再進入深一點章節的人

## 主要目標

- 先把安裝、連線與 profile 建起來
- 先確認即時讀取 (live read) 能穩定成功
- 再決定要走 Dashboard、Alert、Access 或 CI 路線

先講最重要的設計：`grafana-util` 支援不只一種連線方式。你可以：

- 每次執行時直接帶 `--url` 與驗證旗標
- 用 `--prompt-password` 或 `--prompt-token` 互動輸入
- 讓環境變數提供帳號、密碼或 token
- 把常用預設整理進專案本地的 profile，再用 `--profile` 重複使用

profile 的重點是讓日常操作少重打重複參數，不是代表一開始只能用 profile。

## 採用前後對照

- 以前：每條命令都要重打 Grafana 位址與驗證旗標。
- 現在：你可以先用直接旗標證明連線沒問題，再把重複設定整理進 profile。

## 成功判準

- binary 已經安裝好，而且 shell 找得到。
- 至少一條直接的 live 唯讀命令成功。
- 你知道下一步是 `--profile`、環境變數，還是一次性的 bootstrap。

## 失敗時先檢查

- 如果 binary 不在 `PATH`，先修安裝，不要先跳到 profile。
- 如果 direct live read 失敗，先停在這一步，不要繼續變更流程。
- 如果 profile 沒有解析成你預期的欄位，先檢查 profile 檔與環境變數來源。

## 前 10 分鐘完成後，應該長什麼樣

讀完這一章後，理想狀態應該是：

- binary 已經能從你的 shell 直接執行
- 至少一條直接的 live 唯讀命令成功
- 你知道自己現在是用 Basic auth、token、環境變數，還是 `--profile`
- 你已經為同一個目標 Grafana 建好一個可重複使用的 profile
- 你知道下一步該去看 dashboard、alert、access，還是自動化路線

如果你還做不到這些，先停在第一個失敗的唯讀命令，搭配 [疑難排解與名詞解釋](troubleshooting.md) 找原因，不要急著往變更或匯入流程走。

若要對照本章提到的旗標，請一併參考 [profile](../../commands/zh-TW/profile.md)、[status](../../commands/zh-TW/status.md) 與 [overview](../../commands/zh-TW/overview.md)。

---

## 📋 步驟 1：安裝

### 下載並安裝
```bash
# 用途：下載並安裝。
curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-utils/main/scripts/install.sh | sh
```

若您想固定版本，或想指定安裝到哪個 binary 目錄，也可以直接這樣裝：

```bash
# 用途：把固定版本安裝到指定的 binary 目錄。
VERSION=0.7.4 BIN_DIR="$HOME/.local/bin" \
  curl -sSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-utils/main/scripts/install.sh | sh
```

安裝腳本會優先使用您指定的 `BIN_DIR`。若沒有設定，會先嘗試可寫入的 `/usr/local/bin`，再退回 `$HOME/.local/bin`。

如果最後選到的安裝目錄尚未加入 `PATH`，安裝腳本會直接印出對應 `zsh` 或 `bash` 可貼上的設定方式。若想先看完整安裝說明，也可以先執行：

```bash
# 用途：查看安裝腳本支援的參數、BIN_DIR 行為與 PATH 設定提醒。
sh ./scripts/install.sh --help
```

### 檢查版本
```bash
# 用途：檢查版本。
grafana-util --version
```
**預期輸出：**
```text
grafana-util 0.7.4
```
這代表執行檔已在 `PATH` 上，而且版本與目前檢出的發行版一致。

---

## 📋 步驟 2：連線方式與 Profile 設定檔

Profile 這套流程是以「專案本地設定」為中心。`grafana-util profile` 預設會讀寫目前工作目錄中的 `grafana-util.yaml`，若有設定 `GRAFANA_UTIL_CONFIG`，則會優先讀取該路徑。

### 身分驗證模式概覽

`grafana-util` 可以從直接旗標、互動輸入、環境變數或專案本地的 profile 取得連線資訊。建議依序使用：

| 模式 | 適合情境 | 範例 |
| :--- | :--- | :--- |
| 直接基本驗證 (Basic Auth) | 本機引導、緊急接手 (break-glass)、管理員作業 | `grafana-util status live --url http://localhost:3000 --basic-user admin --prompt-password --output-format yaml` |
| `--profile` | 連線已確認後的日常維運、CI、可重複執行工作流 | `grafana-util status live --profile prod --output-format yaml` |
| 直接 Token 驗證 | 單一組織或權限受限的 API 自動化 | `grafana-util overview live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format yaml` |

環境變數也可以直接提供同樣的驗證資訊：

- `GRAFANA_USERNAME`
- `GRAFANA_PASSWORD`
- `GRAFANA_API_TOKEN`

如果是重複執行的工作，建議把這些 reference 寫進 profile，例如 `password_env: GRAFANA_PROD_PASSWORD` 或 `token_env: GRAFANA_DEV_TOKEN`，這樣就不用在每條命令上重打敏感資訊。

### 建議的學習順序

第一次使用時，建議照這個順序：

1. 先用一個直接的唯讀命令確認 Grafana 真的連得到
2. 確認自己正在用哪一種驗證方式
3. 之後再把重複的 URL、帳號與 secret 來源整理進 profile


### 1. 選一種建立 profile 的方式
```bash
# 用途：1. 選一種建立 profile 的方式。
grafana-util profile init --overwrite
```

```bash
# 用途：1. 選一種建立 profile 的方式。
grafana-util profile add dev \
  --url http://127.0.0.1:3000 \
  --basic-user admin \
  --prompt-password
```

```bash
# 用途：1. 選一種建立 profile 的方式。
grafana-util profile add ci \
  --url https://grafana.example.com \
  --token-env GRAFANA_CI_TOKEN \
  --store-secret os
```

```bash
# 用途：1. 選一種建立 profile 的方式。
grafana-util profile example --mode full
```
`profile init` 會產生一份最小可用的 `grafana-util.yaml`。`profile add` 可以一步建立 Basic-auth 或 token-backed 的可用 profile，不用自己手改 YAML。`profile example` 則會印出完整註解版範本，方便你拿去改。

如果你還在驗證基本連線，也可以先不碰 profile，直接跑：

```bash
# 用途：如果你還在驗證基本連線，也可以先不碰 profile，直接跑。
grafana-util status live \
  --url http://localhost:3000 \
  --basic-user admin \
  --prompt-password \
  --output-format yaml
```

確認沒問題後，再把同一組設定整理成可重複使用的 profile：

```bash
# 用途：確認沒問題後，再把同一組設定整理成可重複使用的 profile。
grafana-util profile add dev \
  --url http://127.0.0.1:3000 \
  --basic-user admin \
  --prompt-password
```

```bash
# 用途：確認沒問題後，再把同一組設定整理成可重複使用的 profile。
grafana-util status live --profile dev --output-format yaml
```

如果你想自己確認這些檔案會放哪裡，規則很單純：

| 檔案 | 預設位置 | 用途 |
| :--- | :--- | :--- |
| `grafana-util.yaml` | 目前工作目錄，或 `GRAFANA_UTIL_CONFIG` 指定的路徑 | 專案本地的 profile 設定檔 |
| `.grafana-util.secrets.yaml` | 跟 `grafana-util.yaml` 放同一個目錄 | `encrypted-file` 模式用的加密秘密檔 |
| `.grafana-util.secrets.key` | 跟 `grafana-util.yaml` 放同一個目錄 | 沒有 passphrase 時的本機 key 檔 |

### 2. 列出設定檔中的 profile
```bash
# 用途：2. 列出設定檔中的 profile。
grafana-util profile list
```
**預期輸出：**
```text
dev
prod
```
在剛初始化完成的設定檔中，`profile list` 會一行列出一個 profile 名稱。

若要看每個旗標背後的驗證規則，可再對照 [profile](../../commands/zh-TW/profile.md) 指令頁。

### 3. 查看已解析的 profile
```bash
# 用途：3. 查看已解析的 profile。
grafana-util profile show --profile prod --output-format yaml
```
**預期輸出：**
```text
name: prod
source_path: grafana-util.yaml
profile:
  url: https://grafana.example.com
  username: admin
  password_env: GRAFANA_PROD_PASSWORD
  verify_ssl: true
```
要覆蓋預設選擇規則時，用 `--profile`；想人工確認最後採用的設定時，用 `yaml` 最直觀。

---

## 📋 步驟 3：初步唯讀檢查

只要 Profile 設定檔已準備就緒，建議先透過唯讀指令確認行為，再進行資料異動作業。

### 1. `status live` 入口
```bash
# 用途：1. status live 入口。
grafana-util status live -h
```
**預期輸出：**
```text
Render project status from live Grafana read surfaces. Use current Grafana state plus optional staged context files.

Usage: grafana-util status live [OPTIONS]

Options:
      --profile <PROFILE>
          Load connection defaults from the selected repo-local profile in grafana-util.yaml.
      --url <URL>
          Grafana base URL. [default: http://localhost:3000]
```
`status live` 會直接查詢 Grafana，而且現在統一用 `--output-format` 來指定輸出格式。

### 2. `overview live` 入口
```bash
# 用途：2. overview live 入口。
grafana-util overview live -h
```
**預期輸出：**
```text
Render a live overview by delegating to the shared status live path.
...
Examples:
  grafana-util overview live --url http://localhost:3000 --basic-user admin --basic-password admin --output-format interactive
  grafana-util overview live --url http://localhost:3000 --basic-user admin --basic-password admin --output-format yaml
```
`overview live` 是共用 live status 路徑的人類導向包裝。要看可讀摘要可用 `--output-format yaml`，想進互動式工作台就用 `--output-format interactive`。

### 3. 用兩種常見驗證方式跑同一個唯讀檢查
```bash
# 用途：3. 用兩種常見驗證方式跑同一個唯讀檢查。
grafana-util overview live --profile prod --output-format yaml
```

```bash
# 用途：3. 用兩種常見驗證方式跑同一個唯讀檢查。
grafana-util overview live --url http://localhost:3000 --basic-user admin --prompt-password --output-format interactive
```
平常可重複執行的工作優先用 profile。直接 Basic auth 則保留給 bootstrap、臨時救援或尚未建好 profile 的管理員流程。

如果 shell 已經有環境變數，也可以先不建 profile，直接這樣測：

```bash
# 用途：如果 shell 已經有環境變數，也可以先不建 profile，直接這樣測。
export GRAFANA_USERNAME=admin
export GRAFANA_PASSWORD=admin
grafana-util overview live --url http://localhost:3000 --output-format yaml
```

### 4. 先知道 token 的常見限制

Token 驗證足以處理單一 org 的讀取流程，但跨 org 或管理員範圍的操作常常還是需要使用者身分或具備較廣權限的 Basic auth。

- `--all-orgs` 相關的盤點與匯出流程，最穩妥的是使用管理員憑證支援的 `--profile` 或直接 Basic auth。
- org、user、team 與 service account 管理通常需要管理員等級權限，窄權限 token 可能無法完成。
- 如果 token 看不到所有目標 org，即使旗標要求更廣的範圍，輸出仍會被 token 權限限制。

---

## 🖥️ 互動模式 (TUI)

`grafana-util dashboard browse` 會在終端機中顯示 live dashboard tree；`overview live --output-format interactive` 則會顯示互動式的整體總覽。

---
[🏠 回首頁](index.md) | [➡️ 下一章：系統架構與設計原則](architecture.md)
