# `grafana-util alert import`

## 目的

透過 Grafana API 匯入 alert 資源 JSON 檔。

## 使用時機

- 在 Grafana 內重建已匯出的 alert 套件。
- 搭配 `--replace-existing` 更新既有的 alert 資源。
- 在真正變更前先預覽匯入動作。

## 主要旗標

- `--import-dir` 指向 `raw/` 匯出目錄。
- `--replace-existing` 會更新識別相符的資源。
- `--dry-run` 預覽匯入流程。
- `--json` 將 dry-run 輸出呈現為結構化 JSON。
- `--dashboard-uid-map` 與 `--panel-id-map` 用來在匯入時修正關聯的 alert 規則。

## 範例

```bash
grafana-util alert import --url http://localhost:3000 --import-dir ./alerts/raw --replace-existing
grafana-util alert import --url http://localhost:3000 --import-dir ./alerts/raw --replace-existing --dry-run --json
```

## 相關命令

- [alert](./alert.md)
- [alert export](./alert-export.md)
- [alert diff](./alert-diff.md)
