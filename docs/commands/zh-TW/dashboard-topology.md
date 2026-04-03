# dashboard topology

## 用途
從 JSON 成品建立可重現的儀表板拓樸圖。

## 何時使用
當您需要儀表板、資料夾、變數、datasource 連結，以及可選的 alert contract 資料圖形視圖時，使用這個指令。這個指令也接受 `graph` 別名。

## 重點旗標
- `--governance`：儀表板治理 JSON 輸入。
- `--queries`：可選的儀表板 query-report JSON 輸入。
- `--alert-contract`：可選的 alert contract JSON 輸入。
- `--output-format`：輸出 `text`、`json`、`mermaid` 或 `dot`。
- `--output-file`：將渲染後的拓樸寫到磁碟。
- `--interactive`：開啟互動式終端機瀏覽器。

## 範例
```bash
grafana-util dashboard topology --governance ./governance.json --queries ./queries.json --alert-contract ./alert-contract.json --output-format mermaid
grafana-util dashboard graph --governance ./governance.json --queries ./queries.json --alert-contract ./alert-contract.json --output-format dot --output-file ./dashboard-topology.dot
```

## 相關指令
- [dashboard inspect-export](./dashboard-inspect-export.md)
- [dashboard governance-gate](./dashboard-governance-gate.md)
- [dashboard screenshot](./dashboard-screenshot.md)
