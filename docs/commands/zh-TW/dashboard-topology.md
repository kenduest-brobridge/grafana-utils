# dashboard topology

## 用途
從 JSON 成品建立可重現的儀表板拓樸圖。

## 何時使用
當您需要儀表板、資料夾、變數、datasource 連結，以及可選的 alert contract 資料圖形視圖時，使用這個指令。這個指令也接受 `graph` 別名。

## 採用前後對照

- **採用前**：相依關係通常只存在維護者的記憶、原始 JSON，或是很快過期的臨時圖。
- **採用後**：跑一次 topology，就能得到可重現的相依圖，既能在終端看摘要，也能丟給 Mermaid 或 Graphviz。

## 重點旗標
- `--governance`：儀表板治理 JSON 輸入。
- `--queries`：可選的儀表板 query-report JSON 輸入。
- `--alert-contract`：可選的 alert contract JSON 輸入。
- `--output-format`：輸出 `text`、`json`、`mermaid` 或 `dot`。
- `--output-file`：將渲染後的拓樸寫到磁碟。
- `--interactive`：開啟互動式終端機瀏覽器。

## 範例
```bash
# 用途：從 JSON 成品建立可重現的儀表板拓樸圖。
grafana-util dashboard topology \
  --governance ./governance.json \
  --queries ./queries.json \
  --alert-contract ./alert-contract.json \
  --output-format mermaid
```

```bash
# 用途：從 JSON 成品建立可重現的儀表板拓樸圖。
grafana-util dashboard graph \
  --governance ./governance.json \
  --queries ./queries.json \
  --alert-contract ./alert-contract.json \
  --output-format dot \
  --output-file ./dashboard-topology.dot
```

## 成功判準

- 可以明確指出某次匯出或 live snapshot 牽涉到哪些 dashboard、panel、變數和 datasource 連結
- 同一份 topology 可以同時拿來終端檢查、Mermaid 文件圖和 Graphviz 視覺化，不需要重整資料
- 如果有 alert contract，也能提早看出路由或依賴上的意外影響

## 失敗時先檢查

- 如果圖看起來太空或節點太少，先確認 `governance` 來源是不是正確的匯出樹或 live 環境
- 如果你預期有 alert 邊線卻沒有，先確認是否有帶 `--alert-contract`
- 如果後續視覺化工具讀不進去，先確認你輸出的是 `mermaid`、`dot`、`json` 還是一般 `text`

## 相關指令
- [dashboard inspect-export](./dashboard-inspect-export.md)
- [dashboard governance-gate](./dashboard-governance-gate.md)
- [dashboard screenshot](./dashboard-screenshot.md)
