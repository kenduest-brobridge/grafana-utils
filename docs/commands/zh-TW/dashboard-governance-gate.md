# dashboard governance-gate

## 用途
直接針對 live Grafana 或本地匯出樹套用治理政策檢查，已保存的分析成品則作為進階重用路徑。

## 何時使用
當您想在推進之前先得到政策通過或失敗結果時，使用這個指令。常見流程應直接用 live 或 local 輸入；只有進階重用與 CI pipeline 才保留 `governance-json` 與 `queries-json`。

## 採用前後對照

- **採用前**：即使已經把 dashboard 匯出並 inspect 完，真正的政策違規還是要靠人一條一條看。
- **採用後**：governance-gate 會把這些成品整理成明確的 pass/fail 結果，也能直接交給 CI 或審核流程。

## 重點旗標
- `--policy-source`：選擇 `file` 或 `builtin`。
- `--policy`：使用檔案型政策輸入時的政策檔路徑。
- `--builtin-policy`：使用內建政策輸入時的名稱。
- `--url`：直接分析線上 Grafana。
- `--input-dir`：直接分析本地匯出樹。
- `--input-format`：分析本地匯出時選擇 `raw` 或 `provisioning`。
- `--governance`：儀表板 inspect governance JSON 路徑（`governance-json` 成品，進階重用）。
- `--queries`：儀表板 inspect query-report JSON 路徑（`queries-json` 成品，進階重用）。
- `--output-format`：輸出文字或 JSON。
- `--json-output`：可選擇輸出正規化後的結果 JSON。
- `--interactive`：在互動式終端機瀏覽器中檢視檢查結果。

## 範例
```bash
# 用途：直接對線上 Grafana 套用治理政策檢查。
grafana-util dashboard governance-gate --url http://localhost:3000 --basic-user admin --basic-password admin --policy-source file --policy ./policy.yaml
```

```bash
# 用途：直接對本地匯出樹套用治理政策檢查。
grafana-util dashboard governance-gate --input-dir ./dashboards/raw --input-format raw --policy-source builtin --builtin-policy default --output-format json --json-output ./governance-check.json
```

```bash
# 用途：進階重用：對可重用的分析成品套用治理政策檢查。
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
- [dashboard analyze（本地）](./dashboard-analyze-export.md)
- [dashboard analyze（即時）](./dashboard-analyze-live.md)
- [dashboard topology](./dashboard-topology.md)
