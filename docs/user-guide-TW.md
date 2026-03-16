# Grafana Utilities 維運指南

`grafana-util` 是一個專為 Grafana 管理員與 DevOps 工程師設計的專業 Rust 命令列工具（CLI）。它提供了一整套用於管理 Grafana 資源的工具，包括儀表板（Dashboards）、資料來源（Data Sources）、告警（Alerts）以及存取控制（Access Control）。本指南主要針對 Rust 版本的 CLI 進行說明。

---

## 1. 入門指南

### 1.1 安裝

若要從原始碼編譯 `grafana-util`，請確保已安裝 Rust 工具鏈，並執行：

```bash
cargo build --release --manifest-path rust/Cargo.toml --bin grafana-util
```

編譯完成後的執行檔位於 `rust/target/release/grafana-util`。您可以執行以下指令驗證安裝：

```bash
./rust/target/release/grafana-util --help
```

### 1.2 認證與通用參數

多數指令都支援一組通用的認證與連線參數。

| 參數 | 說明 |
| --- | --- |
| `--url` | Grafana 實例的基礎 URL (預設: `http://localhost:3000`)。 |
| `--token` | Grafana API Token 或 Service Account Token。 |
| `--basic-user` | Username 認證（Basic Authentication）。 |
| `--basic-password` | Password 認證（Basic Authentication）。 |
| `--prompt-password` | 以互動方式輸入認證密碼。 |
| `--org-id` | 指定請求的目標組織 ID。 |
| `--timeout` | HTTP 請求超時時間（秒）。 |
| `--verify-ssl` | 啟用或停用 TLS 憑證驗證。 |

**認證規則：**
- 請使用 Token 認證或 Basic Authentication 其中之一，不可同時使用。
- Basic Authentication 通常用於涉及多個組織的管理任務。

---

## 2. 儀表板管理 (`dashboard`)

`dashboard` 指令群組負責 Grafana 儀表板的生命週期管理，包括盤點、備份與還原。

### 2.1 資產盤點與列表
列出儀表板及其詳細詮釋資料（Metadata），包括資料夾路徑與關聯的資料來源。

```bash
grafana-util dashboard list --url <URL> --basic-user <USER> --table --with-sources
```

### 2.2 匯出儀表板
將儀表板備份到本地目錄。匯出內容包含適用於還原的 `raw` 格式，以及適用於 UI 審閱的 `prompt` 格式。

```bash
grafana-util dashboard export --export-dir ./backups/dashboards --overwrite
```

### 2.3 匯入與還原
從本地目錄匯入儀表板。建議在實際套用前使用 `--dry-run` 預覽變更。

```bash
grafana-util dashboard import \
  --import-dir ./backups/dashboards/raw \
  --replace-existing \
  --dry-run \
  --output-format table
```

### 2.4 差異比對與分析
比較本地定義與線上版本的差異，或分析匯出檔案的一致性。

```bash
grafana-util dashboard diff --dashboard-file ./dashboard.json --url <URL>
```

---

## 3. 資料來源管理 (`datasource`)

管理 Grafana 資料來源，支援自動化設定與金鑰處理。

### 3.1 列表資料來源
查看所有已設定的資料來源及其類型。

```bash
grafana-util datasource list --table
```

### 3.2 新增與修改
透過程式化方式新增資料來源。

```bash
grafana-util datasource add \
  --name "Prometheus-Main" \
  --type "prometheus" \
  --datasource-url "http://prometheus:9090" \
  --access "proxy"
```

### 3.3 備份與遷移
匯出與匯入資料來源設定，支援金鑰佔位符處理。

```bash
# 匯出
grafana-util datasource export --export-dir ./backups/datasources

# 匯入 (測試模式)
grafana-util datasource import \
  --import-dir ./backups/datasources \
  --dry-run \
  --output-format table
```

---

## 4. 告警管理 (`alert`)

管理 Grafana Alerting 資源，包括告警規則、聯絡點與通知策略。

### 4.1 資源列表
列出各類告警組件以供審計與檢閱。

```bash
grafana-util alert list-rules
grafana-util alert list-contact-points
grafana-util alert list-mute-timings
grafana-util alert list-templates
```

### 4.2 同步與遷移
匯出與匯入告警設定，便於在不同環境間遷移。

```bash
grafana-util alert export --export-dir ./backups/alerts
grafana-util alert import --import-dir ./backups/alerts --dry-run
```

---

## 5. 存取與權限管理 (`access`)

管理組織、使用者、團隊與服務帳號。

### 5.1 組織管理
列出或建立 Grafana 組織。

```bash
grafana-util access org list --with-users --table
grafana-util access org add --name "Engineering"
```

### 5.2 使用者與團隊
管理全域或組織內的帳號與團隊成員關係。

```bash
# 全域使用者列表
grafana-util access user list --scope global --table

# 團隊及其成員列表
grafana-util access team list --with-members --table
```

### 5.3 服務帳號 (Service Accounts)
服務帳號的生命週期管理，支援備份與還原。

```bash
# 列表服務帳號
grafana-util access service-account list --table

# 備份服務帳號
grafana-util access service-account export --export-dir ./backups/service-accounts
```

---

## 6. 分階段同步 (GitOps 工作流) (`sync`)

`sync` 指令群組支援 GitOps 風格的工作流，透過比較「期望狀態」檔案與「線上」環境進行同步。

### 6.1 工作流步驟
1.  **Summary**: 取得期望狀態檔案中的資源摘要。
2.  **Plan**: 生成詳細的執行計畫，比較期望狀態與線上狀態。
3.  **Preflight**: 驗證所有依賴項（如資料來源、外掛程式）是否就緒。
4.  **Apply**: 執行計畫以同步 Grafana 資源。

### 6.2 範例：生成同步計畫
```bash
grafana-util sync plan \
  --desired-file ./desired-state.json \
  --live-file ./current-live-state.json
```

---

## 7. 進階用法

### 7.1 輸出格式
多數指令支援 `--output-format`。`table` 適合人工閱讀，`json` 則適合與其他工具（如 `jq`）整合。

### 7.2 多組織支援
針對大規模 Grafana 實例，使用 `--all-orgs`、`--use-export-org` 與 `--create-missing-orgs` 等旗標來有效管理多租戶資源。

### 7.3 測試模式 (Dry-Run)
在執行 `import`、`add` 或 `sync` 指令時，務必先使用 `--dry-run` 驗證預期變更，以確保不會意外損壞線上環境。
