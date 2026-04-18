# dashboard impact

## 用途
直接從 live Grafana、本地匯出樹，或可重用的 dashboard governance 成品評估單一 datasource 的影響範圍。

## 何時使用
當你準備調整、搬移或排查某個 datasource，想先知道有哪些 dashboard 與 alert 相關資產會被牽動，再動到 live 系統時，就該用這個指令。常見流程請優先用 live 或 local 輸入；只有重用治理成品時才保留 artifact 路徑。

## 最短成功路徑

1. 先拿到你要追的 `datasource uid`，不要只靠顯示名稱。
2. 決定來源是 live、本地匯出樹，還是既有 governance 成品。
3. 先跑一次 `--output-format text` 或 `json`。
4. 若還需要把 alert 一起算進來，再補 `--alert-contract`。

## 先選輸入路徑

- **live Grafana**：你要看現在環境裡某個 datasource 會牽動哪些 dashboard
- **本地匯出樹**：你要做離線 review、搬移前盤點或 CI 檢查
- **governance 成品**：你已經有既有分析成品，只想在後續重跑 impact

如果你還沒有治理或相依性成品，通常先跑 [dashboard summary](./dashboard-summary.md) 或 [dashboard dependencies](./dashboard-dependencies.md)，再回來看這頁。

## 採用前後對照

- **採用前**：datasource 的風險通常只能靠記憶、命名習慣或人工在 Grafana 裡搜尋來猜。
- **採用後**：跑一次 `impact`，就能知道某個 datasource UID 往下會影響哪些 dashboard 與 alert 資產。

## 重點旗標
- `--url`：直接檢視線上 Grafana。
- `--input-dir`：直接檢視本地匯出樹。
- `--input-format`：檢視本地來源時選擇 `raw`、`provisioning` 或 `git-sync`。
- `--governance`：dashboard governance JSON 輸入（`governance-json` 成品）。
- `--datasource-uid`：要追蹤的 datasource UID。
- `--alert-contract`：可選的 alert contract JSON 輸入。
- `--output-format`：輸出 `text` 或 `json`。
- `--interactive`：開啟互動式終端機瀏覽器。

## 先決定你要看什麼結果

- `text`：給人快速看一眼 impact 結果
- `json`：給 review、CI 或外部工具接手
- `interactive`：來源已經確認正確，想現場往下鑽時再開

## 範例（依來源分組）

```bash
# 直接從 live Grafana 看某個 datasource 的影響範圍。
grafana-util dashboard impact \
  --url http://localhost:3000 \
  --basic-user admin \
  --basic-password admin \
  --datasource-uid prom-main \
  --output-format text
```

```bash
# 從本地匯出樹做離線 impact 分析。
grafana-util dashboard impact \
  --input-dir ./dashboards/raw \
  --input-format raw \
  --datasource-uid prom-main \
  --output-format json
```

```bash
# 從 repo-backed Git Sync 樹分析 datasource 牽動面。
grafana-util dashboard impact \
  --input-dir ./grafana-oac-repo \
  --input-format git-sync \
  --datasource-uid prom-main \
  --output-format json
```

```bash
# 直接重用 governance 成品，並把 alert contract 一起納入。
grafana-util dashboard impact \
  --governance ./governance.json \
  --datasource-uid prom-main \
  --alert-contract ./alert-contract.json \
  --output-format json
```

## 結果該怎麼判讀

- 如果 dashboard 名單很多，代表這不是單純局部調整，應先走 review / 搬移計畫
- 如果 dashboard 很少但你原本以為很多，先懷疑來源或 `datasource uid` 對不上
- 如果你有帶 `--alert-contract`，就把 alert 一起視為變更範圍，不要只盯著 dashboard

## 成功判準

- 在改 datasource 之前，就能先叫出會受影響的 dashboard 名單
- 如果有 alert contract，也能在同一份結果裡看到被牽動的 alert 資產
- 結果夠具體，能直接放進 review、搬移計畫或事故交接

## 失敗時先檢查

- 如果結果是空的，先確認 `datasource uid` 是不是和 governance 成品裡一致，而不是只填了你記得的顯示名稱
- 如果少了 alert 相關資產，先確認是否有帶 `--alert-contract`
- 如果 JSON 要交給 CI 或外部工具，先驗證 top-level shape，再判斷「零影響」是否可信

## 相關指令
- [dashboard dependencies](./dashboard-dependencies.md)
- [dashboard policy](./dashboard-policy.md)
