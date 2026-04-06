# dashboard list

## 用途
列出儀表板摘要，不寫入匯出檔案。

## 何時使用
當您需要一個非互動式的線上儀表板清單檢視，且可能想加上解析後的來源或結構化輸出時，使用這個指令。

## 重點旗標
- `--page-size`：儀表板搜尋的每頁筆數。
- `--org-id`：列出指定的 Grafana org。
- `--all-orgs`：彙整所有可見 org 的結果。建議使用 Basic auth。
- `--show-sources`：在清單輸出中加入解析後的 datasource 名稱。`--with-sources` 仍接受為相容別名。
- `--output-columns`：選擇顯示欄位。若選到 `sources` 或 `source_uids`，也會一併解析 datasource 名稱。
- `--output-format`、`--json`、`--yaml`、`--csv`、`--table`、`--text`：輸出模式控制。
- `--no-header`：隱藏表格標頭。

## 說明
- 可重複執行的單一 org 盤點優先用 `--profile`。
- `--all-orgs` 最好搭配管理員憑證支援的 `--profile` 或直接 Basic auth，因為 token 權限可能看不到其他 org。

## 範例
```bash
# 用途：列出儀表板摘要，不寫入匯出檔案。
grafana-util dashboard list --profile prod
```

```bash
# 用途：列出儀表板摘要，不寫入匯出檔案。
grafana-util dashboard list --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --json
```

```bash
# 用途：列出儀表板摘要，不寫入匯出檔案。
grafana-util dashboard list --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --json
```

```bash
# 用途：列出儀表板摘要，並在表格中顯示解析後的來源名稱。
grafana-util dashboard list --url http://localhost:3000 --basic-user admin --basic-password admin --show-sources --table
```

## 相關指令
- [dashboard browse](./dashboard-browse.md)
- [dashboard export](./dashboard-export.md)
- [dashboard diff](./dashboard-diff.md)
