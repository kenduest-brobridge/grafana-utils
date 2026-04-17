# dashboard convert export-layout

## 目的
修復舊版 dashboard export tree。舊版可能把 `raw/` 或 `prompt/` 檔案寫在 Grafana folder 的最後一層名稱底下，而不是完整 nested folder path。

## 使用時機
當既有 export 已有正確的 `raw/folders.json` metadata，但檔案像 `raw/Infra/CPU__uid.json`，而不是 `raw/Platform/Team/Infra/CPU__uid.json` 時，使用這個指令。這是離線 artifact 修復，不會連到 Grafana。

## 修復前 / 修復後
- **修復前**：舊 export 會把 nested Grafana folder 壓平成最後一層 folder title。
- **修復後**：`raw/` 與 `prompt/` 會依照 `raw/folders.json` 記錄的 Grafana folder path 排列。

## 主要參數
- `--input-dir`：既有 dashboard export root 或 variant 目錄。
- `--output-dir`：寫出修復後副本，不修改原本 export。
- `--in-place`：直接修復 input tree。
- `--backup-dir`：搭配 `--in-place` 使用，搬移前備份受影響檔案。
- `--variant`：只修 `raw` 或 `prompt`；可重複指定。預設兩者都修。
- `--raw-dir`：修 prompt-only export 時，用來讀 metadata 的 raw lane。
- `--folders-file`：明確指定 folder inventory 檔。
- `--dry-run`：只輸出 move plan，不寫檔案。
- `--overwrite`：允許覆蓋既有 output 檔案。
- `--show-operations`：text output 顯示每個 `MOVE`、`SAME`、`BLOCKED` 與 `EXTRA` operation。
- `--output-format`：輸出 text、table、csv、json 或 yaml。

## 說明
- 預設只修 `raw/` 與 `prompt/`。
- `provisioning/` 會刻意維持不變。
- `raw/folders.json` 是 folder path 的依據。
- prompt 修復會透過相同 dashboard UID 找 raw dashboard 取得 folder identity。
- 當 raw dashboard JSON 沒有 `meta.folderUid` 時，只有在 root export index 的 `folderTitle` 對同一 org 於 `raw/folders.json` 內唯一時，才會用該 folder title 回推。
- metadata 不足時會標成 blocked，不會猜路徑。
- text output 預設只顯示 summary。需要逐筆 dashboard operation 時，加上 `--show-operations`。
- table 與 csv output 預設也輸出 summary；搭配 `--show-operations` 時才輸出逐筆 dashboard operation rows。
- json 與 yaml output 一律輸出完整 plan contract，包含 `summary`、`operations`、`extraFiles`。
- `--dry-run --output-format json` 會輸出 `summary.extraFileCount` 與 `extraFiles`，列出 repaired lane 內存在但沒有出現在 export index 的檔案。copy mode 會保留這些檔案；in-place repair 則不搬動它們。

## 範例
```bash
# 以 table 預覽舊 export layout 修復計畫。
grafana-util dashboard convert export-layout --input-dir ./dashboards --output-dir ./dashboards.fixed --dry-run --output-format table
```

```bash
# 只預覽 summary。
grafana-util dashboard convert export-layout --input-dir ./dashboards --output-dir ./dashboards.fixed --dry-run
```

```bash
# 預覽每個 dashboard operation。
grafana-util dashboard convert export-layout --input-dir ./dashboards --output-dir ./dashboards.fixed --dry-run --show-operations
```

```bash
# 將 operation rows 輸出成 CSV。
grafana-util dashboard convert export-layout --input-dir ./dashboards --output-dir ./dashboards.fixed --dry-run --output-format csv --show-operations
```

```bash
# 寫出修復後副本。
grafana-util dashboard convert export-layout --input-dir ./dashboards --output-dir ./dashboards.fixed --overwrite
```

```bash
# 備份受影響檔案後原地修復。
grafana-util dashboard convert export-layout --input-dir ./dashboards --in-place --backup-dir ./dashboards.layout-backup --overwrite
```

```bash
# 用 raw lane metadata 修復 prompt-only lane。
grafana-util dashboard convert export-layout --input-dir ./dashboards/prompt --raw-dir ./dashboards/raw --output-dir ./dashboards.fixed/prompt --variant prompt --overwrite
```

## 相關指令
- [dashboard export](./dashboard-export.md)
- [dashboard convert raw-to-prompt](./dashboard-convert-raw-to-prompt.md)
- [dashboard import](./dashboard-import.md)
