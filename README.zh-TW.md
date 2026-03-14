# Grafana Utilities 中文說明

英文版： [README.md](README.md)

Grafana Utilities 是一套給 Grafana 操作人員用的實務工具，專門解決一個常見痛點：Grafana 在 UI 上很方便，但一旦你要盤點、匯出、比對、還原，或在同一台與不同環境之間重播設定，就會變得很難控管。

這個工具把那些原本靠人工點選的流程，變成可重複、可審核、可比較的 CLI 操作。

## 這個工具解決什麼痛點

傳統 Grafana 維運常見問題：

- Dashboard、alert、datasource 的狀態分散在 UI，難以一次盤點。
- 很難清楚回答「現在有哪些資源？」「跟上次差在哪？」「這次匯入會改到什麼？」
- 同環境還原與跨環境搬移通常靠手動操作，容易漏、容易漂移，也不容易審核。
- 做變更前往往不知道會 create、update、overwrite 哪些資源。
- User、team、service account、token 的管理在規模變大後會很麻煩。

Grafana Utilities 的價值就是把這些流程明文化：

- 先盤點
- 再匯出
- 先 diff / dry-run
- 最後才匯入或變更

## 核心優勢

- 把 Grafana 狀態轉成可版控的 JSON 檔案
- 能做同環境備份與還原
- 能做跨環境搬移與同步
- 能先做 dry-run，降低誤寫風險
- 能直接比較 live Grafana 與本地匯出結果
- 能用同一套 CLI 處理 dashboard、datasource、alert、access 四類工作

## 支援的 Grafana 項目

| 項目 | List | Export | Import | Diff | Inspect | Add | Modify | Delete | 說明 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Dashboards | ✓ | ✓ | ✓ | ✓ | ✓ | - | - | - | 適合做盤點、備份、還原與跨環境搬移 |
| Datasources | ✓ | ✓ | ✓ | ✓ | - | - | - | - | 適合做 datasource 盤點、重播與漂移比對 |
| Alert rules 與 alerting 資源 | ✓ | ✓ | ✓ | ✓ | - | - | - | - | 包含 alert rules、contact points、mute timings、templates |
| Users | ✓ | ✓ | ✓ | - | - | ✓ | ✓ | ✓ | Rust 支援 access user 的匯出/匯入；Python 仍為即時存取流程 |
| Teams (alias: group) | ✓ | ✓ | ✓ | - | - | ✓ | ✓ | ✓ | 支援 team 與成員管理，Rust 提供匯出/匯入 |
| Service accounts | ✓ | - | - | - | - | ✓ | ✓ | ✓ | 支援 service account 生命週期管理 |
| Service account tokens | ✓ | - | - | - | - | ✓ | - | ✓ | 支援 token 建立、查看與撤銷 |

### Access command 的支援設計

#### Python CLI（`grafana_utils` / `python3 -m grafana_utils`）

Python 版的 `user` 與 `team`（`group`）是以「即時帳號權限管理」為設計核心：
- 不提供 `access ... export` 或 `access ... import` 指令。
- 不做完整 user/team snapshot 的匯出/匯入，因為這類資料與 live 實例綁定很強，包含 ID、角色、密碼欄位、組織關聯與成員上下文。
- 跨環境移轉建議採用：
  1. 先用 `access user/team list` 盤點來源資料
  2. 你的流程外部先做狀態正規化（CSV/JSON/YAML）
  3. 透過 `access ... add/modify/delete` 套用到目標環境
- 這樣可避免直接「套用檔案就導入」造成的不可預期覆寫與權限風險。

#### Rust CLI（`cargo run --bin grafana-util` / `grafana-util`）

- `access user` 與 `access team` 提供 `export` 與 `import` 子命令。
- `team import` 支援成員與管理員同步，若會移除既有成員需加 `--yes` 才能執行。
- 適合做跨環境一致性還原與可控改動預覽。

如果你要做更嚴格的 file-diff 文件比對，請使用 Rust 版 `access ... export|import`；Python 版本仍以即時操作模型為主。 

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
