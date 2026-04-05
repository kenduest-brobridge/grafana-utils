# 技術參考手冊 (Technical Reference)

本章整理 `grafana-util` 目前常用的指令、共用旗標，以及 profile 解析、輸出格式與 staged / live 的 status 用法。

如果你想逐條對照指令與旗標，請搭配 [profile](../../commands/zh-TW/profile.md)、[status](../../commands/zh-TW/status.md)、[overview](../../commands/zh-TW/overview.md) 與 [access](../../commands/zh-TW/access.md) 一起看。

## 適用對象

- 已經知道自己大概要跑哪個 command，但想確認旗標與輸出格式的人
- 要把 `grafana-util` 接進腳本、pipeline 或審查流程的人
- 想先了解 profile、secret、輸出格式與 live/staged 差異的人

## 主要目標

- 先把連線與 secret 規則講清楚
- 再整理常見輸出格式與共用旗標
- 最後讓你在查 help 之前就知道大概該看哪一段

---

## 輸出旗標規則

`grafana-util` 現在把畫面 / 表格 / JSON / YAML 這類格式選擇統一收斂到 `--output-format`。

### 標準格式旗標：`--output-format`

很多 list、review、inspect、dry-run 類指令都用 `--output-format`。

常見對應關係：

- `--output-format table` = `--table`
- `--output-format json` = `--json`
- `--output-format csv` = `--csv`
- `--output-format yaml` = `--yaml`
- `--output-format text` = `--text`

如果你是在腳本、CI 或可重複使用的範本裡，通常建議用完整寫法。  
如果你是在終端直接操作，短旗標會比較順手。

### 幾個常見例外

- 不是每個 command 都會把所有短旗標都做齊
- 有些指令只支援一兩種輸出形態
- `dashboard topology` 是特例：它支援 `text`、`json`、`mermaid`、`dot`，但沒有 `--table` 這類快捷旗標
- `--output-file`，或某些匯出 / draft 指令裡的 `--output`，代表的是輸出檔案路徑，不是輸出格式

如果你不確定某個 command 到底支援哪些格式，最準的還是該 command 的獨立說明頁。

### 給 CI 用的 `change` JSON 文件

`change` 指令群組會輸出多種 JSON contract。最穩妥的判斷順序是：

1. 先看 `kind`
2. 再確認 `schemaVersion`
3. 最後才根據 `summary`、`operations`、`checks`、`drifts` 等欄位往下判斷

CLI 內建快速查詢：

- `grafana-util change --help-schema`
- `grafana-util change plan --help-schema`
- `grafana-util change apply --help-schema`
- `grafana-util change audit --help-schema`

常見對應：

- `change summary --output-format json` -> `grafana-utils-sync-summary`
- `change plan --output-format json` -> `grafana-utils-sync-plan`
- `change review --output-format json` -> `grafana-utils-sync-plan`
- `change apply --output-format json` -> `grafana-utils-sync-apply-intent`
- `change apply --execute-live --output-format json` -> live apply result
- `change audit --output-format json` -> `grafana-utils-sync-audit`
- `change preflight --output-format json` -> `grafana-utils-sync-preflight`
- `change assess-alerts --output-format json` -> `grafana-utils-alert-sync-plan`
- `change bundle-preflight --output-format json` -> `grafana-utils-sync-bundle-preflight`
- `change promotion-preflight --output-format json` -> `grafana-utils-sync-promotion-preflight`

如果你需要每種文件的 top-level 欄位細節，直接看 [change 指令頁](../../commands/zh-TW/change.md) 會最快。

---

## 🔐 Profile、連線與 secret 處理

Profile 是專案本地的設定。`grafana-util profile` 會讀寫目前工作目錄中的 `grafana-util.yaml`，`--profile` 則是從這個檔案裡挑選一個命名 profile。

### 建議的使用順序

| 方法 | 最適合 | 優點 | 限制 / 注意事項 |
| :--- | :--- | :--- | :--- |
| `--profile` | 可重複的日常維運、CI、長期維護的 checkout | 不必重複把 secret 寫在命令列，支援 env 與 secret store | 需要先做一次設定 |
| 直接 Basic auth | bootstrap、break-glass、全域管理員流程 | 直覺，也較適合跨 org 與管理員操作 | 不要把明文密碼留在 shell history；優先用 `--prompt-password` |
| 直接 token | 權限較窄的腳本或單一 org API 操作 | 容易輪替，也容易做最小權限 | 權限範圍可能不足以支援 `--all-orgs`、org 管理或全域管理操作 |

如果你先從實務順序來看，通常會是：先用直接旗標確認連線，再把重複的 URL、帳號和 secret 來源收進 profile，讓日常操作只需要 `--profile`。

### Secret 保存模式：差別與適用情境

`grafana-util` 不只支援一種 secret 存放方式，因為本機操作、CI 與長期維護的 checkout，需求和風險模型並不一樣。

| 模式 | 它是什麼 | 好處 | 限制 / 注意事項 |
| :--- | :--- | :--- | :--- |
| `file` | 直接把明文 secret 放在 `grafana-util.yaml` | 最直覺、最好手改 | secret 會留在設定檔裡，不適合共享 repo 或日常管理流程 |
| `password_env` / `token_env` | profile 只記環境變數名稱，真正 secret 仍留在 environment | 很適合 CI、wrapper script、既有 env-injection 流程 | 還是要自己管理好 process environment |
| `os` | profile 只記 reference key，真正 secret 放在 macOS Keychain 或 Linux Secret Service | 秘密不用留在 YAML，也不必每次重打 | 只支援 macOS / Linux；Linux 也要有可用的 secret-service session |
| `encrypted-file` | profile 只記 reference key，真正 secret 加密後放在 `.grafana-util.secrets.yaml` | 不依賴 OS secret service，也比明文檔安全 | 強度取決於 passphrase 或 local key 的管理方式 |

如果你先從操作順序來想，通常可以這樣排：

1. CI 與自動化：優先用 `password_env` / `token_env`
2. macOS 或 Linux 桌面上的日常維運：優先用 `os`
3. 需要在專案本地加密存放、但不想依賴 OS secret service：用 `encrypted-file`
4. `file` 明文模式只留給 demo、一次性 lab、或明確的 bootstrap 情境

### macOS 與 Linux 的 OS secret store 支援

`os` provider 目前是平台後端：

- macOS：透過 `security` 寫入 Keychain
- Linux：透過系統 keyring 整合寫入 Secret Service

這樣 profile YAML 只會留下像這樣的 reference：

```yaml
password_store:
  provider: os
  key: grafana-util/profile/prod/password
```

真正的密碼不會寫進 `grafana-util.yaml`。

重要限制：

- `os` 只支援 macOS 與 Linux
- Linux 的 server、container、CI shell 不一定有可用的 Secret Service session
- 如果 OS secret store 不可用，請改用 `password_env`、`token_env` 或 `encrypted-file`

### 1. 選對 profile 工作流
| 工作流 | 用途 | 什麼時候用 |
| :--- | :--- | :--- |
| `profile init` | 產生最小版的 `grafana-util.yaml`。 | 想先有一份基本設定檔再慢慢改時。 |
| `profile add` | 直接建立或更新一個 named profile。 | 想用比較順手的一步式流程時。 |
| `profile example` | 輸出完整註解版範本。 | 想拿一份可直接修改的參考設定時。 |

如果你有設定 `GRAFANA_UTIL_CONFIG`，設定檔就會跟著那個路徑走。`encrypted-file` 模式用到的幫助檔也會放在同一個目錄：

| 檔案 | 預設位置 |
| :--- | :--- |
| `grafana-util.yaml` | 目前工作目錄，或 `GRAFANA_UTIL_CONFIG` 指定的路徑 |
| `.grafana-util.secrets.yaml` | 跟 `grafana-util.yaml` 放同一個目錄 |
| `.grafana-util.secrets.key` | 跟 `grafana-util.yaml` 放同一個目錄 |

### 2. 初始化、新增並列出 profile
```bash
# 用途：2. 初始化、新增並列出 profile。
grafana-util profile init --overwrite
```

```bash
# 用途：2. 初始化、新增並列出 profile。
grafana-util profile add dev --url http://127.0.0.1:3000 --basic-user admin --prompt-password
```

```bash
# 用途：2. 初始化、新增並列出 profile。
grafana-util profile add ci --url https://grafana.example.com --token-env GRAFANA_CI_TOKEN --store-secret os
```

```bash
# 用途：2. 初始化、新增並列出 profile。
grafana-util profile list
```

```bash
# 用途：2. 初始化、新增並列出 profile。
grafana-util profile example --mode full
```
**預期輸出：**
```text
Wrote grafana-util.yaml.
dev
prod
```
`init` 會建立本機設定檔，`add` 可以一步直接建立可用 profile，`list` 會一行列出一個已解析的 profile 名稱，而 `example` 則會輸出完整註解版範本，方便你拿去改。

### 3. 顯示已解析的 profile
```bash
# 用途：3. 顯示已解析的 profile。
grafana-util profile show --profile prod --output-format yaml
```

```bash
# 用途：3. 顯示已解析的 profile。
grafana-util profile show --profile prod --show-secrets --output-format yaml
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
要確認最後解析結果時就用 `show`。`--profile` 會覆蓋預設選擇規則，而 `yaml` 最適合人工檢查連線設定。

`--show-secrets` 只適合本機除錯或檢查。它會把 secret-store 參照解成明文輸出。

### 4. 註解版範例輸出
```yaml
# 沒有指定 --profile 時，預設會用這個 profile。
default_profile: dev

profiles:
  # 本機 demo profile，透過環境變數提供 Basic auth 密碼。
  dev:
    url: http://127.0.0.1:3000
    username: admin
    password_env: GRAFANA_DEV_PASSWORD
    timeout: 30
    verify_ssl: false

  # 從環境變數取得 token。適合權限範圍較窄的自動化。
  ci_token:
    url: https://grafana.example.com
    token_env: GRAFANA_CI_TOKEN
    timeout: 30
    verify_ssl: true

  # 明文範例。好改，但密碼會直接留在 grafana-util.yaml 裡。
  prod_plaintext:
    url: https://grafana.example.com
    username: admin
    password: change-me
    verify_ssl: true

  # OS 密碼保管庫範例。密碼會放進 macOS Keychain 或 Linux Secret Service。
  prod_os_store:
    url: https://grafana.example.com
    username: admin
    password_store:
      provider: os
      key: grafana-util/profile/prod_os_store/password

  # 有 passphrase 的加密秘密檔。secret file 預設會跟 grafana-util.yaml 放同一層。
  prod_encrypted:
    url: https://grafana.example.com
    username: admin
    password_store:
      provider: encrypted-file
      key: grafana-util/profile/prod_encrypted/password
      path: .grafana-util.secrets.yaml

  # 沒有 passphrase 的加密秘密檔。可減少直接看到明文，但不等於能抵擋本機帳號被入侵。
  stage_encrypted_local_key:
    url: https://grafana-stage.example.com
    username: stage-bot
    password_store:
      provider: encrypted-file
      key: grafana-util/profile/stage_encrypted_local_key/password
      path: .grafana-util.secrets.yaml
```

### 5. 日常使用時的三種常見驗證範例
```bash
# 用途：5. 日常使用時的三種常見驗證範例。
grafana-util status live --profile prod --output-format yaml
```

```bash
# 用途：5. 日常使用時的三種常見驗證範例。
grafana-util status live --url http://localhost:3000 --basic-user admin --prompt-password --output-format yaml
```

```bash
# 用途：5. 日常使用時的三種常見驗證範例。
grafana-util overview live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format json
```
預設請以 `--profile` 為主。直接 Basic auth 比較適合管理員型流程；token 則適合你已經很清楚權限邊界的 scoped automation。

### 6. 完整的 secret-handling 範例

```bash
# 用環境變數承載密碼，建立可重複使用的本機 profile。
export GRAFANA_PROD_PASSWORD='change-me'
grafana-util profile add prod --url https://grafana.example.com --basic-user admin --password-env GRAFANA_PROD_PASSWORD
```

```bash
# 用環境變數承載密碼，建立可重複使用的本機 profile。
grafana-util status live --profile prod --output-format yaml
```

```bash
# 在 macOS 或 Linux 桌面上，把 secret 放進 OS secret store。
grafana-util profile add prod-os --url https://grafana.example.com --basic-user admin --prompt-password --store-secret os
```

```bash
# 在 macOS 或 Linux 桌面上，把 secret 放進 OS secret store。
grafana-util overview live --profile prod-os --output-format interactive
```

```bash
# 使用帶 passphrase 的加密 secret file。
grafana-util profile add prod-encrypted --url https://grafana.example.com --basic-user admin --prompt-password --store-secret encrypted-file --prompt-secret-passphrase
```

```bash
# 使用帶 passphrase 的加密 secret file。
grafana-util status live --profile prod-encrypted --output-format yaml
```

```bash
# 在自動化流程中，以環境變數承載 scoped token。
export GRAFANA_CI_TOKEN='replace-me'
grafana-util profile add ci --url https://grafana.example.com --token-env GRAFANA_CI_TOKEN
```

```bash
# 在自動化流程中，以環境變數承載 scoped token。
grafana-util overview live --profile ci --output-format json
```

這組範例的重點是：

- 用同一種 `--profile` 操作面，對應不同的 secret backend
- 讓日常執行不必每次都重貼密碼
- 把 env、OS secret store、encrypted-file 與 token 模式都完整交代

### 7. 多 org 與管理員範圍的注意事項

- `--all-orgs` 最適合搭配管理員憑證支援的 `--profile` 或直接 Basic auth。
- Token 只能看到它被授權看到的範圍。多 org inventory、org export/import、user 或 team 管理，都可能因 token 權限不足而只回傳部分資料或直接失敗。
- `access org`、`access user`、`access team` 與 service account 建立、輪替、清理這類操作，通常需要比窄權限 API token 更高的 Grafana 權限。

### 8. Secret storage 常見排解

- `profile add --store-secret os` 在 macOS 失敗：
  先確認 `security` 工具有沒有正常可用，以及目前登入 session 能不能存取 Keychain。
- `profile add --store-secret os` 在 Linux 失敗：
  很可能目前環境沒有可用的 Secret Service session；這在 headless shell、container、精簡 server 上很常見。請改用 `password_env`、`token_env` 或 `encrypted-file`。
- `profile show --show-secrets` 無法解出 secret：
  請確認對應的 env var 仍存在、OS secret store 裡的 key 仍存在，或加密 secret file 與 passphrase/local key 還在。
- `encrypted-file` 在一台機器可用、另一台不行：
  目標 checkout 必須同時有 `.grafana-util.secrets.yaml`，以及相同的 passphrase 或對應的 local key file。
- profile 在一般 live 指令可用，但 `--all-orgs` 或存取管理相關指令失敗：
  代表 credential 本身有效，但權限範圍不夠。請改用管理員等級的 Basic auth 或 admin-backed profile。

---

## 📊 輸出格式對照

`grafana-util` 同時提供各格式旗標與單一 `--output-format` 選擇器。以 dashboard list 來說，目前可用的是 `--json`、`--table`、`--csv`、`--yaml` 與 `--output-format`。

### 0. 先看這張旗標對照表

| 情境 | 寫法 | 常見值 | 補充 |
| :--- | :--- | :--- | :--- |
| 直接切換常見格式 | `--text`、`--table`、`--csv`、`--json`、`--yaml` | `text` / `table` / `csv` / `json` / `yaml` | 適合 list、review、inspect、部分 import / delete dry-run 這類輸出面。 |
| 用單一旗標切換格式 | `--output-format <FORMAT>` | `text` / `table` / `csv` / `json` / `yaml` | 也可能出現 command 專用值，例如 `report-table`、`governance-json`、`mermaid`、`dot`。 |
| live status / overview 類入口 | `--output-format <FORMAT>` | `table` / `csv` / `text` / `json` / `yaml` / `interactive` | 這條路徑現在也統一使用 `--output-format`。 |
| 將結果另外寫入檔案 | `--output-file <PATH>` 或 command 專用旗標 | 視指令而定 | 常見於 topology、governance gate、screenshot 這類輸出型指令。 |

### 1. 表格或 JSON 的選擇
```bash
# 用途：1. 表格或 JSON 的選擇。
grafana-util dashboard list -h
```
**預期輸出：**
```text
--text
--table
--csv
--json
--yaml
--output-format <OUTPUT_FORMAT>
```
`--json` 適合自動化，`--table` 適合快速人工檢查，而 `--output-format` 則適合想用單一旗標切換輸出格式的情境。舊版文件中的 `--limit` 範例已不符合現況，現在應該用 `--page-size` 控制抓取大小、用 `--output-columns` 控制欄位。

### 2. Live status 與 overview 的輸出選擇器
```bash
# 用途：2. Live status 與 overview 的輸出選擇器。
grafana-util status live -h
```

```bash
# 用途：2. Live status 與 overview 的輸出選擇器。
grafana-util overview live -h
```
**預期輸出：**
```text
Render project status from live Grafana read surfaces. Use current Grafana state plus optional staged context files.
...
--output-format <OUTPUT_FORMAT>
    Render project status as table, csv, text, json, yaml, or interactive output.

Render a live overview by delegating to the shared status live path.
...
--output-format <OUTPUT_FORMAT>
    Render project status as table, csv, text, json, yaml, or interactive output.
```
這兩個 live 入口現在都用 `--output-format`。

---

## 🗂️ Dashboard 路徑

- `raw/` 是給 API replay/import 使用的路徑。
- `prompt/` 是給 Grafana UI 匯入使用的路徑。
- `dashboard export` 會直接產生 prompt 路徑。
- `dashboard raw-to-prompt` 可以把一般或 raw 的 dashboard JSON 轉成 prompt JSON，方便修補或遷移成 Grafana UI 匯入格式。
- `dashboard import` 只吃 `raw/` 或 `provisioning/` 輸入，不吃 `prompt/`。

---

## 🤖 自動化與腳本開發 (CI/CD)

### 1. 使用 `jq` 進行過濾 (Bash/Zsh)
```bash
# 取得 org ID 為 5 的所有 Dashboard UID
grafana-util dashboard list --profile prod --json | jq -r '.[] | select(.orgId == 5) | .uid'
```
這是目前可用的 JSON 腳本路徑。如果需要更少或不同欄位，請改用 `--output-columns`，不要再假設舊版表格欄位還存在。

### 2. 處理結束代碼 (Exit Codes)
```bash
# 用途：2. 處理結束代碼 (Exit Codes)。
grafana-util status live --profile prod --output-format json
if [ $? -eq 2 ]; then
  echo "CRITICAL: Grafana 連線受阻！"
  exit 1
fi
```

| Exit Code | 意義 |
| :---: | :--- |
| **0** | **成功 (Success)**：任務完成。 |
| **1** | **一般錯誤**：檢查語法或本地檔案權限。 |
| **2** | **連線受阻**：目標 Grafana 故障或網路拒絕連線。 |
| **3** | **驗證失敗**：專案合約或 dashboard JSON 無效。 |

---
[⬅️ 上一章：維運實戰場景](scenarios.md) | [🏠 回首頁](index.md) | [➡️ 下一章：實戰錦囊與最佳實踐](recipes.md)
