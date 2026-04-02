# Grafana Utilities (`grafana-util`)

[![Version](https://img.shields.io/badge/version-0.6.2-blue.svg)](https://github.com/kenduest-brobridge/grafana-utils/releases)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-linux%20%7C%20macos-lightgrey.svg)](#安裝)

[English Version](README.md) | **繁體中文版**

`grafana-util` 是一套專為 **Grafana 維運人員 (Operators)** 設計的高效能 Rust CLI 工具。它專精於資產盤點、匯出/匯入工作流、差異檢測 (Drift Detection) 以及大規模的變更審查。

## 🚀 為什麼選擇 `grafana-util`？

不同於基礎的 API 腳本，`grafana-util` 是為了解決 estate-level 的維運問題：

- 🛡️ **安全變更**：透過 `dry-run`、`plan` 與 `review` 工作流，避免直接改動環境。
- 📂 **結構化匯出**：提供版本控制友善的 Dashboard、Datasource 與 Alert 目錄結構。
- ⚡ **極致效能**：原生 Rust 編譯，快速處理大規模的 Grafana 實例。

## 🛠️ 主要命令區域

| 區域 | 主要用途 | 常見命令 |
| --- | --- | --- |
| `dashboard` | dashboard 盤點、匯出匯入、diff、分析、截圖 | `list`, `export`, `import`, `diff`, `inspect-export`, `inspect-live`, `browse`, `screenshot` |
| `datasource` | datasource 盤點、masked recovery、live mutation | `list`, `export`, `import`, `diff`, `browse`, `add`, `modify`, `delete` |
| `alert` | alert 管理、review/apply 工作流 | `plan`, `apply`, `delete`, `export`, `import`, `diff`, `list-*`, `add-rule`, `clone-rule` |
| `access` | 組織、使用者、團隊與服務帳號盤點 | `org ...`, `user ...`, `team ...`, `service-account ...` |
| `change` | 暫存變更工作流 | `summary`, `plan`, `review`, `apply`, `preflight`, `bundle-preflight` |
| `overview` | 維運人員導向的整體專案摘要 | `overview`, `overview live` |
| `status` | 正式的 staged/live status contract | `status staged`, `status live` |
| `profile` | repo-local 連線預設值 | `init`, `list`, `show` |

## 📥 安裝

安裝最新版：
```bash
curl -fsSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-utils/main/scripts/install.sh | sh
```

指定版本或安裝路徑：
```bash
curl -fsSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-utils/main/scripts/install.sh | BIN_DIR=/usr/local/bin VERSION=v0.6.2 sh
```

如果你已經有 repo checkout：
```bash
sh ./scripts/install.sh
```

從原始碼建置：
```bash
cd rust && cargo build --release
```

## 🏎️ 快速開始

確認版本與可用命令：

```bash
grafana-util --version
grafana-util version
grafana-util -h
grafana-util dashboard -h
grafana-util datasource -h
grafana-util alert -h
grafana-util access -h
grafana-util change -h
grafana-util overview -h
grafana-util status -h
```

### 範例 (已在 Grafana `12.4.1` 驗證)

**跨 org 列出 dashboard，並附帶 datasource 資訊：**
```bash
grafana-util dashboard list \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --all-orgs \
  --with-sources \
  --table
```

**分析已匯出的 dashboard tree：**
```bash
grafana-util dashboard inspect-export \
  --import-dir ./dashboards/raw \
  --output-format report-table
```

**在真正套用前先預覽 dashboard import：**
```bash
grafana-util dashboard import \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --import-dir ./dashboards/raw \
  --replace-existing \
  --dry-run \
  --table
```

**匯出 alerting 資源做審查或遷移：**
```bash
grafana-util alert export \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --output-dir ./alerts \
  --overwrite
```

**從 desired files 建立可審查的 alert plan：**
```bash
grafana-util alert plan \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --desired-dir ./alerts/desired \
  --prune \
  --output json
```

## ⚠️ 重要工作流規則

### Dashboard export 會刻意拆成不同 lane
`dashboard export` 會寫出三種輸出：
- `raw/`：grafana-util replay/import 用的 canonical lane。
- `prompt/`：給 Grafana UI 匯入用的 lane。
- `provisioning/`：給 Grafana file provisioning 用的 lane。
這三條 lane 不是互換的，請根據工作流選擇正確的目錄。

### Datasource export 使用 masked recovery contract
`datasource export` 會寫出：
- `datasources.json`：canonical masked recovery / replay contract。
- `provisioning/datasources.yaml`：衍生出的 provisioning projection。
真正要拿來 restore 或 replay 的是 `datasources.json`。

### Alert management 刻意分成 authoring 與 apply 兩段
- 先用 `add-rule`、`clone-rule` 等寫好 desired-state files。
- 用 `alert plan` 審查變更。
- 最後用 `alert apply` 執行。

## 📊 各區域成熟度 (Maturity Map)

| 區域 | 目前成熟度 | 備註 |
| --- | --- | --- |
| `dashboard` | 最深 | 分析、匯出匯入、審查工具最完整 |
| `datasource` | 深且成熟 | 同時支援 live mutation 與檔案回放 |
| `alert` | 成熟 | 同時支援 review/apply 管理流程與遷移式 export/import |
| `access` | 成熟 | 最適合做 inventory、replay、受控重建 |
| `change` | 進階 | 重點是 review-first staged workflow |
| `overview` | 穩定的人類入口 | 適合 handoff 與 triage 的第一站 |
| `status` | 穩定 contract surface | 適合需要單一跨 domain staged/live status 時使用 |

## 📖 文件

- 📘 [英文使用者指南](docs/user-guide/en/index.md)
- 📙 [繁體中文使用者指南](docs/user-guide/zh-TW/index.md)
- 🛠️ [開發者手冊](docs/DEVELOPER.md)
- 📜 [變更紀錄](CHANGELOG.md)

## ⚖️ 授權條款

本專案採用 MIT 授權條款。詳見 `LICENSE` 檔案。
