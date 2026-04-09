# migrate dashboard raw-to-prompt

## 用途
把一般 dashboard JSON 或 `raw/` 路徑檔案轉成帶有 `__inputs` 的 Grafana UI prompt JSON。

## 何時使用
當別人給你一般 Grafana dashboard export、legacy raw JSON，或 `raw/` 路徑檔案，而你需要一份能走 Grafana UI `Upload JSON` 流程的 prompt-safe 檔案時，使用這個命令。

## 重點旗標
- `--input-file`：可重複指定一個或多個 dashboard JSON 檔。
- `--input-dir`：轉換整個目錄樹。對 `raw/` 或 export root 而言，預設會寫到旁邊的 `prompt/` 路徑。
- `--output-file`：指定單一輸出檔。只適用於單一 `--input-file`。
- `--output-dir`：把轉換結果寫到一個輸出目錄樹。
- `--overwrite`：覆蓋既有 prompt 檔。
- `--datasource-map`：可選的 JSON/YAML 對照表，用來修補 datasource 參照。
- `--resolution`：可選 `infer-family`、`exact`、`strict`。
- `--profile`、`--url`、`--token`、`--basic-user`、`--basic-password`、`--org-id`：可選的 live datasource lookup 輸入，協助補強 datasource 解析。
- `--dry-run`：只顯示會轉哪些內容，不真的寫檔。
- `--progress`、`--verbose`：顯示進度或更詳細的逐檔輸出。
- `--output-format`：把最後摘要渲染成 `text`、`table`、`json` 或 `yaml`。
- `--log-file`、`--log-format`：把逐項 success/fail 事件寫成文字 log 或 NDJSON。
- `--color`：控制摘要輸出的 `auto`、`always`、`never` 顏色模式。

## 工作流說明
- 單檔模式預設會寫到旁邊的 `*.prompt.json`。
- 多次指定 `--input-file` 時，也會預設各自寫回旁邊的 `*.prompt.json`。
- 一般目錄輸入需要搭配 `--output-dir`，避免生成檔混進任意來源樹。
- `raw/` 或 combined export root 會預設寫到旁邊或新生成的 `prompt/` 路徑，並同時產生 `index.json` 與 `export-metadata.json`。
- `prompt/` 成品是給 Grafana UI `Upload JSON` 用的，不是給 `grafana-util dashboard import` 用的；後者仍然只吃 `raw/` 或 `provisioning/`。
- 如果你提供 `--profile` 或其他 live auth，命令會查詢目標 Grafana 的 datasource inventory，並優先使用這些 live matches 來修補 staged raw inventory。

## Datasource 解析
- `infer-family` 是預設且實務上最常用的模式。它可以從 query shape 修補 Prometheus、Loki、Flux/Influx 等較明確的 family。
- `exact` 需要從嵌入資料、raw inventory 或 `--datasource-map` 找到精確 datasource。
- `exact` 也可以透過可選的 live datasource lookup 成功，只要你提供 `--profile` 或直接 live auth。
- `strict` 則會在任何 datasource 無法精確解析時立刻失敗。
- 若同一份 dashboard 裡有多個可區分的 datasource 參照，命令會保留多個 prompt slot，而不是只按 family 合併。
- 像 generic SQL/search/tracing 這類較模糊的 family，仍然需要更好的原始資料或明確的 `--datasource-map`。

## Placeholder 模型
- `$datasource` 是 dashboard variable 參照，表示 dashboard 或 panel 是透過名為 `datasource` 的 Grafana 變數來選 datasource。
- `${DS_PROMETHEUS}` 或 `${DS_*}` 是 external-import input placeholder，表示 Grafana 在 `Upload JSON` 時要先詢問 datasource，再把結果注入 dashboard。
- 這兩者有關聯，但不是同一件事。生成後的 prompt 檔可以同時包含：
- `__inputs` 裡的 `${DS_*}` 與某些 typed datasource 參照
- panel 層級刻意保留的 `$datasource`
- `raw-to-prompt` 的目標是保留這個邊界，而不是把所有 datasource placeholder 都壓成同一種寫法。
- 如果原始 dashboard 本來就依賴 Grafana datasource variable，轉換後的 prompt 檔仍可能同時看到 `$datasource` 與 `__inputs`。

## 範例
```bash
# Purpose: 把單一 raw dashboard 檔轉成 Grafana UI prompt JSON。
grafana-util migrate dashboard raw-to-prompt --input-file ./dashboards/raw/cpu-main.json
```

```bash
# Purpose: 轉多個 raw dashboard 檔，並顯示進度。
grafana-util migrate dashboard raw-to-prompt --input-file ./legacy/cpu.json --input-file ./legacy/logs.json --progress
```

```bash
# Purpose: 把 raw export tree 轉成旁邊的 prompt/ lane。
grafana-util migrate dashboard raw-to-prompt --input-dir ./dashboards/raw --output-dir ./dashboards/prompt --overwrite
```

```bash
# Purpose: 把 legacy 目錄樹轉成一個明確輸出根，並輸出 table 摘要。
grafana-util migrate dashboard raw-to-prompt --input-dir ./legacy-json --output-dir ./converted/prompt --output-format table
```

```bash
# Purpose: 用明確 mapping 檔修補 datasource 參照。
grafana-util migrate dashboard raw-to-prompt --input-file ./legacy/cpu.json --datasource-map ./datasource-map.yaml --resolution exact --log-file ./raw-to-prompt.log --log-format json
```

```bash
# Purpose: 從保存的 live profile 補強 datasource 修補。
grafana-util migrate dashboard raw-to-prompt --input-file ./legacy/cpu.json --profile prod --org-id 2 --resolution exact
```

## 相關指令
- [migrate](./migrate.md)
- [dashboard export](./dashboard-export.md)
- [dashboard import](./dashboard-import.md)
- [dashboard analyze（本地）](./dashboard-analyze-export.md)
- [dashboard diff](./dashboard-diff.md)
