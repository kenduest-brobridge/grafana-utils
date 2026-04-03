# dashboard governance-gate

## 用途
針對儀表板 inspect JSON 成品套用治理政策檢查。

## 何時使用
當您已經有 `governance-json` 與 query-report 成品，想在推進之前先得到政策通過或失敗結果時，使用這個指令。

## 重點旗標
- `--policy-source`：選擇 `file` 或 `builtin`。
- `--policy`：使用檔案型政策輸入時的政策檔路徑。
- `--builtin-policy`：使用內建政策輸入時的名稱。
- `--governance`：儀表板 inspect governance JSON 路徑。
- `--queries`：儀表板 inspect query-report JSON 路徑。
- `--output-format`：輸出文字或 JSON。
- `--json-output`：可選擇輸出正規化後的結果 JSON。
- `--interactive`：在互動式終端機瀏覽器中檢視發現項目。

## 範例
```bash
grafana-util dashboard governance-gate --policy-source file --policy ./policy.yaml --governance ./governance.json --queries ./queries.json
grafana-util dashboard governance-gate --policy-source builtin --builtin-policy default --governance ./governance.json --queries ./queries.json --output-format json --json-output ./governance-check.json
```

## 相關指令
- [dashboard inspect-export](./dashboard-inspect-export.md)
- [dashboard inspect-live](./dashboard-inspect-live.md)
- [dashboard topology](./dashboard-topology.md)
