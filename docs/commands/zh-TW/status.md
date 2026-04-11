# `grafana-util status`

## Root

用途：透過單一 status surface 讀取 live 與 staged 的 Grafana 狀態。

適用時機：當你想看 readiness、overview、snapshot，或直接讀 live 資料，但還不想進 mutation 流程時。

說明：`status` 是給使用者看的唯讀入口。`live` 用來做即時 gate，`staged` 用來看本機 artifact，`overview` 用來看全域摘要，`snapshot` 用來看 bundle 風格的 review，`resource` 則用來直接讀 live resource。

範例：

```bash
grafana-util status live --profile prod --output-format yaml
grafana-util status staged --desired-file ./desired.json --output-format json
grafana-util status overview --dashboard-export-dir ./dashboards/raw --alert-export-dir ./alerts --output-format table
grafana-util status overview live --url http://localhost:3000 --basic-user admin --basic-password admin --output-format interactive
```

相關指令：`grafana-util export`、`grafana-util workspace`、`grafana-util config profile`。
