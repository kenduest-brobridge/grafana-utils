# dashboard

## 先判斷你現在要做哪一種事

| 你現在想做的事 | 先開哪個命令頁 | 這頁會幫你回答什麼 |
| --- | --- | --- |
| 想先看 live dashboard 現況 | [dashboard browse](./dashboard-browse.md) / [dashboard list](./dashboard-list.md) | 先知道有哪些 dashboard、要不要往下抓內容 |
| 想把 live 或本地內容整理成可重用 review 成品 | [dashboard summary](./dashboard-summary.md) / [dashboard dependencies](./dashboard-dependencies.md) | 先決定來源是 live 還是本地匯出樹 |
| 想知道某個 datasource 會牽動哪些 dashboard 或 alert | [dashboard impact](./dashboard-impact.md) | 先看變更影響面，避免直接動 live |
| 想比對草稿與 live 差異 | [dashboard diff](./dashboard-diff.md) / [dashboard review](./dashboard-review.md) | 先做 review，再決定是否 publish |
| 想先看本地匯出樹套到 live 會改什麼 | [dashboard plan](./dashboard-plan.md) | 先產生 create/update/delete/review hints，再決定是否 import |
| 想做本地草稿與發佈 | [dashboard get](./dashboard-get.md) / [dashboard clone](./dashboard-clone.md) / [dashboard publish](./dashboard-publish.md) | 先進 authoring 路徑，不要直接在 live 上亂改 |
| 想補事故、報告或交接素材 | [dashboard screenshot](./dashboard-screenshot.md) | 先拿可重現的視覺證據 |

## 這個入口是做什麼的

`grafana-util dashboard` 把 dashboard 相關工作收在同一個入口：從瀏覽、草稿、匯出、匯入、比對，到相依性、影響面、政策和截圖。它也可用 `grafana-util db` 呼叫。命令本身維持扁平，help 和文件用分組降低閱讀壓力。

這組頁面採用扁平 command path，例如 `grafana-util dashboard list`、`grafana-util dashboard summary`、`grafana-util dashboard publish`，不再另外包一層 author / migrate / summary namespace。

如果是單一 dashboard 的 authoring 路徑，建議把它想成：
- `dashboard get` 或 `dashboard clone`：先做草稿
- `dashboard serve`：用本地 preview server 持續檢視草稿內容，必要時也能自動打開瀏覽器
- `dashboard review`：先驗證草稿內容
- `dashboard patch`：改寫本地中繼資料
- `dashboard edit-live`：從 live 拉一份進 editor，預設仍先落回本地草稿，而且會依 review 結果決定能不能回寫 live
- `dashboard publish`：沿用 import pipeline 發回 Grafana

`review`、`patch`、`publish` 也都支援 `--input -`，可以直接吃標準輸入的一份 wrapped 或 bare dashboard JSON。這適合外部 generator 已經把 JSON 寫到 stdout 的情況。`patch --input -` 必須搭配 `--output`，若你是在本地反覆編修同一份檔案，則改用 `publish --watch`；它只支援本地檔案路徑，不支援 `--input -`。

## 先選哪一條資料路徑

- **live Grafana**：先用 [dashboard summary](./dashboard-summary.md)、`browse`、`list` 讀現在的環境。
- **本地匯出樹**：先用 [dashboard dependencies](./dashboard-dependencies.md) 做離線分析，再接 `policy`、`impact` 或 `diff`。
- **本地匯出樹對 live**：先用 [dashboard plan](./dashboard-plan.md) 產生 review plan，再決定是否 import 或 prune。
- **單一 dashboard 草稿**：先走 `get` / `clone` / `review` / `publish` 這條草稿路徑。
- **治理成品或 review 成品**：先確認成品格式，再交給 `policy` 或 `impact`，不要把不同輸出格式混用。

## 歷史與還原工作流

如果你的問題不是「現在這份 dashboard 長什麼樣」，而是「哪個舊版本應該被救回來並變成新的最新版本」，就看這一組。

- [dashboard history](./dashboard-history.md)
- `dashboard history list`：列出單一 dashboard UID 的最近版本歷史。
- `dashboard history restore`：把某個歷史版本複製成新的最新 Grafana 版本。
- `dashboard history export`：把歷史版本匯出成可重用的 JSON 成品，方便審查或 CI。

這條路徑最適合要找回已知可用版本，但不想手動重建 dashboard 的情況。

## 這一組頁面怎麼讀比較不會亂

1. 先看這頁，決定你要走哪個子命令。
2. 進到子命令頁後，先讀「何時使用」與「最短成功路徑」。
3. 再看「輸入路徑怎麼選」與「重點旗標」。
4. 最後才看完整範例與相關指令，不要一開始就被所有 flags 淹沒。

## 採用前後對照

- **採用前**：dashboard 動作常分散在 UI、草稿 JSON 與臨時 shell 指令裡，要回頭重跑很麻煩。
- **採用後**：同一條命令群組就能把瀏覽、草稿、檢查、發佈與素材產生串起來。

## 成功判準

- 你能在開始前就判斷這次是要看 live、做草稿、跑檢查，還是直接發佈
- export / summary / diff 的產物能互相對得起來，不會換個步驟就失去上下文
- 需要交給 review 或 CI 時，可以把 dependencies / impact / policy 的結果拿去重跑

## 失敗時先檢查

- 如果 browse 或 summary 結果比預期少，先核對 `--profile`、`--url`、org 與權限
- 如果 dependencies 或 impact 是空的，先確認你餵的是同一次 summary 成品
- 如果政策檢查看起來怪怪的，先看 `governance` 和 `queries` 是否來自相同來源

## 重點旗標

- `--url`：Grafana 基底網址。
- `--token`、`--basic-user`、`--basic-password`：共用的線上 Grafana 憑證。
- `--profile`：從 `grafana-util.yaml` 載入 repo 本地預設值。
- `--color`：控制這個指令群組的 JSON 彩色輸出。

## 範例

```bash
# 先看 dashboard 長什麼樣，再決定下一步要走哪條工作流。
grafana-util dashboard browse --profile prod
```

```bash
# 先盤點現況，再決定要走 browse 或 export。
grafana-util dashboard list --profile prod
```

```bash
# 先做 live 分析，再決定要不要匯出或截圖。
grafana-util dashboard summary --url http://localhost:3000 --basic-user admin --basic-password admin --interactive
```

```bash
# 先產生治理輸出，留給 dependencies 或 policy 接著用。
grafana-util dashboard summary --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format governance
```

```bash
# 先從標準輸入 review 一份生成儀表板，再決定要不要 publish。
jsonnet dashboards/cpu.jsonnet | grafana-util dashboard review --input - --output-format json
```

```bash
# 編修本地草稿時，每次儲存後自動重跑 publish dry-run。
grafana-util dashboard publish --url http://localhost:3000 --basic-user admin --basic-password admin --input ./drafts/cpu-main.json --dry-run --watch
```

```bash
# 開一個本地 preview server，持續檢視單一 dashboard 草稿。
grafana-util dashboard serve --input ./drafts/cpu-main.json --port 18080 --open-browser
```

```bash
# 從 live dashboard 開始編修，但預設先輸出成新的本地草稿。
grafana-util dashboard edit-live --profile prod --dashboard-uid cpu-main --output ./drafts/cpu-main.edited.json
```

## 相關指令

### 瀏覽與檢視

- [dashboard browse](./dashboard-browse.md)
- [dashboard list](./dashboard-list.md)
- [dashboard get](./dashboard-get.md)
- [dashboard variables](./dashboard-variables.md)
- [dashboard history](./dashboard-history.md)

### 匯出與匯入

- [dashboard export](./dashboard-export.md)
- [dashboard import](./dashboard-import.md)
- [dashboard convert raw-to-prompt](./dashboard-convert-raw-to-prompt.md)
- [dashboard convert export-layout](./dashboard-convert-export-layout.md)

### 審查與比對

- [dashboard diff](./dashboard-diff.md)
- [dashboard review](./dashboard-review.md)
- [dashboard summary](./dashboard-summary.md)
- [dashboard dependencies](./dashboard-dependencies.md)
- [dashboard impact](./dashboard-impact.md)
- [dashboard policy](./dashboard-policy.md)

### 編修與發佈

- [dashboard get](./dashboard-get.md)
- [dashboard clone](./dashboard-clone.md)
- [dashboard patch](./dashboard-patch.md)
- [dashboard serve](./dashboard-serve.md)
- [dashboard edit-live](./dashboard-edit-live.md)
- [dashboard publish](./dashboard-publish.md)
- [dashboard delete](./dashboard-delete.md)

### 操作與截圖

- [dashboard screenshot](./dashboard-screenshot.md)
