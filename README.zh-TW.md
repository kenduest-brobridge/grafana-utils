# Grafana Utilities 中文說明

英文版： [README.md](README.md)

Grafana Utilities 是一套偏向管理者與維運治理的 Grafana 工具集。官方 UI 適合日常使用，但當環境規模變大、需要盤點、批次搬移、風險控管與 dry-run 預覽時，單靠 UI 或零散 API call 很快就不夠用了。

一句話來說：

> 官方工具偏向「用 Grafana」，Grafana Utilities 偏向「管 Grafana」。

這個專案用 Python 處理 CLI 與工作流邏輯，用 Rust 補強效能導向路徑與單檔二進位工具。重點不是取代 Grafana，而是把 Grafana 管理變成更可觀測、可審核、可重播的工程化流程。

## 管理者常見痛點

當你要管理數十個 datasource、上百個 dashboard、甚至多個 Grafana 環境時，難的通常不是「怎麼點 UI」，而是以下這些維運問題：

- 匯入/匯出摩擦很高：
  UI 很難做批次處理；直接打 API 又不容易保留資料夾結構、UID 與可重播流程。
- 資產盤點常常有盲區：
  很難清楚回答「現在有哪些資源？」「哪些 datasource 還在被用？」「跟上次差在哪？」
- live 變更風險高：
  datasource 與 access 變更如果沒有預覽，很容易連帶影響 dashboard、alert 或自動化流程。
- 治理流程破碎：
  dashboard、datasource、alert、user、team、service account 經常靠不同人的習慣在管理，而不是一套標準化工作流。

Grafana Utilities 的價值就是把這些流程明文化：

- 先盤點
- 再匯出
- 先 diff / dry-run
- 最後才匯入或變更

## 它做得好的事

- 環境盤點：
  不用一頁一頁點 UI，就能列出 dashboard、datasource、alerting、users、teams、service accounts。
- 備份與重播：
  把 Grafana 狀態轉成可版控的 JSON，做同環境還原或跨環境搬移。
- 變更審查：
  在真正匯入或 cleanup 前，先比較 live 狀態與本地匯出。
- 更安全的 live 操作：
  在寫入 datasource、access 等資源前先做 dry-run。
- 治理導向檢查：
  針對 dashboard 結構、datasource 使用情況與查詢盤點做比一般 UI 更深的檢視。

## 核心能力

1. Dashboard 管理
- 支援 dashboard 的 export、import、diff、inspect。
- 能做 datasource usage 與 query inventory 類型的治理分析。

2. Datasource 管理
- 支援 datasource 的 list、export、import、diff，以及 live add/delete。
- 支援先做 dry-run 再套用 datasource 變更。

3. Access 管理
- 支援 users、teams、service accounts、service-account tokens。
- 支援 access snapshot 的 export/import/diff，做可重播 reconcile。

4. 一套統一 CLI
- 用同一個工具處理 `dashboard`、`datasource`、`alert`、`access`。
- 依情境選擇 table / csv / json 輸出給人看或給自動化用。
## 支援的 Grafana 項目

| 項目 | List | Export | Import | Diff | Inspect | Add | Modify | Delete | 說明 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Dashboards | ✓ | ✓ | ✓ | ✓ | ✓ | - | - | - | 適合做盤點、備份、還原與跨環境搬移 |
| Datasources | ✓ | ✓ | ✓ | ✓ | - | ✓ | - | ✓ | 適合做 datasource 盤點、重播、漂移比對與 live datasource 管理 |
| Alert rules 與 alerting 資源 | ✓ | ✓ | ✓ | ✓ | - | - | - | - | 包含 alert rules、contact points、mute timings、templates |
| Users | ✓ | ✓ | ✓ | ✓ | - | ✓ | ✓ | ✓ | 支援 access user 的快照匯出/匯入/差異比對，也可搭配 `--with-teams` 帶出 membership 狀態 |
| Teams (alias: group) | ✓ | ✓ | ✓ | ✓ | - | ✓ | ✓ | ✓ | 支援 team 與成員管理，也支援匯出/匯入與差異比對 |
| Service accounts | ✓ | ✓ | ✓ | ✓ | - | ✓ | ✓ | ✓ | 支援 service account 生命週期管理、快照匯出匯入與差異比對 |
| Service account tokens | ✓ | - | - | - | - | ✓ | - | ✓ | 支援 token 建立、查看與撤銷 |

### Access command 的支援設計

Access 工作流現在以同一套操作模型提供：

- `access user export|import|diff` 處理使用者快照，以及可選的 team membership 狀態。
- `access team export|import|diff` 處理 team 快照，以及成員與 admin 狀態的漂移比對。
- `access service-account export|import|diff` 處理 automation identity 的快照與可變欄位漂移比對。
- `team import` 會做 deterministic membership sync；如果會移除既有 membership，必須加上 `--yes`。
- 匯出/匯入 snapshot 檔案可用於受控 migration、cleanup review，以及可重播的 reconcile 流程。

如果你現在的流程還是：

- 打開 Grafana
- 手動點資料夾與 dashboard
- 人工匯出
- 人工比對
- 手動匯入另一台

那這個工具就是拿來把這些流程標準化的。

## 它特別適合哪些情境

- 你想做 Grafana 資產盤點
- 你要把 dev / staging / prod 之間的內容做可控搬移
- 你想把 dashboard / datasource / alert 放進 git
- 你要先知道匯入會改什麼，而不是直接寫進去
- 你要整理 user、team、service account、token

## CLI 入口

安裝後主要入口：

```text
grafana-util <domain> <command> [options]
```

在 source tree 中執行：

```text
python3 -m grafana_utils <domain> <command> [options]
cargo run --bin grafana-util -- <domain> <command> [options]
```

主要工作域：

- `dashboard`
- `datasource`
- `alert`
- `access`

## 快速開始

匯出 dashboard：

```bash
grafana-util dashboard export \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --export-dir ./dashboards \
  --overwrite
```

列出目前 dashboard：

```bash
grafana-util dashboard list \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --table
```

先做 dry-run 匯入預覽：

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

比較本地匯出與 live Grafana：

```bash
grafana-util dashboard diff \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --import-dir ./dashboards/raw
```

## 安裝

Python package：

```bash
python3 -m pip install .
```

Rust binary：

```bash
cd rust
cargo build --release
```

## 文件入口

- 英文使用者手冊： [docs/user-guide.md](docs/user-guide.md)
- 繁中使用者手冊： [docs/user-guide-TW.md](docs/user-guide-TW.md)
- Python 實作說明： [docs/overview-python.md](docs/overview-python.md)
- Rust 實作說明： [docs/overview-rust.md](docs/overview-rust.md)
- 維護者文件： [docs/DEVELOPER.md](docs/DEVELOPER.md)

## 相容性

- 支援 RHEL 8 以上
- Python runtime 目標版本：3.9+
