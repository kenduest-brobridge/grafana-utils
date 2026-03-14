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
- 維護者文件： [DEVELOPER.md](DEVELOPER.md)

## 相容性

- 支援 RHEL 8 以上
- Python runtime 目標版本：3.9+

