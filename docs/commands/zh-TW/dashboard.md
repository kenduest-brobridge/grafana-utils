# dashboard

## 用途
`grafana-util dashboard` 是處理即時儀表板工作流程、在地草稿管理、匯出/匯入檢視、檢查、拓樸、截圖，以及 raw 轉 prompt JSON 的命名空間。這個命名空間也可用 `grafana-util db` 呼叫。

## 何時使用
當您需要瀏覽線上儀表板、抓取或複製線上儀表板成為本地 JSON 草稿、比對本地檔案與 Grafana、檢查匯出或線上中繼資料，或將已準備好的儀表板發佈回 Grafana 時，使用這個命名空間。

## 重點旗標
- `--url`：Grafana 基底網址。
- `--token`、`--basic-user`、`--basic-password`：共用的線上 Grafana 憑證。
- `--profile`：從 `grafana-util.yaml` 載入 repo 本地預設值。
- `--color`：控制這個命名空間的 JSON 彩色輸出。

## 驗證說明
- 可重複執行的日常工作優先用 `--profile`。
- bootstrap 或管理員型流程可直接用 Basic auth。
- `--all-orgs` 這類跨 org 工作流，通常比起 token 更適合使用管理員憑證支援的 `--profile` 或 Basic auth。
- `dashboard raw-to-prompt` 通常是離線流程，但也可選擇用 `--profile` 或 live auth 參數查 datasource inventory，協助修補 prompt 檔。

## 範例
```bash
grafana-util dashboard --help
grafana-util dashboard browse --profile prod
grafana-util dashboard raw-to-prompt --input-file ./legacy/cpu-main.json --profile prod --org-id 2
grafana-util dashboard inspect-live --url http://localhost:3000 --basic-user admin --basic-password admin --interactive
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
