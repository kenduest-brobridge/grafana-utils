# 🚀 開始使用 (Getting Started)

本章說明 `grafana-util` 目前的首次使用流程。

若要對照本章提到的旗標，請一併參考 [profile](../../commands/zh-TW/profile.md)、[status](../../commands/zh-TW/status.md) 與 [overview](../../commands/zh-TW/overview.md)。

---

## 📋 步驟 1：安裝

### 下載並安裝
```bash
curl -sSL https://raw.githubusercontent.com/kendlee/grafana-utils/main/scripts/install.sh | bash
```

### 檢查版本
```bash
grafana-util --version
```
**預期輸出：**
```text
grafana-util 0.7.3
```
這代表執行檔已在 `PATH` 上，而且版本與目前檢出的發行版一致。

---

## 📋 步驟 2：Profile 設定檔

Profile 流程是專案本機 (Repo-local) 的。`grafana-util profile` 預設會讀寫目前工作目錄中的 `grafana-util.yaml`，若有設定 `GRAFANA_UTIL_CONFIG` 環境變數，則會優先讀取該路徑。

### 身分驗證模式概覽

建議依序選擇適合的模式：

| 模式 | 適合情境 | 範例 |
| :--- | :--- | :--- |
| `--profile` | 日常維運、CI、可重複執行的工作流 | `grafana-util status live --profile prod --output yaml` |
| 直接基本驗證 (Basic Auth) | 本機引導、緊急存取 (Break-glass)、管理員作業 | `grafana-util status live --url http://localhost:3000 --basic-user admin --prompt-password --output yaml` |
| 直接 Token 驗證 | 單一組織或權限受限的 API 自動化 | `grafana-util overview live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output yaml` |

若要使用環境變數管理機密資訊，建議將其設定在 Profile 中（如 `password_env: GRAFANA_PROD_PASSWORD`），避免將明文密碼直接寫入指令。


### 1. 選一種建立 profile 的方式
```bash
grafana-util profile init --overwrite
grafana-util profile add dev --url http://127.0.0.1:3000 --basic-user admin --prompt-password
grafana-util profile add ci --url https://grafana.example.com --token-env GRAFANA_CI_TOKEN --store-secret os
grafana-util profile example --mode full
```
`profile init` 會產生一份最小可用的 `grafana-util.yaml`。`profile add` 可以一步建立 Basic-auth 或 token-backed 的可用 profile，不用自己手改 YAML。`profile example` 則會印出完整註解版範本，方便你拿去改。

如果你想自己確認這些檔案會放哪裡，規則很單純：

| 檔案 | 預設位置 | 用途 |
| :--- | :--- | :--- |
| `grafana-util.yaml` | 目前工作目錄，或 `GRAFANA_UTIL_CONFIG` 指定的路徑 | repo-local profile 設定檔 |
| `.grafana-util.secrets.yaml` | 跟 `grafana-util.yaml` 放同一個目錄 | `encrypted-file` 模式用的加密秘密檔 |
| `.grafana-util.secrets.key` | 跟 `grafana-util.yaml` 放同一個目錄 | 沒有 passphrase 時的本機 key 檔 |

### 2. 列出設定檔中的 profile
```bash
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
`status live` 會直接查詢 Grafana，而且現在用的是 `--output`，不是舊文件常見的 `--output-format`。

### 2. `overview live` 入口
```bash
grafana-util overview live -h
```
**預期輸出：**
```text
Render a live overview by delegating to the shared status live path.
...
Examples:
  grafana-util overview live --url http://localhost:3000 --basic-user admin --basic-password admin --output interactive
  grafana-util overview live --url http://localhost:3000 --basic-user admin --basic-password admin --output yaml
```
`overview live` 是共用 live status 路徑的人類導向包裝。要看可讀摘要可用 `--output yaml`，想進互動式工作台就用 `--output interactive`。

### 3. 用兩種常見驗證方式跑同一個唯讀檢查
```bash
grafana-util overview live --profile prod --output yaml
grafana-util overview live --url http://localhost:3000 --basic-user admin --prompt-password --output interactive
```
平常可重複執行的工作優先用 profile。直接 Basic auth 則保留給 bootstrap、臨時救援或尚未建好 profile 的管理員流程。

### 4. 先知道 token 的常見限制

Token 驗證足以處理單一 org 的讀取流程，但跨 org 或管理員範圍的操作常常還是需要使用者身分或具備較廣權限的 Basic auth。

- `--all-orgs` 相關的盤點與匯出流程，最穩妥的是使用管理員憑證支援的 `--profile` 或直接 Basic auth。
- org、user、team 與 service-account 管理通常需要管理員等級權限，窄權限 token 可能無法完成。
- 如果 token 看不到所有目標 org，即使旗標要求更廣的範圍，輸出仍會被 token 權限限制。

---

## 🖥️ 互動模式 (TUI)

`grafana-util dashboard browse` 會在終端機中開啟 live dashboard tree；`overview live --output interactive` 則會開啟互動式的整體總覽。

---
[🏠 回首頁](index.md) | [➡️ 下一章：系統架構與設計原則](architecture.md)
