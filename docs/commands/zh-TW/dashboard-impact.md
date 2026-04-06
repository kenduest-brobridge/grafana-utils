# dashboard impact

## 用途
直接從 live Grafana、本地匯出樹，或可重用的 dashboard governance 成品評估單一 datasource 的影響範圍。

## 何時使用
當你準備調整、搬移或排查某個 datasource，想先知道有哪些 dashboard 與 alert 相關資產會被牽動，再動到 live 系統時，就該用這個指令。常見流程請優先用 live 或 local 輸入；只有重用治理成品時才保留 artifact 路徑。

## 採用前後對照

- **採用前**：datasource 的風險通常只能靠記憶、命名習慣或人工在 Grafana 裡搜尋來猜。
- **採用後**：跑一次 `impact`，就能知道某個 datasource UID 往下會影響哪些 dashboard 與 alert 資產。

## 重點旗標
- `--url`：直接分析線上 Grafana。
- `--import-dir`：直接分析本地匯出樹。
- `--input-format`：分析本地匯出時選擇 `raw` 或 `provisioning`。
- `--governance`：dashboard governance JSON 輸入（`governance-json` 成品）。
- `--datasource-uid`：要追蹤的 datasource UID。
- `--alert-contract`：可選的 alert contract JSON 輸入。
- `--output-format`：輸出 `text` 或 `json`。
- `--interactive`：開啟互動式終端機瀏覽器。

## 範例
```bash
# 用途：直接從 live Grafana 評估單一 datasource 的影響範圍。
grafana-util dashboard impact \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --datasource-uid prom-main \
  --output-format text
```

```bash
# 用途：從本地匯出樹評估單一 datasource 的影響範圍。
grafana-util dashboard impact \
  --import-dir ./dashboards/raw \
  --input-format raw \
  --datasource-uid prom-main \
  --output-format json
```

```bash
# 用途：從可重用的治理成品（`governance-json`）評估單一 datasource 的影響範圍。
grafana-util dashboard impact \
  --governance ./governance.json \
  --datasource-uid prom-main \
  --alert-contract ./alert-contract.json \
  --output-format json
```

## 成功判準

- 在改 datasource 之前，就能先叫出會受影響的 dashboard 名單
- 如果有 alert contract，也能在同一份結果裡看到被牽動的 alert 資產
- 結果夠具體，能直接放進 review、搬移計畫或事故交接

## 失敗時先檢查

- 如果結果是空的，先確認 `datasource uid` 是不是和 governance 成品裡一致，而不是只填了你記得的顯示名稱
- 如果少了 alert 相關資產，先確認是否有帶 `--alert-contract`
- 如果 JSON 要交給 CI 或外部工具，先驗證 top-level shape，再判斷「零影響」是否可信

## 相關指令
- [dashboard analyze（本地）](./dashboard-analyze-export.md)
- [dashboard topology](./dashboard-topology.md)
- [dashboard governance-gate](./dashboard-governance-gate.md)
