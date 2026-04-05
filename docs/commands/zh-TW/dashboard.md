# dashboard

## 用途
`grafana-util dashboard` 是處理即時儀表板工作流程、在地草稿管理、匯出 / 匯入檢視、檢查、拓樸、截圖，以及 raw 轉 prompt JSON 的指令群組。這個指令群組也可用 `grafana-util db` 呼叫。

## 何時使用
當你需要瀏覽線上儀表板、抓取或複製線上儀表板成為本地 JSON 草稿、比對本地檔案與 Grafana、檢查匯出或線上中繼資料，或把已準備好的儀表板發佈回 Grafana 時，就會用到這個指令群組。

## 說明
如果你現在處理的是整個 dashboard 工作流，而不是只查某個旗標，先看這一頁最合適。`dashboard` 指令群組把維運時常一起出現的工作整理在同一個入口下，例如盤點、備份與匯出、跨環境搬遷、套用前檢查、live 檢視、拓樸判讀，以及可重現的截圖流程。

對 SRE 或 Grafana 維運人員來說，這頁的作用是先幫你判斷下一步要切去哪個子命令。若你已經知道要做的動作，再從這裡跳到對應子指令頁看精確語法與範例。

## 重點旗標
- `--url`：Grafana 基底網址。
- `--token`、`--basic-user`、`--basic-password`：共用的線上 Grafana 憑證。
- `--profile`：從 `grafana-util.yaml` 載入 repo 本地預設值。
- `--color`：控制這個指令群組的 JSON 彩色輸出。

## 驗證說明
- 可重複執行的日常工作優先用 `--profile`。
- bootstrap 或管理員型流程可直接用 Basic auth。
- `--all-orgs` 這類跨 org 工作流，通常比起 token 更適合使用管理員憑證支援的 `--profile` 或 Basic auth。
- `dashboard raw-to-prompt` 通常是離線流程，但也可選擇用 `--profile` 或 live auth 參數查 datasource inventory，協助修補 prompt 檔。

## 範例
```bash
# 用途：`grafana-util dashboard` 是處理即時儀表板工作流程、在地草稿管理、匯出 / 匯入檢視、檢查、拓樸、截圖，以及 raw 轉 prompt JSON 的指令群組。這個指令群組也可用 `grafana-util db` 呼叫。
grafana-util dashboard --help
```

```bash
# 用途：`grafana-util dashboard` 是處理即時儀表板工作流程、在地草稿管理、匯出 / 匯入檢視、檢查、拓樸、截圖，以及 raw 轉 prompt JSON 的指令群組。這個指令群組也可用 `grafana-util db` 呼叫。
grafana-util dashboard browse --profile prod
```

```bash
# 用途：`grafana-util dashboard` 是處理即時儀表板工作流程、在地草稿管理、匯出 / 匯入檢視、檢查、拓樸、截圖，以及 raw 轉 prompt JSON 的指令群組。這個指令群組也可用 `grafana-util db` 呼叫。
grafana-util dashboard raw-to-prompt --input-file ./legacy/cpu-main.json --profile prod --org-id 2
```

```bash
# 用途：`grafana-util dashboard` 是處理即時儀表板工作流程、在地草稿管理、匯出 / 匯入檢視、檢查、拓樸、截圖，以及 raw 轉 prompt JSON 的指令群組。這個指令群組也可用 `grafana-util db` 呼叫。
grafana-util dashboard inspect-live --url http://localhost:3000 --basic-user admin --basic-password admin --interactive
```

```bash
# 用途：`grafana-util dashboard` 是處理即時儀表板工作流程、在地草稿管理、匯出 / 匯入檢視、檢查、拓樸、截圖，以及 raw 轉 prompt JSON 的指令群組。這個指令群組也可用 `grafana-util db` 呼叫。
grafana-util dashboard inspect-live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format governance-json
```

## 相關指令
- [dashboard browse](./dashboard-browse.md)
- [dashboard get](./dashboard-get.md)
- [dashboard clone-live](./dashboard-clone-live.md)
- [dashboard list](./dashboard-list.md)
- [dashboard export](./dashboard-export.md)
- [dashboard import](./dashboard-import.md)
- [dashboard raw-to-prompt](./dashboard-raw-to-prompt.md)
- [dashboard patch-file](./dashboard-patch-file.md)
- [dashboard review](./dashboard-review.md)
- [dashboard publish](./dashboard-publish.md)
- [dashboard delete](./dashboard-delete.md)
- [dashboard diff](./dashboard-diff.md)
- [dashboard inspect-export](./dashboard-inspect-export.md)
- [dashboard inspect-live](./dashboard-inspect-live.md)
- [dashboard inspect-vars](./dashboard-inspect-vars.md)
- [dashboard governance-gate](./dashboard-governance-gate.md)
- [dashboard topology](./dashboard-topology.md)
- [dashboard screenshot](./dashboard-screenshot.md)
