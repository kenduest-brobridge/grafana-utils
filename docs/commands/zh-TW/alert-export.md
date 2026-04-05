# `grafana-util alert export`

## 目的

將 alert 資源匯出成 `raw/` JSON 檔案。

## 使用時機

- 從 Grafana 擷取 alert 規則、聯絡點、靜音時段、範本與政策。
- 在審閱或匯入前建立本機套件。

## 主要旗標

- `--output-dir` 指定匯出套件的寫入位置，預設為 `alerts`。
- `--flat` 會把資源檔直接寫入各自的資源目錄。
- `--overwrite` 會取代既有的匯出檔。
- 使用 `grafana-util alert` 的共用連線旗標。

## 採用前後對照

- 之前：要從 Grafana UI 一個個把 alert 規則、聯絡點、靜音時段、範本與政策整理出來。
- 之後：一次匯出成可重複使用的 `raw/` 套件，之後可以拿去審閱、比對或匯入。

## 成功判準

- 匯出目錄裡出現預期的 `raw/` 資源檔。
- 套件內容跟你預期要匯出的資源種類一致。
- 這份匯出結果可以直接拿去做 `alert diff` 或 `alert import`。

## 失敗時先檢查

- 先確認連線旗標是不是指到正確的 Grafana 與 org 範圍。
- `--overwrite` 只有在你真的要覆蓋既有匯出時才用。
- 如果匯出內容看起來不完整，先檢查 token 權限或改用範圍更完整的 profile。

## 範例

```bash
# 用途：將 alert 資源匯出成 `raw/` JSON 檔案。
grafana-util alert export --profile prod --output-dir ./alerts --overwrite
```

```bash
# 用途：將 alert 資源匯出成 `raw/` JSON 檔案。
grafana-util alert export --url http://localhost:3000 --basic-user admin --basic-password admin --output-dir ./alerts --flat
```

```bash
# 用途：將 alert 資源匯出成 `raw/` JSON 檔案。
grafana-util alert export --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-dir ./alerts --overwrite
```

## 相關命令

- [alert](./alert.md)
- [alert import](./alert-import.md)
- [alert plan](./alert-plan.md)
