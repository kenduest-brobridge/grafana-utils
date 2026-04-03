# `grafana-util alert plan`

## 目的

根據目標 alert 資源建立一份暫存的 alert 管理計畫。

## 使用時機

- 審閱要讓 Grafana 與目標狀態樹一致所需的變更。
- 在需要時從計畫中刪除僅存在於線上的 alert 資源。
- 在規劃時以 dashboard 或 panel 重新對應方式修復關聯規則。

## 主要旗標

- `--desired-dir` 指向暫存的 alert 目標狀態樹。
- `--prune` 會把僅存在於線上的資源標成刪除候選。
- `--dashboard-uid-map` 與 `--panel-id-map` 用來修復關聯 alert 規則。
- `--output` 可將計畫呈現為 `text` 或 `json`。

## 範例

```bash
grafana-util alert plan --desired-dir ./alerts/desired
grafana-util alert plan --desired-dir ./alerts/desired --prune --dashboard-uid-map ./dashboard-map.json --panel-id-map ./panel-map.json --output json
```

## 相關命令

- [alert](./alert.md)
- [alert apply](./alert-apply.md)
- [alert delete](./alert-delete.md)
