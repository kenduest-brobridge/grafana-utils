# dashboard review

## 用途
檢視一個本地儀表板 JSON 檔，但不會碰到 Grafana。

## 何時使用
當您想在發佈或匯入之前，先對儀表板草稿做一次本地唯讀檢查時，使用這個指令。

## 重點旗標
- `--input`：要檢視的儀表板 JSON 檔。
- `--output-format`：選擇 `text`、`table`、`csv`、`json` 或 `yaml`。
- `--json`、`--table`、`--csv`、`--yaml`：直接輸出選擇器。

## 範例
```bash
# 用途：檢視一個本地儀表板 JSON 檔，但不會碰到 Grafana。
grafana-util dashboard review --input ./drafts/cpu-main.json
```

```bash
# 用途：檢視一個本地儀表板 JSON 檔，但不會碰到 Grafana。
grafana-util dashboard review --input ./drafts/cpu-main.json --output-format yaml
```

## 相關指令
- [dashboard patch-file](./dashboard-patch-file.md)
- [dashboard publish](./dashboard-publish.md)
- [dashboard inspect-export](./dashboard-inspect-export.md)
