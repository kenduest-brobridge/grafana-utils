# migrate

## 用途
`grafana-util migrate` 是專門放修補、正規化與跨格式轉換工作流程的 namespace，用來把一種 Grafana 成品轉成下一步較安全的 migration artifact。

## 何時使用
當工作重點不是日常 live dashboard 操作，而是要把匯出成品修好、補 mapping、或整理成下一個環境更容易重用的格式時，使用這個 namespace。

## 說明
如果這次工作本質上是 migration：修 raw export、補 datasource placeholder、或先整理好檔案再做後續匯入，請先看這頁。`migrate` 把這些轉換流程和 `dashboard` 裡偏 live/operator 的 browse、review、publish、history 分開，讓命令語意更清楚。

## 工作流路徑

- **修補匯出檔**：先修一批 raw dashboard 檔，再做後續匯入或 UI upload。
- **正規化 migration artifact**：把 raw export tree 轉成旁邊的 `prompt/` 路徑。
- **補強解析**：必要時可用 profile 或 live auth 查 Grafana datasource inventory，協助修補 datasource 參照。

## Before / After

- **Before**：raw export 還綁著原本環境，下一個 Grafana 可能直接吃不下。
- **After**：修補後的 artifact 可檢視、可重用，也更適合下一步匯入或 UI upload。

## 成功判準

- 命令語意清楚是 migration step，不會和 live dashboard mutate 混在一起
- 修補後的檔案能直接進下一步 review 或 import，不必手改 JSON
- datasource 修補與 prompt 生成能力保留，但不再塞進 `dashboard` 主 namespace

## 失敗檢查

- 如果下一步是 API import，先確認你要的仍然是 `raw/` 或 `provisioning/`，不是 `prompt/`
- 如果 datasource 解析仍有歧義，先補 `--datasource-map` 或 live profile 再重試
- 如果目錄轉換可能把生成檔混進原始資料樹，請停下來補 `--output-dir`

## 範例
```bash
# Purpose: 先看 migrate namespace 有哪些修補路徑。
grafana-util migrate --help
```

```bash
# Purpose: 把一個 raw dashboard 檔修成 prompt-safe migration artifact。
grafana-util migrate dashboard raw-to-prompt --input-file ./legacy/cpu-main.json
```

```bash
# Purpose: 把整個 raw export root 轉成旁邊的 prompt/ lane，方便後續 UI upload。
grafana-util migrate dashboard raw-to-prompt --input-dir ./dashboards/raw --output-dir ./dashboards/prompt --overwrite
```

## 相關指令

- [migrate dashboard raw-to-prompt](./migrate-dashboard-raw-to-prompt.md)
- [dashboard export](./dashboard-export.md)
- [dashboard import](./dashboard-import.md)
- [dashboard review](./dashboard-review.md)
- [change](./change.md)
