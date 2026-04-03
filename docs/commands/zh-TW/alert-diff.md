# `grafana-util alert diff`

## 目的

比較本機 alert 匯出檔與線上 Grafana 資源的差異。

## 使用時機

- 在匯入或規劃之前，先檢查原始匯出目錄與 Grafana 的差異。
- 將 diff 以純文字或結構化 JSON 呈現。

## 主要旗標

- `--diff-dir` 指向原始匯出目錄。
- `--json` 將 diff 呈現為結構化 JSON。
- `--dashboard-uid-map` 與 `--panel-id-map` 用來在比較時修正關聯的 alert 規則。

## 範例

```bash
grafana-util alert diff --url http://localhost:3000 --diff-dir ./alerts/raw
grafana-util alert diff --url http://localhost:3000 --diff-dir ./alerts/raw --json
```

## 相關命令

- [alert](./alert.md)
- [alert export](./alert-export.md)
- [alert import](./alert-import.md)
