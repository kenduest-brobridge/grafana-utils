# dashboard serve

## 用途
透過輕量本地 preview server 提供一份或多份 dashboard 草稿。

## 何時使用
當你正在反覆編修單一 dashboard 草稿、草稿目錄，或外部 generator 輸出，想先在本地瀏覽器檢視內容，而不是每次都直接 publish 回 Grafana 時，使用這個指令。

## 重點旗標
- `--input`：要載入到 preview server 的本地 dashboard 檔案或目錄。
- `--script`：外部 generator 指令；其 stdout 必須輸出一份 dashboard JSON/YAML，或一組 dashboard 文件陣列。
- `--script-format`：把 `--script` stdout 解析成 `json` 或 `yaml`。
- `--watch`：額外要監看的本地檔案或目錄。
- `--no-watch`：停用背景 polling reload。
- `--open-browser`：在 server 啟動後，使用預設瀏覽器開啟 preview URL。
- `--address`、`--port`：本地 preview server 的綁定位址與埠號。

## 補充說明
- 這是一個輕量的草稿 preview / 文件檢視介面，不是完整內嵌 Grafana renderer。
- `--input` 與 `--script` 互斥。編修本地草稿時用 `--input`，generator 已經能直接產出 payload 時再用 `--script`。
- reload 失敗時，錯誤會留在預覽頁上，方便你繼續修草稿而不用重啟 server。

## 範例
```bash
# 用途：提供單一本地草稿檔案。
grafana-util dashboard serve --input ./drafts/cpu-main.json --port 18080 --open-browser
```

```bash
# 用途：提供一個目錄下的所有 dashboard 草稿。
grafana-util dashboard serve --input ./dashboards/raw
```

```bash
# 用途：提供一份生成儀表板，並監看 generator 輸入路徑以便自動 reload。
grafana-util dashboard serve --script 'jsonnet dashboards/cpu.jsonnet' --watch ./dashboards --watch ./lib --port 18080
```

## 相關指令
- [dashboard review](./dashboard-review.md)
- [dashboard publish](./dashboard-publish.md)
- [dashboard edit-live](./dashboard-edit-live.md)
