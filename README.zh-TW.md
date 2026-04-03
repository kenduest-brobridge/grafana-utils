# Grafana Utilities

[English Version](README.md) | **繁體中文版**

`grafana-util` 是一套以 Rust 為主的 Grafana 維運 CLI，重點放在盤點、匯出匯入、drift review、staged change 工作流，以及專案層級的狀態讀取。

它解的是 estate-level 的操作問題，不是單一物件逐個點選的 UI 管理問題。當你需要先看清楚現況、把資料匯出成可審查檔案、比較 staged 與 live Grafana 的差異，再決定是否套用時，這個工具才是它真正的使用場景。

## 這個工具適合做什麼

適合拿 `grafana-util` 來做這些事：

- 把 dashboard、datasource、alerting 資源或 access 狀態匯出成可審查的本地檔案
- 在真正修改 Grafana 前，先比對 staged 檔案與 live 狀態
- 跑 `dry-run`、review、apply 這種受控流程，而不是直接盲改
- 在專案尺度分析 dashboard query、datasource 依賴、staged inventory
- 用 `overview` 或 `status` 先讀出整個專案的摘要與 readiness

這個專案不是要取代 Grafana 的編輯 UI。它主要解 migration、audit、governance、handoff，以及 operator-safe change workflow。

## 主要命令區域

| 區域 | 主要用途 | 常見命令 |
| --- | --- | --- |
| `dashboard` | dashboard 盤點、匯出匯入、diff、分析、截圖 | `list`、`export`、`import`、`diff`、`inspect-export`、`inspect-live`、`browse`、`screenshot` |
| `datasource` | datasource 盤點、masked recovery 匯出、回放、live mutation | `list`、`export`、`import`、`diff`、`browse`、`add`、`modify`、`delete` |
| `alert` | alert 管理、review/apply 工作流、遷移 bundle | `plan`、`apply`、`delete`、`export`、`import`、`diff`、`list-*`、`add-rule`、`clone-rule` |
| `access` | org、user、team、service account 的盤點與回放 | `org ...`、`user ...`、`team ...`、`service-account ...` |
| `change` | staged review-first 變更工作流 | `summary`、`plan`、`review`、`apply`、`preflight`、`bundle-preflight` |
| `overview` | 人類導向的整體專案摘要 | `overview`、`overview live` |
| `status` | 正式的 staged/live status contract | `status staged`、`status live` |
| `profile` | repo-local 連線預設值 | `init`、`list`、`show` |

## 輸出模式

同一類工作流通常不只一種輸出面：

| 模式 | 適合用途 |
| --- | --- |
| text | 預設維運摘要與 dry-run 預覽 |
| json | CI、自動化、穩定的 machine-readable handoff |
| table / csv | 清單盤點與類試算表輸出 |
| interactive TUI | 導覽式瀏覽與終端機內審查 |

## 安裝

安裝最新版：

```bash
curl -fsSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-utils/main/scripts/install.sh | sh
```

需要固定版本或指定安裝位置時：

```bash
curl -fsSL https://raw.githubusercontent.com/kenduest-brobridge/grafana-utils/main/scripts/install.sh | BIN_DIR=/usr/local/bin VERSION=v0.6.1 sh
```

如果你已經有 repo checkout：

```bash
sh ./scripts/install.sh
```

從原始碼建置：

```bash
cd rust && cargo build --release
```

## 快速開始

先確認版本與目前維護中的命令面：

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

本 README 的 live 範例已在本地 Docker Grafana `12.4.1` 驗證。

跨 org 列出 dashboard，並附帶 datasource 資訊：

```bash
grafana-util dashboard list \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --all-orgs \
  --with-sources \
  --table
```

分析已匯出的 dashboard tree：

```bash
grafana-util dashboard inspect-export \
  --import-dir ./dashboards/raw \
  --output-format report-table
```

在真正套用前先預覽 dashboard import：

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

匯出 alerting 資源做審查或遷移：

```bash
grafana-util alert export \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --output-dir ./alerts \
  --overwrite
```

從 desired files 建立可審查的 alert plan：

```bash
grafana-util alert plan \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --desired-dir ./alerts/desired \
  --prune \
  --output json
```

## 重要工作流規則

### Dashboard export 會刻意拆成不同 lane

`dashboard export` 會寫出三種不同輸出：

- `raw/`：給 grafana-util replay/import 用的 canonical lane
- `prompt/`：給 Grafana UI 匯入的 lane
- `provisioning/`：給 Grafana file provisioning 的 lane

這三條 lane 不是互換的。做什麼流程，就要用對應的 lane。

### Datasource export 使用 masked recovery contract

`datasource export` 會寫出：

- `datasources.json`：canonical masked recovery / replay contract
- `provisioning/datasources.yaml`：衍生出來的 provisioning projection

真正要拿來 restore 或 replay 的是 `datasources.json`。`provisioning/` 應視為 Grafana file provisioning 用的 projection，不是主要 restore source。

### Alert management 刻意分成 authoring 與 apply 兩段

alert surface 是刻意拆開的：

- 先用 `add-rule`、`clone-rule`、`add-contact-point` 之類的命令寫 desired-state files
- 再用 `alert plan` 審查 delta
- 最後只用 `alert apply` 執行已審查的變更

這不是多此一舉，而是刻意把 alert 變更做成可 review 的流程，降低直接動 live Grafana 的風險。

## 各區域成熟度

如果你只想快速看支援深度，先看這張表。

| 區域 | 目前成熟度 | 備註 |
| --- | --- | --- |
| `dashboard` | 最深 | 分析、匯出匯入、審查工具最完整 |
| `datasource` | 深且成熟 | 同時支援 live mutation 與檔案回放 |
| `alert` | 成熟 | 同時支援 review/apply 管理流程與遷移式 export/import |
| `access` | 成熟 | 最適合做 inventory、replay、受控重建 |
| `change` | 進階 | 重點是 review-first staged workflow |
| `overview` | 穩定的人類入口 | 適合 handoff 與 triage 的第一站 |
| `status` | 穩定 contract surface | 適合需要單一跨 domain staged/live status 時使用 |

## 文件

- [英文使用者指南](docs/user-guide.md)
- [繁體中文使用者指南](docs/user-guide-TW.md)
- [開發者手冊](docs/DEVELOPER.md)
- [Rust 技術總覽](docs/overview-rust.md)
- [變更紀錄](CHANGELOG.md)

## Releases

- [最新版 release](https://github.com/kenduest-brobridge/grafana-utils/releases/latest)
- [所有 releases](https://github.com/kenduest-brobridge/grafana-utils/releases)
