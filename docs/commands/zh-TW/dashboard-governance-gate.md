# dashboard governance-gate

## 用途
針對儀表板 inspect JSON 成品套用治理政策檢查。

## 何時使用
當您已經有 `governance-json` 與 query-report 成品，想在推進之前先得到政策通過或失敗結果時，使用這個指令。

## 採用前後對照

- **採用前**：即使已經把 dashboard 匯出並 inspect 完，真正的政策違規還是要靠人一條一條看。
- **採用後**：governance-gate 會把這些成品整理成明確的 pass/fail 結果，也能直接交給 CI 或審核流程。

## 重點旗標
- `--policy-source`：選擇 `file` 或 `builtin`。
- `--policy`：使用檔案型政策輸入時的政策檔路徑。
- `--builtin-policy`：使用內建政策輸入時的名稱。
- `--governance`：儀表板 inspect governance JSON 路徑。
- `--queries`：儀表板 inspect query-report JSON 路徑。
- `--output-format`：輸出文字或 JSON。
- `--json-output`：可選擇輸出正規化後的結果 JSON。
- `--interactive`：在互動式終端機瀏覽器中檢視檢查結果。

## 範例
```bash
# 用途：針對儀表板 inspect JSON 成品套用治理政策檢查。
grafana-util dashboard governance-gate --policy-source file --policy ./policy.yaml --governance ./governance.json --queries ./queries.json
```

```bash
# 用途：針對儀表板 inspect JSON 成品套用治理政策檢查。
grafana-util dashboard governance-gate --policy-source builtin --builtin-policy default --governance ./governance.json --queries ./queries.json --output-format json --json-output ./governance-check.json
```

## 成功判準

- 在 promotion 前就能把政策違規擋下來，而不是等 dashboard 上線後才發現
- 文字輸出夠清楚，適合人工檢查；JSON 輸出也夠穩定，適合接進 CI gate
- 換政策後可以重跑同一批 inspect 成品，不必重新 export 或 inspect

## 失敗時先檢查

- 如果命令一開始就失敗，先確認 policy source、policy 檔案路徑或 builtin policy 名稱是否正確
- 如果 gate 結果看起來不完整，先檢查 `governance` 和 `queries` 是否來自同一次 inspect
- 如果自動化要讀結果，建議用 `--output-format json`，並先驗證 contract 再把 pass/fail 當成最終結果

## 相關指令
- [dashboard inspect-export](./dashboard-inspect-export.md)
- [dashboard inspect-live](./dashboard-inspect-live.md)
- [dashboard topology](./dashboard-topology.md)
