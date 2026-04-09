# `grafana-util snapshot`

## Root

用途：匯出並檢視 Grafana snapshot inventory bundles。

適用時機：當你想建立一個本機 snapshot root，收錄 dashboard、datasource 與 access inventory，供後續檢視時。

說明：如果你需要一份離線 snapshot，之後不用重新連到 Grafana 也能繼續檢視，先看這一頁最合適。`snapshot` 指令群組適合交接、備份、事件回顧，或任何想先留下本機 artifact 再往下分析的工作流。snapshot export 現在會把 dashboard、datasource、access 幾條 lane 收斂到同一個 root，並寫出 `snapshot-metadata.json`，讓後續工具不用靠猜路徑就能找出 lane。

## 採用前後對照

- **採用前**：snapshot 式檢視通常代表要重新查 Grafana，或一個一個打開 dashboard、datasource 與 access 資料。
- **採用後**：先匯出，再把本機 bundle 當成可重複檢視的 artifact，不用再碰 live server。

## 成功判準

- 你可以把 snapshot root 交給別人，對方不用再跟你要 live 存取也能看
- 匯出結果是可保存的 artifact，不是短命的 UI session
- snapshot root 會帶 lane metadata，後續分析不用重掃整棵目錄猜 shape
- review 輸出清楚到可以接後續分析或事故紀錄

## 失敗時先檢查

- 如果 snapshot export 看起來是空的，先核對 auth profile 或 live 連線，不要先假設來源系統沒資料
- 如果 review 輸出和預期差很多，先確認你指向的是不是正確的 snapshot 目錄
- 如果要交給自動化，請把 `--output-format` 寫清楚，讓下游 parser 知道 contract

主要旗標：root 指令本身只是指令群組；操作旗標都在 `export` 和 `review`。共用的 root 旗標是 `--color`。

範例：

```bash
# 用途：從 live Grafana 匯出本機 snapshot bundle。
grafana-util snapshot export --profile prod --output-dir ./snapshot
```

```bash
# 用途：用 JSON 檢視已匯出的 snapshot bundle。
grafana-util snapshot review --input-dir ./snapshot --output-format json
```

```bash
# 用途：用 token 認證從 live Grafana 匯出 snapshot bundle。
grafana-util snapshot export --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-dir ./snapshot
```

相關指令：`grafana-util overview`、`grafana-util status staged`、`grafana-util change bundle`。

## `export`

用途：將 dashboard、datasource 與 access inventory 匯出到本機 snapshot bundle。

適用時機：當你需要一個不必連到 Grafana 也能檢視的本機 snapshot root 時。

會寫出的內容：

- `snapshot/dashboards/`
- `snapshot/datasources/`
- `snapshot/access/users/`
- `snapshot/access/teams/`
- `snapshot/access/orgs/`
- `snapshot/access/service-accounts/`
- `snapshot/snapshot-metadata.json`

主要旗標：`--output-dir`、`--overwrite`、`--prompt`，以及共用的 Grafana 連線與驗證旗標。`--prompt` 會先開一個 terminal multi-select prompt，讓你在匯出前勾選要包含哪些 lane。datasource lane 會匯出 config 與 `secureJsonDataPlaceholders`，但不會匯出 datasource secret 明文，因為 Grafana live API 本身不會回這些值。

範例：

```bash
# 用途：export。
grafana-util snapshot export --profile prod --output-dir ./snapshot
```

```bash
# 用途：export。
grafana-util snapshot export --url http://localhost:3000 --basic-user admin --basic-password admin --output-dir ./snapshot --overwrite
```

```bash
# 用途：export。
grafana-util snapshot export --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-dir ./snapshot
```

```bash
# 用途：先用 terminal prompt 勾選要匯出的 snapshot lane。
grafana-util snapshot export --profile prod --prompt --output-dir ./snapshot
```

相關指令：`snapshot review`、`change bundle`、`overview`。

## `review`

用途：在不接觸 Grafana 的情況下檢視本機 snapshot inventory。

適用時機：當你想把匯出的 snapshot root 以 table、csv、text、json、yaml 或 interactive 格式查看時。

review summary 現在也會一起顯示 users、teams、orgs、service accounts 的 access 計數。

主要旗標：`--input-dir`、`--interactive`、`--output-format`。

範例：

```bash
# 用途：在不接觸 Grafana 的情況下檢視本機 snapshot inventory。
grafana-util snapshot review --input-dir ./snapshot --output-format table
```

```bash
# 用途：在不接觸 Grafana 的情況下檢視本機 snapshot inventory。
grafana-util snapshot review --input-dir ./snapshot --interactive
```

相關指令：`snapshot export`、`overview`、`status staged`。
