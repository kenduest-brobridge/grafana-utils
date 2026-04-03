# `grafana-util overview`

## Root

用途：將已分階段的 artifact 彙總成一份全專案總覽。

適用時機：當你想在查看 status 或推進變更前，先一次看完 dashboard、datasource、access、alert 與 change 相關 artifact 時。

主要旗標：分階段輸入，例如 `--dashboard-export-dir`、`--dashboard-provisioning-dir`、`--datasource-export-dir`、`--datasource-provisioning-file`、`--access-user-export-dir`、`--access-team-export-dir`、`--access-org-export-dir`、`--access-service-account-export-dir`、`--desired-file`、`--source-bundle`、`--target-inventory`、`--alert-export-dir`、`--availability-file`、`--mapping-file` 和 `--output`。

範例：

```bash
grafana-util overview --dashboard-export-dir ./dashboards/raw --alert-export-dir ./alerts --desired-file ./desired.json --output table
grafana-util overview --source-bundle ./sync-source-bundle.json --target-inventory ./target-inventory.json --availability-file ./availability.json --mapping-file ./mapping.json --output text
```

相關指令：`grafana-util status staged`、`grafana-util change summary`、`grafana-util snapshot review`。

## `live`

用途：透過共用的 status live 路徑，渲染 live overview。

適用時機：當你需要與 `status live` 相同的 live readout，但希望從 overview 命名空間來操作時。

主要旗標：共用 status live 路徑的 live 連線與驗證旗標，以及 `--sync-summary-file`、`--bundle-preflight-file`、`--promotion-summary-file`、`--mapping-file`、`--availability-file` 和 `--output`。

說明：
- 可重複執行的 live overview 工作優先用 `--profile`。
- 想拿到較廣 org 可見度時，直接 Basic auth 會更穩定。
- Token 驗證適合權限邊界明確的讀取流程，但最後可見結果仍受 token 權限範圍限制。

範例：

```bash
grafana-util overview live --profile prod --output yaml
grafana-util overview live --url http://localhost:3000 --basic-user admin --basic-password admin --output interactive
grafana-util overview live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output json
```

相關指令：`grafana-util status live`、`grafana-util change apply`、`grafana-util profile show`。
