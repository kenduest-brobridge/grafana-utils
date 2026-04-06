# `grafana-util overview`

## Root

用途：把已準備好的 artifact 彙整成一份全專案總覽。

適用時機：當你想在查看 status 或推進變更前，先一次看完 dashboard、datasource、access、alert 與 change 相關 artifact 時。

說明：如果你需要先看一份全專案總覽，再決定要切到哪個較窄的工作流，先看這一頁最合適。`overview` 指令群組適合想一次掃過 staged artifact 或 live 狀態的人，不必先把每個資產指令都打開。

主要旗標：分階段輸入，例如 `--dashboard-export-dir`、`--dashboard-provisioning-dir`、`--datasource-export-dir`、`--datasource-provisioning-file`、`--access-user-export-dir`、`--access-team-export-dir`、`--access-org-export-dir`、`--access-service-account-export-dir`、`--desired-file`、`--source-bundle`、`--target-inventory`、`--alert-export-dir`、`--availability-file`、`--mapping-file` 和 `--output-format`。

範例：

```bash
# 用途：彙總 staged 的 dashboard、alert 與 access 產物。
grafana-util overview --dashboard-export-dir ./dashboards/raw --alert-export-dir ./alerts --desired-file ./desired.json --output-format table
```

```bash
# 用途：在 promotion 前先檢視 sync bundle 的輸入。
grafana-util overview --source-bundle ./sync-source-bundle.json --target-inventory ./target-inventory.json --availability-file ./availability.json --mapping-file ./mapping.json --output-format text
```

相關指令：`grafana-util status staged`、`grafana-util change inspect`、`grafana-util snapshot review`。

## `live`

用途：透過共用的 status live 流程，輸出 live overview。

適用時機：當你需要與 `status live` 相同的 live readout，但想從 overview 這個指令群組來操作時。

主要旗標：共用 status live 流程的 live 連線與驗證旗標，以及 `--sync-summary-file`、`--bundle-preflight-file`、`--promotion-summary-file`、`--mapping-file`、`--availability-file` 和 `--output-format`。

說明：
- 可重複執行的 live overview 工作優先用 `--profile`。
- 想拿到較廣 org 可見度時，直接 Basic auth 會更穩定。
- Token 驗證適合權限邊界明確的讀取流程，但最後可見結果仍受 token 權限範圍限制。

範例：

```bash
# 用途：live。
grafana-util overview live --profile prod --output-format yaml
```

```bash
# 用途：live。
grafana-util overview live --url http://localhost:3000 --basic-user admin --basic-password admin --output-format interactive
```

```bash
# 用途：live。
grafana-util overview live --url http://localhost:3000 --token "$GRAFANA_API_TOKEN" --output-format json
```

相關指令：`grafana-util status live`、`grafana-util change apply`、`grafana-util profile show`。
