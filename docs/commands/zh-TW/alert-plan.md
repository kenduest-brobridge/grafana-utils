# `grafana-util alert plan`

## 目的

根據目標 alert 資源建立一份暫存的 alert 管理計畫。

## 使用時機

- 審閱要讓 Grafana 與目標狀態樹一致所需的變更。
- 在需要時從計畫中刪除僅存在於線上的 alert 資源。
- 在規劃時以 dashboard 或 panel 重新對應方式修復關聯規則。

## 採用前後對照

- **採用前**：告警變更通常要到 apply 前後才看得出全貌。
- **採用後**：同一份 plan 會先把 create、update、delete 意圖與 linked rule 修復選項列清楚，再決定是否進 live mutation。

## 主要旗標

- `--desired-dir` 指向暫存的 alert 目標狀態樹。
- `--prune` 會把僅存在於線上的資源標成刪除候選。
- `--dashboard-uid-map` 與 `--panel-id-map` 用來修復關聯 alert 規則。
- `--output-format` 可將計畫呈現為 `text` 或 `json`。

## 成功判準

- 在 apply 前就能審查 alert 變更內容
- linked dashboard 或 panel 的修復選項會先反映在 plan 裡
- delete 候選是明確列出來的，而不是模糊地混在一起

## 失敗時先檢查

- 如果 plan 少了預期的 rule，先回頭檢查 desired tree
- 如果 linked rule 看起來還是壞的，先確認 dashboard 與 panel mapping 檔
- 如果 `--prune` 看起來太激進，先移掉再比一次，不要直接 apply

## 範例

```bash
# 用途：根據目標 alert 資源建立一份暫存的 alert 管理計畫。
grafana-util alert plan --desired-dir ./alerts/desired
```

```bash
# 用途：根據目標 alert 資源建立一份暫存的 alert 管理計畫。
grafana-util alert plan --desired-dir ./alerts/desired --prune --dashboard-uid-map ./dashboard-map.json --panel-id-map ./panel-map.json --output-format json
```

## 相關命令

- [alert](./alert.md)
- [alert apply](./alert-apply.md)
- [alert delete](./alert-delete.md)
