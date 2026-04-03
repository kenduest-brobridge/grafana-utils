# dashboard raw-to-prompt

## 用途
把一般 dashboard JSON 或 `raw/` lane 檔案轉成帶有 `__inputs` 的 Grafana UI prompt JSON。

## 何時使用
當別人給您一般 Grafana dashboard export、legacy raw JSON，或您手上只有 `raw/` lane 檔案，但接下來要走 Grafana UI `Upload JSON` 匯入流程時，使用這個指令。

## 重點旗標
- `--input-file`：可重複指定一個或多個 dashboard JSON 檔。
- `--input-dir`：轉換一整個目錄樹。若輸入是 `raw/` 或 export root，預設輸出到 sibling `prompt/` lane。
- `--output-file`：指定單一輸出檔，只能配單一 `--input-file`。
- `--output-dir`：把轉換結果寫到一個輸出目錄樹。
- `--overwrite`：覆蓋既有 prompt 檔。
- `--datasource-map`：可選的 JSON/YAML mapping，用來修補 datasource 參照。
- `--resolution`：選擇 `infer-family`、`exact` 或 `strict`。
- `--profile`、`--url`、`--token`、`--basic-user`、`--basic-password`、`--org-id`：可選的 live datasource lookup 參數，用來補強 datasource 解析。
- `--dry-run`：只預覽，不寫檔。
- `--progress`、`--verbose`：顯示進度或逐檔詳細結果。
- `--output-format`：最後 summary 用 `text`、`table`、`json` 或 `yaml` 輸出。
- `--log-file`、`--log-format`：把逐檔 success/fail 事件寫成文字 log 或 NDJSON。
- `--color`：用 `auto`、`always` 或 `never` 控制彩色輸出。

## 工作流說明
- 單檔模式預設輸出為同目錄下的 `*.prompt.json`。
- 多個 `--input-file` 也會預設各自輸出到原檔旁邊。
- 一般目錄輸入必須帶 `--output-dir`，避免把生成檔混進任意來源目錄。
- 若輸入是 `raw/` 或 combined export root，預設會生成 sibling `prompt/` lane，並一併寫出 `index.json` 與 `export-metadata.json`。
- `prompt/` 產物是給 Grafana UI `Upload JSON` 用的，不是給 `grafana-util dashboard import` API 匯入路徑直接吃的。
- 如果提供 `--profile` 或其他 live auth 參數，這個命令會先查目標 Grafana 的 datasource inventory，並優先使用 live match。

## Datasource 解析
- 預設 `infer-family` 是實務上最方便的模式，可從 query 形狀推測 Prometheus、Loki、Flux/Influx 這類明確 family。
- `exact` 要求 datasource 必須能從檔案內嵌資訊、raw inventory 或 `--datasource-map` 被精準解析。
- `exact` 也可以搭配 `--profile` 或直接 live auth 參數，透過 live datasource lookup 成功解析。
- `strict` 只要有 datasource 不能精準解析就直接失敗。
- 若同一 dashboard 內可區分出多個 datasource 參照，這個命令會保留多個 prompt slot，不會只因為 family 相同就直接合併。
- 若是泛用 SQL/search/tracing 這類模糊 family，通常仍建議補 `--datasource-map`。

## Placeholder 模型
- `$datasource` 是 dashboard variable 參照，代表 dashboard 或 panel 是透過名為 `datasource` 的 Grafana 變數來選 datasource。
- `${DS_PROMETHEUS}` 或 `${DS_*}` 是 external import input placeholder，代表 Grafana 在 `Upload JSON` 時要先詢問 datasource，再把結果注入 dashboard。
- 這兩者有關聯，但不是同一件事。生成後的 prompt 檔可以同時包含：
  - `__inputs` 內的 `${DS_*}` 與某些 typed datasource 參照
  - panel 層級刻意保留的 `$datasource`
- `raw-to-prompt` 的目標是保留這個邊界，而不是把所有 datasource placeholder 都壓成同一種寫法。
- 如果原始 dashboard 本來就依賴 Grafana datasource variable，轉換後的 prompt 檔仍可能同時看到 `$datasource` 與 `__inputs`。

## 範例
```bash
grafana-util dashboard raw-to-prompt --input-file ./dashboards/raw/cpu-main.json
grafana-util dashboard raw-to-prompt --input-file ./legacy/cpu.json --input-file ./legacy/logs.json --progress
grafana-util dashboard raw-to-prompt --input-dir ./dashboards/raw --overwrite
grafana-util dashboard raw-to-prompt --input-dir ./legacy-json --output-dir ./converted/prompt --output-format table
grafana-util dashboard raw-to-prompt --input-file ./legacy/cpu.json --datasource-map ./datasource-map.yaml --resolution exact --log-file ./raw-to-prompt.log --log-format json
grafana-util dashboard raw-to-prompt --input-file ./legacy/cpu.json --profile prod --org-id 2 --resolution exact
```

## 相關指令
- [dashboard export](./dashboard-export.md)
- [dashboard import](./dashboard-import.md)
- [dashboard inspect-export](./dashboard-inspect-export.md)
- [dashboard diff](./dashboard-diff.md)
