# `grafana-util alert import`

## 目的

透過 Grafana API 匯入 alert 資源 JSON 檔。

## 使用時機

- 在 Grafana 內重建已匯出的 alert 套件。
- 搭配 `--replace-existing` 更新既有的 alert 資源。
- 在真正變更前先預覽匯入動作。

## 主要旗標

- `--input-dir` 指向 `raw/` 匯出目錄。
- `--replace-existing` 會更新識別相符的資源。
- `--dry-run` 預覽匯入流程。
- `--json` 將 dry-run 輸出呈現為結構化 JSON。
- `--dashboard-uid-map` 與 `--panel-id-map` 用來在匯入時修正關聯的 alert 規則。

## 採用前後對照

- 之前：一筆一筆手動重建 alert 資源，或把 JSON 分開貼回 Grafana。
- 之後：直接匯入整份匯出套件，讓命令自己處理整個資源集。

## 成功判準

- dry-run 輸出看起來跟你預期要匯入的資源一致。
- 實際匯入後，規則、聯絡點、靜音時段、範本與政策都回到預期狀態。
- 如果有 dashboard 連動規則，匯入後的對應也還是對的。

## 失敗時先檢查

- `--input-dir` 要指向 `raw/` 目錄，不要只指到上層匯出資料夾。
- 如果 dashboard 連動規則有搬動，匯入前先補上 dashboard / panel 對應表。
- 需要覆蓋既有資源時，記得加 `--replace-existing`。

## 範例

```bash
# 用途：透過 Grafana API 匯入 alert 資源 JSON 檔。
grafana-util alert import --url http://localhost:3000 --input-dir ./alerts/raw --replace-existing
```

```bash
# 用途：透過 Grafana API 匯入 alert 資源 JSON 檔。
grafana-util alert import --url http://localhost:3000 --input-dir ./alerts/raw --replace-existing --dry-run --json
```

## 相關命令

- [alert](./alert.md)
- [alert export](./alert-export.md)
- [alert diff](./alert-diff.md)
